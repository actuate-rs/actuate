use crate::{prelude::*, ScopeData, TaskWaker};
use compose::AnyCompose;
use slotmap::{DefaultKey, SlotMap};
use std::{
    any::TypeId,
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{mpsc, Arc},
    task::{Context, Waker},
};
use tokio::sync::{RwLock, RwLockWriteGuard};

/// An update to apply to a composable.
pub struct Update {
    pub(crate) f: Box<dyn FnOnce()>,
}

impl Update {
    /// Apply this update.
    ///
    /// # Safety
    /// The caller must ensure the composable triggering this update still exists.
    pub unsafe fn apply(self) {
        (self.f)();
    }
}

type RuntimeFuture = Pin<Box<dyn Future<Output = ()>>>;

/// Runtime for a [`Composer`].
#[derive(Clone)]
pub struct Runtime {
    /// Updater for this runtime.
    pub(crate) updater: Arc<dyn Updater>,

    /// Local task stored on this runtime.
    pub(crate) tasks: Rc<RefCell<SlotMap<DefaultKey, RuntimeFuture>>>,

    /// Waker for local tasks.
    pub(crate) task_tx: mpsc::Sender<DefaultKey>,

    /// Update lock for shared tasks.
    pub(crate) lock: Arc<RwLock<()>>,
}

impl Runtime {
    /// Get the current [`Runtime`].
    ///
    /// # Panics
    /// Panics if called outside of a runtime.
    pub fn current() -> Self {
        RUNTIME.with(|runtime| {
            runtime
                .borrow()
                .as_ref()
                .expect("Runtime::current() called outside of a runtime")
                .clone()
        })
    }

    /// Enter this runtime, making it available to [`Runtime::current`].
    pub fn enter(&self) {
        RUNTIME.with(|runtime| {
            *runtime.borrow_mut() = Some(self.clone());
        });
    }

    /// Queue an update to run after [`Composer::compose`].
    pub fn update(&self, f: impl FnOnce() + 'static) {
        self.updater.update(Update { f: Box::new(f) });
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

/// Updater for a [`Composer`].
pub trait Updater: Send + Sync {
    /// Update the content of a [`Composer`].
    fn update(&self, update: Update);
}

#[cfg(feature = "rt")]
struct DefaultUpdater;

#[cfg(feature = "rt")]
impl Updater for DefaultUpdater {
    fn update(&self, update: Update) {
        unsafe {
            update.apply();
        }
    }
}

struct UpdateWrapper<U> {
    updater: U,
    lock: Arc<RwLock<()>>,
}

impl<U: Updater> Updater for UpdateWrapper<U> {
    fn update(&self, update: Update) {
        let lock = self.lock.clone();
        self.updater.update(Update {
            f: Box::new(move || {
                let _guard = lock.blocking_write();
                unsafe { update.apply() }
            }),
        });
    }
}

/// Executor for async tasks.
pub trait Executor {
    /// Spawn a future on this executor.
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static;
}

impl<T: Executor> Executor for Box<T> {
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        (**self).spawn(future);
    }
}

#[cfg(feature = "rt")]
#[cfg_attr(docsrs, doc(cfg(feature = "rt")))]
impl Executor for tokio::runtime::Runtime {
    fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn(future);
    }
}

pub(crate) trait AnyExecutor {
    fn spawn_any(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>);
}

impl<E: Executor> AnyExecutor for E {
    fn spawn_any(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        self.spawn(future);
    }
}

/// Context that contains the current [`Executor`].
pub struct ExecutorContext {
    pub(crate) rt: Box<dyn AnyExecutor>,
}

impl ExecutorContext {
    /// Spawn a future on the current runtime.
    pub fn spawn<F>(&self, future: F)
    where
        F: Future<Output = ()> + Send + 'static,
    {
        self.spawn_boxed(Box::pin(future))
    }

    /// Spawn a boxed future on the current runtime.
    pub fn spawn_boxed(&self, future: Pin<Box<dyn Future<Output = ()> + Send>>) {
        self.rt.spawn_any(future);
    }
}

/// Composer for composable content.
pub struct Composer {
    compose: Box<dyn AnyCompose>,
    scope_state: Box<ScopeData<'static>>,
    rt: Runtime,
    task_rx: mpsc::Receiver<DefaultKey>,
}

impl Composer {
    /// Create a new [`Composer`] with the given content and default updater.
    #[cfg(feature = "rt")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rt")))]
    pub fn new(content: impl Compose + 'static) -> Self {
        let rt = tokio::runtime::Runtime::new().unwrap();
        Self::with_updater(content, DefaultUpdater, rt)
    }

