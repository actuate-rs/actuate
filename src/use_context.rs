use crate::Scope;
use std::any::TypeId;

pub fn use_context<T: 'static>(cx: &Scope) -> T {
    let scope = cx.inner.borrow();
    let value = scope.contexts.get(&TypeId::of::<T>()).unwrap();
    *(*value.value).clone_any().downcast().unwrap()
}
