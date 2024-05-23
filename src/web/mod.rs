use crate::{use_context, Scope, View};

pub struct Div {}

impl View for Div {
    fn body(&self, cx: &Scope) -> impl View {
        let node: web_sys::Node = use_context(cx);

        let element = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .create_element("div")
            .unwrap();
        node.append_child(&element);
    }
}
