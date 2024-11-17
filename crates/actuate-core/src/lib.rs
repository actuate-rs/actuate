use slotmap::{DefaultKey, SlotMap};
use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell, UnsafeCell},
    collections::HashMap,
    fmt,
    future::Future,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
    pin::Pin,
    rc::Rc,
    sync::{mpsc, Arc, Mutex},
    task::{Poll, Wake, Waker},
};
use thiserror::Error;

pub use actuate_macros::Data;

pub mod prelude {
    pub use crate::{
        use_context, use_drop, use_memo, use_mut, use_provider, use_ref, Cow, Data, DataField,
        FieldWrap, FnField, Map, Mut, Ref, RefMap, Scope, ScopeState, StateField, StaticField,
    };

    pub use crate::compose::{self, Compose, DynCompose, Memo};
}

pub mod compose;
use self::compose::{AnyCompose, Compose};

mod data;
pub use self::data::{Data, DataField, FieldWrap, FnField, StateField, StaticField};

/// Clone-on-write value.
///
/// This represents either a borrowed or owned value.
/// A borrowed value is stored as a [`RefMap`], which can be either a reference or a mapped reference.
pub enum Cow<'a, T> {
    Borrowed(RefMap<'a, T>),
    Owned(T),
}

impl<'a, T> Cow<'a, T> {
    pub fn into_owned(self) -> T
    where
        T: Clone,
    {
        match self {
            Cow::Borrowed(value) => (*value).clone(),
            Cow::Owned(value) => value,
        }
    }
}

impl<T> Deref for Cow<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            Cow::Borrowed(ref_map) => &*ref_map,
            Cow::Owned(value) => &value,
        }
    }
}

impl<'a, T> From<RefMap<'a, T>> for Cow<'a, T> {
    fn from(value: RefMap<'a, T>) -> Self {
        Cow::Borrowed(value)
    }
}

impl<'a, T> From<Ref<'a, T>> for Cow<'a, T> {
    fn from(value: Ref<'a, T>) -> Self {
        RefMap::from(value).into()
    }
}

impl<'a, T> From<Map<'a, T>> for Cow<'a, T> {
    fn from(value: Map<'a, T>) -> Self {
        RefMap::from(value).into()
    }
}

/// Immutable reference or mapped reference to a value.
pub enum RefMap<'a, T: ?Sized> {
    Ref(Ref<'a, T>),
    Map(Map<'a, T>),
}

impl<T: ?Sized> Clone for RefMap<'_, T> {
    fn clone(&self) -> Self {
        match self {
            RefMap::Ref(r) => RefMap::Ref(r.clone()),
            RefMap::Map(map) => RefMap::Map(map.clone()),
        }
    }
}

impl<T: ?Sized> Deref for RefMap<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        match self {
            RefMap::Ref(r) => &*r,
            RefMap::Map(map) => &*map,
        }
    }
}

impl<T: Hash + ?Sized> Hash for RefMap<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

impl<'a, T: ?Sized> From<Ref<'a, T>> for RefMap<'a, T> {
    fn from(value: Ref<'a, T>) -> Self {
        RefMap::Ref(value)
    }
}

impl<'a, T: ?Sized> From<Map<'a, T>> for RefMap<'a, T> {
    fn from(value: Map<'a, T>) -> Self {
        RefMap::Map(value)
    }
}

unsafe impl<T: Data> Data for RefMap<'_, T> {
    type Id = RefMap<'static, T::Id>;
}

impl<C: Compose> Compose for RefMap<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let state = use_ref(&cx, || {
            let mut state = ScopeData::default();
            state.contexts = cx.contexts.clone();
            state
        });

        state.is_parent_changed.set(cx.is_parent_changed.get());

        unsafe { (**cx.me()).any_compose(state) }
    }
}

/// Mapped immutable reference to a value of type `T`.
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

