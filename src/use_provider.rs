use crate::{Scope, WasmNotSend};
use std::any::TypeId;

pub fn use_provider<T: Clone + WasmNotSend + 'static>(cx: &Scope, f: impl FnOnce() -> T) -> T {
    let scope = unsafe { &mut *cx.inner.get() };
    let contexts = scope.contexts.as_mut().unwrap();

    let any = if let Some(any) = contexts.get(&TypeId::of::<T>()) {
        any
    } else {
        contexts.insert(TypeId::of::<T>(), Box::new(f()));
        contexts.get(&TypeId::of::<T>()).unwrap()
    };

    *any.clone_any().downcast().unwrap()
}
