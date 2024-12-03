use crate::prelude::*;
use alloc::borrow::Cow;
use core::cell::RefCell;

/// Create a new memoized composable.
///
/// The content of the memoized composable is only re-composed when the dependency changes.
///
/// Children of this `Memo` may still be re-composed if their state has changed.
pub fn memo<D, C>(dependency: D, content: C) -> Memo<D, C>
where
    D: Data + Clone + PartialEq + 'static,
    C: Compose,
{
    Memo {
        dependency,
        content,
    }
}

/// Memoized composable.
///
/// See [`memo`] for more.
#[derive(Data)]
#[actuate(path = "crate")]
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct Memo<T, C> {
    dependency: T,
    content: C,
}

impl<T, C> Compose for Memo<T, C>
where
    T: Clone + Data + PartialEq + 'static,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let last = use_ref(&cx, RefCell::default);
        let mut last = last.borrow_mut();
        if let Some(last) = &mut *last {
            if cx.me().dependency != *last {
                *last = cx.me().dependency.clone();
                cx.is_parent_changed.set(true);
            }
        } else {
            *last = Some(cx.me().dependency.clone());
            cx.is_parent_changed.set(true);
        }

        unsafe { Signal::map_unchecked(cx.me(), |me| &me.content) }
    }

    fn name() -> Option<Cow<'static, str>> {
        Some(
            C::name()
                .map(|name| format!("Memo<{}>", name).into())
                .unwrap_or("Memo".into()),
        )
    }
}
