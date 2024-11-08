use slotmap::{DefaultKey, SlotMap};
use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
    mem,
    ops::Deref,
    rc::Rc,
};

pub use actuate_macros::Data;

pub struct Mut<'a, T> {
    key: DefaultKey,
    idx: usize,
    value: &'a T,
}

impl<T: 'static> Mut<'_, T> {
    pub fn update(&self, f: impl FnOnce(&mut T) + 'static) {
        let mut cell = Some(f);
        Runtime::current().update(self.key, self.idx, move |any| {
            let value = any.downcast_mut().unwrap();
            cell.take().unwrap()(value);
        });
    }
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

pub fn use_ref<T: 'static>(scope: &ScopeState, make_value: impl FnOnce() -> T) -> &T {
    use_ref_with_idx(scope, make_value).0
}

pub fn use_mut<T: 'static>(scope: &ScopeState, make_value: impl FnOnce() -> T) -> Mut<T> {
    let (value, idx) = use_ref_with_idx(scope, || make_value());

    Mut {
        value,
        idx,
        key: Runtime::current().key(),
    }
}

fn use_ref_with_idx<T: 'static>(scope: &ScopeState, make_value: impl FnOnce() -> T) -> (&T, usize) {
    let hooks = unsafe { &mut *scope.hooks.get() };

    let idx = scope.hook_idx.get();
    scope.hook_idx.set(idx + 1);

    let any = if idx >= hooks.len() {
        hooks.push(Box::new(make_value()));
        hooks.last().unwrap()
    } else {
        hooks.get(idx).unwrap()
    };
    (any.downcast_ref().unwrap(), idx)
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

pub unsafe trait StateField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StateField for &T {}

unsafe impl<T: 'static> StateField for Mut<'_, T> {}

pub unsafe trait DataField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: Data> DataField for &&T {}

pub unsafe trait Data {}

unsafe impl Data for () {}

unsafe impl<T: Data> Data for &T {}

unsafe impl Data for Box<dyn AnyCompose + '_> {}

unsafe impl Data for Rc<dyn AnyCompose + '_> {}

pub trait Compose: Data {
    fn compose(cx: Scope<Self>) -> impl Compose;
}

impl Compose for () {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Runtime::current().set_is_empty(true)
    }
}

impl<C: Compose> Compose for &C {
    fn compose(cx: Scope<Self>) -> impl Compose {
        C::compose(Scope {
            me: *cx.me,
            state: cx.state,
        })
    }
}

impl Compose for Box<dyn AnyCompose + '_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me).any_compose(cx.state)
    }
}

impl Compose for Rc<dyn AnyCompose + '_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me).any_compose(cx.state)
    }
}

macro_rules! impl_tuples {
    ($($t:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {}

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            #[allow(non_snake_case)]
            fn compose(cx: Scope<Self>) -> impl Compose {
                let ($($t,)*) = cx.me;

                $(let $t: *const dyn AnyCompose = unsafe { mem::transmute($t as *const dyn AnyCompose) };)*

                Runtime::current()
                    .inner
                    .borrow_mut()
                    .children
                    .extend([
                        $($t),*
                    ]);
            }
        }
    };
}

impl_tuples!(T1);
impl_tuples!(T1, T2);
impl_tuples!(T1, T2, T3);
impl_tuples!(T1, T2, T3, T4);
impl_tuples!(T1, T2, T3, T4, T5);
impl_tuples!(T1, T2, T3, T4, T5, T6);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7);
impl_tuples!(T1, T2, T3, T4, T5, T6, T7, T8);

pub trait AnyCompose {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a>;
}

impl<C: Compose> AnyCompose for C {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a> {
        Box::new(C::compose(Scope { me: self, state }))
    }
}

struct Update {
    key: DefaultKey,
    idx: usize,
    f: Box<dyn FnMut(&mut dyn Any)>,
}

#[derive(Default)]
struct Inner {
    is_empty: bool,
    key: DefaultKey,
    updates: Vec<Update>,
    children: Vec<*const dyn AnyCompose>,
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

    pub fn update(&self, key: DefaultKey, idx: usize, f: impl FnMut(&mut dyn Any) + 'static) {
        self.inner.borrow_mut().updates.push(Update {
            key,
            idx,
            f: Box::new(f),
        });
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

enum NodeCompose {
    Box(Box<dyn AnyCompose>),
    Ptr(*const dyn AnyCompose),
}

impl NodeCompose {
    unsafe fn compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a> {
        match self {
            Self::Box(b) => b.any_compose(state),
            Self::Ptr(p) => (&**p).any_compose(state),
        }
    }
}

struct Node {
    state: ScopeState,
    compose: NodeCompose,
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
            compose: NodeCompose::Box(Box::new(content)),
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

        let mut keys = vec![self.root];

        while let Some(key) = keys.pop() {
            let node = self.nodes.get(key).unwrap();

            self.rt.inner.borrow_mut().key = key;
            let child: Box<dyn AnyCompose> = unsafe { node.compose.compose(&node.state) };

            if self.rt.is_empty() {
                self.rt.set_is_empty(false);
            } else {
                let compose = unsafe { mem::transmute(child) };
                let child_node = Node {
                    state: ScopeState::default(),
                    compose: NodeCompose::Box(compose),
                    children: Vec::new(),
                };

                let child_key = self.nodes.insert(child_node);
                self.nodes.get_mut(key).unwrap().children.push(child_key);
                keys.push(child_key);

                for child_ptr in mem::take(&mut self.rt.inner.borrow_mut().children) {
                    let child_node = Node {
                        state: ScopeState::default(),
                        compose: NodeCompose::Ptr(child_ptr),
                        children: Vec::new(),
                    };

                    let child_key = self.nodes.insert(child_node);
                    self.nodes.get_mut(key).unwrap().children.push(child_key);
                    keys.push(child_key);
                }
            }
        }

        let updates = mem::take(&mut self.rt.inner.borrow_mut().updates);
        for mut update in updates {
            let node = self.nodes.get_mut(update.key).unwrap();
            let value = node.state.hooks.get();
            let value = unsafe { &mut *value };
            let any = value.get_mut(update.idx).unwrap();
            (update.f)(&mut **any);
        }
    }

    pub fn recompose(&mut self) {
        let mut keys = vec![self.root];
        while let Some(key) = keys.pop() {
            let node = self.nodes.get(key).unwrap();
            node.state.hook_idx.set(0);
            let content = unsafe { node.compose.compose(&node.state) };
            // TODO

            keys.extend_from_slice(&node.children);
        }
    }

    fn remove_node(&mut self, key: DefaultKey) {
        let node = self.nodes.remove(key).unwrap();
        for child_key in node.children {
            self.remove_node(child_key);
        }
    }
}

impl Drop for Composer {
    fn drop(&mut self) {
        self.remove_node(self.root);
    }
}
