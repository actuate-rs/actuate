use alloc::{rc::Rc, sync::Arc};
use core::{future::Future, pin::Pin};

/// Executor for async tasks.
pub trait Executor {
    /// Spawn a boxed future on this executor.
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);
}

#[cfg(feature = "rt")]
#[cfg_attr(docsrs, doc(cfg(feature = "rt")))]
impl Executor for tokio::runtime::Runtime {
    fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        self.spawn(future);
    }
}

macro_rules! impl_executor {
    ($($t:tt),*) => {
        $(
            impl<T: Executor + ?Sized> Executor for $t<T> {
                fn spawn(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
                    (**self).spawn(future);
                }
            }
        )*
    };
}

impl_executor!(Box, Rc, Arc);

/// Context that contains the current [`Executor`].
pub struct ExecutorContext {
    pub(crate) executor: Box<dyn Executor>,
}

#[cfg(feature = "rt")]
impl Default for ExecutorContext {
    fn default() -> Self {
        Self::new(tokio::runtime::Runtime::new().unwrap())
    }
}

impl ExecutorContext {
    /// Create a new [`ExecutorContext`] with the provided [`Executor`].
    pub fn new(executor: impl Executor + 'static) -> Self {
        Self {
            executor: Box::new(executor),
        }
    }

    /// Spawn a future on the current runtime.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn_boxed(Box::pin(future))
    }

    /// Spawn a boxed future on the current runtime.
    pub fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        self.executor.spawn(future);
    }
}
