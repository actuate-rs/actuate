use crate::{prelude::*, ScopeData};
use alloc::borrow::Cow;
use core::{
    any::TypeId,
    cell::{RefCell, UnsafeCell},
    error::Error as StdError,
    mem,
};

mod catch;
pub use self::catch::{catch, Catch};

mod dyn_compose;
pub use self::dyn_compose::{dyn_compose, DynCompose};

mod from_iter;
pub use self::from_iter::{from_iter, FromIter};

mod memo;
pub use self::memo::{memo, Memo};

/// A composable function.
///
/// For a dynamically-typed composable, see [`DynCompose`].
///
/// Composables are the building blocks of reactivity in Actuate.
/// A composable is essentially a function that is re-run whenever its state (or its parent state) is changed.
/// Composables may return one or more children, that run after their parent.
///
/// When a composable is re-run, we call that "recomposition".
/// For example, on the initial composition, hooks may initialize their state.
/// Then on recomposition, hooks update their state from the last set value.
///
/// Triggering a state update will recompose each parent, and then each child,
/// until either a [`Memo`] is reached or the composition is complete.
///
/// [`Memo`] is special in that it will only recompose in two cases:
/// 1. It's provided dependencies have changed (see [`memo`] for more)
/// 2. Its own state has changed, which will then trigger the above parent-to-child process for its children.
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub trait Compose: Data {
    /// Compose this function.
    fn compose(cx: Scope<Self>) -> impl Compose;

    #[doc(hidden)]
    fn name() -> Option<Cow<'static, str>> {
        let name = core::any::type_name::<Self>();
        Some(
            name.split('<')
                .next()
                .unwrap_or(name)
                .split("::")
                .last()
                .unwrap_or(name)
                .into(),
        )
    }
}

impl Compose for () {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let _ = cx;
    }
}

impl<C: Compose> Compose for &C {
    fn compose(cx: Scope<Self>) -> impl Compose {
        unsafe {
            (**cx.me()).any_compose(&cx);
        }
    }
}

impl<C: Compose> Compose for Option<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let state_cell: &RefCell<Option<ScopeData>> = use_ref(&cx, || RefCell::new(None));
        let mut state_cell = state_cell.borrow_mut();

        if let Some(content) = &*cx.me() {
            if let Some(state) = &*state_cell {
                state.is_parent_changed.set(cx.is_parent_changed.get());
                unsafe {
                    content.any_compose(state);
                }
            } else {
                let mut state = ScopeData::default();
                state.contexts = cx.contexts.clone();
                *state_cell = Some(state);
                unsafe {
                    content.any_compose(state_cell.as_ref().unwrap());
                }
            }
        } else {
            *state_cell = None;
        }
    }
}

/// Composable error.
///
/// This can be handled by a parent composable with [`Catch`].
#[derive(Data)]
pub struct Error {
    make_error: Box<dyn Fn() -> Box<dyn core::error::Error>>,
}

impl Error {
    /// Create a new composable error.
    pub fn new(error: impl StdError + Clone + 'static) -> Self {
        Self {
            make_error: Box::new(move || Box::new(error.clone())),
        }
    }
}

impl<C: Compose> Compose for Result<C, Error> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let catch_cx = use_context::<CatchContext>(&cx).unwrap();

        cx.is_container.set(true);

        let state_cell: &RefCell<Option<ScopeData>> = use_ref(&cx, || RefCell::new(None));
        let mut state_cell = state_cell.borrow_mut();

        match &*cx.me() {
            Ok(content) => {
                if let Some(state) = &*state_cell {
                    state.is_parent_changed.set(cx.is_parent_changed.get());
                    unsafe {
                        content.any_compose(state);
                    }
                } else {
                    let mut state = ScopeData::default();
                    state.contexts = cx.contexts.clone();
                    *state_cell = Some(state);
                    unsafe {
                        content.any_compose(state_cell.as_ref().unwrap());
                    }
                }
            }
            Err(error) => {
                *state_cell = None;

                (catch_cx.f)((error.make_error)());
            }
        }
    }
}

