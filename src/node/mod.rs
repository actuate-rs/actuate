use crate::{scope::AnyClone, view::FnWaker};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
    task::{Context, Poll},
};

pub enum Change {
    Push(Box<dyn Any + Send>),
}

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

    fn view(&self, _cx: &mut ViewContext, _element: &mut Self::Element) -> Option<Vec<Change>> {
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

pub struct TupleElement<T>(T);

pub struct TupleNode<T> {
    node: T,
    waker: Option<Arc<FnWaker>>,
}

macro_rules! impl_node_for_tuple {
    ($($t:tt: $idx:tt),*) => {

        impl<$($t: Element),*> Element for TupleElement<($(TupleNode<$t>),*)> {
            fn remove(&self) -> Option<Vec<Change>> {
                let mut changes = Vec::new();

                $(
                    if let Some(new) = self.0.$idx.node.remove() {
                        changes.extend(new);
                    }
                )*

                Some(changes)
            }
        }

        impl<$($t: Node),*> Node for ($($t),*) {
            type Element = TupleElement<($(TupleNode<$t::Element>),*)>;

            fn build(&self) -> Self::Element {
                TupleElement(($( TupleNode { node: self.$idx.build(), waker: None} ),*))
            }

            fn poll_ready(
                &self,
                cx: &mut Context,
                element: &mut Self::Element,
                is_changed: bool,
            ) -> Poll<()> {
                let polls = [ $(
                    if let Some(ref fn_waker) = element.0.$idx.waker {
                        if *fn_waker.is_ready.lock().unwrap() {
                            let waker = std::task::Waker::from(fn_waker.clone());
                            let mut child_cx = Context::from_waker(&waker);

                            self.$idx.poll_ready(&mut child_cx, &mut element.0.$idx.node, is_changed)
                        } else {
                            Poll::Pending
                        }
                    } else {
                        let waker = Arc::new(FnWaker {
                            waker: cx.waker().clone(),
                            is_ready: std::sync::Mutex::new(false)
                        });
                        element.0.$idx.waker = Some(waker.clone());

                        let waker = std::task::Waker::from(waker);
                        let mut child_cx = Context::from_waker(&waker);
                        self.$idx.poll_ready(&mut child_cx, &mut element.0.$idx.node, is_changed)
                    }
                ),* ];

                if polls.iter().any(Poll::is_ready) {
                    Poll::Ready(())
                } else {
                    Poll::Pending
                }
            }

            fn view(
                &self,
                cx: &mut ViewContext,
                element: &mut Self::Element,
            ) -> Option<Vec<Change>> {
                let mut changes = Vec::new();

                $(
                    if let Some(new) = self.$idx.view(cx, &mut element.0.$idx.node) {
                        changes.extend(new);
                    }
                )*

                Some(changes)
            }
        }
    };
}

impl_node_for_tuple!(N1: 0, N2: 1, N3: 2);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8, N10: 9);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8, N10: 9, N11: 10);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8, N10: 9, N11: 10, N12: 11);
