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

macro_rules! impl_node_for_tuple {
    ($($t:tt: $idx:tt),*) => {
        impl<$($t: Node),*> Node for ($($t),*) {
            type State = ($($t::State),*);

            fn build(&self, tree: &mut Tree, contexts: &Rc<HashMap<TypeId, ScopeContext>>) -> Self::State {
                ( $( self.$idx.build(tree, contexts) ),* )
            }

            fn init(&self, tree: &mut Tree, state: &mut Self::State) {
                $( self.$idx.init(tree, &mut state.$idx); )*

            }

            fn rebuild(&self, tree: &mut Tree, state: &mut Self::State) {
                $( self.$idx.rebuild(tree, &mut state.$idx); )*
            }
        }
    };
}

impl_node_for_tuple!(N1: 0, N2: 1);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8, N10: 9);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8, N10: 9, N11: 10);
impl_node_for_tuple!(N1: 0, N2: 1, N3: 2, N4: 3, N5: 4, N6: 5, N7: 6, N8: 7, N9: 8, N10: 9, N11: 10, N12: 11);
