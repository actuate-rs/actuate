use slotmap::{DefaultKey, SlotMap};
use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
    mem,
    ops::Deref,
    rc::Rc,
};

pub struct Mut<'a, T> {
    key: DefaultKey,
    idx: usize,
    value: &'a T,
}

impl<T> Clone for Mut<'_, T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key,
            idx: self.idx,
            value: self.value,
        }
    }
}

impl<T> Copy for Mut<'_, T> {}

impl<'a, T> Deref for Mut<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Default)]
pub struct ScopeState {
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
}

impl ScopeState {
    pub fn use_ref<T: 'static>(&self, make_value: impl FnOnce() -> T) -> &T {
        let hooks = unsafe { &mut *self.hooks.get() };

        let idx = self.hook_idx.get();
        self.hook_idx.set(idx + 1);

        dbg!(idx);

        let any = if idx >= hooks.len() {
            hooks.push(Box::new(make_value()));
            hooks.last().unwrap()
        } else {
            hooks.get(idx).unwrap()
        };
        any.downcast_ref().unwrap()
    }

    pub fn use_mut<T: 'static>(&self, make_value: impl FnOnce() -> T) -> Mut<T> {
        let (value, idx) = self.use_ref_with_idx(|| make_value());

        Mut {
            value,
            idx,
            key: Runtime::current().key(),
        }
    }

    fn use_ref_with_idx<T: 'static>(&self, make_value: impl FnOnce() -> T) -> (&T, usize) {
        let hooks = unsafe { &mut *self.hooks.get() };

        let idx = self.hook_idx.get();
        self.hook_idx.set(idx + 1);

        let any = if idx >= hooks.len() {
            hooks.push(Box::new(make_value()));
            hooks.last().unwrap()
        } else {
            hooks.get(idx).unwrap()
        };
        (any.downcast_ref().unwrap(), idx)
    }
}

pub struct Scope<'a, C: ?Sized> {
    pub me: &'a C,
    pub state: &'a ScopeState,
}

impl<C> Clone for Scope<'_, C> {
    fn clone(&self) -> Self {
        Self {
            me: self.me,
            state: self.state,
        }
    }
}

impl<C> Copy for Scope<'_, C> {}

impl<'a, C> Deref for Scope<'a, C> {
    type Target = &'a ScopeState;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

pub trait Compose {
    fn compose(cx: Scope<Self>) -> impl Compose;
}

impl Compose for () {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Runtime::current().set_is_empty(true)
    }
}

pub trait AnyCompose {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a>;
}

impl<C: Compose> AnyCompose for C {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a> {
        Box::new(C::compose(Scope { me: self, state }))
    }
}

#[derive(Default)]
struct Inner {
    is_empty: bool,
    key: DefaultKey,
}

#[derive(Clone, Default)]
pub struct Runtime {
    inner: Rc<RefCell<Inner>>,
}

impl Runtime {
    pub fn current() -> Self {
        RUNTIME.with(|runtime| {
            runtime
                .borrow()
                .as_ref()
                .expect("Runtime::current() called outside of a runtime")
                .clone()
        })
    }

    pub fn enter(&self) {
        RUNTIME.with(|runtime| {
            *runtime.borrow_mut() = Some(self.clone());
        });
    }

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty
    }

    pub fn set_is_empty(&self, is_empty: bool) {
        self.inner.borrow_mut().is_empty = is_empty;
    }

    pub fn key(&self) -> DefaultKey {
        self.inner.borrow().key
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

struct Node {
    state: ScopeState,
    compose: Box<dyn AnyCompose>,
    children: Vec<DefaultKey>,
}

pub struct Composer {
    rt: Runtime,
    nodes: SlotMap<DefaultKey, Node>,
    root: DefaultKey,
}

impl Composer {
    pub fn new(content: impl Compose + 'static) -> Self {
        let node = Node {
            state: ScopeState::default(),
            compose: Box::new(content),
            children: Vec::new(),
        };

        let mut nodes = SlotMap::new();
        let root = nodes.insert(node);

        Self {
            rt: Runtime::default(),
            nodes,
            root,
        }
    }

    pub fn compose(&mut self) {
        self.rt.enter();

        let mut key = self.root;

        loop {
            let node = self.nodes.get(key).unwrap();
            let child: Box<dyn AnyCompose> = node.compose.any_compose(&node.state);

            if self.rt.is_empty() {
                self.rt.set_is_empty(false);
                break;
            } else {
                let compose = unsafe { mem::transmute(child) };
                let child_node = Node {
                    state: ScopeState::default(),
                    compose,
                    children: Vec::new(),
                };

                let child_key = self.nodes.insert(child_node);
                self.nodes.get_mut(key).unwrap().children.push(child_key);
                key = child_key;
            }
        }
    }
}
