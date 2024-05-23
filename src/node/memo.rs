use crate::{Node, ScopeContext, Tree, View};
use std::{any::TypeId, collections::HashMap, rc::Rc};

pub struct MemoNode<V, N> {
    pub(crate) view: V,
    pub(crate) node: N,
}

impl<V: View + PartialEq + Clone, N: Node> Node for MemoNode<V, N> {
    type State = (V, N::State);

    fn build(&self, tree: &mut Tree, contexts: &Rc<HashMap<TypeId, ScopeContext>>) -> Self::State {
        (self.view.clone(), self.node.build(tree, contexts))
    }

    fn init(&self, tree: &mut Tree, state: &mut Self::State) {
        self.node.init(tree, &mut state.1)
    }

    fn rebuild(&self, tree: &mut Tree, state: &mut Self::State) {
        if self.view != state.0 {
            state.0 = self.view.clone();

            self.node.rebuild(tree, &mut state.1)
        }
    }
}
