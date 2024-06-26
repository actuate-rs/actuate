use crate::{Scope, Update, UpdateSender};
use slotmap::DefaultKey;
use std::marker::PhantomData;

pub fn use_state<T: 'static>(cx: &Scope, make_value: impl FnOnce() -> T) -> (&T, SetState<T>) {
    let mut scope = cx.inner.borrow_mut();
    let idx = scope.hook_idx;
    scope.hook_idx += 1;
    let hooks = unsafe { &mut *scope.hooks.get() };

    let value = if let Some(hook) = hooks.get(idx) {
        hook
    } else {
        let hooks = unsafe { &mut *scope.hooks.get() };
        hooks.push(Box::new(make_value()));
        hooks.last().unwrap()
    };

    let setter = SetState {
        key: scope.key,
        tx: scope.tx.clone(),
        idx,
        _marker: PhantomData,
    };

    (value.downcast_ref().unwrap(), setter)
}

pub struct SetState<T> {
    key: DefaultKey,
    tx: UpdateSender,
    idx: usize,
    _marker: PhantomData<fn(T)>,
}

impl<T> SetState<T>
where
    T: 'static,
{
    pub fn modify(&self, f: impl FnOnce(&mut T) + 'static) {
        let mut f_cell = Some(f);
        self.tx
            .send(Update {
                key: self.key,
                idx: self.idx,
                f: Box::new(move |any| f_cell.take().unwrap()(any.downcast_mut().unwrap())),
            })
            .unwrap();
    }

    pub fn set(&self, value: T) {
        self.modify(move |target| *target = value)
    }
}

impl<T> Clone for SetState<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            tx: self.tx.clone(),
            idx: self.idx,
            _marker: PhantomData,
        }
    }
}
