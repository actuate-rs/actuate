use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell, UnsafeCell},
    collections::HashMap,
    fmt,
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
    rc::Rc,
};

pub use actuate_macros::Data;
use thiserror::Error;

pub mod prelude {
    pub use crate::{
        use_context, use_memo, use_mut, use_provider, use_ref, Compose, Data, DataField,
        DynCompose, FieldWrap, FnField, Map, Memo, Mut, Ref, Scope, StateField, StaticField,
    };
}

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
            let mut state = ScopeState::default();
            state.contexts = cx.contexts.clone();
            state
        });

        state.is_parent_changed.set(cx.is_parent_changed.get());

        (**cx.me()).any_compose(state);
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
            let mut state = ScopeState::default();
            state.contexts = cx.contexts.clone();
            state
        });

        state.is_parent_changed.set(cx.is_parent_changed.get());

        (**cx.me()).any_compose(state);
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
    value: &'a T,
    scope_is_changed: *const Cell<bool>,
    generation: *const Cell<u64>,
}

impl<'a, T: 'static> Mut<'a, T> {
    /// Queue an update to this value, triggering an update to the component owning this value.
    pub fn update(self, f: impl FnOnce(&mut T) + 'static) {
        let mut cell = Some(f);
        let ptr = self.ptr;
        let is_changed = self.scope_is_changed;
        let generation = self.generation;

        Runtime::current().update(move || {
            let value = unsafe { &mut *ptr };
            cell.take().unwrap()(value);

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
            value: self.value,
            generation: self.generation,
        }
    }
}

impl<T> Clone for Mut<'_, T> {
    fn clone(&self) -> Self {
        Self {
            ptr: self.ptr,
            value: self.value,
            scope_is_changed: self.scope_is_changed,
            generation: self.generation,
        }
    }
}

impl<T> Copy for Mut<'_, T> {}

impl<T> Deref for Mut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
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
    f: Box<dyn FnMut()>,
}

impl Update {
    /// Apply this update.
    ///
    /// # Safety
    /// The caller must ensure the composable triggering this update still exists.
    pub unsafe fn apply(&mut self) {
        (self.f)();
    }
}

/// Runtime for a [`Composer`].
#[derive(Clone)]
pub struct Runtime {
    updater: Rc<dyn Updater>,
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
    pub fn update(&self, f: impl FnMut() + 'static) {
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

/// State of a composable.
#[derive(Default)]
pub struct ScopeState {
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
    is_changed: Cell<bool>,
    is_parent_changed: Cell<bool>,
    is_empty: Cell<bool>,
    is_container: Cell<bool>,
    contexts: RefCell<Contexts>,
    drops: RefCell<Vec<usize>>,
    generation: Cell<u64>,
}

impl ScopeState {
    pub fn set_changed(&self) {
        self.is_changed.set(true);
    }

    pub fn is_parent_changed(&self) -> bool {
        self.is_parent_changed.get()
    }
}

impl Drop for ScopeState {
    fn drop(&mut self) {
        for idx in &*self.drops.borrow() {
            let hooks = unsafe { &mut *self.hooks.get() };
            hooks
                .get_mut(*idx)
                .unwrap()
                .downcast_mut::<Box<dyn FnMut()>>()
                .unwrap()();
        }
    }
}

/// Composable scope.
pub struct Scope<'a, C: ?Sized> {
    me: &'a C,
    state: &'a ScopeState,
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

/// Use an immutable reference to a value of type `T`.
///
/// `make_value` will only be called once to initialize this value.
pub fn use_ref<T: 'static>(cx: &ScopeState, make_value: impl FnOnce() -> T) -> &T {
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
pub fn use_mut<T: 'static>(cx: &ScopeState, make_value: impl FnOnce() -> T) -> Mut<T> {
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
        value: &state.value,
        scope_is_changed: &cx.is_changed,
        generation: &state.generation,
    }
}

pub fn use_callback<'a, T, R>(
    cx: &'a ScopeState,
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
pub fn use_context<T: 'static>(cx: &ScopeState) -> Result<Rc<T>, ContextError<T>> {
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
pub fn use_provider<T: 'static>(cx: &ScopeState, make_value: impl FnOnce() -> T) -> Rc<T> {
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
pub fn use_memo<D, T>(cx: &ScopeState, dependency: D, make_value: impl FnOnce() -> T) -> Ref<T>
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

pub fn use_drop<'a>(cx: &'a ScopeState, f: impl FnOnce() + 'static) {
    let mut f_cell = Some(f);

    use_ref(cx, || {
        cx.drops.borrow_mut().push(cx.hook_idx.get());
        let f = Box::new(move || {
            f_cell.take().unwrap()();
        }) as Box<dyn FnMut()>;
        f
    });
}

/// Composable data.
///
/// This trait should be derived with `#[derive(Data)]`.
pub unsafe trait Data: Sized {
    type Id: 'static;

    unsafe fn reborrow(self, ptr: *mut ()) {
        let x = ptr as *mut Self;
        *x = self;
    }
}

