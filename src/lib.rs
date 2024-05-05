use slotmap::{DefaultKey, SlotMap, SparseSecondaryMap};
use std::fmt;
use std::{any::Any, cell::UnsafeCell};
use tokio::sync::mpsc;

mod use_state;
pub use self::use_state::{use_state, Setter};

mod tree;
pub use self::tree::{Tree, ViewTree};

mod view;
pub use self::view::View;

mod view_builder;
pub use self::view_builder::ViewBuilder;

struct Inner {
    hooks: Vec<Box<dyn Any>>,
    idx: usize,
}

pub struct Scope {
    key: DefaultKey,
    inner: UnsafeCell<Inner>,
    tx: mpsc::UnboundedSender<Update>,
}

trait AnyView {
    fn name(&self) -> &'static str;
}

impl<VB: ViewBuilder> AnyView for VB {
    fn name(&self) -> &'static str {
        std::any::type_name::<VB>()
    }
}

struct Node {
    view: *const dyn AnyView,
    children: Vec<DefaultKey>,
}

pub struct Context {
    nodes: SlotMap<DefaultKey, Node>,
    tx: mpsc::UnboundedSender<Update>,
    pending_updates: SparseSecondaryMap<DefaultKey, Vec<Update>>,
}

pub fn virtual_dom(view: impl ViewBuilder) -> VirtualDom<impl Tree> {
    let (tx, rx) = mpsc::unbounded_channel();

    VirtualDom {
        tree: view.into_tree(),
        state: None,
        cx: Context {
            nodes: SlotMap::new(),
            tx,
            pending_updates: SparseSecondaryMap::new(),
        },
        roots: Vec::new(),
        rx,
    }
}

struct Update {
    key: DefaultKey,
    idx: usize,
    value: Box<dyn Any>,
}

pub struct VirtualDom<T> {
    tree: T,
    state: Option<Box<dyn Any>>,
    cx: Context,
    rx: mpsc::UnboundedReceiver<Update>,
    roots: Vec<DefaultKey>,
}

impl<T> VirtualDom<T> {
    pub async fn run(&mut self)
    where
        T: Tree,
    {
        if let Some(ref mut state) = self.state {
            // Wait for at least one update.
            let update = self.rx.recv().await.unwrap();

            if let Some(updates) = self.cx.pending_updates.get_mut(update.key) {
                updates.push(update);
            } else {
                self.cx.pending_updates.insert(update.key, vec![update]);
            }

            // Flush any pending updates.
            while let Ok(update) = self.rx.try_recv() {
                if let Some(updates) = self.cx.pending_updates.get_mut(update.key) {
                    updates.push(update);
                } else {
                    self.cx.pending_updates.insert(update.key, vec![update]);
                }
            }

            self.tree
                .rebuild(&mut self.cx, state.downcast_mut().unwrap())
        } else {
            let state = self.tree.build(&mut self.cx, &mut self.roots);
            self.state = Some(Box::new(state));
        }
    }

    pub fn slice(&self, key: DefaultKey) -> Slice<T> {
        Slice {
            vdom: self,
            node: self.cx.nodes.get(key).unwrap(),
        }
    }
}

impl<T> fmt::Debug for VirtualDom<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut t = f.debug_tuple("VirtualDom");

        for key in &self.roots {
            t.field(&self.slice(*key));
        }

        t.finish()
    }
}

pub struct Slice<'a, T> {
    vdom: &'a VirtualDom<T>,
    node: &'a Node,
}

impl<T> fmt::Debug for Slice<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let view = unsafe { &*self.node.view };
        let mut t = f.debug_tuple(view.name());

        for child_key in &self.node.children {
            t.field(&self.vdom.slice(*child_key));
        }

        t.finish()
    }
}
