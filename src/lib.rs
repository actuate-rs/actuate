use std::{any::Any, future, marker::PhantomData};
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

pub struct VirtualDom<V, S, E> {
    view: V,
    state: S,
    elements: VecStack<E>,
}

impl<V, S, E> VirtualDom<V, S, E> {
    pub fn new(view: V) -> Self
    where
        V: View<Element = S>,
    {
        let state = view.build();
        Self {
            view,
            state,
            elements: VecStack {
                items: Vec::new(),
                idx: 0,
            },
        }
    }

    pub async fn run(&mut self)
    where
        V: View<Element = S>,
        E: 'static,
    {
        future::poll_fn(|cx| self.view.poll_ready(cx, &mut self.state)).await;
        self.view.view(&mut self.elements, &mut self.state);
    }
}

pub trait Stack {
    fn push(&mut self, element: Box<dyn Any>);

    fn update(&mut self) -> &mut dyn Any;

    fn skip(&mut self, n: usize);

    fn remove(&mut self, n: usize);

    fn len(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

pub struct VecStack<T> {
    pub items: Vec<T>,
    pub idx: usize,
}

impl<T> Default for VecStack<T> {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            idx: 0,
        }
    }
}

impl<T: 'static> Stack for VecStack<T> {
    fn push(&mut self, element: Box<dyn Any>) {
        self.items.push(*element.downcast().unwrap());
        self.idx += 1;
    }

    fn update(&mut self) -> &mut dyn Any {
        let idx = self.idx;
        self.idx += 1;

        self.items.get_mut(idx).unwrap()
    }

    fn skip(&mut self, n: usize) {
        self.idx += n;
    }

    fn remove(&mut self, n: usize) {
        for i in 0..n {
            self.items.remove(self.idx + i);
        }
        self.idx += n;
    }

    fn len(&self) -> usize {
        self.items.len()
    }
}