// Safety: The `Map` is dereferenced every re-compose, so it's guranteed not to point to
// an invalid memory location (e.g. an `Option` that previously returned `Some` is now `None`).
impl<C: Compose> Compose for Map<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let state = use_ref(&cx, || {
            let mut state = ScopeData::default();
            state.contexts = cx.contexts.clone();
            state
        });

        state.is_parent_changed.set(cx.is_parent_changed.get());

        unsafe { (**cx.me()).any_compose(state) }
    }

    #[cfg(feature = "tracing")]
    fn name() -> std::borrow::Cow<'static, str> {
        C::name()
    }
}

/// Immutable reference to a value of type `T`.
#[derive(Hash)]
pub struct Ref<'a, T: ?Sized> {
    value: &'a T,
    generation: *const Cell<u64>,
}

impl<'a, T> Ref<'a, T> {
    /// Map this reference to a value of type `U`.
    pub fn map<U: ?Sized>(me: Self, f: fn(&T) -> &U) -> Map<'a, U> {
        Map {
            ptr: me.value as *const _ as _,
            map_fn: f as _,
            deref_fn: |ptr, g| unsafe {
                let g: fn(&T) -> &U = mem::transmute(g);
                g(&*(ptr as *const T))
            },
        }
    }
}

impl<T: ?Sized> Clone for Ref<'_, T> {
    fn clone(&self) -> Self {
        Self {
            value: self.value,
            generation: self.generation,
        }
    }
}

impl<T: ?Sized> Copy for Ref<'_, T> {}

impl<T: ?Sized> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

impl<T> Memoize for Ref<'_, T> {
    type Value = u64;

    fn memoized(self) -> Self::Value {
        unsafe { &*self.generation }.get()
    }
}

/// Mutable reference to a value of type `T`.
pub struct Mut<'a, T> {
    ptr: *mut T,
    scope_is_changed: *const Cell<bool>,
    generation: *const Cell<u64>,
    phantom: PhantomData<&'a ()>,
}

impl<'a, T: 'static> Mut<'a, T> {
    /// Queue an update to this value, triggering an update to the component owning this value.
    pub fn update(self, f: impl FnOnce(&mut T) + 'static) {
        let ptr = self.ptr;
        let is_changed = self.scope_is_changed;
        let generation = self.generation;

        Runtime::current().update(move || {
            let value = unsafe { &mut *ptr };
            f(value);

            unsafe {
                (*is_changed).set(true);

                let g = &*generation;
                g.set(g.get() + 1)
            }
        });
    }

    /// Queue an update to this value wtihout triggering an update.
    pub fn with(self, f: impl FnOnce(&mut T) + 'static) {
        let mut cell = Some(f);
        let ptr = self.ptr;

        Runtime::current().update(move || {
            let value = unsafe { &mut *ptr };
            cell.take().unwrap()(value);
        });
    }

    /// Convert this mutable reference to an immutable reference.
    pub fn as_ref(self) -> Ref<'a, T> {
        Ref {
            value: unsafe { &*self.ptr },
            generation: self.generation,
        }
    }
}

unsafe impl<T: Send> Send for Mut<'_, T> {}

unsafe impl<T: Sync> Sync for Mut<'_, T> {}

impl<T> Clone for Mut<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            scope_is_changed: self.scope_is_changed,
            generation: self.generation,
            phantom: self.phantom,
        }
    }
}

impl<T> Copy for Mut<'_, T> {}

impl<T> Deref for Mut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.ptr }
    }
}

impl<T> Hash for Mut<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ptr.hash(state);
        self.generation.hash(state);
    }
}

/// An update to apply to a composable.
pub struct Update {
    f: Box<dyn FnOnce()>,
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

/// Runtime for a [`Composer`].
#[derive(Clone)]
pub struct Runtime {
    updater: Arc<dyn Updater>,
    tasks: Rc<RefCell<SlotMap<DefaultKey, Pin<Box<dyn Future<Output = ()>>>>>>,
    task_tx: mpsc::Sender<DefaultKey>,
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
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

/// Map of [`TypeId`] to context values.
#[derive(Clone, Default)]
struct Contexts {
    values: HashMap<TypeId, Rc<dyn Any>>,
}

pub type ScopeState<'a> = &'a ScopeData<'a>;

/// State of a composable.
#[derive(Default)]
pub struct ScopeData<'a> {
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
    is_changed: Cell<bool>,
    is_parent_changed: Cell<bool>,
    is_empty: Cell<bool>,
    is_container: Cell<bool>,
    contexts: RefCell<Contexts>,
    drops: RefCell<Vec<usize>>,
    generation: Cell<u64>,
    _marker: PhantomData<&'a fn(ScopeData<'a>) -> ScopeData<'a>>,
}

