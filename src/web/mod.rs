use js_sys::wasm_bindgen::{closure::Closure, JsCast};

use crate::{use_context, use_effect, use_provider, use_state, Scope, View};
use std::{
    borrow::Cow,
    cell::RefCell,
    future::{Future, IntoFuture},
    rc::Rc,
};

pub trait HandlerOutput<Marker> {
    fn run(self) -> impl Future<Output = ()> + 'static;
}

impl HandlerOutput<()> for () {
    async fn run(self) {}
}

pub struct FutureMarker;

impl<F> HandlerOutput<FutureMarker> for F
where
    F: IntoFuture<Output = ()> + 'static,
{
    async fn run(self) {
        self.into_future().await
    }
}

pub fn div<C: View + Clone>(content: C) -> Div<C> {
    Div {
        content,
        handlers: Vec::new(),
    }
}

pub struct Div<C> {
    content: C,
    handlers: Vec<Rc<dyn Fn()>>,
}

impl<C> Div<C> {
    pub fn on_click<Marker, R>(mut self, handler: impl FnMut() -> R + 'static) -> Self
    where
        R: HandlerOutput<Marker>,
    {
        let handler = Rc::new(RefCell::new(handler));
        self.handlers.push(Rc::new(move || {
            wasm_bindgen_futures::spawn_local(handler.borrow_mut()().run());
        }));
        self
    }
}

impl<C: View + Clone> View for Div<C> {
    fn body(&self, cx: &Scope) -> impl View {
        let parent: web_sys::Node = use_context(cx);

        let ((node, handlers, _), _) = use_state(cx, || {
            let element = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_element("div")
                .unwrap();
            parent.append_child(&element).unwrap();

            let mut handlers = Vec::new();
            let closures: Vec<_> = self
                .handlers
                .iter()
                .cloned()
                .map(|f| {
                    let handler: Rc<RefCell<Rc<dyn Fn()>>> =
                        Rc::new(RefCell::new(Rc::new(move || {
                            f();
                        })));
                    handlers.push(handler.clone());

                    let closure =
                        Closure::wrap(Box::new(move || handler.borrow_mut()()) as Box<dyn FnMut()>);

                    element
                        .add_event_listener_with_callback("click", closure.as_ref().unchecked_ref())
                        .unwrap();

                    closure
                })
                .collect();

            let node: web_sys::Node = element.into();
            (node, handlers, closures)
        });

        for (f, handler) in self.handlers.iter().zip(handlers) {
            *handler.borrow_mut() = f.clone();
        }

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

        let (node, _) = use_state(cx, || {
            let node = web_sys::window()
                .unwrap()
                .document()
                .unwrap()
                .create_text_node(&self.content);
            parent.append_child(&node).unwrap();
            node
        });

        use_effect(cx, self.content.clone(), || {
            node.set_text_content(Some(&self.content));
        });
    }
}
