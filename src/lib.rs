use std::{
    future,
    marker::PhantomData,
    sync::{Arc, Mutex},
    task::{Context, Poll, Wake, Waker},
};

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

pub fn from_fn<F, V>(f: F) -> FromFn<F, V>
where
    F: Fn() -> V,
    V: View,
{
    FromFn {
        f,
        _marker: PhantomData,
    }
}

pub struct FnWaker {
    is_ready: Mutex<bool>,
    waker: Waker,
}

impl Wake for FnWaker {
    fn wake(self: Arc<Self>) {
        *self.is_ready.lock().unwrap() = true;
        self.waker.wake_by_ref()
    }
}

pub struct FromFn<F, V> {
    f: F,
    _marker: PhantomData<V>,
}

impl<F, V> View for FromFn<F, V>
where
    F: Fn() -> V,
    V: View,
{
    type State = Option<(V, V::State, Option<Arc<FnWaker>>)>;

    fn build(&self) -> Self::State {
        None
    }

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()> {
        if let Some((ref mut last, ref mut body_state, ref mut waker_cell)) = state {
            let mut is_init = true;
            let wake = waker_cell.get_or_insert_with(|| {
                is_init = false;
                Arc::new(FnWaker {
                    is_ready: Mutex::new(false),
                    waker: cx.waker().clone(),
                })
            });

            let waker = Waker::from(wake.clone());
            let mut body_cx = Context::from_waker(&waker);

            if !is_init {
                last.poll_ready(&mut body_cx, body_state)
            } else if let Some(ref waker) = waker_cell {
                let mut is_ready = waker.is_ready.lock().unwrap();
                if *is_ready {
                    while last.poll_ready(&mut body_cx, body_state).is_ready() {}
                    *is_ready = false;
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            } else {
                todo!()
            }
        } else {
            Poll::Ready(())
        }
    }

    fn view(&self, state: &mut Self::State) {
        let body = (self.f)();

        if let Some((ref mut last, ref mut state, _)) = state {
            body.view(state);
            *last = body;
        } else {
            let mut body_state = body.build();
            body.view(&mut body_state);
            *state = Some((body, body_state, None))
        }
    }
}

pub async fn run(view: impl View) {
    let mut state = view.build();

    loop {
        future::poll_fn(|cx| view.poll_ready(cx, &mut state)).await;
        view.view(&mut state);
    }
}