pub(crate) struct CatchContext {
    f: Box<dyn Fn(Box<dyn StdError>)>,
}

impl CatchContext {
    pub(crate) fn new(f: impl Fn(Box<dyn StdError>) + 'static) -> Self {
        Self { f: Box::new(f) }
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {}

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
                cx.is_container.set(true);

                $(
                    let state = use_ref(&cx, || {
                        ScopeData::default()
                    });

                    *state.contexts.borrow_mut() = cx.contexts.borrow().clone();
                    state
                        .contexts
                        .borrow_mut()
                        .values
                        .extend(cx.child_contexts.borrow().values.clone());

                    state.is_parent_changed.set(cx.is_parent_changed.get());

                    unsafe { cx.me().$idx.any_compose(state) }
                )*
            }
        }
    };
}

impl_tuples!(T1:0);
impl_tuples!(T1:0, T2:1);
impl_tuples!(T1:0, T2:1, T3:2);
impl_tuples!(T1:0, T2:1, T3:2, T4:3);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6, T8:7);

pub(crate) trait AnyCompose {
    fn data_id(&self) -> TypeId;

    fn as_ptr_mut(&mut self) -> *mut ();

    unsafe fn reborrow(&mut self, ptr: *mut ());

    /// Safety: The caller must ensure `&self` is valid for the lifetime of `state`.
    unsafe fn any_compose(&self, state: &ScopeData);
}

impl<C> AnyCompose for C
where
    C: Compose + Data,
{
    fn data_id(&self) -> TypeId {
        typeid::of::<C>()
    }

    fn as_ptr_mut(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }

    unsafe fn reborrow(&mut self, ptr: *mut ()) {
        core::ptr::swap(self, ptr as _);
    }

    unsafe fn any_compose(&self, state: &ScopeData) {
        // Reset the hook index.
        state.hook_idx.set(0);

        // Increment the scope's current generation.
        state.generation.set(state.generation.get() + 1);

        // Transmute the lifetime of `&Self`, `&ScopeData`, and the `Scope` containing both to the same`'a`.
        // Safety: `self` and `state` are guranteed to have the same lifetime..
        let state: ScopeState = unsafe { mem::transmute(state) };
        let cx: Scope<'_, C> = Scope { me: self, state };
        let cx: Scope<'_, C> = unsafe { mem::transmute(cx) };

        // Cell for the Box used to re-allocate this composable.
        let cell: &UnsafeCell<Option<Box<dyn AnyCompose>>> = use_ref(&cx, || UnsafeCell::new(None));
        // Safety: This cell is only accessed by this composable.
        let cell = unsafe { &mut *cell.get() };

        if typeid::of::<C>() == typeid::of::<()>() {
            return;
        }

        // Scope for this composable's content.
        let child_state = use_ref(&cx, ScopeData::default);

        if cell.is_none()
            || cx.is_changed.take()
            || cx.is_parent_changed.get()
            || cx.is_container.get()
        {
            #[cfg(feature = "tracing")]
            if !cx.is_container.get() {
                if let Some(name) = C::name() {
                    tracing::trace!("Compose: {}", name);
                }
            }

            let mut child = C::compose(cx);

            cx.is_parent_changed.set(false);

            *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
            child_state
                .contexts
                .borrow_mut()
                .values
                .extend(cx.child_contexts.borrow().values.clone());

            child_state.is_parent_changed.set(true);

            unsafe {
                if let Some(ref mut content) = cell {
                    child.reborrow((**content).as_ptr_mut());
                } else {
                    let boxed: Box<dyn AnyCompose> = Box::new(child);
                    let boxed: Box<dyn AnyCompose> = mem::transmute(boxed);
                    *cell = Some(boxed);
                }
            }
        } else {
            child_state.is_parent_changed.set(false);
        }

        let child = cell.as_mut().unwrap();
        (*child).any_compose(child_state);
    }
}
