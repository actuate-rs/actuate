use std::task::Poll;
use crate::{
    node::{Change, Element, ViewContext},
    Node, View,
};

pub struct Div {}

impl Div {
    pub fn new() -> Self {
        Self {}
    }
}

impl View for Div {
    fn body(&self, cx: &crate::Scope) -> impl View {}

    fn into_node(self) -> impl Node {
        self
    }
}

pub struct DivElement {
    element: Option<web_sys::Element>,
}

impl Element for DivElement {
    fn remove(&self) -> Option<Vec<Change>> {
        todo!()
    }
}

impl Node for Div {
    type Element = DivElement;

    fn build(&self) -> Self::Element {
        DivElement { element: None }
    }

    fn poll_ready(
        &self,
        cx: &mut std::task::Context,
        element: &mut Self::Element,
        is_changed: bool,
    ) -> Poll<()> {
        if let Some(ref elem) = element.element {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }

    fn view(&self, cx: &mut ViewContext, element: &mut Self::Element) -> Option<Vec<Change>> {
        if let Some(ref elem) = element.element {
            None
        } else {
            let document = web_sys::window().unwrap().document().unwrap();
            let elem = document.create_element("div").unwrap();
            element.element = Some(elem.clone());

            let node: &web_sys::Node = &*elem;

            Some(vec![Change::Push(Box::new(node.clone()))])
        }
    }
}
