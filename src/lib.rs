use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
    hash::{DefaultHasher, Hash, Hasher},
    mem,
    ops::Deref,
    rc::Rc,
};
use tokio::sync::mpsc;

pub use actuate_macros::Data;

pub struct Ref<'a, T> {
    value: &'a T,
}

impl<T> Clone for Ref<'_, T> {
    fn clone(&self) -> Self {
        Self { value: self.value }
    }
}

impl<T> Copy for Ref<'_, T> {}

impl<'a, T> Deref for Ref<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

#[derive(Hash)]
pub struct Mut<'a, T> {
    ptr: *mut T,
    value: &'a T,
    is_changed: *const Cell<bool>,
}

impl<'a, T: 'static> Mut<'a, T> {
    pub fn update(&self, f: impl FnOnce(&mut T) + 'static) {
        let mut cell = Some(f);
        let ptr = self.ptr;
        let is_changed = self.is_changed;

        Runtime::current().update(move || {
            let value = unsafe { &mut *ptr };
            cell.take().unwrap()(value);

            unsafe {
                (*is_changed).set(true);
            }
        });
    }

    pub fn with(&self, f: impl FnOnce(&mut T) + 'static) {
        let mut cell = Some(f);
        let ptr = self.ptr;

        Runtime::current().update(move || {
            let value = unsafe { &mut *ptr };
            cell.take().unwrap()(value);
        });
    }

    pub fn as_ref(&self) -> Ref<'a, T> {
        Ref { value: self.value }
    }
}

impl<T> Clone for Mut<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            value: self.value,
            is_changed: self.is_changed,
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
    is_changed: Cell<bool>,
}

pub fn use_ref<T: 'static>(scope: &ScopeState, make_value: impl FnOnce() -> T) -> &T {
    let hooks = unsafe { &mut *scope.hooks.get() };

    let idx = scope.hook_idx.get();
    scope.hook_idx.set(idx + 1);

    let any = if idx >= hooks.len() {
        hooks.push(Box::new(make_value()));
        hooks.last().unwrap()
    } else {
        hooks.get(idx).unwrap()
    };
    any.downcast_ref().unwrap()
}

pub fn use_mut<T: 'static>(scope: &ScopeState, make_value: impl FnOnce() -> T) -> Mut<T> {
    let hooks = unsafe { &mut *scope.hooks.get() };

    let idx = scope.hook_idx.get();
    scope.hook_idx.set(idx + 1);

    let any = if idx >= hooks.len() {
        hooks.push(Box::new(make_value()));
        hooks.last_mut().unwrap()
    } else {
        hooks.get_mut(idx).unwrap()
    };
    let value = any.downcast_mut().unwrap();

    Mut {
        ptr: value as *mut T,
        value,
        is_changed: &scope.is_changed,
    }
}

pub fn use_memo<D, T>(scope: &ScopeState, dependency: D, make_value: impl FnOnce() -> T) -> Ref<T>
where
    D: PartialEq + 'static,
    T: 'static,
{
    let mut make_value_cell = Some(make_value);
    let value_mut = use_mut(scope, || make_value_cell.take().unwrap()());

    let mut dependency_cell = Some(dependency);
    let dependency_mut = use_mut(scope, || dependency_cell.take().unwrap());

    if let Some(dependency) = dependency_cell {
        if *dependency_mut != dependency {
            let value = make_value_cell.take().unwrap()();
            value_mut.with(move |update| *update = value);
        }
    }

    value_mut.as_ref()
}

pub struct Scope<'a, C: ?Sized> {
    me: &'a C,
    state: &'a ScopeState,
}

impl<'a, C: ?Sized> Scope<'a, C> {
    pub fn me(&self) -> &C {
        self.me
    }

    pub fn state(&self) -> &'a ScopeState {
        self.state
    }
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

#[derive(Clone, Copy, Default)]
pub struct RebuildContext {
    is_changed: bool,
}

pub trait Node {
    type State: 'static;

    fn build(&self) -> Self::State;

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext);
}

pub struct ComposeNodeState {
    scope: Box<ScopeState>,
    node: Box<dyn AnyNode>,
    node_state: Box<dyn Any>,
}

pub struct ComposeNode<C> {
    compose: C,
}

impl<C: Compose> Node for ComposeNode<C> {
    type State = ComposeNodeState;

