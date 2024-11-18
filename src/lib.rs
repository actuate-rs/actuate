//! # Actuate
//! Actuate is a native, declarative, and friendly user-interface (UI) framework.
//!
//! ## Hooks
//! Functions that begin with `use_` are called `hooks` in Actuate.
//! Hooks are used to manage state and side effects in composables.
//!
//! Hooks must be used in the same order for every re-compose.
//! Don’t use hooks inside loops, conditions, nested functions, or match blocks.
//! Instead, always use hooks at the top level of your composable, before any early returns.

use actuate_core::{prelude::*, Executor};
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

pub mod view;

pub mod ui;
use self::ui::text::{FontContext, TextContext};

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::view::View;

    pub use crate::ui::{use_font, Canvas, Flex, Text, Window};

    pub use parley::GenericFamily;

    pub use taffy::prelude::*;

    pub use vello::peniko::Color;

    pub use winit::window::WindowAttributes;
}

pub struct LayoutContext {
    parent_id: NodeId,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    MouseInput {
        button: MouseButton,
        state: ElementState,
        pos: Vec2,
    },
    MouseIn,
    MouseMove {
        pos: Vec2,
    },
    MouseOut,
}

type ListenerFn = Rc<dyn Fn(Event)>;

pub struct WindowContext {
    scene: RefCell<Scene>,
    taffy: RefCell<TaffyTree>,
    is_changed: Cell<bool>,
    is_layout_changed: Cell<bool>,
    canvas_update_fns: RefCell<HashMap<NodeId, Box<dyn Fn()>>>,
    listeners: Rc<RefCell<HashMap<NodeId, Vec<ListenerFn>>>>,
    base_color: Cell<Color>,
}

#[derive(Data)]
struct RenderRoot<C> {
    content: C,
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

pub fn run_with_executor(content: impl Compose + 'static, executor: impl Executor + 'static) {
    actuate_winit::run_with_executor(RenderRoot { content }, executor);
}
