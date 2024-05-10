use crate::scope::AnyClone;
use std::{
    any::TypeId,
    collections::HashMap,
    task::{Context, Poll},
};

pub enum Change {}

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
    fn remove(&self) -> Option<Vec<Change>>;
}

impl Element for () {
    fn remove(&self) -> Option<Vec<Change>> {
        None
    }
}

impl<S: Element> Element for Option<S> {
    fn remove(&self) -> Option<Vec<Change>> {
        if let Some(state) = self {
            state.remove()
        } else {
            None
        }
    }
}

pub trait Node: Send + 'static {
    type Element: Element;

    fn build(&self) -> Self::Element;

    fn poll_ready(
        &self,
        cx: &mut Context,
        element: &mut Self::Element,
        is_changed: bool,
    ) -> Poll<()>;

    fn view(&self, cx: &mut ViewContext, element: &mut Self::Element) -> Option<Vec<Change>>;
}

impl Node for () {
    type Element = ();

    fn build(&self) -> Self::Element {}

    fn poll_ready(
        &self,
        _cx: &mut Context,
        _element: &mut Self::Element,
        _is_changed: bool,
    ) -> Poll<()> {
        Poll::Pending
    }

    fn view(&self, cx: &mut ViewContext, element: &mut Self::Element) -> Option<Vec<Change>> {
        None
    }
}

impl<V: Node> Node for Option<V> {
    type Element = Option<V::Element>;

    fn build(&self) -> Self::Element {
        self.as_ref().map(Node::build)
    }

    fn poll_ready(
        &self,
        cx: &mut Context,
        element: &mut Self::Element,
        is_changed: bool,
    ) -> Poll<()> {
        if let Some(view) = self {
            if let Some(state) = element {
                return view.poll_ready(cx, state, is_changed);
            }
        }

        Poll::Ready(())
    }

    fn view(&self, cx: &mut ViewContext, element: &mut Self::Element) -> Option<Vec<Change>> {
        if let Some(view) = self {
            if let Some(state) = element {
                return view.view(cx, state);
            } else {
                let mut new_state = view.build();
                let changes = view.view(cx, &mut new_state);
                *element = Some(new_state);
                return changes;
            }
        } else if let Some(state) = element {
            return state.remove();
        }

        None
    }
}

pub struct TupleState<S1, S2>(S1, S2, bool);

impl<S1: Element, S2: Element> Element for TupleState<S1, S2> {
    fn remove(&self) -> Option<Vec<Change>> {
        let a = self.0.remove();
        let b = self.1.remove();
        a.map(|mut a| {
            if let Some(b) = b {
                a.extend(b);
            }
            a
        })
    }
}

impl<V1: Node, V2: Node> Node for (V1, V2) {
    type Element = TupleState<V1::Element, V2::Element>;

    fn build(&self) -> Self::Element {
        TupleState(self.0.build(), self.1.build(), false)
    }

    fn poll_ready(
        &self,
        cx: &mut Context,
        element: &mut Self::Element,
        is_changed: bool,
    ) -> Poll<()> {
        loop {
            if element.2 && self.1.poll_ready(cx, &mut element.1, is_changed).is_ready() {
                element.2 = false;
                break Poll::Ready(());
            } else if self.0.poll_ready(cx, &mut element.0, is_changed).is_ready() {
                element.2 = true;
            } else {
                break Poll::Pending;
            }
        }
    }

    fn view(&self, cx: &mut ViewContext, element: &mut Self::Element) -> Option<Vec<Change>> {
        let a = self.0.view(cx, &mut element.0);
        let b = self.1.view(cx, &mut element.1);

        a.map(|mut a| {
            if let Some(b) = b {
                a.extend(b);
            }
            a
        })
    }
}
