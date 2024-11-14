use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell, UnsafeCell},
    collections::HashMap,
    hash::{DefaultHasher, Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
    rc::Rc,
};

pub use actuate_macros::Data;

pub mod prelude {
    pub use crate::{
        use_context, use_memo, use_mut, use_provider, use_ref, Compose, Data, DataField,
        DynCompose, Map, Mut, Ref, Scope, StateField,
    };
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

/// Immutable reference to a value of type `T`.
pub struct Ref<'a, T: ?Sized> {
    value: &'a T,
}

impl<'a, T> Ref<'a, T> {
    /// Map this reference to a value of type `U`.
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

impl<T: ?Sized> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

/// Mutable reference to a value of type `T`.
#[derive(Hash)]
pub struct Mut<'a, T> {
    ptr: *mut T,
    value: &'a T,
    is_changed: *const Cell<bool>,
}

impl<'a, T: 'static> Mut<'a, T> {
    /// Queue an update to this value, triggering an update to the component owning this value.
    pub fn update(self, f: impl FnOnce(&mut T) + 'static) {
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
}

impl ScopeState {
    pub fn set_changed(&self) {
        self.is_changed.set(true);
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
        Ref { value: self.me }
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

/// Use a mutable reference to a value of type `T`.
///
/// `make_value` will only be called once to initialize this value.
pub fn use_mut<T: 'static>(cx: &ScopeState, make_value: impl FnOnce() -> T) -> Mut<T> {
    let hooks = unsafe { &mut *cx.hooks.get() };

    let idx = cx.hook_idx.get();
    cx.hook_idx.set(idx + 1);

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
        is_changed: &cx.is_changed,
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

/// Use a context value of type `T`.
///
/// # Panics
/// Panics if the context value is not found.
pub fn use_context<T: 'static>(cx: &ScopeState) -> Rc<T> {
    let Some(any) = cx.contexts.borrow().values.get(&TypeId::of::<T>()).cloned() else {
        panic!(
            "Context value not found for type: {}",
            std::any::type_name::<T>()
        );
    };

    any.downcast().unwrap()
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

/// Use a memoized value of type `T` with a dependency of type `D`.
///
/// `make_value` will update the returned value whenver `dependency` is changed.
pub fn use_memo<D, T>(cx: &ScopeState, dependency: D, make_value: impl FnOnce() -> T) -> Ref<T>
where
    D: Hash,
    T: 'static,
{
    let mut hasher = DefaultHasher::new();
    dependency.hash(&mut hasher);
    let hash = hasher.finish();

    let mut make_value_cell = Some(make_value);
    let value_mut = use_mut(cx, || make_value_cell.take().unwrap()());

    let hash_mut = use_mut(cx, || hash);

    if let Some(make_value) = make_value_cell {
        if hash != *hash_mut {
            let value = make_value();
            value_mut.with(move |update| *update = value);

            hash_mut.with(move |dst| *dst = hash);
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
    type Id = PhantomData<&'static T::Id>;
}

unsafe impl<T: Data + ?Sized> Data for Ref<'_, T> {
    type Id = PhantomData<Ref<'static, T::Id>>;
}

unsafe impl<T: Data + ?Sized> Data for Map<'_, T> {
    type Id = PhantomData<Map<'static, T::Id>>;
}

unsafe impl Data for DynCompose<'_> {
    type Id = PhantomData<DynCompose<'static>>;
}

#[doc(hidden)]
pub unsafe trait StateField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: 'static> StateField for &T {}

unsafe impl<T: 'static> StateField for Mut<'_, T> {}

#[doc(hidden)]
pub unsafe trait DataField {
    fn check(&self) {
        let _ = self;
    }
}

unsafe impl<T: Data> DataField for &&T {}

/// A composable function.
pub trait Compose: Data {
    fn compose(cx: Scope<Self>) -> impl Compose;

    #[cfg(feature = "tracing")]
    #[doc(hidden)]
    fn name() -> &'static str {
        std::any::type_name::<Self>()
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

impl<C: Compose> Compose for Map<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me()).any_compose(&cx);
    }

    #[cfg(feature = "tracing")]
    fn name() -> &'static str {
        C::name()
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
                use_ref(&cx, || {
                    cx.is_container.set(true);
                });

                $(cx.me().$idx.any_compose(use_ref(&cx, || {
                    let mut state = ScopeState::default();
                    state.contexts=  cx.contexts.clone();
                    state.is_parent_changed = cx.is_parent_changed.clone();
                    state
                }));)*
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
    fn name(&self) -> &'static str;
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
            tracing::info!("Compose::compose: {}", self.name());

            let child = C::compose(cx);

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
            tracing::info!("Skip: {}", self.name());
        }

        let child = cell.as_mut().unwrap();
        (*child).any_compose(child_state);
    }

    #[cfg(feature = "tracing")]
    fn name(&self) -> &'static str {
        C::name()
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
    use std::{cell::Cell, rc::Rc};

    #[derive(Data)]
    struct Counter {
        x: Rc<Cell<i32>>,
    }

    impl Compose for Counter {
        fn compose(cx: crate::Scope<Self>) -> impl Compose {
            cx.me().x.set(cx.me().x.get() + 1);

            cx.set_changed();
        }
    }

    #[test]
    fn it_works() {
        #[derive(Data)]
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: crate::Scope<Self>) -> impl Compose {
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
}
