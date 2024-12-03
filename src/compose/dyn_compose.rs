use super::AnyCompose;
use crate::{prelude::*, ScopeData};
use core::{any::TypeId, cell::UnsafeCell, mem};

/// Create a new dynamically-typed composable.
pub fn dyn_compose<'a>(content: impl Compose + 'a) -> DynCompose<'a> {
    DynCompose {
        compose: UnsafeCell::new(Some(Box::new(content))),
    }
}

/// Dynamically-typed composable.
#[must_use = "Composables do nothing unless composed or returned from other composables."]
pub struct DynCompose<'a> {
    compose: UnsafeCell<Option<Box<dyn AnyCompose + 'a>>>,
}

struct DynComposeState {
    compose: Box<dyn AnyCompose>,
    data_id: TypeId,
}

impl Compose for DynCompose<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let cell: &UnsafeCell<Option<DynComposeState>> = use_ref(&cx, || UnsafeCell::new(None));
        let cell = unsafe { &mut *cell.get() };

        let inner = unsafe { &mut *cx.me().compose.get() };

        let child_state = use_ref(&cx, || UnsafeCell::new(ScopeData::default()));
        let child_state = unsafe { &mut *child_state.get() };

        *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
        child_state
            .contexts
            .borrow_mut()
            .values
            .extend(cx.child_contexts.borrow().values.clone());

        child_state
            .is_parent_changed
            .set(cx.is_parent_changed.get());

        if let Some(any_compose) = inner.take() {
            let mut compose: Box<dyn AnyCompose> = unsafe { mem::transmute(any_compose) };

            if let Some(state) = cell {
                if state.data_id != compose.data_id() {
                    *child_state = ScopeData::default();
                    state.compose = compose;
                } else {
                    let ptr = (*state.compose).as_ptr_mut();
                    unsafe {
                        compose.reborrow(ptr);
                    }
                }
            } else {
                *cell = Some(DynComposeState {
                    data_id: compose.data_id(),
                    compose,
                })
            }
        }

        unsafe { cell.as_mut().unwrap().compose.any_compose(child_state) }
    }
}
