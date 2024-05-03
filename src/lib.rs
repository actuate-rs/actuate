use std::{any::Any, borrow::Cow, cell::RefCell, collections::HashMap, fmt, mem, rc::Rc};

#[derive(Default)]
struct Inner {
    states: Vec<Box<dyn Any>>,
    idx: usize,
    is_empty: bool,
    child_ids: Vec<u64>,
    pending_children: Vec<Box<dyn AnyView>>,
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

impl<V1: View + Clone, V2: View + Clone> View for (V1, V2) {
    fn view(&self) -> impl View {
        let cx = Context::get();
        let mut cx = cx.inner.borrow_mut();

        cx.pending_children.push(Box::new(self.0.clone()));
        cx.pending_children.push(Box::new(self.1.clone()));
    }
}

pub trait AnyView {
    fn name(&self) -> Cow<'static, str>;

    fn as_any(&self) -> &dyn Any;

    fn view_any(&self) -> Box<dyn AnyView>;
}

impl<V: View> AnyView for V {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(std::any::type_name::<V>())
    }

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
    pending: Vec<(u64, Option<Context>)>,
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
            pending: vec![(0, None)],
        }
    }

    pub fn slice(&self, id: u64) -> Slice {
        Slice { vdom: self, id }
    }

    pub fn run(&mut self) {
        while let Some(pending) = self.pending.pop() {
            let node = self.nodes.get_mut(&pending.0).unwrap();

            if !node.is_init {
                node.scope.clone().enter();
                let content = node.view.view_any();

                let mut new_nodes = Vec::new();
                let children = mem::take(&mut node.scope.inner.borrow_mut().pending_children);
                for child in children {
                    if node.scope.inner.borrow().is_empty {
                        continue;
                    }

                    let child_id = self.next_id;
                    self.next_id += 1;

                    new_nodes.push((
                        child_id,
                        Node {
                            scope: Context::default(),
                            view: child,
                            is_init: false,
                        },
                    ));
                }

                if node.scope.inner.borrow().is_empty {
                    break;
                }

                let content_id = self.next_id;
                self.next_id += 1;

                if let Some(ref parent_scope) = pending.1 {
                    parent_scope.inner.borrow_mut().child_ids.push(pending.0)
                }
                self.pending.push((content_id, Some(node.scope.clone())));

                self.nodes.insert(
                    content_id,
                    Node {
                        scope: Context::default(),
                        view: content,
                        is_init: false,
                    },
                );

                for (id, node) in new_nodes {
                    self.nodes.insert(id, node);
                }
            }
        }
    }
}

pub struct Slice<'a> {
    vdom: &'a VirtualDom,
    id: u64,
}

impl fmt::Debug for Slice<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let node = &self.vdom.nodes[&self.id];
        let scope = node.scope.inner.borrow();

        let mut tuple = f.debug_tuple(&node.view.name());

        for child_id in &scope.child_ids {
            let child_slice = self.vdom.slice(*child_id);
            tuple.field(&child_slice);
        }

        tuple.finish()
    }
}

pub fn run(view: impl View) {
    Context::default().enter();

    view.view();
}
