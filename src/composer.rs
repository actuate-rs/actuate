use crate::{
    compose::{AnyCompose, CatchContext, Compose},
    ScopeData,
};
use alloc::{rc::Rc, sync::Arc, task::Wake};
use core::{
    any::TypeId,
    cell::{Cell, RefCell},
    error::Error,
    fmt,
    future::Future,
    mem,
    pin::Pin,
    task::{Context, Poll, Waker},
};
use crossbeam_queue::SegQueue;
use slotmap::{DefaultKey, SlotMap};
use std::collections::VecDeque;

#[cfg(feature = "executor")]
use tokio::sync::RwLock;

type RuntimeFuture = Pin<Box<dyn Future<Output = ()>>>;

pub(crate) enum ComposePtr {
    Boxed(Box<dyn AnyCompose>),
    Ptr(*const dyn AnyCompose),
}

impl AnyCompose for ComposePtr {
    fn data_id(&self) -> TypeId {
        match self {
            ComposePtr::Boxed(compose) => compose.data_id(),
            ComposePtr::Ptr(ptr) => unsafe { (**ptr).data_id() },
        }
    }

    fn as_ptr_mut(&mut self) -> *mut () {
        match self {
            ComposePtr::Boxed(compose) => compose.as_ptr_mut(),
            ComposePtr::Ptr(ptr) => *ptr as *mut (),
        }
    }

    unsafe fn reborrow(&mut self, ptr: *mut ()) {
        match self {
            ComposePtr::Boxed(compose) => compose.reborrow(ptr),
            // TODO
            ComposePtr::Ptr(_) => {}
        }
    }

    unsafe fn any_compose(&self, state: &ScopeData) {
        match self {
            ComposePtr::Boxed(compose) => compose.any_compose(state),
            ComposePtr::Ptr(ptr) => (**ptr).any_compose(state),
        }
    }

    fn name(&self) -> Option<std::borrow::Cow<'static, str>> {
        match self {
            ComposePtr::Boxed(compose) => compose.name(),
            ComposePtr::Ptr(_) => None,
        }
    }
}

// Safety: `scope` must be dropped before `compose`.
pub(crate) struct Node {
    pub(crate) compose: RefCell<ComposePtr>,
    pub(crate) scope: ScopeData<'static>,
    pub(crate) parent: Option<DefaultKey>,
    pub(crate) children: RefCell<Vec<DefaultKey>>,
}

/// Runtime for a [`Composer`].
#[derive(Clone)]
pub(crate) struct Runtime {
    /// Local task stored on this runtime.
    pub(crate) tasks: Rc<RefCell<SlotMap<DefaultKey, RuntimeFuture>>>,

    /// Queue for ready local tasks.
    pub(crate) task_queue: Arc<SegQueue<DefaultKey>>,

    /// Queue for updates that mutate the composition tree.
    pub(crate) update_queue: Rc<SegQueue<Box<dyn FnMut()>>>,

    #[cfg(feature = "executor")]
    /// Update lock for shared tasks.
    pub(crate) lock: Arc<RwLock<()>>,

    pub(crate) waker: RefCell<Option<Waker>>,

    pub(crate) nodes: Rc<RefCell<SlotMap<DefaultKey, Rc<Node>>>>,

    pub(crate) current_key: Rc<Cell<DefaultKey>>,

    pub(crate) root: DefaultKey,

