use actuate_core::prelude::*;
use std::{cell::RefCell, num::NonZeroUsize};
use vello::{
    kurbo::{Affine, Circle},
    peniko::{Color, Fill},
    util::{RenderContext, RenderSurface},
    wgpu::PresentMode,
    AaConfig, Renderer, RendererOptions, Scene,
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
    cx: RefCell<RenderContext>,
}

struct State<'a> {
    renderer: Renderer,
    surface: RenderSurface<'a>,
}

#[derive(Data)]
pub struct Window {
    pub attributes: WindowAttributes,
}

impl Compose for Window {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let renderer_cx = use_context::<RendererContext>(&cx);

        actuate_winit::Window::new(
            WindowAttributes::default(),
            move |window, event| match event {
                Event::Resumed => {}
                Event::WindowEvent { window_id, event } => match event {
                    WindowEvent::RedrawRequested => {
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

                        let mut scene = Scene::new();
                        scene.fill(
                            Fill::NonZero,
                            Affine::IDENTITY,
                            Color::rgb(0.9529, 0.5451, 0.6588),
                            None,
                            &Circle::new((420.0, 200.0), 120.0),
                        );

                        let device = &renderer_cx.cx.borrow().devices[surface.dev_id];
                        renderer
                            .render_to_surface(
                                &device.device,
                                &device.queue,
                                &scene,
                                &texture,
                                &vello::RenderParams {
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
            },
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
        use_provider(&cx, || RendererContext {
            cx: RefCell::new(RenderContext::new()),
        });

        Ref::map(cx.me(), |me| &me.content)
    }
}

pub fn run(content: impl Compose + 'static) {
    actuate_winit::run(RenderRoot { content });
}
