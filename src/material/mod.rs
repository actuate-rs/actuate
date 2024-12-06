use bevy_color::Color;

mod button;
pub use self::button::{button, Button};

mod container;
pub use self::container::{container, Container};

mod radio;
pub use self::radio::{radio_button, RadioButton};

/// Text composables.
pub mod text;

/// Material UI theme.
pub struct MaterialTheme {
    /// Primary color.
    pub primary: Color,

    /// Surface container color.
    pub surface_container: Color,
}

impl Default for MaterialTheme {
    fn default() -> Self {
        Self {
            primary: Color::srgb_u8(103, 80, 164),
            surface_container: Color::srgb_u8(230, 224, 233),
        }
    }
}
