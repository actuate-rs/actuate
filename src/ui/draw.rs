use crate::prelude::*;
use parley::Rect;
use peniko::Fill;
use taffy::Layout;
use vello::{kurbo::Affine, Scene};

pub trait Draw {
    fn pre_process(&self, layout: &Layout, scene: &mut Scene) {
        let _ = layout;
        let _ = scene;
    }

    fn post_process(&self, layout: &Layout, scene: &mut Scene) {
        let _ = layout;
        let _ = scene;
    }
}

#[derive(Data)]
pub struct BackgroundColor {
    pub color: Color,
}

impl Draw for BackgroundColor {
    fn pre_process(&self, layout: &Layout, scene: &mut Scene) {
        scene.fill(
            Fill::NonZero,
            Affine::default(),
            self.color,
            None,
            &Rect::new(0., 0., layout.size.width as _, layout.size.height as _),
        );
    }
}
