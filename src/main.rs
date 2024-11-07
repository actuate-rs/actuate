use std::{
    any::Any,
    cell::{Cell, RefCell, UnsafeCell},
    marker::PhantomData,
    mem,
    ops::Deref,
};

struct A<'a> {
    name: &'a String,
    count: &'a State<i32>,
}

fn a<'a>(scope: Scope<'a, A<'a>>) -> Option<Component<'a>> {
    dbg!(scope.props.name);

    dbg!(*scope.props.count.borrow());

    scope.props.count.update(|x| *x += 1);

    None
}

fn app(scope: Scope<String>) -> Option<Component> {
    dbg!(scope.props);

    let count = scope.use_state(|| 0);

    Some(Component::new(
        A {
            name: scope.props,
            count,
        },
        a,
    ))
}

fn main() {
    let mut scope = ScopeState::new(Component::new(String::from("A"), app));
    scope.run();
    scope.run();
}

pub struct ScopeState {
    component: Component<'static>,
    next: Option<Box<Self>>,
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: Cell<usize>,
    updates: Updates,
}

impl ScopeState {
    pub fn new(component: Component<'_>) -> Self {
        Self {
            component: unsafe { mem::transmute(component) },
            next: None,
            hooks: UnsafeCell::new(Vec::new()),
            hook_idx: Cell::new(0),
            updates: Updates::default(),
        }
    }

    pub fn run(&mut self) {
        let next = (self.component.f)(self.component.props as _, self as *const _ as _);

        self.hook_idx.set(0);
        for mut f in mem::take(&mut *self.updates.fns.borrow_mut()) {
            f();
        }

        if let Some(next) = next {
            let next_scope = Self {
                component: next,
                next: None,
                hooks: UnsafeCell::new(Vec::new()),
                hook_idx: Cell::new(0),
                updates: Updates::default(),
            };
            self.next = Some(Box::new(next_scope));
            self.next.as_mut().unwrap().run();
        }
    }
}

#[derive(Default)]
struct Updates {
    fns: RefCell<Vec<Box<dyn FnMut()>>>,
}

pub struct Ref<'a, T> {
    value: &'a T,
}

impl<T: 'static> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value
    }
}

pub struct State<T> {
    value: T,
    updates: *const Updates,
    ptr: *mut Self,
}

impl<T: 'static> State<T> {
    pub fn borrow(&self) -> Ref<T> {
        Ref { value: &self.value }
    }

    pub fn update(&self, f: impl FnOnce(&mut T) + 'static) {
        let updates = unsafe { &mut *(self.updates as *mut Updates) };
        let mut cell = Some(f);
        let ptr = self.ptr;
        updates.fns.borrow_mut().push(Box::new(move || {
            let f = cell.take().unwrap();
            let input = unsafe { &mut *ptr };
            f(&mut input.value)
        }));
    }
}

pub struct Scope<'a, T> {
    props: &'a T,
    state: &'a ScopeState,
}

impl<'a, T> Scope<'a, T> {
    pub fn use_state<S: 'static>(&self, make_state: impl FnOnce() -> S) -> &'a State<S> {
        let hooks = unsafe { &mut *self.state.hooks.get() };

        let idx = self.state.hook_idx.get();
        if idx < hooks.len() {
            self.state.hook_idx.set(idx + 1);

            hooks[idx].downcast_ref().unwrap()
        } else {
            self.state.hook_idx.set(idx + 1);

            hooks.push(Box::new(State {
                value: make_state(),
                updates: &self.state.updates,
                ptr: std::ptr::null_mut(),
            }));

            let hook_ref = hooks
                .last_mut()
                .unwrap()
                .downcast_mut::<State<S>>()
                .unwrap();
            let ptr = hook_ref as *mut _;
            hook_ref.ptr = ptr;
            hook_ref
        }
    }
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
                    props: unsafe { &*(props_ptr as *const T) },
                    state: unsafe { &*(state_ptr as *const ScopeState) },
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
