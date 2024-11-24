use crate::prelude::*;
use crate::ui::{use_layout, ListenerFn, WindowContext};
use peniko::Mix;
use std::{cell::RefCell, mem, rc::Rc};
use taffy::{Layout, Style};
use vello::{
    kurbo::{Affine, RoundedRect, Vec2},
    Scene,
};

#[derive(Clone, Default)]
pub(crate) struct CanvasContext {
    pub(crate) draws: RefCell<Vec<Rc<dyn Draw>>>,
    pub(crate) pending_listeners: Rc<RefCell<Vec<ListenerFn>>>,
    pub(crate) border_radius: f64,
}

type DrawFn<'a> = Box<dyn Fn(Layout, &mut Scene) + 'a>;

/// Canvas composable.
#[derive(Data)]
#[must_use = "Composables do nothing unless composed with `actuate::run` or returned from other composables"]
pub struct Canvas<'a> {
    style: Style,
    f: DrawFn<'a>,
}

impl<'a> Canvas<'a> {
    /// Create a new canvas from its style and draw function.
    pub fn new(style: Style, draw_fn: impl Fn(Layout, &mut Scene) + 'a) -> Self {
        Self {
            style,
            f: Box::new(draw_fn),
        }
    }
}

impl Compose for Canvas<'_> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let canvas_cx = use_context::<CanvasContext>(&cx).unwrap();
        let renderer_cx = use_context::<WindowContext>(&cx).unwrap();

        let (key, layout) = use_layout(&cx, cx.me().style.clone());

        use_ref(&cx, || {
            let listeners = canvas_cx.pending_listeners.borrow().clone();
            renderer_cx.listeners.borrow_mut().insert(key, listeners);

            let f: Box<dyn Fn()> = Box::new(move || {
                cx.set_changed();
            });

            // Safety: `f` is removed from `canvas_update_fns` on drop.
            let f: Box<dyn Fn()> = unsafe { mem::transmute(f) };

            renderer_cx.canvas_update_fns.borrow_mut().insert(key, f);
        });

        // Safety: We must remove `f` here to make the above valid.
        use_drop(&cx, move || {
            renderer_cx.canvas_update_fns.borrow_mut().remove(&key);
        });

        let scene = use_ref(&cx, || RefCell::new(Scene::new()));

        let mut parent_scene = renderer_cx.scene.borrow_mut();

        if cx.is_parent_changed() {
            renderer_cx.is_changed.set(true);
            return;
        }

        let last_layout = use_mut(&cx, || None);
        if Some(layout) != *last_layout {
            Mut::with(last_layout, move |dst| *dst = Some(layout));
            renderer_cx.is_changed.set(true);
            if last_layout.is_none() {
                return;
            }
        }

        scene.borrow_mut().reset();

        scene.borrow_mut().push_layer(
            Mix::Clip,
            1.0,
            Affine::default(),
            &RoundedRect::new(
                0.,
                0.,
                layout.size.width as _,
                layout.size.height as _,
                canvas_cx.border_radius,
            ),
        );

        for draw in &*canvas_cx.draws.borrow() {
            draw.pre_process(&layout, &mut scene.borrow_mut());
        }

        (cx.me().f)(layout, &mut scene.borrow_mut());

        for draw in &*canvas_cx.draws.borrow() {
            draw.post_process(&layout, &mut scene.borrow_mut());
        }

        scene.borrow_mut().pop_layer();

        parent_scene.append(
            &scene.borrow(),
            Some(Affine::translate(Vec2::new(
                layout.location.x as _,
                layout.location.y as _,
            ))),
        );
    }
}
