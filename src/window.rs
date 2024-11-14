use actuate_core::prelude::*;
use masonry::vello::{
        self,
        peniko::Color,
        AaConfig, RenderParams, Renderer, RendererOptions,
    };
use std::num::NonZeroUsize;
use taffy::{prelude::TaffyMaxContent, Size};
use wgpu::PresentMode;
use winit::{
    event::{Event, WindowEvent},
    window::WindowAttributes,
};
use crate::RendererContext;

pub struct Window<C> {
    pub attributes: WindowAttributes,
    pub content: C,
}

impl<C> Window<C > {
    pub fn new(content:C) -> Self {
        Self { attributes: WindowAttributes::default(), content }
    }
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