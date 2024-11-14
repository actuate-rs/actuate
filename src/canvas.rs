use crate::{prelude::*, RendererContext};
use masonry::vello::{
    kurbo::{Affine, Vec2},
    Scene,
};
use std::cell::RefCell;
use taffy::{Layout, Style};

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
        cx.set_changed();

        let last_layout = use_mut(&cx, || None);

        if Some(layout) != *last_layout {
            last_layout.with(move |dst| *dst = Some(layout));

            (cx.me().f)(layout, &mut scene.borrow_mut());

            parent_scene.append(
                &scene.borrow(),
                Some(Affine::translate(Vec2::new(
                    layout.location.x as _,
                    layout.location.y as _,
                ))),
            );
            scene.borrow_mut().reset();

            renderer_cx.is_changed.set(true);
            cx.set_changed();
        }
    }
}
