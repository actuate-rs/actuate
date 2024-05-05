use crate::{Scope, Update};
use slotmap::DefaultKey;
use std::marker::PhantomData;
use tokio::sync::mpsc;

pub fn use_state<T: 'static>(cx: &Scope, make_value: impl FnOnce() -> T) -> (&T, Setter<T>) {
    let cx_ref = unsafe { &mut *cx.inner.get() };

    let idx = cx_ref.idx;
    cx_ref.idx += 1;

    let value = if let Some(any) = cx_ref.hooks.get(idx) {
        any.downcast_ref().unwrap()
    } else {
        let cx = unsafe { &mut *cx.inner.get() };

        cx.hooks.push(Box::new(make_value()));
        cx.hooks.last().unwrap().downcast_ref().unwrap()
    };

    let setter = Setter {
        key: cx.key,
        idx,
        tx: cx.tx.clone(),
        _marker: PhantomData,
    };

    (value, setter)
}

pub struct Setter<T> {
    key: DefaultKey,
    idx: usize,
    tx: mpsc::UnboundedSender<Update>,
    _marker: PhantomData<T>,
}

impl<T> Setter<T> {
    pub fn set(&self, value: T)
    where
        T: 'static,
    {
        self.tx
            .send(Update {
                key: self.key,
                idx: self.idx,
                value: Box::new(value),
            })
            .unwrap();
    }
}
