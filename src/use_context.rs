use crate::{Scope, WasmNotSend};
use std::any::TypeId;

pub fn use_context<T: Clone + WasmNotSend + 'static>(cx: &Scope) -> Option<T> {
    let scope = unsafe { &mut *cx.inner.get() };

    scope
        .contexts
        .as_ref()
        .unwrap()
        .get(&TypeId::of::<T>())
        .map(|any| *any.clone_any().downcast().unwrap())
}
