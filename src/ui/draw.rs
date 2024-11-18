use crate::prelude::*;
use parley::Rect;
use peniko::Fill;
use taffy::Layout;
use vello::{kurbo::Affine, Scene};

/// Drawable modifiers.
pub trait Draw {
    /// Pre-process the scene, this is run before a composable in rendered.
    fn pre_process(&self, layout: &Layout, scene: &mut Scene) {
        let _ = layout;
        let _ = scene;
    }

    /// Post-process the scene, this is run after a composable in rendered.
    fn post_process(&self, layout: &Layout, scene: &mut Scene) {
        let _ = layout;
        let _ = scene;
    }
}

/// Background color modifier.
#[derive(Data)]
pub struct BackgroundColor {
    /// Background color.
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