    /// Create a new [`Composer`] with the given content, updater, and task executor.
    pub fn with_updater(
        content: impl Compose + 'static,
        updater: impl Updater + 'static,
        executor: impl Executor + 'static,
    ) -> Self {
        let lock = Arc::new(RwLock::new(()));
        let updater = Arc::new(UpdateWrapper {
            updater,
            lock: lock.clone(),
        });
        let (task_tx, task_rx) = mpsc::channel();

        let scope_data = ScopeData::default();

        let executor_cx = Rc::new(ExecutorContext {
            rt: Box::new(executor),
        });
        scope_data
            .contexts
            .borrow_mut()
            .values
            .insert(TypeId::of::<ExecutorContext>(), executor_cx.clone());
        scope_data
            .child_contexts
            .borrow_mut()
            .values
            .insert(TypeId::of::<ExecutorContext>(), executor_cx);

        Self {
            compose: Box::new(content),
            scope_state: Box::new(scope_data),
            rt: Runtime {
                updater: updater.clone(),
                tasks: Rc::new(RefCell::new(SlotMap::new())),
                task_tx,
                lock,
            },
            task_rx,
        }
    }

    /// Compose the content of this composer.
    pub fn compose(&mut self) {
        #[cfg(feature = "tracing")]
        tracing::trace!("Composer::compose");

        self.rt.enter();

        while let Ok(key) = self.task_rx.try_recv() {
            let waker = Waker::from(Arc::new(TaskWaker {
                key,
                updater: Runtime::current().updater.clone(),
                tx: self.rt.task_tx.clone(),
            }));
            let mut cx = Context::from_waker(&waker);

            let mut tasks = self.rt.tasks.borrow_mut();
            let task = tasks.get_mut(key).unwrap();
            let _ = task.as_mut().poll(&mut cx);
        }

        // Safety: `self.compose` is guaranteed to live as long as `self.scope_state`.
        unsafe { self.compose.any_compose(&self.scope_state) }
    }

    /// Lock updates to the content of this composer.
    pub fn lock(&self) -> RwLockWriteGuard<()> {
        self.rt.lock.blocking_write()
    }
}

#[cfg(all(test, feature = "rt"))]
mod tests {
    use crate::{composer::Composer, prelude::*};
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    #[derive(Data)]
    struct Counter {
        x: Rc<Cell<i32>>,
    }

    impl Compose for Counter {
        fn compose(cx: Scope<Self>) -> impl Compose {
            cx.me().x.set(cx.me().x.get() + 1);

            cx.set_changed();
        }
    }

    #[derive(Data)]
    struct NonUpdateCounter {
        x: Rc<Cell<i32>>,
    }

    impl Compose for NonUpdateCounter {
        fn compose(cx: Scope<Self>) -> impl Compose {
            cx.me().x.set(cx.me().x.get() + 1);
        }
    }

    #[test]
    fn it_composes() {
        #[derive(Data)]
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: Scope<Self>) -> impl Compose {
                Counter {
                    x: cx.me().x.clone(),
                }
            }
        }

        let x = Rc::new(Cell::new(0));
        let mut composer = Composer::new(Wrap { x: x.clone() });

        composer.compose();
        assert_eq!(x.get(), 1);

        composer.compose();
        assert_eq!(x.get(), 2);
    }

    #[test]
    fn it_skips_recomposes() {
        #[derive(Data)]
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: Scope<Self>) -> impl Compose {
                NonUpdateCounter {
                    x: cx.me().x.clone(),
                }
            }
        }

        let x = Rc::new(Cell::new(0));
        let mut composer = Composer::new(Wrap { x: x.clone() });

        composer.compose();
        assert_eq!(x.get(), 1);

        composer.compose();
        assert_eq!(x.get(), 1);
    }

    #[test]
    fn it_composes_any_compose() {
        #[derive(Data)]
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: crate::Scope<Self>) -> impl Compose {
                DynCompose::new(Counter {
                    x: cx.me().x.clone(),
                })
            }
        }

        let x = Rc::new(Cell::new(0));
        let mut composer = Composer::new(Wrap { x: x.clone() });

        composer.compose();
        assert_eq!(x.get(), 1);

        composer.compose();
        assert_eq!(x.get(), 2);
    }

    #[test]
    fn it_memoizes_composables() {
        #[derive(Data)]
        struct B {
            x: Rc<RefCell<i32>>,
        }

        impl Compose for B {
            fn compose(cx: Scope<Self>) -> impl Compose {
                *cx.me().x.borrow_mut() += 1;
            }
        }

        #[derive(Data)]
        struct A {
            x: Rc<RefCell<i32>>,
        }

        impl Compose for A {
            fn compose(cx: Scope<Self>) -> impl Compose {
                let x = cx.me().x.clone();
                Memo::new((), B { x })
            }
        }

        let x = Rc::new(RefCell::new(0));
        let mut compsoer = Composer::new(A { x: x.clone() });

        compsoer.compose();
        assert_eq!(*x.borrow(), 1);

        compsoer.compose();
        assert_eq!(*x.borrow(), 1);
    }
}
