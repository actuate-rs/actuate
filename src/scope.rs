use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
};
use tokio::sync::mpsc;

pub(crate) trait AnyClone: Send {
    fn clone_any(&self) -> Box<dyn Any>;

    fn clone_any_clone(&self) -> Box<dyn AnyClone>;
}

impl<T: Clone + Send + 'static> AnyClone for T {
    fn clone_any(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }

    fn clone_any_clone(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

pub(crate) enum UpdateKind {
    Value(Box<dyn Any + Send>),
}

pub(crate) struct Update {
    pub(crate) idx: usize,
    pub(crate) kind: UpdateKind,
}

pub(crate) struct ScopeInner {
    pub(crate) hooks: Vec<Box<dyn Any + Send>>,
    pub(crate) idx: usize,
    pub(crate) contexts: Option<HashMap<TypeId, Box<dyn AnyClone>>>,
}

pub struct Scope {
    pub(crate) tx: mpsc::UnboundedSender<Update>,
    pub(crate) inner: UnsafeCell<ScopeInner>,
}
