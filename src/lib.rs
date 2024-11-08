use std::mem;

pub struct ScopeState {}

pub struct Scoped<'a, C: ?Sized> {
    pub me: &'a C,
    pub state: &'a ScopeState,
}

pub type Scope<'a, C> = &'a Scoped<'a, C>;

pub trait Compose {
    fn compose(cx: Scope<Self>) -> impl Compose;
}

pub trait AnyCompose {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a>;
}

impl<C: Compose> AnyCompose for C {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a> {
        let scoped: Scope<'a, C> = unsafe { mem::transmute(&Scoped { me: self, state }) };
        Box::new(C::compose(scoped))
    }
}
