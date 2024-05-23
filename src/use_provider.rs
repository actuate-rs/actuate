use crate::{Scope, ScopeContext};
use std::{any::TypeId, rc::Rc};

pub fn use_provider<T: Clone + 'static>(cx: &Scope, make_value: impl FnOnce() -> T) {
    let mut scope = cx.inner.borrow_mut();
    let contexts = Rc::make_mut(&mut scope.contexts);

    //TODO
    //if !contexts.contains_key(&TypeId::of::<T>()) {
    contexts.insert(
        TypeId::of::<T>(),
        ScopeContext {
            value: Box::new(make_value()),
        },
    );
}
