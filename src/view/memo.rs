use crate::{node::MemoNode, Node, Scope, View};

pub struct Memo<V> {
    pub(super) view: V,
}

impl<V: View + PartialEq + Clone> View for Memo<V> {
    fn body(&self, _cx: &Scope) -> impl View {}

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
        MemoNode {
            view: self.view.clone(),
            node: self.view.into_node(),
        }
    }
}
