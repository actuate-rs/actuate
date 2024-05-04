use crate::{Context, Update, UpdateKind};
use std::marker::PhantomData;
use tokio::sync::mpsc;

pub fn use_state<T: Clone + 'static>(f: impl FnOnce() -> T) -> (T, SetState<T>) {
    let cx = Context::get();
    let mut cx = cx.inner.borrow_mut();

    let idx = cx.idx;
    cx.idx += 1;

    let any = if let Some(state) = cx.states.get(idx) {
        state
    } else {
        cx.states.push(Box::new(f()));
        cx.states.last().unwrap()
    };
    let value = any.downcast_ref::<T>().unwrap().clone();

    let set_state = SetState {
        id: cx.id,
        tx: cx.tx.clone(),
        idx,
        _marker: PhantomData,
    };

    (value, set_state)
}

pub struct SetState<T> {
    id: u64,
    idx: usize,
    tx: mpsc::UnboundedSender<Update>,
    _marker: PhantomData<T>,
}

impl<T> SetState<T> {
    pub fn set(&self, value: T)
    where
        T: 'static,
    {
        self.tx
            .send(Update {
                id: self.id,
                idx: self.idx,
                kind: UpdateKind::Value(Box::new(value)),
            })
            .unwrap();
    }

    pub fn update(&self, f: impl FnOnce(&mut T) + 'static)
    where
        T: 'static,
    {
        let mut cell = Some(f);
        self.tx
            .send(Update {
                id: self.id,
                idx: self.idx,
                kind: UpdateKind::Setter(Box::new(move |any| {
                    let f = cell.take().unwrap();
                    f(any.downcast_mut().unwrap())
                })),
            })
            .unwrap();
    }
}
