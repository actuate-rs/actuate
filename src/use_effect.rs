use crate::Scope;

pub fn use_effect<T: PartialEq + 'static>(cx: &Scope, deps: T, effect: impl FnOnce()) {
    let mut scope = cx.inner.borrow_mut();
    let idx = scope.hook_idx;
    scope.hook_idx += 1;

    let hooks = unsafe { &mut *scope.hooks.get() };

    if let Some(hook) = hooks.get_mut(idx) {
        let old_deps = (*hook).downcast_mut::<T>().unwrap();

        if deps != *old_deps {
            *old_deps = deps;
            effect()
        }
    } else {
        let hooks = unsafe { &mut *scope.hooks.get() };
        hooks.push(Box::new(deps));
        effect()
    };
}
