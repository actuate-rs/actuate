mod memo;
use std::any::TypeId;
use std::collections::HashMap;
use std::rc::Rc;

use crate::{ScopeContext, Tree};

pub use self::memo::MemoNode;

mod view;
pub use self::view::ViewNode;

pub trait Node: 'static {
    type State;

    fn build(&self, tree: &mut Tree, contexts: &Rc<HashMap<TypeId, ScopeContext>>) -> Self::State;

    fn init(&self, tree: &mut Tree, state: &mut Self::State);

    fn rebuild(&self, tree: &mut Tree, state: &mut Self::State);
}

impl Node for () {
    type State = ();

    fn build(
        &self,
        _tree: &mut Tree,
        _contexts: &Rc<HashMap<TypeId, ScopeContext>>,
    ) -> Self::State {
    }

    fn init(&self, _tree: &mut Tree, _statee: &mut Self::State) {}

    fn rebuild(&self, _tree: &mut Tree, _state: &mut Self::State) {}
}