    pub(crate) pending: Rc<RefCell<VecDeque<DefaultKey>>>,
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
    pub fn update(&self, f: impl FnOnce() + Send + 'static) {
        let mut f_cell = Some(f);

        #[cfg(feature = "executor")]
        let lock = self.lock.clone();

        self.update_queue.push(Box::new(move || {
            #[cfg(feature = "executor")]
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

/// Error for [`Composer::try_compose`].
#[derive(Debug)]
pub enum TryComposeError {
    /// No updates are ready to be applied.
    Pending,

    /// An error occurred during composition.
    Error(Box<dyn Error>),
}

impl PartialEq for TryComposeError {
    fn eq(&self, other: &Self) -> bool {
        mem::discriminant(self) == mem::discriminant(other)
    }
}

/// Composer for composable content.
pub struct Composer {
    rt: Runtime,
    task_queue: Arc<SegQueue<DefaultKey>>,
    update_queue: Rc<SegQueue<Box<dyn FnMut()>>>,
    is_initial: bool,
}

impl Composer {
    /// Create a new [`Composer`] with the given content, updater, and task executor.
    pub fn new(content: impl Compose + 'static) -> Self {
        #[cfg(feature = "executor")]
        let lock = Arc::new(RwLock::new(()));

        let task_queue = Arc::new(SegQueue::new());
        let update_queue = Rc::new(SegQueue::new());

        let mut nodes = SlotMap::new();
        let root_key = nodes.insert(Rc::new(Node {
            compose: RefCell::new(ComposePtr::Boxed(Box::new(content))),
            scope: ScopeData::default(),
            parent: None,
            children: RefCell::new(Vec::new()),
        }));

        Self {
            rt: Runtime {
                tasks: Rc::new(RefCell::new(SlotMap::new())),
                task_queue: task_queue.clone(),
                update_queue: update_queue.clone(),
                waker: RefCell::new(None),
                #[cfg(feature = "executor")]
                lock,
                nodes: Rc::new(RefCell::new(nodes)),
                current_key: Rc::new(Cell::new(root_key)),
                root: root_key,
                pending: Rc::new(RefCell::new(VecDeque::new())),
            },
            task_queue,
            update_queue,
            is_initial: true,
        }
    }

    /// Try to immediately compose the content in this composer.
    pub fn try_compose(&mut self) -> Result<(), TryComposeError> {
        let mut is_pending = true;

        for res in self.by_ref() {
            res.map_err(TryComposeError::Error)?;

            is_pending = false;
        }

        if is_pending {
            Err(TryComposeError::Pending)
        } else {
            Ok(())
        }
    }

    /// Poll a composition of the content in this composer.
    pub fn poll_compose(&mut self, cx: &mut Context) -> Poll<Result<(), Box<dyn Error>>> {
        *self.rt.waker.borrow_mut() = Some(cx.waker().clone());

        match self.try_compose() {
            Ok(()) => Poll::Ready(Ok(())),
            Err(TryComposeError::Pending) => Poll::Pending,
            Err(TryComposeError::Error(error)) => Poll::Ready(Err(error)),
        }
    }

    /// Compose the content of this composer.
    pub async fn compose(&mut self) -> Result<(), Box<dyn Error>> {
        futures::future::poll_fn(|cx| self.poll_compose(cx)).await
    }
}

impl Drop for Composer {
    fn drop(&mut self) {
        let node = self.rt.nodes.borrow()[self.rt.root].clone();
        drop_recursive(&self.rt, self.rt.root, node)
    }
}

fn drop_recursive(rt: &Runtime, key: DefaultKey, node: Rc<Node>) {
    let children = node.children.borrow().clone();
    for child_key in children {
        let child = rt.nodes.borrow()[child_key].clone();
        drop_recursive(rt, child_key, child)
    }

    rt.nodes.borrow_mut().remove(key);
}

impl Iterator for Composer {
    type Item = Result<(), Box<dyn Error>>;

    fn next(&mut self) -> Option<Self::Item> {
        self.rt.enter();

        let error_cell = Rc::new(Cell::new(None));
        let error_cell_handle = error_cell.clone();

        let root = self.rt.nodes.borrow().get(self.rt.root).unwrap().clone();
        root.scope.contexts.borrow_mut().values.insert(
            TypeId::of::<CatchContext>(),
            Rc::new(CatchContext::new(move |error| {
                error_cell_handle.set(Some(error));
            })),
        );

        if !self.is_initial {
            let key_cell = self.rt.pending.borrow_mut().pop_front();
            if let Some(key) = key_cell {
                self.rt.current_key.set(key);

                let node = self.rt.nodes.borrow().get(key).unwrap().clone();

                // Safety: `self.compose` is guaranteed to live as long as `self.scope_state`.
                unsafe { node.compose.borrow().any_compose(&node.scope) };
            } else {
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
                }

                while let Some(mut update) = self.update_queue.pop() {
                    update();
                }

                return None;
            }
        } else {
            self.is_initial = false;

            self.rt.current_key.set(self.rt.root);

            // Safety: `self.compose` is guaranteed to live as long as `self.scope_state`.
            unsafe { root.compose.borrow().any_compose(&root.scope) };
        }

        Some(error_cell.take().map(Err).unwrap_or(Ok(())))
    }
}

impl fmt::Debug for Composer {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Composer")
            .field(
                "nodes",
                &Debugger {
                    nodes: &self.rt.nodes.borrow(),
                    key: self.rt.root,
                },
            )
            .finish()
    }
}

struct Debugger<'a> {
    nodes: &'a SlotMap<DefaultKey, Rc<Node>>,
    key: DefaultKey,
}

impl fmt::Debug for Debugger<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let node = &self.nodes[self.key];
        let name = node.compose.borrow().name().unwrap_or_default();

        let mut dbg_tuple = f.debug_tuple(&name);

        for child in &*node.children.borrow() {
            dbg_tuple.field(&Debugger {
                nodes: self.nodes,
                key: *child,
            });
        }

        dbg_tuple.finish()
    }
}

