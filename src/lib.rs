use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
    hash::{Hash, Hasher},
    mem,
    ops::Deref,
};

use compose::AnyNode;
use tokio::sync::mpsc;

pub mod compose;
use self::compose::RebuildContext;
pub use self::compose::{AnyCompose, Compose, Data, DataField, Memo, StateField};

pub mod native;

pub use actuate_macros::Data;

pub struct Map<'a, T: ?Sized> {
    ptr: *const (),
    map_fn: *const (),
    deref_fn: fn(*const (), *const ()) -> &'a T,
}

impl<T: ?Sized> Clone for Map<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            map_fn: self.map_fn,
            deref_fn: self.deref_fn,
        }
    }
}

impl<T: ?Sized> Copy for Map<'_, T> {}

impl<'a, T: ?Sized> Deref for Map<'a, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        (self.deref_fn)(self.ptr, self.map_fn)
    }
}

impl<T: Hash + ?Sized> Hash for Map<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

pub struct Ref<'a, T> {
    value: &'a T,
}

impl<'a, T> Ref<'a, T> {
    pub fn map<U: ?Sized>(self, f: fn(&T) -> &U) -> Map<'a, U> {
        Map {
            ptr: self.value as *const _ as _,
            map_fn: f as _,
            deref_fn: |ptr, g| unsafe {
                let g: fn(&T) -> &U = mem::transmute(g);
                g(&*(ptr as *const T))
            },
        }
    }
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

struct UseDrop {
    f: Box<dyn FnMut()>,
}

impl Drop for UseDrop {
    fn drop(&mut self) {
        (self.f)()
    }
}

pub fn use_drop<'a>(scope: &'a ScopeState, f: impl FnOnce() + 'a) {
    let mut f_cell = Some(f);

    let f: Box<dyn FnMut() + 'a> = Box::new(move || {
        f_cell.take().unwrap()();
    });
    let f: Box<dyn FnMut() + 'static> = unsafe { mem::transmute(f) };

    use_ref(scope, move || UseDrop { f });
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

pub struct Scope<'a, C> {
    me: &'a C,
    state: &'a ScopeState,
}

impl<'a, C> Scope<'a, C> {
    pub fn me(&self) -> Ref<'a, C> {
        Ref { value: self.me }
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
    node: Box<dyn AnyNode>,
    state: Option<Box<dyn Any>>,
}

impl Composer {
    pub fn new(compose: impl Compose + 'static) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            rt: Runtime { tx },
            rx,
            node: Box::new(compose.into_node()),
            state: None,
        }
    }

    pub async fn run(&mut self) {
        self.rt.enter();

        let mut state = self.node.any_build();

        while let Some(mut update) = self.rx.recv().await {
            (update.f)();

            while let Ok(mut update) = self.rx.try_recv() {
                (update.f)();
            }

            self.node
                .any_rebuild(&mut state, &RebuildContext { is_changed: false });
        }
    }

    pub fn build(&mut self) {
        self.rt.enter();

        let state = self.node.any_build();
        self.state = Some(state);
    }

    pub fn rebuild(&mut self) {
        let state = self.state.as_mut().unwrap();
        self.node
            .any_rebuild(&mut **state, &RebuildContext { is_changed: false });
    }
}

pub async fn run(compose: impl Compose + 'static) {
    Composer::new(compose).run().await;
}
