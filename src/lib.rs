use slotmap::{DefaultKey, SlotMap};
use std::{cell::RefCell, mem, rc::Rc};

pub struct ScopeState {}

pub struct Scoped<'a, C: ?Sized> {
    pub me: &'a C,
    pub state: &'a ScopeState,
}

impl<C> Clone for Scoped<'_, C> {
    fn clone(&self) -> Self {
        Self {
            me: self.me,
            state: self.state,
        }
    }
}

impl<C> Copy for Scoped<'_, C> {}

pub type Scope<'a, C> = Scoped<'a, C>;

pub trait Compose {
    fn compose(cx: Scope<Self>) -> impl Compose;
}

impl Compose for () {
    fn compose(_cx: Scope<Self>) -> impl Compose {
        Runtime::current().set_is_empty(true)
    }
}

pub trait AnyCompose {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a>;
}

impl<C: Compose> AnyCompose for C {
    fn any_compose<'a>(&'a self, state: &'a ScopeState) -> Box<dyn AnyCompose + 'a> {
        Box::new(C::compose(Scoped { me: self, state }))
    }
}

#[derive(Default)]
struct Inner {
    is_empty: bool,
}

#[derive(Clone, Default)]
pub struct Runtime {
    inner: Rc<RefCell<Inner>>,
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

    pub fn is_empty(&self) -> bool {
        self.inner.borrow().is_empty
    }

    pub fn set_is_empty(&self, is_empty: bool) {
        self.inner.borrow_mut().is_empty = is_empty;
    }
}

thread_local! {
    static RUNTIME: RefCell<Option<Runtime>> = RefCell::new(None);
}

struct Node {
    state: ScopeState,
    compose: Box<dyn AnyCompose>,
    children: Vec<DefaultKey>,
}

pub struct Composer {
    rt: Runtime,
    nodes: SlotMap<DefaultKey, Node>,
    root: DefaultKey,
}

impl Composer {
    pub fn new(content: impl Compose + 'static) -> Self {
        let node = Node {
            state: ScopeState {},
            compose: Box::new(content),
            children: Vec::new(),
        };

        let mut nodes = SlotMap::new();
        let root = nodes.insert(node);

        Self {
            rt: Runtime::default(),
            nodes,
            root,
        }
    }

    pub fn compose(&mut self) {
        self.rt.enter();

        let mut key = self.root;

        loop {
            let node = self.nodes.get(key).unwrap();
            let child: Box<dyn AnyCompose> = node.compose.any_compose(&node.state);

            if self.rt.is_empty() {
                self.rt.set_is_empty(false);
                break;
            } else {
                let compose = unsafe { mem::transmute(child) };
                let child_node = Node {
                    state: ScopeState {},
                    compose,
                    children: Vec::new(),
                };

                let child_key = self.nodes.insert(child_node);
                self.nodes.get_mut(key).unwrap().children.push(child_key);
                key = child_key;
            }
        }
    }
}