    fn build(&self) -> Self::State {
        let scope = Box::new(ScopeState::default());

        let child = C::compose(Scope {
            me: &self.compose,
            state: unsafe { mem::transmute(&*scope) },
        });

        let node: Box<dyn AnyNode> = Box::new(child.into_node());
        let node_state = node.any_build();

        let node = unsafe { mem::transmute(node) };

        ComposeNodeState {
            scope,
            node,
            node_state,
        }
    }

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
        let mut cx = *cx;
        if cx.is_changed || state.scope.is_changed.take() {
            cx.is_changed = true;

            state.scope.hook_idx.set(0);

            let child = C::compose(Scope {
                me: &self.compose,
                state: &state.scope,
            });

            let node: Box<dyn AnyNode> = Box::new(child.into_node());
            state.node = unsafe { mem::transmute(node) };
        }

        state.node.any_rebuild(&mut *state.node_state, &cx);
    }
}

pub trait AnyNode {
    fn any_build(&self) -> Box<dyn Any>;

    fn any_rebuild(&self, state: &mut dyn Any, cx: &RebuildContext);
}

impl<T: Node> AnyNode for T {
    fn any_build(&self) -> Box<dyn Any> {
        Box::new(self.build())
    }

    fn any_rebuild(&self, state: &mut dyn Any, cx: &RebuildContext) {
        self.rebuild(state.downcast_mut().unwrap(), cx)
    }
}

pub trait Compose: Data + Sized {
    fn compose(cx: Scope<Self>) -> impl Compose;

    fn into_node(self) -> impl Node {
        ComposeNode { compose: self }
    }
}

impl Compose for () {
    fn compose(_cx: Scope<Self>) -> impl Compose {}

    fn into_node(self) -> impl Node {}
}

impl Node for () {
    type State = ();

    fn build(&self) -> Self::State {}

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
        let _ = state;
        let _ = cx;
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

pub struct Memo<C> {
    compose: C,
}

impl<C: Compose + Hash> Memo<C> {
    pub fn new(compose: C) -> Self {
        Self { compose }
    }
}

unsafe impl<C: Data> Data for Memo<C> {}

impl<C: Compose + Hash> Compose for Memo<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let _ = cx;
    }

    fn into_node(self) -> impl Node {
        let mut hasher = DefaultHasher::new();
        self.compose.hash(&mut hasher);

        MemoNode {
            hash: hasher.finish(),
            node: self.compose.into_node(),
        }
    }
}
pub struct MemoNode<T> {
    hash: u64,
    node: T,
}

impl<T: Node> Node for MemoNode<T> {
    type State = (u64, T::State);

    fn build(&self) -> Self::State {
        (self.hash, self.node.build())
    }

    fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
        let _ = cx;

        let is_changed = if self.hash != state.0 {
            state.0 = self.hash;
            true
        } else {
            false
        };

        self.node
            .rebuild(&mut state.1, &RebuildContext { is_changed });
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {}

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
               let _ = cx;
            }

            fn into_node(self) -> impl Node {
                ($(self.$idx.into_node(),)*)
            }
        }

        impl<$($t: Node),*> Node for ($($t,)*) {
            type State = ($($t::State,)*);

            fn build(&self) -> Self::State {
                ($(self.$idx.build(),)*)
            }

            fn rebuild(&self, state: &mut Self::State, cx: &RebuildContext) {
                $(self.$idx.rebuild(&mut state.$idx, cx);)*
            }
        }
    };
}

impl_tuples!(T1:0);
impl_tuples!(T1:0, T2:1);
impl_tuples!(T1:0, T2:1, T3:2);
impl_tuples!(T1:0, T2:1, T3:2, T4:3);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6);
impl_tuples!(T1:0, T2:1, T3:2, T4:3, T5:4, T6:5, T7:6, T8:7);

pub trait AnyCompose {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a>;
}

impl<C: Compose> AnyCompose for C {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a> {
        Box::new(C::compose(Scope { me: self, state }))
    }
}

struct Update {
    f: Box<dyn FnMut()>,
}

#[derive(Clone)]
pub struct Runtime {
    tx: mpsc::UnboundedSender<Update>,
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

    pub fn update(&self, f: impl FnMut() + 'static) {
        self.tx.send(Update { f: Box::new(f) }).unwrap();
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

pub struct Composer {
    rt: Runtime,
    rx: mpsc::UnboundedReceiver<Update>,
}

impl Composer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            rt: Runtime { tx },
            rx,
        }
    }

    pub async fn run(&mut self, compose: impl Compose) {
        self.rt.enter();

        let node = compose.into_node();
        let mut state = node.build();

        while let Some(mut update) = self.rx.recv().await {
            (update.f)();

            while let Ok(mut update) = self.rx.try_recv() {
                (update.f)();
            }

            node.rebuild(&mut state, &RebuildContext { is_changed: false });
        }
    }
}

pub async fn run(compose: impl Compose) {
    Composer::new().run(compose).await;
}
