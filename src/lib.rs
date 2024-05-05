use slotmap::{DefaultKey, SlotMap, SparseSecondaryMap};
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

pub struct Context {
    nodes: SlotMap<DefaultKey, *const dyn AnyView>,
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
            let state = self.tree.build(&mut self.cx);
            self.state = Some(Box::new(state));
        }
    }
}
