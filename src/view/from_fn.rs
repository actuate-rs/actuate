use super::{Element, ViewContext};
use crate::{scope::ScopeInner, Scope, Stack, Update, UpdateKind, View};
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};
use tokio::sync::mpsc;

/// Create a component from a function that returns a view.
pub fn from_fn<F, V>(f: F) -> FromFn<F, V>
where
    F: Fn(&Scope) -> V + Send,
    V: View,
{
    FromFn {
        f,
        _marker: PhantomData,
    }
}

struct FnWaker {
    is_ready: Mutex<bool>,
    waker: Waker,
}

impl Wake for FnWaker {
    fn wake(self: Arc<Self>) {
        *self.is_ready.lock().unwrap() = true;
        self.waker.wake_by_ref()
    }
}

struct FnStateInner<V, S> {
    view: V,
    view_cx: ViewContext,
    view_state: S,
    view_waker: Option<Arc<FnWaker>>,
    scope: Scope,
    rx: mpsc::UnboundedReceiver<Update>,
    rx_waker: Option<Arc<FnWaker>>,
    is_body_ready: bool,
    is_rx_ready: bool,
}

pub struct FnState<V, S>(Option<FnStateInner<V, S>>);

impl<V, S> Element for FnState<V, S>
where
    V: View,
    S: Element,
{
    fn remove(&self, stack: &mut dyn Stack) {
        if let Some(ref state) = self.0 {
            state.view_state.remove(stack);
        }
    }
}

pub struct FromFn<F, V> {
    f: F,
    _marker: PhantomData<V>,
}

impl<F, V> View for FromFn<F, V>
where
    F: Fn(&Scope) -> V + Send,
    V: View,
{
    type Element = FnState<V, V::Element>;

    fn build(&self) -> Self::Element {
        FnState(None)
    }

    fn poll_ready(&self, cx: &mut Context, element: &mut Self::Element) -> Poll<()> {
        if let Some(ref mut state) = element.0 {
            let body_ret = {
                let mut is_init = true;
                let wake = state.view_waker.get_or_insert_with(|| {
                    is_init = false;
                    Arc::new(FnWaker {
                        is_ready: Mutex::new(false),
                        waker: cx.waker().clone(),
                    })
                });

                let waker = Waker::from(wake.clone());
                let mut body_cx = Context::from_waker(&waker);

                if !is_init {
                    state.view.poll_ready(&mut body_cx, &mut state.view_state)
                } else if let Some(ref waker) = state.view_waker {
                    let is_ready = *waker.is_ready.lock().unwrap();
                    if is_ready {
                        while state
                            .view
                            .poll_ready(&mut body_cx, &mut state.view_state)
                            .is_ready()
                        {}

                        *waker.is_ready.lock().unwrap() = false;

                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                } else {
                    todo!()
                }
            };

            let rx_ret = {
                let mut is_init = true;
                let wake = state.rx_waker.get_or_insert_with(|| {
                    is_init = false;
                    Arc::new(FnWaker {
                        is_ready: Mutex::new(false),
                        waker: cx.waker().clone(),
                    })
                });

                let waker = Waker::from(wake.clone());
                let mut rx_cx = Context::from_waker(&waker);

                if !is_init {
                    let mut is_ready = false;
                    while let Poll::Ready(Some(update)) = state.rx.poll_recv(&mut rx_cx) {
                        let scope = state.scope.inner.get_mut();
                        if let Some(hook) = scope.hooks.get_mut(update.idx) {
                            match update.kind {
                                UpdateKind::Value(value) => *hook = value,
                            }
                        }
                        is_ready = true;
                    }
                    if is_ready {
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                } else if let Some(ref waker) = state.rx_waker {
                    let is_ready = *waker.is_ready.lock().unwrap();
                    if is_ready {
                        let mut is_poll_ready = false;
                        while let Poll::Ready(Some(update)) = state.rx.poll_recv(&mut rx_cx) {
                            let scope = state.scope.inner.get_mut();
                            if let Some(hook) = scope.hooks.get_mut(update.idx) {
                                match update.kind {
                                    UpdateKind::Value(value) => *hook = value,
                                }
                            }
                            is_poll_ready = true;
                        }

                        *waker.is_ready.lock().unwrap() = false;

                        if is_poll_ready {
                            Poll::Ready(())
                        } else {
                            Poll::Pending
                        }
                    } else {
                        Poll::Pending
                    }
                } else {
                    todo!()
                }
            };

            if body_ret.is_ready() || rx_ret.is_ready() {
                state.is_body_ready = body_ret.is_ready();
                state.is_rx_ready = rx_ret.is_ready();

                Poll::Ready(())
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(())
        }
    }

    fn view(&self, cx: &mut ViewContext, stack: &mut dyn Stack, element: &mut Self::Element) {
        if let Some(ref mut state) = element.0 {
            if state.is_rx_ready {
                let scope = unsafe { &mut *state.scope.inner.get() };
                scope.idx = 0;

                let body = (self.f)(&state.scope);
                state.view = body;
            }

            if state.is_rx_ready || state.is_body_ready {
                state
                    .view
                    .view(&mut state.view_cx, stack, &mut state.view_state);
            }
        } else {
            let mut view_cx = cx.clone();
            let (tx, rx) = mpsc::unbounded_channel();
            let mut scope = Scope {
                tx,
                inner: UnsafeCell::new(ScopeInner {
                    hooks: Vec::new(),
                    idx: 0,
                    contexts: Some(view_cx.clone().contexts),
                }),
            };

            let body = (self.f)(&scope);
            view_cx.contexts = scope.inner.get_mut().contexts.take().unwrap();

            let mut view_state = body.build();
            body.view(&mut view_cx, stack, &mut view_state);

            element.0 = Some(FnStateInner {
                view: body,
                view_cx,
                view_state,
                view_waker: None,
                scope,
                rx,
                rx_waker: None,
                is_body_ready: false,
                is_rx_ready: false,
            })
        }
    }
}
