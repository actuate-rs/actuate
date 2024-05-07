use crate::{scope::AnyClone, Stack};
use std::{
    any::TypeId,
    collections::HashMap,
    task::{Context, Poll},
};

mod from_fn;
pub use self::from_fn::{from_fn, FnState, FromFn};

#[derive(Default)]
pub struct ViewContext {
    pub(crate) contexts: HashMap<TypeId, Box<dyn AnyClone>>,
}

impl Clone for ViewContext {
    fn clone(&self) -> Self {
        Self {
            contexts: self
                .contexts
                .iter()
                .map(|(key, any)| (*key, any.clone_any_clone()))
                .collect(),
        }
    }
}

pub trait Element: Send {
    fn remove(&self, stack: &mut dyn Stack);
}

impl Element for () {
    fn remove(&self, _stack: &mut dyn Stack) {}
}

impl<S: Element> Element for Option<S> {
    fn remove(&self, stack: &mut dyn Stack) {
        if let Some(state) = self {
            state.remove(stack)
        }
    }
}

pub trait View: Send {
    type Element: Element;

    fn build(&self) -> Self::Element;

    fn poll_ready(&self, cx: &mut Context, element: &mut Self::Element) -> Poll<()>;

    fn view(&self, cx: &mut ViewContext, stack: &mut dyn Stack, element: &mut Self::Element);
}

impl View for () {
    type Element = ();

    fn build(&self) -> Self::Element {}

    fn poll_ready(&self, _cx: &mut Context, _element: &mut Self::Element) -> Poll<()> {
        Poll::Pending
    }

    fn view(&self, _cx: &mut ViewContext, _stack: &mut dyn Stack, _element: &mut Self::Element) {}
}

impl<V: View> View for Option<V> {
    type Element = Option<V::Element>;

    fn build(&self) -> Self::Element {
        self.as_ref().map(View::build)
    }

    fn poll_ready(&self, cx: &mut Context, element: &mut Self::Element) -> Poll<()> {
        if let Some(view) = self {
            if let Some(state) = element {
                return view.poll_ready(cx, state);
            }
        }

        Poll::Ready(())
    }

    fn view(&self, cx: &mut ViewContext, stack: &mut dyn Stack, element: &mut Self::Element) {
        if let Some(view) = self {
            if let Some(state) = element {
                view.view(cx, stack, state);
            } else {
                let mut new_state = view.build();
                view.view(cx, stack, &mut new_state);
                *element = Some(new_state);
            }
        } else if let Some(state) = element {
            state.remove(stack)
        }
    }
}

pub struct TupleState<S1, S2>(S1, S2, bool);

impl<S1: Element, S2: Element> Element for TupleState<S1, S2> {
    fn remove(&self, stack: &mut dyn Stack) {
        self.0.remove(stack);
        self.1.remove(stack);
    }
}

impl<V1: View, V2: View> View for (V1, V2) {
    type Element = TupleState<V1::Element, V2::Element>;

    fn build(&self) -> Self::Element {
        TupleState(self.0.build(), self.1.build(), false)
    }

    fn poll_ready(&self, cx: &mut Context, element: &mut Self::Element) -> Poll<()> {
        loop {
            if element.2 && self.1.poll_ready(cx, &mut element.1).is_ready() {
                element.2 = false;
                break Poll::Ready(());
            } else if self.0.poll_ready(cx, &mut element.0).is_ready() {
                element.2 = true;
            } else {
                break Poll::Pending;
            }
        }
    }

    fn view(&self, cx: &mut ViewContext, stack: &mut dyn Stack, element: &mut Self::Element) {
        self.0.view(cx, stack, &mut element.0);
        self.1.view(cx, stack, &mut element.1);
    }
}
