use crate::{node::ViewNode, Node, Scope};
use std::marker::PhantomData;

mod memo;
pub use self::memo::Memo;

pub trait View: 'static {
    fn body(&self, cx: &Scope) -> impl View;

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
        ViewNode {
            view: self,
            body_fn: |me: &'static Self, cx: &'static Scope| me.body(cx).into_node(),
            _marker: PhantomData,
        }
    }

    fn memo(self) -> Memo<Self>
    where
        Self: PartialEq + Clone,
    {
        Memo { view: self }
    }
}

impl View for () {
    fn body(&self, _cx: &Scope) -> impl View {}

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
    }
}
