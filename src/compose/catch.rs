use super::CatchContext;
use crate::{compose::Compose, data::Data, use_provider, Scope, Signal};
use core::{error::Error as StdError, mem};

/// Create a composable that catches errors from its children.
///
/// If a child returns a `Result<T, actuate::Error>`,
/// any errors will be caught by this composable by calling `on_error`.
///
/// # Examples
///
/// ```no_run
/// use actuate::prelude::*;
///
/// #[derive(Data)]
/// struct A;
///
/// impl Compose for A {
///     fn compose(_cx: Scope<Self>) -> impl Compose {
///         let _: i32 = "".parse().map_err(Error::new)?;
///
///         Ok(())
///     }
/// }
///
/// #[derive(Data)]
/// struct App;
///
/// impl Compose for App {
///     fn compose(_cx: Scope<Self>) -> impl Compose {
///         catch(
///             |error| {
///                 dbg!(error);
///             },
///             A,
///         )
///     }
/// }
/// ```
pub fn catch<'a, C: Compose>(
    on_error: impl Fn(Box<dyn StdError>) + 'a,
    content: C,
) -> Catch<'a, C> {
    Catch {
        content,
        f: Box::new(on_error),
    }
}

/// Error catch composable.
///
/// See [`catch`] for more.
#[derive(Data)]
#[actuate(path = "crate")]
pub struct Catch<'a, C> {
    content: C,
    f: Box<dyn Fn(Box<dyn StdError>) + 'a>,
}

impl<C: Compose> Compose for Catch<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let f: &dyn Fn(Box<dyn StdError>) = &*cx.me().f;
        let f: &dyn Fn(Box<dyn StdError>) = unsafe { mem::transmute(f) };
        use_provider(&cx, move || CatchContext { f: Box::new(f) });

        unsafe { Signal::map_unchecked(cx.me(), |me| &me.content) }
    }
}
