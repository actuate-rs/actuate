use crate::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use vello::{kurbo::Vec2, Scene};
use view::{
    canvas::CanvasContext,
    text::{FontContext, TextContext},
};
use winit::event::{ElementState, MouseButton};

/// Drawable modifiers.
pub mod draw;
pub use self::draw::Draw;

/// View modifiers.
pub mod view;

/// Use a new layout node.
pub fn use_layout(cx: ScopeState, style: Style) -> (NodeId, Layout) {
    let layout_cx = use_context::<LayoutContext>(cx).unwrap();
    let renderer_cx = use_context::<WindowContext>(cx).unwrap();

    let parent_key = layout_cx.parent_id;
    let key = *use_ref(cx, || {
        let key = renderer_cx
            .taffy
            .borrow_mut()
            .new_leaf(style.clone())
            .unwrap();
        renderer_cx
            .taffy
            .borrow_mut()
            .add_child(parent_key, key)
            .unwrap();

        renderer_cx.is_layout_changed.set(true);

        key
    });

    let last_style = use_ref(cx, || style.clone());
    if style != *last_style {
        renderer_cx.is_layout_changed.set(true);
        renderer_cx
            .taffy
            .borrow_mut()
            .set_style(key, style.clone())
            .unwrap();
    }

    use_drop(cx, move || {
        renderer_cx.taffy.borrow_mut().remove(key).unwrap();
        renderer_cx.listeners.borrow_mut().remove(&key);
    });

    let layout = *renderer_cx.taffy.borrow().layout(key).unwrap();
    (key, layout)
}

struct LayoutContext {
    parent_id: NodeId,
}

/// User interface event.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    /// Mouse input event.
    MouseInput {
        /// Mouse button.
        button: MouseButton,
        /// Element state.
        state: ElementState,
        /// Cursor position.
        pos: Vec2,
    },
    /// Mouse in event.
    MouseIn,
    /// Mouse move event.
    MouseMove {
        /// Cursor position.
        pos: Vec2,
    },
    /// Mouse out event.
    MouseOut,
}

type ListenerFn = Rc<dyn Fn(Event)>;

pub(crate) struct WindowContext {
    scene: RefCell<Scene>,
    taffy: RefCell<TaffyTree>,
    is_changed: Cell<bool>,
    is_layout_changed: Cell<bool>,
    canvas_update_fns: RefCell<HashMap<NodeId, Box<dyn Fn()>>>,
    listeners: Rc<RefCell<HashMap<NodeId, Vec<ListenerFn>>>>,
    base_color: Cell<Color>,
}

#[derive(Data)]
pub(crate) struct RenderRoot<C> {
    pub(crate) content: C,
}

impl<C: Compose> Compose for RenderRoot<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, CanvasContext::default);

        use_provider(&cx, FontContext::default);

        let text_context = use_context::<TextContext>(&cx).map(|rc| (*rc).clone());
        use_provider(&cx, || text_context.unwrap_or_default());

        Ref::map(cx.me(), |me| &me.content)
    }
}
