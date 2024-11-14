use actuate_core::prelude::*;
use masonry::{
    vello::{
        self,
        peniko::{Color, Fill},
        util::RenderContext,
        AaConfig, RenderParams, Renderer, RendererOptions, Scene,
    },
    Affine, Rect,
};
use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    rc::Rc,
};
use taffy::{prelude::TaffyMaxContent, NodeId, Size, Style, TaffyTree};
use text::FontContext;
use wgpu::PresentMode;
use winit::{
    event::{Event, WindowEvent},
    window::WindowAttributes,
};

pub use actuate_core as core;

mod canvas;
pub use self::canvas::Canvas;

mod flex;
pub use self::flex::Flex;

mod text;
pub use self::text::{use_font, Text};

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

pub struct Window<C> {
    pub attributes: WindowAttributes,
    pub content: C,
}

unsafe impl<C: Data> Data for Window<C> {
    type Id = Window<C::Id>;
}

impl<C: Compose> Compose for Window<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let renderer_cx = use_context::<RendererContext>(&cx);

        actuate_winit::Window::new(
            WindowAttributes::default(),
            move |window, event| {
                match event {
                    Event::Resumed => {}
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::RedrawRequested => {
                            #[cfg(feature = "tracing")]
                            tracing::trace!("Redraw");

                            // TODO
                            renderer_cx
                                .taffy
                                .borrow_mut()
                                .compute_layout(*renderer_cx.parent_key.borrow(), Size::MAX_CONTENT)
                                .unwrap();

                            let surface =
                                pollster::block_on(renderer_cx.cx.borrow_mut().create_surface(
                                    window,
                                    window.inner_size().width,
                                    window.inner_size().height,
                                    PresentMode::AutoVsync,
                                ))
                                .unwrap();

                            let mut renderer = Renderer::new(
                                &renderer_cx.cx.borrow().devices[surface.dev_id].device,
                                RendererOptions {
                                    surface_format: Some(surface.format),
                                    use_cpu: false,
                                    antialiasing_support: vello::AaSupport::all(),
                                    num_init_threads: NonZeroUsize::new(1),
                                },
                            )
                            .unwrap();

                            let texture = surface.surface.get_current_texture().unwrap();

                            let scene = renderer_cx.scene.borrow_mut();

                            let device = &renderer_cx.cx.borrow().devices[surface.dev_id];

                            renderer
                                .render_to_surface(
                                    &device.device,
                                    &device.queue,
                                    &scene,
                                    &texture,
                                    &RenderParams {
                                        base_color: Color::BLACK,
                                        width: window.inner_size().width,
                                        height: window.inner_size().height,
                                        antialiasing_method: AaConfig::Msaa16,
                                    },
                                )
                                .unwrap();

                            texture.present();
                        }
                        _ => {}
                    },
                    _ => {}
                }

                if renderer_cx.is_changed.take() {
                    window.request_redraw();
                }
            },
            Ref::map(cx.me(), |me| &me.content),
        )
    }
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
            let root_key = taffy.new_leaf(Style::default()).unwrap();

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

        Ref::map(cx.me(), |me| &me.content)
    }
}

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}
