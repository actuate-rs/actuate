use slotmap::{DefaultKey, SlotMap};
use std::{
    any::{Any, TypeId},
    cell::{RefCell, UnsafeCell},
    collections::HashMap,
    future,
    rc::Rc,
    task::{Poll, Waker},
};

pub mod node;
pub use self::node::Node;

mod use_context;
pub use self::use_context::use_context;

mod use_effect;
pub use self::use_effect::use_effect;

mod use_provider;
pub use self::use_provider::use_provider;

mod use_state;
pub use self::use_state::{use_state, SetState};

pub mod view;
pub use self::view::View;

#[cfg(feature = "web")]
pub mod web;

pub struct ScopeContext {
    value: Box<dyn AnyClone>,
}

impl Clone for ScopeContext {
    fn clone(&self) -> Self {
        Self {
            value: self.value.clone_any_clone(),
        }
    }
}

struct ScopeInner {
    key: DefaultKey,
    tx: UpdateSender,
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: usize,
    contexts: Rc<HashMap<TypeId, ScopeContext>>,
}

pub struct Scope {
    inner: Rc<RefCell<ScopeInner>>,
}

trait AnyClone {
    fn clone_any(&self) -> Box<dyn Any>;

    fn clone_any_clone(&self) -> Box<dyn AnyClone>;
}

impl<T: Clone + 'static> AnyClone for T {
    fn clone_any(&self) -> Box<dyn Any> {
        Box::new(self.clone())
    }

    fn clone_any_clone(&self) -> Box<dyn AnyClone> {
        Box::new(self.clone())
    }
}

struct Update {
    key: DefaultKey,
    idx: usize,
    f: Box<dyn FnMut(&mut dyn Any)>,
}

struct TreeNode {
    node: *const dyn AnyNode,
    state: *mut dyn Any,
    scope: Option<Scope>,
}

pub struct Tree {
    nodes: SlotMap<DefaultKey, TreeNode>,
    tx: UpdateSender,
}

trait AnyNode {
    fn rebuild_any(&self, tree: &mut Tree, state: &mut dyn Any);
}

impl<T: Node> AnyNode for T {
    fn rebuild_any(&self, tree: &mut Tree, state: &mut dyn Any) {
        self.rebuild(tree, state.downcast_mut().unwrap())
    }
}

pub async fn run(view: impl View) {
    let (tx, mut rx) = update_channel();
    let mut tree = Tree {
        nodes: SlotMap::new(),
        tx,
    };

    let node = view.into_node();
    let mut state = node.build(&mut tree, &Rc::default());
    node.init(&mut tree, &mut state);

    while let Some(mut update) = rx.recv().await {
        if let Some(tree_node) = tree.nodes.get(update.key) {
            (update.f)(
                &mut *tree_node
                    .scope
                    .as_ref()
                    .unwrap()
                    .inner
                    .borrow_mut()
                    .hooks
                    .get_mut()[update.idx],
            );

            let node = unsafe { &*tree_node.node };
            let state = unsafe { &mut *tree_node.state };
            node.rebuild_any(&mut tree, state);
        }
    }
}

fn update_channel() -> (UpdateSender, UpdateReceiver) {
    let shared = Rc::new(RefCell::new(Shared {
        updates: Vec::new(),
        waker: None,
    }));
    (
        UpdateSender {
            shared: shared.clone(),
        },
        UpdateReceiver { shared },
    )
}

struct Shared {
    updates: Vec<Update>,
    waker: Option<Waker>,
}

#[derive(Clone)]
struct UpdateSender {
    shared: Rc<RefCell<Shared>>,
}

impl UpdateSender {
    fn send(&self, update: Update) -> Result<(), ()> {
        let mut shared = self.shared.borrow_mut();
        shared.updates.push(update);

        if let Some(waker) = shared.waker.take() {
            waker.wake()
        }

        Ok(())
    }
}

struct UpdateReceiver {
    shared: Rc<RefCell<Shared>>,
}

impl UpdateReceiver {
    async fn recv(&mut self) -> Option<Update> {
        future::poll_fn(|cx| {
            let mut shared = self.shared.borrow_mut();
            shared.waker = Some(cx.waker().clone());

            if let Some(update) = shared.updates.pop() {
                Poll::Ready(Some(update))
            } else {
                Poll::Pending
            }
        })
        .await
    }
}

#[cfg(feature = "web")]
pub struct WebView<V> {
    view: V,
    node: web_sys::Node,
}

#[cfg(feature = "web")]
impl<V: View + Clone> View for WebView<V> {
    fn body(&self, cx: &Scope) -> impl View {
        use_provider(cx, || self.node.clone());

        self.view.clone()
    }
}

#[cfg(feature = "web")]
pub fn mount(view: impl View + Clone, node: web_sys::Node) {
    wasm_bindgen_futures::spawn_local(run(WebView { view, node }))
}

#[macro_export]
macro_rules! clone {
    ($($v:tt),*) => {
        $( let $v = $v.clone(); )*
    };
}