unsafe impl Data for () {
    type Id = ();
}

unsafe impl Data for String {
    type Id = Self;
}

unsafe impl Data for &str {
    type Id = &'static str;
}

unsafe impl<T: ?Sized + Data> Data for &T {
    type Id = &'static T::Id;
}

unsafe impl<T: Data> Data for Option<T> {
    type Id = Option<T::Id>;
}

unsafe impl<T: Data + ?Sized> Data for Ref<'_, T> {
    type Id = PhantomData<Ref<'static, T::Id>>;
}

unsafe impl<T: Data + ?Sized> Data for Map<'_, T> {
    type Id = PhantomData<Map<'static, T::Id>>;
}

unsafe impl<T: Data> Data for Mut<'_, T> {
    type Id = PhantomData<Mut<'static, T::Id>>;
}

unsafe impl Data for DynCompose<'_> {
    type Id = PhantomData<DynCompose<'static>>;
}

pub struct FieldWrap<T>(pub T);

#[doc(hidden)]
pub unsafe trait StateField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StateField for FieldWrap<&T> {}

#[doc(hidden)]
pub unsafe trait FnField<Marker> {
    fn check(&self) {
        let _ = self;
    }
}

macro_rules! impl_data_for_fns {
    ($($t:tt),*) => {
        unsafe impl<$($t,)* F: Fn($($t,)*)> FnField<fn($($t,)*)> for &FieldWrap<F> {}
    }
}

impl_data_for_fns!();
impl_data_for_fns!(T1);
impl_data_for_fns!(T1, T2);
impl_data_for_fns!(T1, T2, T3);
impl_data_for_fns!(T1, T2, T3, T4);
impl_data_for_fns!(T1, T2, T3, T4, T5);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6, T7);
impl_data_for_fns!(T1, T2, T3, T4, T5, T6, T7, T8);

#[doc(hidden)]
pub unsafe trait DataField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: Data> DataField for &FieldWrap<T> {}

#[doc(hidden)]
pub unsafe trait StaticField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StaticField for &&FieldWrap<T> {}

/// A composable function.
///
/// For a dynamically-typed composable, see [`DynCompose`].
pub trait Compose: Data {
    fn compose(cx: Scope<Self>) -> impl Compose;

    #[cfg(feature = "tracing")]
    #[doc(hidden)]
    fn name() -> std::borrow::Cow<'static, str> {
        std::any::type_name::<Self>().into()
    }
}

impl Compose for () {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_empty.set(true);
    }
}

impl<C: Compose> Compose for &C {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me()).any_compose(&cx);
    }
}

impl<C: Compose> Compose for Option<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let state_cell: &RefCell<Option<ScopeState>> = use_ref(&cx, || RefCell::new(None));

        if let Some(content) = &*cx.me() {
            if let Some(state) = &*state_cell.borrow() {
                state.is_parent_changed.set(cx.is_parent_changed.get());
                content.any_compose(state);
            } else {
                let mut state = ScopeState::default();
                state.contexts = cx.contexts.clone();
                *state_cell.borrow_mut() = Some(state);
                content.any_compose(&*state_cell.borrow().as_ref().unwrap());
            }
        } else {
            *state_cell.borrow_mut() = None;
        }
    }
}

#[derive(Data)]
pub struct Memo<T, C> {
    dependency: T,
    content: C,
}

impl<T, C> Memo<T, C> {
    pub fn new(dependency: impl Memoize<Value = T>, content: C) -> Self {
        Self {
            dependency: dependency.memoized(),
            content,
        }
    }
}

impl<T, C> Compose for Memo<T, C>
where
    T: Clone + Data + PartialEq + 'static,
    C: Compose,
{
    fn compose(cx: Scope<Self>) -> impl Compose {
        let last = use_ref(&cx, RefCell::default);
        let mut last = last.borrow_mut();
        if let Some(last) = &mut *last {
            if cx.me().dependency != *last {
                *last = cx.me().dependency.clone();
                cx.is_parent_changed.set(true);
            }
        } else {
            *last = Some(cx.me().dependency.clone());
            cx.is_parent_changed.set(true);
        }

        Ref::map(cx.me(), |me| &me.content)
    }

    #[cfg(feature = "tracing")]
    fn name() -> std::borrow::Cow<'static, str> {
        format!("Memo<{}>", C::name()).into()
    }
}

/// Dynamically-typed composable.
pub struct DynCompose<'a> {
    compose: UnsafeCell<Option<Box<dyn AnyCompose + 'a>>>,
}

impl<'a> DynCompose<'a> {
    pub fn new(content: impl Compose + 'a) -> Self {
        Self {
            compose: UnsafeCell::new(Some(Box::new(content))),
        }
    }
}

struct DynComposeState {
    compose: Box<dyn AnyCompose>,
    data_id: TypeId,
}

