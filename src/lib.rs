use core::fmt;
use std::{fmt::Debug, mem};

pub trait View: Sized + 'static {
    fn body(&self) -> impl View;

    fn into_node(self) -> impl TreeNode {
        Node {
            view: self,
            body_fn: |me: &'static Self| me.body().into_node(),
            body: None,
        }
    }
}

impl View for () {
    fn body(&self) -> impl View {}

    fn into_node(self) -> impl TreeNode {}
}

impl<V1: View, V2: View> View for (V1, V2) {
    fn body(&self) -> impl View {}

    fn into_node(self) -> impl TreeNode {
        (self.0.into_node(), self.1.into_node())
    }
}

pub trait TreeNode: Debug + 'static {
    fn build(&mut self);
}

impl TreeNode for () {
    fn build(&mut self) {}
}

impl<T1: TreeNode, T2: TreeNode> TreeNode for (T1, T2) {
    fn build(&mut self) {
        self.0.build();
        self.1.build();
    }
}

pub struct Node<V, F, B> {
    view: V,
    body_fn: F,
    body: Option<B>,
}

impl<V, F, B: fmt::Debug> fmt::Debug for Node<V, F, B> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut tuple = f.debug_tuple(&std::any::type_name::<V>());

        if let Some(ref body) = self.body {
            tuple.field(body);
        }

        tuple.finish()
    }
}

impl<V, F, B> TreeNode for Node<V, F, B>
where
    V: View,
    F: Fn(&'static V) -> B + 'static,
    B: TreeNode,
{
    fn build(&mut self) {
        let view = unsafe { mem::transmute(&self.view) };
        let mut body = (self.body_fn)(view);

        body.build();

        self.body = Some(body);
    }
}

pub fn run(view: impl View) {
    let mut node = view.into_node();
    node.build();
    dbg!(node);
}
