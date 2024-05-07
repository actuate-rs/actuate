use std::{
    any::Any,
    cell::UnsafeCell,
    future,
    marker::PhantomData,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};
use tokio::sync::mpsc;

pub trait View {
    type State;

    fn build(&self) -> Self::State;

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()>;

    fn view(&self, state: &mut Self::State);
}

impl View for () {
    type State = ();

    fn build(&self) -> Self::State {}

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()> {
        Poll::Pending
    }

    fn view(&self, state: &mut Self::State) {}
}

enum UpdateKind {
    Value(Box<dyn Any + Send>),
}

struct Update {
    idx: usize,
    kind: UpdateKind,
}

struct ScopeInner {
    hooks: Vec<Box<dyn Any + Send>>,
    idx: usize,
}

pub struct Scope {
    tx: mpsc::UnboundedSender<Update>,
    inner: UnsafeCell<ScopeInner>,
}

pub struct Setter<T> {
    idx: usize,
    tx: mpsc::UnboundedSender<Update>,
    _marker: PhantomData<T>,
}

impl<T> Setter<T> {
    pub fn set(&self, value: T)
    where
        T: Send + 'static,
    {
        self.tx
            .send(Update {
                idx: self.idx,
                kind: UpdateKind::Value(Box::new(value)),
            })
            .unwrap();
    }
}

pub fn use_state<T: Send + 'static>(cx: &Scope, f: impl FnOnce() -> T) -> (&T, Setter<T>) {
    let scope = unsafe { &mut *cx.inner.get() };
    let idx = scope.idx;
    scope.idx += 1;

    let any = if let Some(any) = scope.hooks.get(idx) {
        any
    } else {
        let scope = unsafe { &mut *cx.inner.get() };
        scope.hooks.push(Box::new(f()));
        scope.hooks.last().unwrap()
    };

    let setter = Setter {
        idx,
        tx: cx.tx.clone(),
        _marker: PhantomData,
    };

    (any.downcast_ref().unwrap(), setter)
}

pub fn from_fn<F, V>(f: F) -> FromFn<F, V>
where
    F: Fn(&Scope) -> V,
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

pub struct FnState<V, S> {
    view: V,
    view_state: S,
    view_waker: Option<Arc<FnWaker>>,
    scope: Scope,
    rx: mpsc::UnboundedReceiver<Update>,
    rx_waker: Option<Arc<FnWaker>>,
}

pub struct FromFn<F, V> {
    f: F,
    _marker: PhantomData<V>,
}

impl<F, V> View for FromFn<F, V>
where
    F: Fn(&Scope) -> V,
    V: View,
{
    type State = Option<FnState<V, V::State>>;

    fn build(&self) -> Self::State {
        None
    }

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()> {
        if let Some(ref mut state) = state {
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
                    let mut is_ready = waker.is_ready.lock().unwrap();
                    if *is_ready {
                        *is_ready = false;

                        while state
                            .view
                            .poll_ready(&mut body_cx, &mut state.view_state)
                            .is_ready()
                        {}

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
                        is_ready = true;
                    }
                    if is_ready {
                        Poll::Ready(())
                    } else {
                        Poll::Pending
                    }
                } else if let Some(ref waker) = state.rx_waker {
                    let mut is_ready = waker.is_ready.lock().unwrap();
                    if *is_ready {
                        *is_ready = false;

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
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        } else {
            Poll::Ready(())
        }
    }

    fn view(&self, state: &mut Self::State) {
        if let Some(ref mut state) = state {
            {
                let scope = unsafe { &mut *state.scope.inner.get() };
                scope.idx = 0
            }

            let body = (self.f)(&state.scope);
            body.view(&mut state.view_state);
            state.view = body;
        } else {
            let (tx, rx) = mpsc::unbounded_channel();
            let scope = Scope {
                tx,
                inner: UnsafeCell::new(ScopeInner {
                    hooks: Vec::new(),
                    idx: 0,
                }),
            };

            let body = (self.f)(&scope);
            let mut view_state = body.build();
            body.view(&mut view_state);

            *state = Some(FnState {
                view: body,
                view_state,
                view_waker: None,
                scope,
                rx,
                rx_waker: None,
            })
        }
    }
}

impl<V1: View, V2: View> View for (V1, V2) {
    type State = (V1::State, V2::State, bool);

    fn build(&self) -> Self::State {
        (self.0.build(), self.1.build(), false)
    }

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()> {
        loop {
            if state.2 && self.1.poll_ready(cx, &mut state.1).is_ready() {
                state.2 = false;
                break Poll::Ready(());
            } else if self.0.poll_ready(cx, &mut state.0).is_ready() {
                state.2 = true;
            } else {
                break Poll::Pending;
            }
        }
    }

    fn view(&self, state: &mut Self::State) {
        self.0.view(&mut state.0);
        self.1.view(&mut state.1);
    }
}

pub async fn run(view: impl View) {
    let mut state = view.build();

    loop {
        future::poll_fn(|cx| view.poll_ready(cx, &mut state)).await;
        view.view(&mut state);
    }
}