impl<'a> Compose for DynCompose<'a> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_container.set(true);

        let cell: &UnsafeCell<Option<DynComposeState>> = use_ref(&cx, || UnsafeCell::new(None));
        let cell = unsafe { &mut *cell.get() };

        let inner = unsafe { &mut *cx.me().compose.get() };

        let child_state = use_ref(&cx, ScopeState::default);

        *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
        child_state
            .is_parent_changed
            .set(cx.is_parent_changed.get());

        if let Some(any_compose) = inner.take() {
            let mut compose: Box<dyn AnyCompose> = unsafe { mem::transmute(any_compose) };

            if let Some(state) = cell {
                if state.data_id != compose.data_id() {
                    todo!()
                }

                let ptr = (*state.compose).as_ptr_mut();

                unsafe {
                    compose.reborrow(ptr);
                }
            } else {
                *cell = Some(DynComposeState {
                    data_id: compose.data_id(),
                    compose,
                })
            }
        }

        cell.as_mut().unwrap().compose.any_compose(child_state);
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {
            type Id = ($($t::Id,)*);
        }

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
                cx.is_container.set(true);

                $(
                    let state = use_ref(&cx, || {
                        ScopeState::default()
                    });

                    *state.contexts.borrow_mut() = cx.contexts.borrow().clone();
                    state.is_parent_changed.set(cx.is_parent_changed.get());

                    cx.me().$idx.any_compose(state);
                )*
            }

            fn name() -> std::borrow::Cow<'static, str> {
                let mut s = String::from('(');

                $(s.push_str(&$t::name());)*

                s.push(')');
                s.into()
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

trait AnyCompose {
    fn data_id(&self) -> TypeId;

    fn as_ptr_mut(&mut self) -> *mut ();

    unsafe fn reborrow(&mut self, ptr: *mut ());

    fn any_compose<'a>(&'a self, state: &'a ScopeState);

    #[cfg(feature = "tracing")]
    fn name(&self) -> std::borrow::Cow<'static, str>;
}

impl<C> AnyCompose for C
where
    C: Compose + Data,
{
    fn data_id(&self) -> TypeId {
        TypeId::of::<C::Id>()
    }

    fn as_ptr_mut(&mut self) -> *mut () {
        self as *mut Self as *mut ()
    }

    unsafe fn reborrow(&mut self, ptr: *mut ()) {
        std::ptr::swap(self, ptr as _);
    }

    fn any_compose<'a>(&'a self, state: &'a ScopeState) {
        state.hook_idx.set(0);

        let cx = Scope { me: self, state };

        let cell: &UnsafeCell<Option<Box<dyn AnyCompose>>> = use_ref(&cx, || UnsafeCell::new(None));
        let cell = unsafe { &mut *cell.get() };

        let child_state = use_ref(&cx, ScopeState::default);

        if cell.is_none()
            || cx.is_changed.take()
            || cx.is_parent_changed.get()
            || cx.is_container.get()
        {
            #[cfg(feature = "tracing")]
            tracing::trace!("Compose::compose: {}", self.name());

            let child = C::compose(cx);

            cx.is_parent_changed.set(false);
            if cx.state.is_empty.take() {
                return;
            }

            *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();
            child_state.is_parent_changed.set(true);

            unsafe {
                if let Some(ref mut content) = cell {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("Reborrow composable");

                    child.reborrow((**content).as_ptr_mut());
                } else {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("Allocate new composable");

                    let boxed: Box<dyn AnyCompose> = Box::new(child);
                    *cell = Some(mem::transmute(boxed));
                }
            }
        } else {
            child_state.is_parent_changed.set(false);

            #[cfg(feature = "tracing")]
            tracing::trace!("Skip: {}", self.name());
        }

        let child = cell.as_mut().unwrap();
        (*child).any_compose(child_state);
    }

    #[cfg(feature = "tracing")]
    fn name(&self) -> std::borrow::Cow<'static, str> {
        format!("Memo<{}>", C::name()).into()
    }
}

/// Updater for a [`Composer`].
pub trait Updater {
    fn update(&self, update: Update);
}

struct DefaultUpdater;

impl Updater for DefaultUpdater {
    fn update(&self, mut update: crate::Update) {
        unsafe {
            update.apply();
        }
    }
}

/// Composer for composable content.
pub struct Composer {
    compose: Box<dyn AnyCompose>,
    scope_state: Box<ScopeState>,
    rt: Runtime,
}

impl Composer {
    /// Create a new [`Composer`] with the given content and default updater.
    pub fn new(content: impl Compose + 'static) -> Self {
        Self::with_updater(content, DefaultUpdater)
    }

    /// Create a new [`Composer`] with the given content and default updater.
    pub fn with_updater(content: impl Compose + 'static, updater: impl Updater + 'static) -> Self {
        let updater = Rc::new(updater);
        Self {
            compose: Box::new(content),
            scope_state: Box::new(ScopeState::default()),
            rt: Runtime {
                updater: updater.clone(),
            },
        }
    }

    /// Compose the content of this composer.
    pub fn compose(&mut self) {
        self.rt.enter();

        self.compose.any_compose(&Scope {
            me: &self.compose,
            state: &self.scope_state,
        });
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
