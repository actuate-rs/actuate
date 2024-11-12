use std::{
    any::{Any, TypeId},
    cell::{Cell, RefCell, UnsafeCell},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
};
use tokio::sync::mpsc;

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

#[derive(Default)]
pub struct ScopeState {
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
    is_empty: Cell<bool>,
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
    any.downcast_ref().unwrap()
}

pub unsafe trait Data {
    type Id: 'static;
}

unsafe impl Data for () {
    type Id = ();
}

unsafe impl<T: ?Sized + Data> Data for &T {
    type Id = PhantomData<&'static T::Id>;
}

unsafe impl<T: Data + ?Sized> Data for Ref<'_, T> {
    type Id = PhantomData<Ref<'static, T::Id>>;
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
        cx.me().any_compose(&cx);
    }
}

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

        let compose = unsafe { &mut *cx.me().compose.get() }.take().unwrap();
        let compose: Box<dyn AnyCompose> = unsafe { mem::transmute(compose) };

        if let Some(state) = cell {
            if state.data_id != compose.data_id() {
                todo!()
            }

            unsafe { *(&mut *(state.compose.as_ptr_mut() as *mut _)) = compose }
        } else {
            *cell = Some(DynComposeState {
                data_id: compose.data_id(),
                compose,
            });
        }

        cell.as_mut().unwrap().compose.any_compose(cx.state);
    }
}

macro_rules! impl_tuples {
    ($($t:tt : $idx:tt),*) => {
        unsafe impl<$($t: Data),*> Data for ($($t,)*) {
            type Id = ($($t::Id,)*);
        }

        impl<$($t: Compose),*> Compose for ($($t,)*) {
            fn compose(cx: Scope<Self>) -> impl Compose {
                $(cx.me().$idx.any_compose(use_ref(&cx, ScopeState::default));)*
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

    fn any_compose<'a>(&'a self, state: &'a ScopeState) {
        let cx = Scope { me: self, state };

        let cell: &UnsafeCell<Option<Box<dyn AnyCompose>>> = use_ref(&cx, || UnsafeCell::new(None));
        let cell = unsafe { &mut *cell.get() };

        let child = C::compose(cx);
        unsafe {
            if let Some(ref mut content) = cell {
                *(&mut *(content.as_ptr_mut() as *mut _)) = child
            } else {
                let boxed: Box<dyn AnyCompose> = Box::new(child);
                *cell = Some(mem::transmute(boxed));
            }
        }

        if cx.state.is_empty.get() {
            cx.state.is_empty.set(false);
            return;
        }

        let child_state = use_ref(&cx, || ScopeState::default());
        let child = cell.as_mut().unwrap();
        (**child).any_compose(child_state);
    }
}

pub struct Composer {
    compose: Box<dyn AnyCompose>,
    scope_state: ScopeState,
    rt: Runtime,
    rx: mpsc::UnboundedReceiver<Update>,
}

impl Composer {
    pub fn new(content: impl Compose + 'static) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            compose: Box::new(content),
            scope_state: ScopeState::default(),
            rt: Runtime { tx },
            rx,
        }
    }

    pub fn compose(&mut self) {
        self.rt.enter();

        self.compose.any_compose(&Scope {
            me: &self.compose,
            state: &self.scope_state,
        });

        while let Ok(mut update) = self.rx.try_recv() {
            (update.f)();
        }
    }
}

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
        let mut composer = Composer::new(Wrap { x: x.clone() });

        composer.compose();
        assert_eq!(x.get(), 1);

        composer.compose();
        assert_eq!(x.get(), 2);
    }
}
