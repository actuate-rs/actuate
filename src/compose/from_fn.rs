use crate::{compose::Compose, Data, Scope, ScopeState};
use std::marker::PhantomData;

/// Create a composable from a function.
pub fn from_fn<F, C>(f: F) -> FromFn<F, C>
where
    F: Fn(ScopeState) -> C,
    C: Compose,
{
    FromFn {
        f,
        _marker: PhantomData,
    }
}

/// Function composable.
///
/// For more see [`from_fn`].
pub struct FromFn<F, C> {
    f: F,
    _marker: PhantomData<C>,
}

unsafe impl<F, C> Data for FromFn<F, C>
where
    F: Fn(ScopeState) -> C,
    C: Compose,
{
}

impl<F, C> Compose for FromFn<F, C>
where
    F: Fn(ScopeState) -> C,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        (cx.me().f)(&cx)
    }
}
