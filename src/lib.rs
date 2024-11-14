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
            }
        });

        use_provider(&cx, FontContext::default);

        unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) }
    }
}

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}
