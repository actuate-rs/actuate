use crate::RendererContext;
use actuate_core::{prelude::*, MapCompose};
use masonry::{
    vello::{
        self,
        peniko::{Color, Fill},
        util::RenderSurface,
        AaConfig, RenderParams, Renderer, RendererOptions,
    },
    Affine, Rect, Vec2,
};
use std::{cell::RefCell, mem, num::NonZeroUsize};
use taffy::{prelude::TaffyMaxContent, Size};
use wgpu::PresentMode;
use winit::{
    event::{Event, WindowEvent},
    window::WindowAttributes,
};

struct State {
    renderer: Renderer,
    render_surface: RenderSurface<'static>,
}

pub struct Window<C> {
    pub attributes: WindowAttributes,
    pub content: C,
}

impl<C> Window<C> {
    pub fn new(content: C) -> Self {
        Self {
            attributes: WindowAttributes::default(),
            content,
        }
    }
}

unsafe impl<C: Data> Data for Window<C> {
    type Id = Window<C::Id>;
}

impl<C: Compose> Compose for Window<C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let renderer_cx = use_context::<RendererContext>(&cx);

        let cursor_pos = use_ref(&cx, RefCell::default);

        let state = use_ref(&cx, || RefCell::new(None));

        actuate_winit::Window::new(
            WindowAttributes::default(),
            move |window, event| {
                match event {
                    Event::Resumed => {
                        let surface =
                            pollster::block_on(renderer_cx.cx.borrow_mut().create_surface(
                                window,
                                window.inner_size().width,
                                window.inner_size().height,
                                PresentMode::AutoVsync,
                            ))
                            .unwrap();

                        let renderer = Renderer::new(
                            &renderer_cx.cx.borrow().devices[surface.dev_id].device,
                            RendererOptions {
                                surface_format: Some(surface.format),
                                use_cpu: false,
                                antialiasing_support: vello::AaSupport::all(),
                                num_init_threads: NonZeroUsize::new(1),
                            },
                        )
                        .unwrap();

                        *state.borrow_mut() = Some(State {
                            render_surface: unsafe { mem::transmute(surface) },
                            renderer,
                        })
                    }
                    Event::WindowEvent { event, .. } => match event {
                        WindowEvent::CursorMoved { position, .. } => {
                            *cursor_pos.borrow_mut() = Vec2::new(position.x, position.y);
                        }
                        WindowEvent::MouseInput { .. } => {
                            let pos = *cursor_pos.borrow();
                            let taffy = renderer_cx.taffy.borrow();

                            let mut keys =
                                vec![(Vec2::default(), *renderer_cx.parent_key.borrow())];

                            let mut target = None;

                            while let Some((parent_pos, key)) = keys.pop() {
                                let layout = taffy.layout(key).unwrap();
                                if pos.x >= parent_pos.x + layout.location.x as f64
                                    && pos.y >= parent_pos.y + layout.location.y as f64
                                    && pos.x
                                        <= parent_pos.x
                                            + layout.location.x as f64
                                            + layout.size.width as f64
                                    && pos.y
                                        <= parent_pos.y
                                            + layout.location.y as f64
                                            + layout.size.height as f64
                                {
                                    target = Some(key);

                                    keys.extend(taffy.children(key).unwrap().into_iter().map(
                                        |key| {
                                            (
                                                parent_pos
                                                    + Vec2::new(
                                                        layout.location.x as _,
                                                        layout.location.y as _,
                                                    ),
                                                key,
                                            )
                                        },
                                    ));
                                }
                            }

                            if let Some(key) = target {
                                if let Some(listeners) = renderer_cx.listeners.borrow().get(&key) {
                                    for f in listeners {
                                        f()
                                    }
                                }
                            }
                        }
                        WindowEvent::RedrawRequested => {
                            #[cfg(feature = "tracing")]
                            tracing::trace!("Redraw");

                            // TODO
                            renderer_cx
                                .taffy
                                .borrow_mut()
                                .compute_layout(*renderer_cx.parent_key.borrow(), Size::MAX_CONTENT)
                                .unwrap();

                            let Some(state) = &mut *state.borrow_mut() else {
                                return;
                            };

                            let texture =
                                state.render_surface.surface.get_current_texture().unwrap();
                            let mut scene = renderer_cx.scene.borrow_mut();
                            let device_handle =
                                &renderer_cx.cx.borrow().devices[state.render_surface.dev_id];

                            state
                                .renderer
                                .render_to_surface(
                                    &device_handle.device,
                                    &device_handle.queue,
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
                            device_handle.device.poll(wgpu::Maintain::Poll);

                            scene.reset();
                            scene.fill(
                                Fill::NonZero,
                                Affine::default(),
                                Color::BLACK,
                                None,
                                &Rect::new(
                                    0.,
                                    0.,
                                    window.inner_size().width as _,
                                    window.inner_size().height as _,
                                ),
                            );
                        }
                        _ => {}
                    },
                    _ => {}
                }

                if renderer_cx.is_changed.take() {
                    window.request_redraw();

                    for f in &*renderer_cx.canvas_update_fns.borrow() {
                        f()
                    }
                }
            },
            unsafe { MapCompose::new(Ref::map(cx.me(), |me| &me.content)) },
        )
    }
}
