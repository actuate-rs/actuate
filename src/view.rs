use crate::Context;

pub trait View: PartialEq + 'static {
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
