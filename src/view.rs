use crate::{
    node::{Element, ViewContext},
    scope::ScopeInner,
    Node, Scope, Stack, Update, UpdateKind,
};
use std::{
    cell::UnsafeCell,
    marker::PhantomData,
    mem,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};
use tokio::sync::mpsc;

pub trait View: Send + Sized + 'static {
    fn body(&self, cx: &Scope) -> impl View;

    fn into_node(self) -> impl Node {
        ViewNode {
            view: self,
            f: |me: &'static Self, cx: &'static Scope| me.body(cx).into_node(),
            _marker: PhantomData,
        }
    }
}

impl View for () {
    fn body(&self, cx: &Scope) -> impl View {}

    fn into_node(self) -> impl Node {}
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
    V: Node,
    S: Element,
{
    fn remove(&self, stack: &mut dyn Stack) {
        if let Some(ref state) = self.0 {
            state.view_state.remove(stack);
        }
    }
}

pub struct ViewNode<V, F, B> {
    view: V,
    f: F,
    _marker: PhantomData<B>,
}

impl<V, F, B> Node for ViewNode<V, F, B>
where
    V: View,
    F: Fn(&'static V, &'static Scope) -> B + Send + 'static,
    B: Node,
{
    type Element = FnState<B, B::Element>;

    fn build(&self) -> Self::Element {
        FnState(None)
    }

    fn poll_ready(
        &self,
        cx: &mut Context,
        element: &mut Self::Element,
        is_changed: bool,
    ) -> Poll<()> {
        if let Some(ref mut state) = element.0 {
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
                    state.view.poll_ready(
                        &mut body_cx,
                        &mut state.view_state,
                        is_changed || rx_ret.is_ready(),
                    )
                } else if let Some(ref waker) = state.view_waker {
                    let is_ready = *waker.is_ready.lock().unwrap();
                    if is_ready {
                        while state
                            .view
                            .poll_ready(
                                &mut body_cx,
                                &mut state.view_state,
                                is_changed || rx_ret.is_ready(),
                            )
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

            if is_changed || body_ret.is_ready() || rx_ret.is_ready() {
                state.is_body_ready = body_ret.is_ready();
                state.is_rx_ready = rx_ret.is_ready() || is_changed;

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

                let view = unsafe { mem::transmute(&self.view) };
                let scope = unsafe { mem::transmute(&state.scope) };

                let body = (self.f)(view, scope);
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

            let view = unsafe { mem::transmute(&self.view) };
            let scope_ref = unsafe { mem::transmute(&scope) };

            let body = (self.f)(view, scope_ref);
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
