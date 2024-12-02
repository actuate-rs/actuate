use crate::{prelude::*, ScopeData};
use compose::{AnyCompose, CatchContext};
use crossbeam_queue::SegQueue;
use slotmap::{DefaultKey, SlotMap};
use std::{
    any::TypeId,
    cell::{Cell, RefCell},
    error::Error,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::Arc,
    task::{Context, Poll, Wake, Waker},
};
use tokio::sync::{RwLock, RwLockWriteGuard};

type RuntimeFuture = Pin<Box<dyn Future<Output = ()>>>;

/// Runtime for a [`Composer`].
#[derive(Clone)]
pub struct Runtime {
    /// Local task stored on this runtime.
    pub(crate) tasks: Rc<RefCell<SlotMap<DefaultKey, RuntimeFuture>>>,

    /// Queue for ready local tasks.
    pub(crate) task_queue: Arc<SegQueue<DefaultKey>>,

    /// Queue for updates that mutate the composition tree.
    pub(crate) update_queue: Arc<SegQueue<Box<dyn FnMut()>>>,

    /// Update lock for shared tasks.
    pub(crate) lock: Arc<RwLock<()>>,

    pub(crate) waker: RefCell<Option<Waker>>,
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
        let lock = self.lock.clone();
        let mut f_cell = Some(f);

        self.update_queue.push(Box::new(move || {
            let _guard = lock.blocking_write();

            let f = f_cell.take().unwrap();
            f()
        }));
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = const { RefCell::new(None) };
}

struct TaskWaker {
    key: DefaultKey,
    queue: Arc<SegQueue<DefaultKey>>,
    waker: Option<Waker>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.queue.push(self.key);
        if let Some(waker) = self.waker.as_ref() {
            waker.wake_by_ref();
        }
    }
}

/// Composer for composable content.
pub struct Composer {
    compose: Box<dyn AnyCompose>,
    scope_state: Box<ScopeData<'static>>,
    rt: Runtime,
    task_queue: Arc<SegQueue<DefaultKey>>,
    update_queue: Arc<SegQueue<Box<dyn FnMut()>>>,
    is_initial: bool,
}

impl Composer {
    /// Create a new [`Composer`] with the given content, updater, and task executor.
    pub fn new(content: impl Compose + 'static) -> Self {
        let lock = Arc::new(RwLock::new(()));

        let task_queue = Arc::new(SegQueue::new());
        let update_queue = Arc::new(SegQueue::new());

        let scope_data = ScopeData::default();
        Self {
            compose: Box::new(content),
            scope_state: Box::new(scope_data),
            rt: Runtime {
                tasks: Rc::new(RefCell::new(SlotMap::new())),
                task_queue: task_queue.clone(),
                update_queue: update_queue.clone(),
                lock,
                waker: RefCell::new(None),
            },
            task_queue,
            update_queue,
            is_initial: true,
        }
    }

    /// Try to immediately compose the content in this composer.
    pub fn try_compose(&mut self) -> Option<Result<(), Box<dyn Error>>> {
        self.rt.enter();

        let error_cell = Rc::new(Cell::new(None));
        let error_cell_handle = error_cell.clone();
        self.scope_state.contexts.borrow_mut().values.insert(
            TypeId::of::<CatchContext>(),
            Rc::new(CatchContext::new(move |error| {
                error_cell_handle.set(Some(error));
            })),
        );

        if !self.is_initial {
            let mut is_ready = false;

            while let Some(key) = self.task_queue.pop() {
                let waker = Waker::from(Arc::new(TaskWaker {
                    key,
                    waker: self.rt.waker.borrow().clone(),
                    queue: self.rt.task_queue.clone(),
                }));
                let mut cx = Context::from_waker(&waker);

                let mut tasks = self.rt.tasks.borrow_mut();
                let task = tasks.get_mut(key).unwrap();
                let _ = task.as_mut().poll(&mut cx);

                is_ready = true;
            }

            while let Some(mut update) = self.update_queue.pop() {
                update();
                is_ready = true;
            }

            if !is_ready {
                return None;
            }
        } else {
            self.is_initial = false;
        }

        #[cfg(feature = "tracing")]
        tracing::trace!("Start composition");

        // Safety: `self.compose` is guaranteed to live as long as `self.scope_state`.
        unsafe { self.compose.any_compose(&self.scope_state) };

        Some(error_cell.take().map(Err).unwrap_or(Ok(())))
    }

    /// Poll a composition of the content in this composer.
    pub fn poll_compose(&mut self, cx: &mut Context) -> Poll<Result<(), Box<dyn Error>>> {
        *self.rt.waker.borrow_mut() = Some(cx.waker().clone());

        if let Some(result) = self.try_compose() {
            Poll::Ready(result)
        } else {
            Poll::Pending
        }
    }

    /// Compose the content of this composer.
    pub async fn compose(&mut self) -> Result<(), Box<dyn Error>> {
        futures::future::poll_fn(|cx| self.poll_compose(cx)).await
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

        composer.try_compose().unwrap().unwrap();
        assert_eq!(x.get(), 1);

        composer.try_compose().unwrap().unwrap();
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

        composer.try_compose().unwrap().unwrap();
        assert_eq!(x.get(), 1);

        composer.try_compose().unwrap().unwrap();
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

        composer.try_compose().unwrap().unwrap();
        assert_eq!(x.get(), 1);

        composer.try_compose().unwrap().unwrap();
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
                memo((), B { x })
            }
        }

        let x = Rc::new(RefCell::new(0));
        let mut composer = Composer::new(A { x: x.clone() });

        composer.try_compose().unwrap().unwrap();
        assert_eq!(*x.borrow(), 1);

        composer.try_compose().unwrap().unwrap();
        assert_eq!(*x.borrow(), 1);
    }
}
