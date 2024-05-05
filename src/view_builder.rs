use crate::{Scope, Tree, View, ViewTree};

pub trait ViewBuilder {
    fn into_tree(self) -> impl Tree;
}

impl ViewBuilder for () {
    fn into_tree(self) -> impl Tree {}
}

impl<V: View> ViewBuilder for V {
    fn into_tree(self) -> impl Tree {
        ViewTree {
            view: self,
            body: None,
            f: |view: &'static V, cx: &'static Scope| view.body(cx).into_tree(),
        }
    }
}
