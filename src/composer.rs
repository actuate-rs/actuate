use crate::{prelude::*, ScopeData};
use compose::AnyCompose;
use slotmap::{DefaultKey, SlotMap};
use std::{
    cell::RefCell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{mpsc, Arc},
    task::{Context, Wake, Waker},
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

struct DefaultUpdater;

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

struct TaskWaker {
    key: DefaultKey,
    updater: Arc<dyn Updater>,
    tx: mpsc::Sender<DefaultKey>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        let key = self.key;
        let pending = self.tx.clone();
        self.updater.update(Update {
            f: Box::new(move || {
                pending.send(key).unwrap();
            }),
        });
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
    pub fn new(content: impl Compose + 'static) -> Self {
        Self::with_updater(content, DefaultUpdater)
    }

    /// Create a new [`Composer`] with the given content, updater, and task executor.
    pub fn with_updater(content: impl Compose + 'static, updater: impl Updater + 'static) -> Self {
        let lock = Arc::new(RwLock::new(()));
        let updater = Arc::new(UpdateWrapper {
            updater,
            lock: lock.clone(),
        });
        let (task_tx, task_rx) = mpsc::channel();

        let scope_data = ScopeData::default();
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