impl ScopeData<'_> {
    pub fn set_changed(&self) {
        self.is_changed.set(true);
    }

    pub fn is_parent_changed(&self) -> bool {
        self.is_parent_changed.get()
    }
}

impl Drop for ScopeData<'_> {
    fn drop(&mut self) {
        for idx in &*self.drops.borrow() {
            let hooks = unsafe { &mut *self.hooks.get() };
            let any = hooks.get_mut(*idx).unwrap();
            (**any).downcast_mut::<Box<dyn FnMut()>>().unwrap()();
        }
    }
}

/// Composable scope.
pub struct Scope<'a, C: ?Sized> {
    me: &'a C,
    state: ScopeState<'a>,
}

impl<'a, C> Scope<'a, C> {
    pub fn me(&self) -> Ref<'a, C> {
        Ref {
            value: self.me,
            generation: &self.state.generation,
        }
    }

    pub unsafe fn me_as_ref(self) -> &'a C {
        self.me
    }

    pub fn state(&self) -> &'a ScopeData {
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
    type Target = ScopeState<'a>;

    fn deref(&self) -> &Self::Target {
        &self.state
    }
}

/// Use an immutable reference to a value of type `T`.
///
/// `make_value` will only be called once to initialize this value.
pub fn use_ref<T: 'static>(cx: ScopeState, make_value: impl FnOnce() -> T) -> &T {
    let hooks = unsafe { &mut *cx.hooks.get() };

    let idx = cx.hook_idx.get();
    cx.hook_idx.set(idx + 1);

    let any = if idx >= hooks.len() {
        hooks.push(Box::new(make_value()));
        hooks.last().unwrap()
    } else {
        hooks.get(idx).unwrap()
    };
    (**any).downcast_ref().unwrap()
}

struct MutState<T> {
    value: T,
    generation: Cell<u64>,
}

/// Use a mutable reference to a value of type `T`.
///
/// `make_value` will only be called once to initialize this value.
pub fn use_mut<T: 'static>(cx: ScopeState, make_value: impl FnOnce() -> T) -> Mut<'_, T> {
    let hooks = unsafe { &mut *cx.hooks.get() };

    let idx = cx.hook_idx.get();
    cx.hook_idx.set(idx + 1);

    let any = if idx >= hooks.len() {
        let state = MutState {
            value: make_value(),
            generation: Cell::new(0),
        };
        hooks.push(Box::new(state));
        hooks.last_mut().unwrap()
    } else {
        hooks.get_mut(idx).unwrap()
    };
    let state: &mut MutState<T> = any.downcast_mut().unwrap();

    Mut {
        ptr: &mut state.value as *mut T,
        scope_is_changed: &cx.is_changed,
        generation: &state.generation,
        phantom: PhantomData::<&()>,
    }
}

pub fn use_callback<'a, T, R>(
    cx: ScopeState<'a>,
    f: impl FnMut(T) -> R + 'a,
) -> &'a Rc<dyn Fn(T) -> R + 'a>
where
    T: 'static,
    R: 'static,
{
    let f_cell: Option<Box<dyn FnMut(T) -> R + 'a>> = Some(Box::new(f));
    let mut f_cell: Option<Box<dyn FnMut(T) -> R>> = unsafe { mem::transmute(f_cell) };

    let callback = use_ref(cx, || Rc::new(RefCell::new(f_cell.take().unwrap()))).clone();

    if let Some(f) = f_cell {
        *callback.borrow_mut() = f;
    }

    use_ref(cx, move || {
        let f = callback.clone();
        Rc::new(move |input| f.borrow_mut()(input)) as Rc<dyn Fn(T) -> R>
    })
}

