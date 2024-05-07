use std::{future, marker::PhantomData};

use tokio::sync::mpsc;

mod scope;
pub use self::scope::Scope;
use self::scope::{Update, UpdateKind};

pub mod view;
pub use self::view::View;

pub fn use_state<T: Send + 'static>(cx: &Scope, f: impl FnOnce() -> T) -> (&T, Setter<T>) {
    let scope = unsafe { &mut *cx.inner.get() };
    let idx = scope.idx;
    scope.idx += 1;

    let any = if let Some(any) = scope.hooks.get(idx) {
        any
    } else {
        let scope = unsafe { &mut *cx.inner.get() };
        scope.hooks.push(Box::new(f()));
        scope.hooks.last().unwrap()
    };

    let setter = Setter {
        idx,
        tx: cx.tx.clone(),
        _marker: PhantomData,
    };

    (any.downcast_ref().unwrap(), setter)
}

pub struct Setter<T> {
    idx: usize,
    tx: mpsc::UnboundedSender<Update>,
    _marker: PhantomData<T>,
}

impl<T> Setter<T> {
    pub fn set(&self, value: T)
    where
        T: Send + 'static,
    {
        self.tx
            .send(Update {
                idx: self.idx,
                kind: UpdateKind::Value(Box::new(value)),
            })
            .unwrap();
    }
}

pub async fn run(view: impl View) {
    let mut state = view.build();

    loop {
        future::poll_fn(|cx| view.poll_ready(cx, &mut state)).await;
        view.view(&mut state);
    }
}
