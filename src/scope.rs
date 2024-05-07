use std::{any::Any, cell::UnsafeCell};
use tokio::sync::mpsc;

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
}

pub struct Scope {
    pub(crate) tx: mpsc::UnboundedSender<Update>,
    pub(crate) inner: UnsafeCell<ScopeInner>,
}