#[derive(Error)]
pub struct ContextError<T> {
    _marker: PhantomData<T>,
}

impl<T> fmt::Debug for ContextError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("ContextError")
            .field(&std::any::type_name::<T>())
            .finish()
    }
}

impl<T> fmt::Display for ContextError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format!(
            "Context value not found for type: {}",
            std::any::type_name::<T>()
        ))
    }
}

/// Use a context value of type `T`.
///
/// # Panics
/// Panics if the context value is not found.
pub fn use_context<T: 'static>(cx: &ScopeData) -> Result<Rc<T>, ContextError<T>> {
    let Some(any) = cx.contexts.borrow().values.get(&TypeId::of::<T>()).cloned() else {
        return Err(ContextError {
            _marker: PhantomData,
        });
    };

    Ok(any.downcast().unwrap())
}

/// Provide a context value of type `T`.
///
/// This value will be available to [`use_context`] to all children of this composable.
pub fn use_provider<'a, T: 'static>(cx: ScopeState<'_>, make_value: impl FnOnce() -> T) -> Rc<T> {
    // TODO
    let r = use_ref(cx, || {
        let value = Rc::new(make_value());
        cx.contexts
            .borrow_mut()
            .values
            .insert(TypeId::of::<T>(), value.clone());
        value
    });
    (*r).clone()
}

pub trait Memoize {
    type Value: PartialEq + 'static;

    fn memoized(self) -> Self::Value;
}

impl<T: PartialEq + 'static> Memoize for T {
    type Value = T;

    fn memoized(self) -> Self::Value {
        self
    }
}

impl<T> Memoize for Mut<'_, T> {
    type Value = u64;

    fn memoized(self) -> Self::Value {
        unsafe { &*self.generation }.get()
    }
}

/// Use a memoized value of type `T` with a dependency of type `D`.
///
/// `make_value` will update the returned value whenver `dependency` is changed.
pub fn use_memo<'a, D, T>(
    cx: ScopeState<'_>,
    dependency: D,
    make_value: impl FnOnce() -> T,
) -> Ref<T>
where
    D: Memoize,
    T: 'static,
{
    let mut dependency_cell = Some(dependency.memoized());

    let mut make_value_cell = Some(make_value);
    let value_mut = use_mut(cx, || make_value_cell.take().unwrap()());

    let hash_mut = use_mut(cx, || dependency_cell.take().unwrap());

    if let Some(make_value) = make_value_cell {
        if let Some(dependency) = dependency_cell.take() {
            if dependency != *hash_mut {
                let value = make_value();
                value_mut.with(move |update| *update = value);

                hash_mut.with(move |dst| *dst = dependency);
            }
        }
    }

    value_mut.as_ref()
}

/// Use a function that will be called when this scope is dropped.
pub fn use_drop<'a>(cx: ScopeState<'a>, f: impl FnOnce() + 'a) {
    let mut f_cell = Some(f);

    let cell = use_ref(cx, || {
        let f: Box<dyn FnOnce()> = Box::new(f_cell.take().unwrap());

        // Safety `f` is guranteed to live as long as `cx`.
        let f: Box<dyn FnOnce()> = unsafe { mem::transmute(f) };

        RefCell::new(Some(f))
    });

    let idx = cx.hook_idx.get();
    use_ref(cx, || {
        cx.drops.borrow_mut().push(idx);

        let f: Box<dyn FnMut()> = Box::new(move || {
            cell.borrow_mut().take().unwrap()();
        });

        // Safety `f` is guranteed to live as long as `cx`.
        let f: Box<dyn FnMut()> = unsafe { mem::transmute(f) };
        f
    });

    if let Some(f) = f_cell {
        let f: Box<dyn FnOnce()> = Box::new(f);

        // Safety `f` is guranteed to live as long as `cx`.
        let f: Box<dyn FnOnce()> = unsafe { mem::transmute(f) };

        *cell.borrow_mut() = Some(f);
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

pub fn use_local_task<'a, F>(cx: ScopeState<'a>, make_task: impl FnOnce() -> F)
where
    F: Future<Output = ()> + 'a,
{
    let key = *use_ref(cx, || {
        let task: Pin<Box<dyn Future<Output = ()>>> = Box::pin(make_task());
        let task: Pin<Box<dyn Future<Output = ()>>> = unsafe { mem::transmute(task) };

        let rt = Runtime::current();
        let key = rt.tasks.borrow_mut().insert(task);
        rt.task_tx.send(key).unwrap();
        key
    });

    use_drop(cx, move || {
        Runtime::current().tasks.borrow_mut().remove(key);
    })
}

