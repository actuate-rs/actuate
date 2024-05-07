use crate::{view::ViewContext, VecStack, View};
use std::future;

pub struct VirtualDom<V, S, E> {
    view: V,
    state: S,
    elements: VecStack<E>,
    cx: ViewContext,
}

impl<V, S, E> VirtualDom<V, S, E> {
    pub fn new(view: V) -> Self
    where
        V: View<Element = S>,
    {
        let state = view.build();
        Self {
            view,
            state,
            elements: VecStack {
                items: Vec::new(),
                idx: 0,
            },
            cx: ViewContext::default(),
        }
    }

    pub async fn run(&mut self)
    where
        V: View<Element = S>,
        E: 'static,
    {
        future::poll_fn(|cx| self.view.poll_ready(cx, &mut self.state, false)).await;
        self.view
            .view(&mut self.cx, &mut self.elements, &mut self.state);
    }
}
