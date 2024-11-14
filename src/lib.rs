use actuate_core::prelude::*;
use std::{
    cell::{Cell, RefCell},
    num::NonZeroUsize,
    rc::Rc,
};
use taffy::{prelude::TaffyMaxContent, NodeId, Size, Style, TaffyTree};
use vello::{
    kurbo::{Affine, Vec2},
    peniko::Color,
    util::RenderContext,
    wgpu::PresentMode,
    AaConfig, RenderParams, Renderer, RendererOptions, Scene,
};
use winit::{
    event::{Event, WindowEvent},
    window::WindowAttributes,
};

pub use actuate_core as core;

pub mod prelude {
    pub use crate::core::prelude::*;

    pub use crate::Window;
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
                            tracing::info!("Redraw");

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

pub struct Canvas<'a> {
    style: Style,
    f: Box<dyn Fn(&mut Scene) + 'a>,
}

impl<'a> Canvas<'a> {
    pub fn new(style: Style, draw_fn: impl Fn(&mut Scene) + 'a) -> Self {
        Self {
            style,
            f: Box::new(draw_fn),
        }
    }
}

unsafe impl Data for Canvas<'_> {
    type Id = Canvas<'static>;
}

impl Compose for Canvas<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let renderer_cx = use_context::<RendererContext>(&cx);

        let key = use_ref(&cx, || {
            let key = renderer_cx
                .taffy
                .borrow_mut()
                .new_leaf(cx.me().style.clone())
                .unwrap();
            renderer_cx
                .taffy
                .borrow_mut()
                .add_child(*renderer_cx.parent_key.borrow(), key)
                .unwrap();
            key
        });

        let scene = use_ref(&cx, || RefCell::new(Scene::new()));

        let layout = *renderer_cx.taffy.borrow().layout(*key).unwrap();
        let mut parent_scene = renderer_cx.scene.borrow_mut();

        let last_layout = use_mut(&cx, || layout);

        if layout != *last_layout {
            last_layout.with(move |dst| *dst = layout);

            (cx.me().f)(&mut scene.borrow_mut());

            parent_scene.append(
                &scene.borrow(),
                Some(Affine::translate(Vec2::new(
                    layout.location.x as _,
                    layout.location.y as _,
                ))),
            );

            renderer_cx.is_changed.set(true);
        }
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

            RendererContext {
                cx: Rc::new(RefCell::new(RenderContext::new())),
                scene: RefCell::new(Scene::new()),
                taffy: RefCell::new(taffy),
                parent_key: RefCell::new(root_key),
                is_changed: Cell::new(false),
            }
        });

        Ref::map(cx.me(), |me| &me.content)
    }
}

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}