struct WrappedFuture {
    lock: Arc<Mutex<bool>>,
    task: Pin<Box<dyn Future<Output = ()> + Send>>,
    rt: Runtime,
}

impl Future for WrappedFuture {
    type Output = ();

    fn poll(
        mut self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        let me = &mut *self;
        let guard = me.lock.lock().unwrap();

        if *guard {
            me.rt.enter();

            me.task.as_mut().poll(cx)
        } else {
            Poll::Ready(())
        }
    }
}

unsafe impl Send for WrappedFuture {}

pub struct RuntimeContext {
    rt: tokio::runtime::Runtime,
}

pub fn use_task<'a, F>(cx: ScopeState<'a>, make_task: impl FnOnce() -> F)
where
    F: Future<Output = ()> + Send + 'a,
{
    let runtime_cx = use_context::<RuntimeContext>(cx).unwrap();
    let lock = use_ref(cx, || {
        let lock = Arc::new(Mutex::new(true));

        let task: Pin<Box<dyn Future<Output = ()> + Send>> = Box::pin(make_task());
        let task: Pin<Box<dyn Future<Output = ()> + Send>> = unsafe { mem::transmute(task) };

        runtime_cx.rt.spawn(WrappedFuture {
            lock: lock.clone(),
            task,
            rt: Runtime::current(),
        });

        lock
    });

    use_drop(cx, || {
        *lock.lock().unwrap() = false;
    });
}

/// Updater for a [`Composer`].
pub trait Updater: Send + Sync {
    fn update(&self, update: Update);
}

struct DefaultUpdater;

impl Updater for DefaultUpdater {
    fn update(&self, update: crate::Update) {
        unsafe {
            update.apply();
        }
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

    /// Create a new [`Composer`] with the given content and default updater.
    pub fn with_updater(content: impl Compose + 'static, updater: impl Updater + 'static) -> Self {
        let updater = Arc::new(updater);
        let (task_tx, task_rx) = mpsc::channel();

        let scope_data = ScopeData::default();
        scope_data.contexts.borrow_mut().values.insert(
            TypeId::of::<RuntimeContext>(),
            Rc::new(RuntimeContext {
                rt: tokio::runtime::Runtime::new().unwrap(),
            }),
        );

        Self {
            compose: Box::new(content),
            scope_state: Box::new(scope_data),
            rt: Runtime {
                updater: updater.clone(),
                tasks: Rc::new(RefCell::new(SlotMap::new())),
                task_tx,
            },
            task_rx,
        }
    }

    /// Compose the content of this composer.
    pub fn compose(&mut self) {
        self.rt.enter();

        while let Ok(key) = self.task_rx.try_recv() {
            let waker = Waker::from(Arc::new(TaskWaker {
                key,
                updater: Runtime::current().updater.clone(),
                tx: self.rt.task_tx.clone(),
            }));
            let mut cx = std::task::Context::from_waker(&waker);

            let mut tasks = self.rt.tasks.borrow_mut();
            let task = tasks.get_mut(key).unwrap();
            let _ = task.as_mut().poll(&mut cx);
        }

        unsafe { self.compose.any_compose(&*self.scope_state) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::*, Composer};
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
