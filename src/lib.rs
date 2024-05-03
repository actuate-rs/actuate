use std::{any::Any, cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default)]
struct Inner {
    states: Vec<Box<dyn Any>>,
    idx: usize,
    is_empty: bool,
    children: Vec<Box<dyn AnyView>>,
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

pub trait View: 'static {
    fn view(&self) -> impl View;
}

impl View for () {
    fn view(&self) -> impl View {
        Context::get().inner.borrow_mut().is_empty = true;
    }
}

pub trait AnyView {
    fn as_any(&self) -> &dyn Any;

    fn view_any(&self) -> Box<dyn AnyView>;
}

impl<V: View> AnyView for V {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn view_any(&self) -> Box<dyn AnyView> {
        Box::new(self.view())
    }
}

struct Node {
    scope: Context,
    view: Box<dyn AnyView>,
    is_init: bool,
}

pub struct VirtualDom {
    next_id: u64,
    nodes: HashMap<u64, Node>,
    pending_id: u64,
}

impl VirtualDom {
    pub fn new(content: impl View) -> Self {
        let view = Box::new(content);
        let node = Node {
            scope: Context::default(),
            view,
            is_init: false,
        };

        let mut nodes = HashMap::new();
        nodes.insert(0, node);

        Self {
            next_id: 1,
            nodes,
            pending_id: 0,
        }
    }

    pub fn run(&mut self) {
        loop {
            let node = self.nodes.get_mut(&self.pending_id).unwrap();

            if !node.is_init {
                node.scope.clone().enter();
                let content = node.view.view_any();

                if node.scope.inner.borrow().is_empty {
                    break;
                }

                let id = self.next_id;
                self.next_id += 1;
                self.pending_id = id;

                self.nodes.insert(
                    id,
                    Node {
                        scope: Context::default(),
                        view: content,
                        is_init: false,
                    },
                );
            }
        }
    }
}

pub fn run(view: impl View) {
    Context::default().enter();

    view.view();
}
