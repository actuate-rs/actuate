use slotmap::{DefaultKey, SlotMap};
use std::{
    any::Any,
    cell::{RefCell, UnsafeCell},
    marker::PhantomData,
    mem, ptr,
    rc::Rc,
};
use tokio::sync::mpsc;

struct ScopeInner {
    key: DefaultKey,
    tx: mpsc::UnboundedSender<Update>,
    hooks: UnsafeCell<Vec<Box<dyn Any>>>,
    hook_idx: usize,
}

pub struct Scope {
    inner: Rc<RefCell<ScopeInner>>,
}

pub fn use_state<T: 'static>(cx: &Scope, make_value: impl FnOnce() -> T) -> (&T, SetState<T>) {
    let mut scope = cx.inner.borrow_mut();
    let idx = scope.hook_idx;
    let hooks = unsafe { &mut *scope.hooks.get() };

    let value = if let Some(hook) = hooks.get(idx) {
        scope.hook_idx += 1;
        hook
    } else {
        let hooks = unsafe { &mut *scope.hooks.get() };
        hooks.push(Box::new(make_value()));
        hooks.last().unwrap()
    };

    let setter = SetState {
        key: scope.key,
        tx: scope.tx.clone(),
        idx,
        _marker: PhantomData,
    };

    (value.downcast_ref().unwrap(), setter)
}

pub struct SetState<T> {
    key: DefaultKey,
    tx: mpsc::UnboundedSender<Update>,
    idx: usize,
    _marker: PhantomData<fn(T)>,
}

impl<T> SetState<T>
where
    T: 'static,
{
    pub fn modify(&self, f: impl FnOnce(&mut T) + 'static) {
        let mut f_cell = Some(f);
        self.tx
            .send(Update {
                key: self.key,
                idx: self.idx,
                f: Box::new(move |any| f_cell.take().unwrap()(any.downcast_mut().unwrap())),
            })
            .unwrap();
    }

    pub fn set(&self, value: T) {
        self.modify(move |target| *target = value)
    }
}

pub trait View: 'static {
    fn body(&self, cx: &Scope) -> impl View;

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
        ViewNode {
            view: self,
            body_fn: |me: &'static Self, cx: &'static Scope| me.body(cx).into_node(),
            _marker: PhantomData,
        }
    }

    fn memo(self) -> Memo<Self>
    where
        Self: PartialEq + Clone,
    {
        Memo { view: self }
    }
}

impl View for () {
    fn body(&self, _cx: &Scope) -> impl View {}

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
    }
}

struct Update {
    key: DefaultKey,
    idx: usize,
    f: Box<dyn FnMut(&mut dyn Any)>,
}

struct TreeNode {
    node: *const dyn AnyNode,
    scope: Option<Scope>,
}

pub struct Tree {
    nodes: SlotMap<DefaultKey, TreeNode>,
    tx: mpsc::UnboundedSender<Update>,
}

pub trait AnyNode {
    fn rebuild_any(&self, tree: &mut Tree, state: &mut dyn Any);
}

impl<T: Node> AnyNode for T {
    fn rebuild_any(&self, tree: &mut Tree, state: &mut dyn Any) {
        self.rebuild(tree, state.downcast_mut().unwrap())
    }
}

pub trait Node: 'static {
    type State;

    fn build(&self, tree: &mut Tree) -> Self::State;

    fn init(&self, tree: &mut Tree, state: &mut Self::State);

    fn rebuild(&self, tree: &mut Tree, state: &mut Self::State);
}

impl Node for () {
    type State = ();

    fn build(&self, _tree: &mut Tree) -> Self::State {}

    fn init(&self, _tree: &mut Tree, _statee: &mut Self::State) {}

    fn rebuild(&self, _tree: &mut Tree, _statee: &mut Self::State) {}
}

pub struct ViewNode<V, F, B> {
    view: V,
    body_fn: F,
    _marker: PhantomData<fn() -> B>,
}

impl<V, F, B> Node for ViewNode<V, F, B>
where
    V: View,
    F: Fn(&'static V, &'static Scope) -> B + 'static,
    B: Node,
{
    type State = (B, B::State, DefaultKey);

    fn build(&self, tree: &mut Tree) -> Self::State {
        let view = unsafe { mem::transmute(&self.view) };

        let key = tree.nodes.insert(TreeNode {
            node: ptr::null::<Self>(),
            scope: None,
        });
        let scope = Scope {
            inner: Rc::new(RefCell::new(ScopeInner {
                key,
                tx: tree.tx.clone(),
                hooks: UnsafeCell::default(),
                hook_idx: 0,
            })),
        };
        let scope_ref = unsafe { mem::transmute(&scope) };

        let body = (self.body_fn)(view, scope_ref);
        let body_state = body.build(tree);

        tree.nodes[key].scope = Some(scope);

        (body, body_state, key)
    }

    fn init(&self, tree: &mut Tree, state: &mut Self::State) {
        tree.nodes[state.2].node = self as _;

        state.0.init(tree, &mut state.1);
    }

    fn rebuild(&self, tree: &mut Tree, state: &mut Self::State) {
        let tree_node = &mut tree.nodes[state.2];
        tree_node.node = self as _;

        let scope = tree_node.scope.as_ref().unwrap();
        scope.inner.borrow_mut().hook_idx = 0;

        let scope_ref = unsafe { mem::transmute(scope) };
        let view = unsafe { mem::transmute(&self.view) };

        let body = (self.body_fn)(view, scope_ref);
        body.rebuild(tree, &mut state.1);
    }
}

pub struct Memo<V> {
    view: V,
}

impl<V: View + PartialEq + Clone> View for Memo<V> {
    fn body(&self, _cx: &Scope) -> impl View {}

    fn into_node(self) -> impl Node
    where
        Self: Sized,
    {
        MemoNode {
            view: self.view.clone(),
            node: self.view.into_node(),
        }
    }
}

pub struct MemoNode<V, N> {
    view: V,
    node: N,
}

impl<V: View + PartialEq + Clone, N: Node> Node for MemoNode<V, N> {
    type State = (V, N::State);

    fn build(&self, tree: &mut Tree) -> Self::State {
        (self.view.clone(), self.node.build(tree))
    }

    fn init(&self, tree: &mut Tree, state: &mut Self::State) {
        self.node.init(tree, &mut state.1)
    }

    fn rebuild(&self, tree: &mut Tree, state: &mut Self::State) {
        if self.view != state.0 {
            state.0 = self.view.clone();

            self.node.rebuild(tree, &mut state.1)
        }
    }
}

pub async fn run(view: impl View) {
    let (tx, mut rx) = mpsc::unbounded_channel();
    let mut tree = Tree {
        nodes: SlotMap::new(),
        tx,
    };

    let node = view.into_node();
    let mut state = node.build(&mut tree);
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
            node.rebuild_any(&mut tree, &mut state);
        }
    }
}
