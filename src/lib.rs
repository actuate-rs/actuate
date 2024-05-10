use std::mem;

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

pub trait TreeNode: 'static {
    fn build(&mut self);
}

impl TreeNode for () {
    fn build(&mut self) {}
}

pub struct Node<V, F, B> {
    view: V,
    body_fn: F,
    body: Option<B>,
}

impl<V, F, B> TreeNode for Node<V, F, B>
where
    V: View,
    F: Fn(&'static V) -> B + 'static,
    B: 'static,
{
    fn build(&mut self) {
        let view = unsafe { mem::transmute(&self.view) };
        let body = (self.body_fn)(view);
        self.body = Some(body);
    }
}

pub fn run(view: impl View) {
    let mut node = view.into_node();
    node.build();
}
