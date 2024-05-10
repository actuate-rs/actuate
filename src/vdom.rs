use crate::{node::ViewContext, Node};
use std::future;

pub struct VirtualDom<V, S> {
    view: V,
    state: S,
    cx: ViewContext,
}

impl<V, S> VirtualDom<V, S> {
    pub fn new(view: V) -> Self
    where
        V: Node<Element = S>,
    {
        let state = view.build();
        Self {
            view,
            state,

            cx: ViewContext::default(),
        }
    }

    pub async fn run(&mut self)
    where
        V: Node<Element = S>,
    {
        future::poll_fn(|cx| self.view.poll_ready(cx, &mut self.state, false)).await;
        self.view.view(&mut self.cx, &mut self.state);
    }
}