#[cfg(all(test, feature = "rt"))]
mod tests {
    use crate::{
        composer::{Composer, TryComposeError},
        prelude::*,
    };
    use std::{
        cell::{Cell, RefCell},
        rc::Rc,
    };

    #[derive(Data)]
    #[actuate(path = "crate")]
    struct Counter {
        x: Rc<Cell<i32>>,
    }

    impl Compose for Counter {
        fn compose(cx: Scope<Self>) -> impl Compose {
            let updater = use_mut(&cx, || ());
            SignalMut::set(updater, ());

            cx.me().x.set(cx.me().x.get() + 1);
        }
    }

    #[derive(Data)]
    #[actuate(path = "crate")]
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
        #[actuate(path = "crate")]
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

        composer.try_compose().unwrap();
        assert_eq!(x.get(), 1);

        composer.try_compose().unwrap();
        assert_eq!(x.get(), 2);
    }

    #[test]
    fn it_skips_recomposes() {
        #[derive(Data)]
        #[actuate(path = "crate")]
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

        composer.try_compose().unwrap();
        assert_eq!(x.get(), 1);

        assert_eq!(composer.try_compose(), Err(TryComposeError::Pending));
        assert_eq!(x.get(), 1);
    }

    #[test]
    fn it_composes_any_compose() {
        #[derive(Data)]
        #[actuate(path = "crate")]
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: crate::Scope<Self>) -> impl Compose {
                dyn_compose(Counter {
                    x: cx.me().x.clone(),
                })
            }
        }

        let x = Rc::new(Cell::new(0));
        let mut composer = Composer::new(Wrap { x: x.clone() });

        composer.try_compose().unwrap();
        assert_eq!(x.get(), 1);

        composer.try_compose().unwrap();
        assert_eq!(x.get(), 2);
    }

    #[test]
    fn it_memoizes_composables() {
        #[derive(Data)]
        #[actuate(path = "crate")]
        struct B {
            x: Rc<RefCell<i32>>,
        }

        impl Compose for B {
            fn compose(cx: Scope<Self>) -> impl Compose {
                *cx.me().x.borrow_mut() += 1;
            }
        }

        #[derive(Data)]
        #[actuate(path = "crate")]
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

        composer.try_compose().unwrap();
        assert_eq!(*x.borrow(), 1);

        assert_eq!(composer.try_compose(), Err(TryComposeError::Pending));
        assert_eq!(*x.borrow(), 1);
    }
}
