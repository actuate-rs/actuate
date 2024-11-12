use std::{
    any::Any,
    cell::{Cell, UnsafeCell},
    mem,
    ops::Deref,
};

/// An immutable reference to a value of type `T`.
pub struct Ref<'a, T: ?Sized> {
    value: &'a T,
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

pub trait Compose {
    fn compose(cx: Scope<Self>) -> impl Compose;
}

impl Compose for () {
    fn compose(cx: Scope<Self>) -> impl Compose {
        cx.is_empty.set(true);
    }
}

impl Compose for Box<dyn AnyCompose> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        (**cx.me()).any_compose(cx.state())
    }
}

pub trait AnyCompose {
    fn as_ptr_mut(&mut self) -> *mut ();

    fn any_compose<'a>(&'a self, state: &'a ScopeState);
}

impl<C: Compose> AnyCompose for C {
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
    use crate::{AnyCompose, Compose, Composer};
    use std::{cell::Cell, rc::Rc};

    struct B2 {
        x: Rc<Cell<i32>>,
    }

    impl Compose for B2 {
        fn compose(cx: crate::Scope<Self>) -> impl Compose {
            cx.me().x.set(1);
        }
    }

    #[test]
    fn it_works() {
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: crate::Scope<Self>) -> impl Compose {
                B2 {
                    x: cx.me().x.clone(),
                }
            }
        }

        let x = Rc::new(Cell::new(0));
        Composer::new(Wrap { x: x.clone() }).compose();
        assert_eq!(x.get(), 1);
    }

    #[test]
    fn it_composes_any_compose() {
        struct Wrap {
            x: Rc<Cell<i32>>,
        }

        impl Compose for Wrap {
            fn compose(cx: crate::Scope<Self>) -> impl Compose {
                Box::new(B2 {
                    x: cx.me().x.clone(),
                }) as Box<dyn AnyCompose>
            }
        }

        let x = Rc::new(Cell::new(0));
        Composer::new(Wrap { x: x.clone() }).compose();
        assert_eq!(x.get(), 1);
    }
}
