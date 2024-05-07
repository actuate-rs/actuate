use crate::Stack;
use std::task::{Context, Poll};

mod from_fn;
pub use self::from_fn::{from_fn, FnState, FromFn};

pub trait State: Send {
    fn remove(&self, stack: &mut dyn Stack);
}

impl State for () {
    fn remove(&self, stack: &mut dyn Stack) {}
}

impl<S: State> State for Option<S> {
    fn remove(&self, stack: &mut dyn Stack) {
        if let Some(state) = self {
            state.remove(stack)
        }
    }
}

pub trait View: Send {
    type State: State;

    fn build(&self) -> Self::State;

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()>;

    fn view(&self, stack: &mut dyn Stack, state: &mut Self::State);
}

impl View for () {
    type State = ();

    fn build(&self) -> Self::State {}

    fn poll_ready(&self, _cx: &mut Context, _state: &mut Self::State) -> Poll<()> {
        Poll::Pending
    }

    fn view(&self, _stack: &mut dyn Stack, _state: &mut Self::State) {}
}

impl<V: View> View for Option<V> {
    type State = Option<V::State>;

    fn build(&self) -> Self::State {
        self.as_ref().map(View::build)
    }

    fn poll_ready(&self, cx: &mut Context, state: &mut Self::State) -> Poll<()> {
        if let Some(view) = self {
            if let Some(state) = state {
                return view.poll_ready(cx, state);
            }
        }

        Poll::Ready(())
    }

    fn view(&self, stack: &mut dyn Stack, state: &mut Self::State) {
        if let Some(view) = self {
            if let Some(state) = state {
                view.view(stack, state);
            } else {
                let mut new_state = view.build();
                view.view(stack, &mut new_state);
                *state = Some(new_state);
            }
        } else if let Some(state) = state {
            state.remove(stack)
        }
    }
}

pub struct TupleState<S1, S2>(S1, S2, bool);

impl<S1: State, S2: State> State for TupleState<S1, S2> {
    fn remove(&self, stack: &mut dyn Stack) {
        self.0.remove(stack);
        self.1.remove(stack);
    }
}

impl<V1: View, V2: View> View for (V1, V2) {
    type State = TupleState<V1::State, V2::State>;

    fn build(&self) -> Self::State {
        TupleState(self.0.build(), self.1.build(), false)
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

    fn view(&self, stack: &mut dyn Stack, state: &mut Self::State) {
        self.0.view(stack, &mut state.0);
        self.1.view(stack, &mut state.1);
    }
}
