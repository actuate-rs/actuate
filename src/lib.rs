use std::{any::Any, cell::RefCell, rc::Rc};
use tokio::sync::mpsc;

mod any_view;
pub use self::any_view::AnyView;

pub mod vdom;
pub use self::vdom::VirtualDom;

mod view;
pub use self::view::View;

mod use_state;
pub use self::use_state::{use_state, SetState};

enum UpdateKind {
    Value(Box<dyn Any>),
    Setter(Box<dyn FnMut(&mut dyn Any)>),
}

struct Update {
    id: u64,
    idx: usize,
    kind: UpdateKind,
}

struct Inner {
    id: u64,
    states: Vec<Box<dyn Any>>,
    idx: usize,
    is_empty: bool,
    child_ids: Vec<u64>,
    pending_children: Vec<Box<dyn AnyView>>,
    tx: mpsc::UnboundedSender<Update>,
}

#[derive(Clone)]
pub struct Context {
    inner: Rc<RefCell<Inner>>,
}

impl Context {
    pub fn enter(self) {
        CONTEXT.set(Some(self));
    }

    pub fn get() -> Self {
        CONTEXT.with(|cell| cell.borrow().as_ref().unwrap().clone())
    }
}

thread_local! {
    static CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
}
