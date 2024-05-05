use slotmap::{DefaultKey, SlotMap};
use std::mem;

pub trait View: 'static {
    fn body(&self) -> impl ViewBuilder;
}

pub trait ViewBuilder {
    fn into_tree(self) -> impl Tree;
}

impl ViewBuilder for () {
    fn into_tree(self) -> impl Tree {
        
    }
}

impl<V: View> ViewBuilder for V {
    fn into_tree(self) -> impl Tree {
        ViewTree {
            view: self,
            body: None,
            f: |view: &'static V| view.body(),
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
    fn build(&mut self, cx: &mut Context);
}

impl Tree for () {
    fn build(&mut self, cx: &mut Context) {
        
    }
}

pub struct ViewTree<V, B, F> {
    view: V,
    body: Option<B>,
    f: F,
}

impl<V, B, F> Tree for ViewTree<V, B, F>
where
    V: View,
    B: 'static,
    F: Fn(&'static V) -> B,
{
    fn build(&mut self, cx: &mut Context) {
        cx.nodes.insert(&self.view as *const V);

        let view_ref: &'static V = unsafe { mem::transmute(&self.view) };
        let body = (self.f)(view_ref);
        self.body = Some(body);
    }
}

pub fn virtual_dom(view: impl ViewBuilder) -> VirtualDom<impl Tree> {
    VirtualDom {
        tree: view.into_tree(),
        cx: Context::default(),
    }
}

pub struct VirtualDom<T> {
    tree: T,
    cx: Context,
}

impl<T> VirtualDom<T> {
    pub fn build(&mut self)
    where
        T: Tree,
    {
        self.tree.build(&mut self.cx);
    }
}
