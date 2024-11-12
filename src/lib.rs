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

pub mod native;

/// A mapped immutable reference to a value of type `T`.
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

/// An immutable reference to a value of type `T`.
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

pub struct Update {
    f: Box<dyn FnMut()>,
}

#[derive(Clone)]
pub struct Runtime {
    updater: Rc<dyn Updater>,
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
        self.updater.update(Update { f: Box::new(f) });
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

#[derive(Clone, Default)]
struct Contexts {
    values: HashMap<TypeId, Rc<dyn Any>>,
}

#[derive(Default)]
pub struct ScopeState {
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
    is_changed: Cell<bool>,
    is_parent_changed: Cell<bool>,
    is_empty: Cell<bool>,
    contexts: RefCell<Contexts>,
}

impl ScopeState {
    pub fn set_changed(&self) {
        self.is_changed.set(true);
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

pub fn use_context<T: 'static>(scope: &ScopeState) -> Rc<T> {
    scope
        .contexts
        .borrow()
        .values
        .get(&TypeId::of::<T>())
        .unwrap()
        .clone()
        .downcast()
        .unwrap()
}

pub fn use_provider<T: 'static>(scope: &ScopeState, make_value: impl FnOnce() -> T) -> Rc<T> {
    // TODO
    let r = use_ref(scope, || {
        let value = Rc::new(make_value());
        scope
            .contexts
            .borrow_mut()
            .values
            .insert(TypeId::of::<T>(), value.clone());
        value
    });
    (*r).clone()
}

pub fn use_memo<D, T>(scope: &ScopeState, dependency: D, make_value: impl FnOnce() -> T) -> Ref<T>
where
    D: Hash,
    T: 'static,
{
    let mut hasher = DefaultHasher::new();
    dependency.hash(&mut hasher);
    let hash = hasher.finish();

    let mut make_value_cell = Some(make_value);
    let value_mut = use_mut(scope, || make_value_cell.take().unwrap()());

    let hash_mut = use_mut(scope, || hash);

    if let Some(make_value) = make_value_cell {
        if hash != *hash_mut {
            let value = make_value();
            value_mut.with(move |update| *update = value);

            hash_mut.with(move |dst| *dst = hash);
        }
    }

    value_mut.as_ref()
}

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

pub trait Compose: Data {
    fn compose(cx: Scope<Self>) -> impl Compose;
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
}

enum DynComposeInner<'a> {
    Boxed(Box<dyn AnyCompose + 'a>),
    Ptr(*const dyn AnyCompose),
}

pub struct DynCompose<'a> {
    compose: UnsafeCell<Option<DynComposeInner<'a>>>,
}

impl<'a> DynCompose<'a> {
    pub fn new(content: impl Compose + 'a) -> Self {
        Self {
            compose: UnsafeCell::new(Some(DynComposeInner::Boxed(Box::new(content)))),
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

        let inner = unsafe { &mut *cx.me().compose.get() }.take().unwrap();

        let child_state = use_ref(&cx, || ScopeState {
            contexts: cx.state.contexts.clone(),
            ..Default::default()
        });

        match inner {
            DynComposeInner::Boxed(any_compose) => {
                let mut compose: Box<dyn AnyCompose> = unsafe { mem::transmute(any_compose) };

                let ptr = if let Some(state) = cell {
                    if state.data_id != compose.data_id() {
                        todo!()
                    }

                    let ptr = (*state.compose).as_ptr_mut();

                    unsafe {
                        compose.reborrow(ptr);
                    }

                    ptr
                } else {
                    let ptr = (*compose).as_ptr_mut();
                    *cell = Some(DynComposeState {
                        data_id: compose.data_id(),
                        compose,
                    });
                    ptr
                };

                cell.as_mut().unwrap().compose.any_compose(child_state);

                *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();

                *unsafe { &mut *cx.me().compose.get() } = Some(DynComposeInner::Ptr(ptr));
            }
            DynComposeInner::Ptr(ptr) => {
                *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();

                unsafe { &*ptr }.any_compose(child_state);
            }
        }
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {
            type Id = ($($t::Id,)*);
        }

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
                $(cx.me().$idx.any_compose(use_ref(&cx, || ScopeState {
                    contexts: cx.contexts.clone(),
                    is_parent_changed: cx.is_parent_changed.clone(),
                    ..Default::default()
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

        let child_state = use_ref(&cx, || ScopeState {
            contexts: state.contexts.clone(),
            ..Default::default()
        });

        if cell.is_none() || cx.is_changed.take() || cx.is_parent_changed.get() {
            let child = C::compose(cx);

            *child_state.contexts.borrow_mut() = cx.contexts.borrow().clone();

            unsafe {
                if let Some(ref mut content) = cell {
                    child.reborrow((**content).as_ptr_mut());
                } else {
                    let boxed: Box<dyn AnyCompose> = Box::new(child);
                    *cell = Some(mem::transmute(boxed));
                }
            }

            if cx.state.is_empty.take() {
                cx.state.is_empty.set(false);
                return;
            }

            child_state.is_parent_changed.set(true);
        }

        let child = cell.as_mut().unwrap();
        (**child).any_compose(child_state);
    }
}

pub trait Updater {
    fn update(&self, update: Update);
}

pub struct Composer {
    compose: Box<dyn AnyCompose>,
    scope_state: Box<ScopeState>,
    rt: Runtime,
}

impl Composer {
    pub fn new(content: impl Compose + 'static, updater: impl Updater + 'static) -> Self {
        let updater = Rc::new(updater);
        Self {
            compose: Box::new(content),
            scope_state: Box::new(ScopeState::default()),
            rt: Runtime {
                updater: updater.clone(),
            },
        }
    }

    pub fn compose(&mut self) {
        self.rt.enter();

        self.compose.any_compose(&Scope {
            me: &self.compose,
            state: &self.scope_state,
        });
    }
}

/*
#[cfg(test)]
mod tests {
    use crate::{Compose, Composer, Data, DynCompose};
    use std::{cell::Cell, rc::Rc};

    struct Counter {
        x: Rc<Cell<i32>>,
    }

    unsafe impl Data for Counter {
        type Id = Self;
    }

    impl Compose for Counter {
        fn compose(cx: crate::Scope<Self>) -> impl Compose {
            cx.me().x.set(cx.me().x.get() + 1);

            cx.set_changed();
        }
    }


    #[test]
    fn it_works() {
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        unsafe impl Data for Wrap {
            type Id = Self;
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
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        unsafe impl Data for Wrap {
            type Id = Self;
        }

        impl Compose for Wrap {
            fn compose(cx: crate::Scope<Self>) -> impl Compose {
                DynCompose::new(Counter {
                    x: cx.me().x.clone(),
                })
            }
        }

        let x = Rc::new(Cell::new(0));
        let mut composer = Composer::new(Wrap { x: x.clone() }, updater);

        composer.compose();
        assert_eq!(x.get(), 1);

        composer.compose();
        assert_eq!(x.get(), 2);
    }

}
*/
