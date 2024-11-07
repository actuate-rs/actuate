use std::{marker::PhantomData, mem};

pub struct ScopeState {
    component: Component<'static>,
    next: Option<Box<Self>>,
}

impl ScopeState {
    pub fn new(component: Component<'_>) -> Self {
        Self {
            component: unsafe { mem::transmute(component) },
            next: None,
        }
    }

    pub fn run(&mut self) {
        let next = (self.component.f)(self.component.props as _, self as *const _ as _);

        if let Some(next) = next {
            let next_scope = Self {
                component: next,
                next: None,
            };
            self.next = Some(Box::new(next_scope));
            self.next.as_mut().unwrap().run();
        }
    }
}

pub struct Scope<'a, T> {
    props: &'a T,
    state: &'a ScopeState,
}

impl<T> Clone for Scope<'_, T> {
    fn clone(&self) -> Self {
        Self {
            props: self.props,
            state: self.state,
        }
    }
}

impl<T> Copy for Scope<'_, T> {}

pub struct Component<'a> {
    props: *mut (),
    f: Box<dyn Fn(*const (), *const ()) -> Option<Self>>,
    _marker: PhantomData<fn(&'a ScopeState)>,
}

impl<'a> Component<'a> {
    pub fn new<'b, T>(props: T, f: fn(Scope<'a, T>) -> Option<Component<'b>>) -> Self
    where
        'a: 'b,
        T: 'a,
    {
        let f: Box<dyn Fn(_, _) -> Option<Self>> =
            Box::new(move |props_ptr: *const (), state_ptr: *const ()| {
                let scope = Scope {
                    props: unsafe { &*(props_ptr as *const _) },
                    state: unsafe { *(state_ptr as *const _) },
                };
                unsafe { mem::transmute(f(scope)) }
            });

        Self {
            props: Box::into_raw(Box::new(props)) as _,
            f: unsafe { mem::transmute(f) },
            _marker: PhantomData,
        }
    }
}

impl Drop for Component<'_> {
    fn drop(&mut self) {
        drop(unsafe { Box::from_raw(self.props) })
    }
}

fn app(scope: Scope<String>) -> Option<Component> {
    dbg!(scope.props);

    Some(Component::new(scope.props, |scope: Scope<&String>| {
        dbg!(scope.props);

        None
    }))
}

fn main() {
    let mut scope = ScopeState::new(Component::new(String::from("A"), app));
    scope.run();
}
