use std::{cell::RefCell, collections::HashMap, fmt, mem, rc::Rc};
use tokio::sync::mpsc;

use crate::{AnyView, Context, Inner, Update, UpdateKind, View};

struct Node {
    scope: Context,
    view: Box<dyn AnyView>,
    next_view: Option<Box<dyn AnyView>>,
    is_init: bool,
    updates: Vec<(usize, UpdateKind)>,
}

pub struct VirtualDom {
    next_id: u64,
    nodes: HashMap<u64, Node>,
    pending: Vec<(u64, Option<Context>, bool)>,
    tx: mpsc::UnboundedSender<Update>,
    rx: mpsc::UnboundedReceiver<Update>,
    is_init: bool,
}

impl VirtualDom {
    pub fn new(content: impl View) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        let view = Box::new(content);
        let node = Node {
            scope: Context {
                inner: Rc::new(RefCell::new(Inner {
                    id: 0,
                    states: Default::default(),
                    idx: Default::default(),
                    is_empty: Default::default(),
                    content_id: None,
                    child_ids: Default::default(),
                    pending_children: Default::default(),
                    tx: tx.clone(),
                })),
            },
            view,
            next_view: None,
            is_init: false,
            updates: Vec::new(),
        };

        let mut nodes = HashMap::new();
        nodes.insert(0, node);

        Self {
            next_id: 1,
            nodes,
            pending: vec![(0, None, false)],
            tx,
            rx,
            is_init: false,
        }
    }

    pub fn slice(&self, id: u64) -> Slice {
        Slice { vdom: self, id }
    }

    pub async fn run(&mut self) {
        if self.is_init {
            let update = self.rx.recv().await.unwrap();
            if let Some(node) = self.nodes.get_mut(&update.id) {
                node.updates.push((update.idx, update.kind))
            }
        } else {
            self.is_init = true;
        }

        self.run_inner();

        self.pending.push((0, None, false));
    }

    fn run_inner(&mut self) {
        while let Some(pending) = self.pending.pop() {
            let node = self.nodes.get_mut(&pending.0).unwrap();

            if !node.is_init {
                node.is_init = true;

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
                            scope: Context {
                                inner: Rc::new(RefCell::new(Inner {
                                    id: 0,
                                    states: Default::default(),
                                    idx: Default::default(),
                                    is_empty: Default::default(),
                                    content_id: None,
                                    child_ids: Default::default(),
                                    pending_children: Default::default(),
                                    tx: self.tx.clone(),
                                })),
                            },
                            view: child,
                            next_view: None,
                            is_init: false,
                            updates: Vec::new(),
                        },
                    ));
                }

                if node.scope.inner.borrow().is_empty {
                    break;
                }

                let content_id = self.next_id;
                self.next_id += 1;

                if let Some(ref parent_scope) = pending.1 {
                    if pending.2 {
                        parent_scope.inner.borrow_mut().content_id = Some(pending.0);
                    } else {
                        parent_scope.inner.borrow_mut().child_ids.push(pending.0)
                    }
                }
                self.pending
                    .push((content_id, Some(node.scope.clone()), true));

                self.nodes.insert(
                    content_id,
                    Node {
                        scope: Context {
                            inner: Rc::new(RefCell::new(Inner {
                                id: 0,
                                states: Default::default(),
                                idx: Default::default(),
                                is_empty: Default::default(),
                                content_id: None,
                                child_ids: Default::default(),
                                pending_children: Default::default(),
                                tx: self.tx.clone(),
                            })),
                        },
                        view: content,
                        next_view: None,
                        is_init: false,
                        updates: Vec::new(),
                    },
                );

                for (id, node) in new_nodes {
                    self.pending.push((id, Some(node.scope.clone()), false));
                    self.nodes.insert(id, node);
                }
            } else {
                let mut new = Vec::new();
                if !node.updates.is_empty() {
                    for (idx, update) in node.updates.drain(..) {
                        let mut scope = node.scope.inner.borrow_mut();
                        let state = &mut scope.states[idx];
                        match update {
                            UpdateKind::Value(any) => *state = any,
                            UpdateKind::Setter(mut f) => f(&mut *state),
                        }

                        scope.idx = 0;
                        drop(scope);

                        node.scope.clone().enter();
                        let content = node.view.view_any();
                        let scope = node.scope.inner.borrow_mut();
                        let content_id = scope.content_id.unwrap();
                        self.pending.push((content_id, None, true));

                        new.push((content_id, content));
                    }
                } else if let Some(next_view) = node.next_view.take() {
                    if !node.view.any_eq(next_view.as_any()) {
                        node.view = next_view;

                        node.scope.clone().enter();
                        let content = node.view.view_any();
                        let scope = node.scope.inner.borrow_mut();
                        if let Some(content_id) = scope.content_id {
                            self.pending.push((content_id, None, true));

                            new.push((content_id, content));
                        }
                    }
                }

                for (id, content) in new {
                    let content_node = self.nodes.get_mut(&id).unwrap();
                    content_node.next_view = Some(content);
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

        if let Some(content_id) = scope.content_id {
            let content_slice = self.vdom.slice(content_id);
            tuple.field(&content_slice);
        }

        for child_id in &scope.child_ids {
            let child_slice = self.vdom.slice(*child_id);
            tuple.field(&child_slice);
        }

        tuple.finish()
    }
}
