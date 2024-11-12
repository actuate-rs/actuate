use std::{
    any::{Any, TypeId},
    cell::{Cell, UnsafeCell},
    hash::{Hash, Hasher},
    marker::PhantomData,
    mem,
    ops::Deref,
};

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

#[derive(Default)]
pub struct ScopeState {
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
    is_empty: Cell<bool>,
}
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

    fn data_id() -> Self::Id;
}

unsafe impl Data for () {
    type Id = ();

    fn data_id() -> Self::Id {}
}

pub struct RefMarker;

unsafe impl<T: Data + ?Sized> Data for Ref<'_, T> {
    type Id = (RefMarker, T::Id);

    fn data_id() -> Self::Id {
        (RefMarker, T::data_id())
    }
}

unsafe impl Data for DynCompose<'_> {
    type Id = PhantomData<DynCompose<'static>>;

    fn data_id() -> Self::Id {
        PhantomData
    }
}

pub trait Compose: Data {
    fn compose(cx: Scope<Self>) -> impl Compose;
}

impl Compose for () {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_empty.set(true);
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
        C::data_id().type_id()
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
}

impl Composer {
    pub fn new(content: impl Compose + 'static) -> Self {
        Self {
            compose: Box::new(content),
            scope_state: ScopeState::default(),
        }
    }

    pub fn compose(&mut self) {
        self.compose.any_compose(&Scope {
            me: &self.compose,
            state: &self.scope_state,
        });
    }
}

#[cfg(test)]
mod tests {
    use crate::{Compose, Composer, Data, DynCompose};
    use std::{cell::Cell, marker::PhantomData, rc::Rc};

    struct Counter {
        x: Rc<Cell<i32>>,
    }

    unsafe impl Data for Counter {
        type Id = PhantomData<Self>;

        fn data_id() -> Self::Id {
            PhantomData
        }
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
            type Id = PhantomData<Self>;

            fn data_id() -> Self::Id {
                PhantomData
            }
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
            type Id = PhantomData<Self>;

            fn data_id() -> Self::Id {
                PhantomData
            }
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
