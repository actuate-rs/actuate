use crate::{Node, Scope, ScopeContext, ScopeInner, Tree, TreeNode, View};
use slotmap::DefaultKey;
use std::{
    any::TypeId,
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    marker::PhantomData,
    mem, ptr,
    rc::Rc,
};

pub struct ViewNode<V, F, B> {
    pub(crate) view: V,
    pub(crate) body_fn: F,
    pub(crate) _marker: PhantomData<fn() -> B>,
}

impl<V, F, B> Node for ViewNode<V, F, B>
where
    V: View,
    F: Fn(&'static V, &'static Scope) -> B + 'static,
    B: Node,
{
    type State = (B, B::State, DefaultKey);

    fn build(&self, tree: &mut Tree, contexts: &Rc<HashMap<TypeId, ScopeContext>>) -> Self::State {
        let view = unsafe { mem::transmute(&self.view) };

        let key = tree.nodes.insert(TreeNode {
            node: ptr::null::<Self>(),
            scope: None,
            state: ptr::null_mut::<Self::State>(),
        });
        let scope = Scope {
            inner: Rc::new(RefCell::new(ScopeInner {
                key,
                tx: tree.tx.clone(),
                hooks: UnsafeCell::default(),
                hook_idx: 0,
                contexts: contexts.clone(),
            })),
        };
        let scope_ref = unsafe { mem::transmute(&scope) };

        let body = (self.body_fn)(view, scope_ref);
        let body_state = body.build(tree, &scope.inner.borrow().contexts);

        tree.nodes[key].scope = Some(scope);

        (body, body_state, key)
    }

    fn init(&self, tree: &mut Tree, state: &mut Self::State) {
        tree.nodes[state.2].node = self as _;
        tree.nodes[state.2].state = state as _;

        state.0.init(tree, &mut state.1);
    }

    fn rebuild(&self, tree: &mut Tree, state: &mut Self::State) {
        let tree_node = &mut tree.nodes[state.2];
        tree_node.node = self as _;

        let scope = tree_node.scope.as_ref().unwrap();
        scope.inner.borrow_mut().hook_idx = 0;

        let scope_ref = unsafe { mem::transmute(scope) };
        let view = unsafe { mem::transmute(&self.view) };

        let body = (self.body_fn)(view, scope_ref);
        body.rebuild(tree, &mut state.1);
    }
}
