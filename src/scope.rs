use crate::Tx;
use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
};

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
pub(crate) trait AnyClone: Send {
    fn clone_any(&self) -> Box<dyn Any>;

    fn clone_any_clone(&self) -> Box<dyn AnyClone>;
}

#[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
impl<T: Clone + Send + 'static> AnyClone for T {
    fn clone_any(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }

    fn clone_any_clone(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
pub(crate) trait AnyClone {
    fn clone_any(&self) -> Box<dyn Any>;

    fn clone_any_clone(&self) -> Box<dyn AnyClone>;
}

#[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
impl<T: Clone + 'static> AnyClone for T {
    fn clone_any(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }

    fn clone_any_clone(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

pub(crate) enum UpdateKind {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    Value(Box<dyn Any + Send>),
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    Value(Box<dyn Any>),
}

pub(crate) struct Update {
    pub(crate) idx: usize,
    pub(crate) kind: UpdateKind,
}

pub(crate) struct ScopeInner {
    #[cfg(not(all(target_arch = "wasm32", target_os = "unknown")))]
    pub(crate) hooks: Vec<Box<dyn Any + Send>>,
    #[cfg(all(target_arch = "wasm32", target_os = "unknown"))]
    pub(crate) hooks: Vec<Box<dyn Any>>,

    pub(crate) idx: usize,
    pub(crate) contexts: Option<HashMap<TypeId, Box<dyn AnyClone>>>,
}

pub struct Scope {
    pub(crate) tx: Tx<Update>,
    pub(crate) inner: UnsafeCell<ScopeInner>,
}
