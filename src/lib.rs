use actuate_core::prelude::*;
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};
use taffy::{NodeId, TaffyTree};
use ui::canvas::CanvasContext;
use vello::{kurbo::Vec2, peniko::Color, Scene};
use winit::event::{ElementState, MouseButton};

pub use actuate_core as core;

pub mod draw;
use self::draw::Draw;

pub mod modify;

pub mod ui;
use self::ui::text::{FontContext, TextContext};

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::modify::View;

    pub use crate::ui::{use_font, Canvas, Flex, Text, Window};

    pub use parley::GenericFamily;

    pub use taffy::prelude::*;

    pub use vello::peniko::Color;

    pub use winit::window::WindowAttributes;
}

pub struct LayoutContext {
    parent_id: NodeId,
}

pub struct WindowContext {
    scene: RefCell<Scene>,
    taffy: RefCell<TaffyTree>,
    is_changed: Cell<bool>,
    is_layout_changed: Cell<bool>,
    canvas_update_fns: RefCell<HashMap<NodeId, Box<dyn Fn()>>>,
    listeners: Rc<RefCell<HashMap<NodeId, Vec<Rc<dyn Fn(MouseButton, ElementState, Vec2)>>>>>,
    pending_listeners: Rc<RefCell<Vec<Rc<dyn Fn(MouseButton, ElementState, Vec2)>>>>,
    base_color: Cell<Color>,
}

struct RenderRoot<C> {
    content: C,
}

unsafe impl<C: Data> Data for RenderRoot<C> {
    type Id = RenderRoot<C::Id>;
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

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}
