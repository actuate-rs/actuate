use crate::ecs::{spawn, Spawn};
use bevy_text::TextFont;
use bevy_ui::prelude::Text;

/// Create a material UI text label.
pub fn headline<'a>(content: impl Into<String>) -> Spawn<'a> {
    spawn((
        Text::new(content),
        TextFont {
            font_size: 36.,
            ..Default::default()
        },
    ))
}

/// Create a material UI text label.
pub fn label<'a>(content: impl Into<String>) -> Spawn<'a> {
    spawn((
        Text::new(content),
        TextFont {
            font_size: 16.,
            ..Default::default()
        },
    ))
}
