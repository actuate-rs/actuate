use std::{any::Any, cell::RefCell, rc::Rc};

#[derive(Default)]
struct Inner {
    states: Vec<Box<dyn Any>>,
    idx: usize,
}

#[derive(Clone, Default)]
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

pub fn use_state<T: Clone + 'static>(f: impl FnOnce() -> T) -> T {
    let cx = Context::get();
    let mut cx = cx.inner.borrow_mut();

    let idx = cx.idx;
    cx.idx += 1;

    let state = if let Some(state) = cx.states.get(idx) {
        state
    } else {
        cx.states.push(Box::new(f()));
        cx.states.last().unwrap()
    };
    state.downcast_ref::<T>().unwrap().clone()
}

pub trait View {
    fn view(&self) -> impl View;
}

impl View for () {
    fn view(&self) -> impl View {
        todo!()
    }
}

pub fn run(view: impl View) {
    Context::default().enter();

    view.view();
}
