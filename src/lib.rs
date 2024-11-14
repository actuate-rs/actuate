use actuate_core::{prelude::*, MapCompose};
use masonry::{
    vello::{
        peniko::{Color, Fill},
        util::RenderContext,
        Scene,
    },
    Affine, Rect,
};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    mem,
    rc::Rc,
};
use taffy::{FlexDirection, NodeId, Style, TaffyTree};
use text::FontContext;

pub use actuate_core as core;

mod canvas;
pub use self::canvas::Canvas;

mod flex;
pub use self::flex::Flex;

mod text;
pub use self::text::{use_font, Text};

mod window;
pub use self::window::Window;

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::{use_font, Canvas, Flex, Text, Window};

    pub use winit::window::WindowAttributes;
}

pub struct RendererContext {
    cx: Rc<RefCell<RenderContext>>,

    // TODO move this to window-specific context
    scene: RefCell<Scene>,
    taffy: RefCell<TaffyTree>,
    parent_key: RefCell<NodeId>,
    is_changed: Cell<bool>,
    is_layout_changed: Cell<bool>,
    canvas_update_fns: RefCell<Vec<Box<dyn Fn()>>>,
    listeners: Rc<RefCell<HashMap<NodeId, Vec<Rc<dyn Fn()>>>>>,
    pending_listeners: Rc<RefCell<Vec<Rc<dyn Fn()>>>>,
}

struct RenderRoot<C> {
    content: C,
}

unsafe impl<C: Data> Data for RenderRoot<C> {
    type Id = RenderRoot<C::Id>;
}

impl<C: Compose> Compose for RenderRoot<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        use_provider(&cx, || {
            let mut taffy = TaffyTree::new();
            let root_key = taffy
                .new_leaf(Style {
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                })
                .unwrap();

            let mut scene = Scene::new();
            scene.fill(
                Fill::NonZero,
                Affine::default(),
                Color::BLACK,
                None,
                &Rect::new(0., 0., 500., 500.),
            );

            RendererContext {
                cx: Rc::new(RefCell::new(RenderContext::new().unwrap())),
                scene: RefCell::new(scene),
                taffy: RefCell::new(taffy),
                parent_key: RefCell::new(root_key),
                is_changed: Cell::new(false),
                is_layout_changed: Cell::new(false),
                canvas_update_fns: RefCell::new(Vec::new()),
                listeners: Rc::default(),
                pending_listeners: Rc::default(),
            }
        });

        use_provider(&cx, FontContext::default);

        unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) }
    }
}

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}

pub trait View: Compose {
    fn on_click<'a>(self, on_click: impl Fn() + 'a) -> Clickable<'a, Self>;
}

impl<C: Compose> View for C {
    fn on_click<'a>(self, on_click: impl Fn() + 'a) -> Clickable<'a, Self> {
        Clickable::new(on_click, self)
    }
}

pub struct Clickable<'a, C> {
    on_click: Rc<dyn Fn() + 'a>,
    content: C,
}

impl<'a, C> Clickable<'a, C> {
    pub fn new(on_click: impl Fn() + 'a, content: C) -> Self {
        Self {
            on_click: Rc::new(on_click),
            content,
        }
    }
}

unsafe impl<C: Data> Data for Clickable<'_, C> {
    type Id = Clickable<'static, C::Id>;
}

impl<C: Compose> Compose for Clickable<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let renderer_cx = use_context::<RendererContext>(&cx);

        // TODO remove on drop (unsound).
        use_ref(&cx, || {
            renderer_cx
                .pending_listeners
                .borrow_mut()
                .push(unsafe { mem::transmute(cx.me().on_click.clone()) });
        });

        unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) }
    }
}
