use slotmap::{DefaultKey, SlotMap};
use std::{any::Any, mem};

pub trait View: 'static {
    fn body(&self) -> impl ViewBuilder;
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
            f: |view: &'static V| view.body().into_tree(),
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

#[derive(Default)]
pub struct Context {
    nodes: SlotMap<DefaultKey, *const dyn AnyView>,
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
    F: Fn(&'static V) -> B,
{
    type State = (DefaultKey, B::State);

    fn build(&mut self, cx: &mut Context) -> Self::State {
        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let body = (self.f)(view_ref);
        self.body = Some(body);
        let body_state = self.body.as_mut().unwrap().build(cx);

        let key = cx.nodes.insert(&self.view as *const V);
        (key, body_state)
    }

    fn rebuild(&mut self, cx: &mut Context, state: &mut Self::State) {
        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let body = (self.f)(view_ref);
        self.body = Some(body);

        self.body.as_mut().unwrap().rebuild(cx, &mut state.1);

        let node = cx.nodes.get_mut(state.0).unwrap();
        *node = &self.view as _;
    }
}

pub fn virtual_dom(view: impl ViewBuilder) -> VirtualDom<impl Tree> {
    VirtualDom {
        tree: view.into_tree(),
        state: None,
        cx: Context::default(),
    }
}

pub struct VirtualDom<T> {
    tree: T,
    state: Option<Box<dyn Any>>,
    cx: Context,
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
