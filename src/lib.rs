use slotmap::{DefaultKey, SlotMap};
use std::{marker::PhantomData, mem};

pub trait View: 'static {
    fn body(&self) -> impl View;

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
        ViewNode {
            view: self,
            body_fn: |me: &'static Self| me.body().into_node(),
            _marker: PhantomData,
        }
    }
}

impl View for () {
    fn body(&self) -> impl View {}

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
    }
}

#[derive(Default)]
pub struct Tree {
    nodes: SlotMap<DefaultKey, *const dyn AnyNode>,
}

pub trait AnyNode {}

impl<T: Node> AnyNode for T {}

pub trait Node: 'static {
    type State;

    fn build(&self) -> Self::State;

    fn init(&self, tree: &mut Tree, state: &mut Self::State);
}

impl Node for () {
    type State = ();

    fn build(&self) -> Self::State {}

    fn init(&self, tree: &mut Tree, state: &mut Self::State) {}
}

pub struct ViewNode<V, F, B> {
    view: V,
    body_fn: F,
    _marker: PhantomData<fn() -> B>,
}

impl<V, F, B> Node for ViewNode<V, F, B>
where
    V: View,
    F: Fn(&'static V) -> B + 'static,
    B: Node,
{
    type State = (B, B::State, Option<DefaultKey>);

    fn build(&self) -> Self::State {
        let view = unsafe { mem::transmute(&self.view) };
        let body = (self.body_fn)(view);
        let body_state = body.build();
        (body, body_state, None)
    }

    fn init(&self, tree: &mut Tree, state: &mut Self::State) {
        let key = tree.nodes.insert(self as _);
        state.2 = Some(key);

        state.0.init(tree, &mut state.1);
    }
}

pub fn run(view: impl View) {
    let mut tree = Tree::default();

    let node = view.into_node();
    let mut state = node.build();
    node.init(&mut tree, &mut state);
}
