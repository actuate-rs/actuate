use slotmap::{DefaultKey, SlotMap};
use std::{any::Any, cell::UnsafeCell, marker::PhantomData, mem};
use tokio::sync::mpsc;

struct Inner {
    hooks: Vec<Box<dyn Any>>,
    idx: usize,
}

pub struct Scope {
    key: DefaultKey,
    inner: UnsafeCell<Inner>,
    tx: mpsc::UnboundedSender<Update>,
}

pub struct Setter<T> {
    key: DefaultKey,
    idx: usize,
    tx: mpsc::UnboundedSender<Update>,
    _marker: PhantomData<T>,
}

impl<T> Setter<T> {
    pub fn set(&self, value: T)
    where
        T: 'static,
    {
        self.tx
            .send(Update {
                key: self.key,
                idx: self.idx,
                value: Box::new(value),
            })
            .unwrap();
    }
}

pub fn use_state<T: 'static>(cx: &Scope, make_value: impl FnOnce() -> T) -> (&T, Setter<T>) {
    let cx_ref = unsafe { &mut *cx.inner.get() };

    let idx = cx_ref.idx;
    cx_ref.idx += 1;

    let value = if let Some(any) = cx_ref.hooks.get(idx) {
        any.downcast_ref().unwrap()
    } else {
        let cx = unsafe { &mut *cx.inner.get() };

        cx.hooks.push(Box::new(make_value()));
        cx.hooks.last().unwrap().downcast_ref().unwrap()
    };

    let setter = Setter {
        key: cx.key,
        idx,
        tx: cx.tx.clone(),
        _marker: PhantomData,
    };

    (value, setter)
}

pub trait View: 'static {
    fn body(&self, cx: &Scope) -> impl ViewBuilder;
}

pub trait ViewBuilder {
    fn into_tree(self) -> impl Tree;
}

impl ViewBuilder for () {
    fn into_tree(self) -> impl Tree {}
}

impl<V: View> ViewBuilder for V {
    fn into_tree(self) -> impl Tree {
        ViewTree {
            view: self,
            body: None,
            f: |view: &'static V, cx: &'static Scope| view.body(cx).into_tree(),
        }
    }
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
}

pub trait Tree {
    type State: 'static;

    fn build(&mut self, cx: &mut Context) -> Self::State;

    fn rebuild(&mut self, cx: &mut Context, state: &mut Self::State);
}

impl Tree for () {
    type State = ();

    fn build(&mut self, cx: &mut Context) -> Self::State {}

    fn rebuild(&mut self, cx: &mut Context, state: &mut Self::State) {}
}

pub struct ViewTree<V, B, F> {
    view: V,
    body: Option<B>,
    f: F,
}

impl<V, B, F> Tree for ViewTree<V, B, F>
where
    V: View,
    B: Tree + 'static,
    F: Fn(&'static V, &'static Scope) -> B,
{
    type State = (DefaultKey, Box<Scope>, B::State);

    fn build(&mut self, cx: &mut Context) -> Self::State {
        let key = cx.nodes.insert(&self.view as *const V);

        let scope = Box::new(Scope {
            key,
            inner: UnsafeCell::new(Inner {
                hooks: Vec::new(),
                idx: 0,
            }),
            tx: cx.tx.clone(),
        });

        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let scope_ref: &'static Scope = unsafe { mem::transmute(&*scope) };
        let body = (self.f)(view_ref, scope_ref);
        self.body = Some(body);

        let body_state = self.body.as_mut().unwrap().build(cx);

        (key, scope, body_state)
    }

    fn rebuild(&mut self, cx: &mut Context, state: &mut Self::State) {
        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let scope_ref: &'static Scope = unsafe { mem::transmute(&*state.1) };
        let body = (self.f)(view_ref, scope_ref);
        self.body = Some(body);

        self.body.as_mut().unwrap().rebuild(cx, &mut state.2);

        let node = cx.nodes.get_mut(state.0).unwrap();
        *node = &self.view as _;
    }
}

pub fn virtual_dom(view: impl ViewBuilder) -> VirtualDom<impl Tree> {
    let (tx, rx) = mpsc::unbounded_channel();

    VirtualDom {
        tree: view.into_tree(),
        state: None,
        cx: Context {
            nodes: SlotMap::new(),
            tx,
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
    pub fn run(&mut self)
    where
        T: Tree,
    {
        if let Some(ref mut state) = self.state {
            self.tree
                .rebuild(&mut self.cx, state.downcast_mut().unwrap())
        } else {
            let state = self.tree.build(&mut self.cx);
            self.state = Some(Box::new(state));
        }
    }
}
