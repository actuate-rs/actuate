use crate::{prelude::*, Draw, RendererContext};
use masonry::vello::{
    kurbo::{Affine, Vec2},
    Scene,
};
use std::{cell::RefCell, mem, rc::Rc};
use taffy::{Layout, Style};

#[derive(Clone, Default)]
pub(crate) struct CanvasContext {
    pub(crate) draws: RefCell<Vec<Rc<dyn Draw>>>,
}

pub struct Canvas<'a> {
    style: Style,
    f: Box<dyn Fn(Layout, &mut Scene) + 'a>,
}

impl<'a> Canvas<'a> {
    pub fn new(style: Style, draw_fn: impl Fn(Layout, &mut Scene) + 'a) -> Self {
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
        let canvas_cx = use_context::<CanvasContext>(&cx);
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

            renderer_cx.is_layout_changed.set(true);

            let listeners = mem::take(&mut *renderer_cx.pending_listeners.borrow_mut());
            renderer_cx.listeners.borrow_mut().insert(key, listeners);

            let f: Box<dyn Fn()> = Box::new(move || {
                cx.set_changed();
            });
            let f = unsafe { mem::transmute(f) };

            // TODO remove on drop (unsound).
            renderer_cx.canvas_update_fns.borrow_mut().push(f);

            key
        });

        let last_style = use_ref(&cx, || cx.me().style.clone());
        if cx.me().style != *last_style {
            renderer_cx.is_layout_changed.set(true);
            renderer_cx
                .taffy
                .borrow_mut()
                .set_style(*key, cx.me().style.clone())
                .unwrap();
        }

        let scene = use_ref(&cx, || RefCell::new(Scene::new()));

        let layout = *renderer_cx.taffy.borrow().layout(*key).unwrap();
        let mut parent_scene = renderer_cx.scene.borrow_mut();

        let last_layout = use_mut(&cx, || None);
        if last_layout.is_none() {
            last_layout.with(move |dst| *dst = Some(layout));
            renderer_cx.is_changed.set(true);
            return;
        }
        if Some(layout) != *last_layout {
            last_layout.with(move |dst| *dst = Some(layout));
            renderer_cx.is_changed.set(true);
            cx.set_changed();
        }

        scene.borrow_mut().reset();
        for draw in &*canvas_cx.draws.borrow() {
            draw.pre_process(&layout, &mut scene.borrow_mut());
        }

        (cx.me().f)(layout, &mut scene.borrow_mut());

        for draw in &*canvas_cx.draws.borrow() {
            draw.post_process(&layout, &mut scene.borrow_mut());
        }

        parent_scene.append(
            &scene.borrow(),
            Some(Affine::translate(Vec2::new(
                layout.location.x as _,
                layout.location.y as _,
            ))),
        );
    }
}
