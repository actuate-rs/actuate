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

macro_rules! impl_view_for_tuple {
    ($($t:tt: $idx:tt),*) => {
        impl<$($t: View),*> View for ($($t),*) {
            fn body(&self, _cx: &Scope) -> impl View {}

            fn into_node(self) -> impl Node {
                ($(self.$idx.into_node()),*)
            }
        }
    };
}

impl_view_for_tuple!(V1: 0, V2: 1);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5, V7: 6);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5, V7: 6, V8: 7);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5, V7: 6, V8: 7, V9: 8);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5, V7: 6, V8: 7, V9: 8, V10: 9);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5, V7: 6, V8: 7, V9: 8, V10: 9, V11: 10);
impl_view_for_tuple!(V1: 0, V2: 1, V3: 2, V4: 3, V5: 4, V6: 5, V7: 6, V8: 7, V9: 8, V10: 9, V11: 10, V12: 11);
