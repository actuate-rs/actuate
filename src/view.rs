use crate::{
    node::{ViewNode, WrapNode},
    Node, Scope,
};
use std::marker::PhantomData;

pub trait View: Send + Sized + 'static {
    fn body(&self, cx: &Scope) -> impl View;

    fn into_node(self) -> impl Node {
        ViewNode {
            view: self,
            f: |me: &'static Self, cx: &'static Scope| WrapNode(me.body(cx).into_node()),
            _marker: PhantomData,
        }
    }
}

impl View for () {
    fn body(&self, _cx: &Scope) -> impl View {}

    fn into_node(self) -> impl Node {}
}

macro_rules! impl_view_for_tuple {
    ($($t: tt),*) => {
        impl<$($t: View),*> View for ($($t),*) {
            fn body(&self, _cx: &Scope) -> impl View {}

            fn into_node(self) -> impl Node {
                #[allow(non_snake_case)]
                let ($($t),*) =  self;
                ($( $t.into_node() ),*)
            }
        }
    };
}

impl_view_for_tuple!(V1, V2);
impl_view_for_tuple!(V1, V2, V3);
impl_view_for_tuple!(V1, V2, V3, V4);
impl_view_for_tuple!(V1, V2, V3, V4, V5);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6, V7);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6, V7, V8);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6, V7, V8, V9);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6, V7, V8, V9, V10);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11);
impl_view_for_tuple!(V1, V2, V3, V4, V5, V6, V7, V8, V9, V10, V11, V12);
