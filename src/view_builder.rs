use crate::{Scope, Tree, View, ViewTree};

pub trait ViewBuilder {
    fn into_tree(self) -> impl Tree;
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

impl ViewBuilder for () {
    fn into_tree(self) -> impl Tree {}
}

impl<VB: ViewBuilder> ViewBuilder for Option<VB> {
    fn into_tree(self) -> impl Tree {
        self.map(VB::into_tree)
    }
}

impl<VB1: ViewBuilder, VB2: ViewBuilder> ViewBuilder for (VB1, VB2) {
    fn into_tree(self) -> impl Tree {
        (self.0.into_tree(), self.1.into_tree())
    }
}
