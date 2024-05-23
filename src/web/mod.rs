use crate::{use_context, use_effect, use_provider, use_state, Scope, View};
use std::borrow::Cow;

pub fn div<C: View + Clone>(content: C) -> Div<C> {
    Div { content }
}

pub struct Div<C> {
    content: C,
}

impl<C: View + Clone> View for Div<C> {
    fn body(&self, cx: &Scope) -> impl View {
        let parent: web_sys::Node = use_context(cx);

        let (node, _) = use_state(cx, || {
            let element = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_element("div")
                .unwrap();
            parent.append_child(&element).unwrap();
        });

        use_provider(cx, || node.clone());

        self.content.clone()
    }
}

pub fn text(content: impl Into<Cow<'static, str>>) -> Text {
    Text {
        content: content.into(),
    }
}

#[derive(Clone)]
pub struct Text {
    content: Cow<'static, str>,
}

impl View for Text {
    fn body(&self, cx: &Scope) -> impl View {
        let parent: web_sys::Node = use_context(cx);

        use_effect(cx, (), || {
            let node = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_text_node(&self.content);
            parent.append_child(&node).unwrap();
        });
    }
}
