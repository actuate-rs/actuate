use std::task::{Context, Poll};

mod from_fn;
pub use self::from_fn::{from_fn, FnState, FromFn};

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
