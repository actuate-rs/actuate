use std::ops::Index;

use bevy_color::Color;

mod button;
pub use self::button::{button, Button};

mod container;
pub use self::container::{container, Container};

mod radio;
pub use self::radio::{radio_button, RadioButton};

/// Text composables.
pub mod text;

/// Colors for a [`MaterialTheme`].
pub struct Colors {
    /// Primary color.
    pub primary: Color,

    /// Surface container color.
    pub surface_container: Color,

    /// Text color.
    pub text: Color,
}

/// Typography style.
pub struct TypographyStyle {
    /// Font size.
    pub font_size: f32,

    /// Font weight.
    pub font_weight: f32,

    /// Line height.
    pub line_height: f32,
}

/// Typography style kind.
#[derive(Clone, Copy)]
pub enum TypographyStyleKind {
    /// Small typography style.
    Small,

    /// Medium typography style.
    Medium,

    /// Large typography style.
    Large,
}

/// Typography design token.
pub struct TypographyToken {
    /// Small typography style.
    pub small: TypographyStyle,

    /// Medium typography style.
    pub medium: TypographyStyle,

    /// Large typography style.
    pub large: TypographyStyle,
}

impl Index<TypographyStyleKind> for TypographyToken {
    type Output = TypographyStyle;

    fn index(&self, index: TypographyStyleKind) -> &Self::Output {
        match index {
            TypographyStyleKind::Small => &self.small,
            TypographyStyleKind::Medium => &self.medium,
            TypographyStyleKind::Large => &self.large,
        }
    }
}

/// Typography style kind.
#[derive(Clone, Copy)]
pub enum TypographyKind {
    /// Headline typography.
    Headline,

    /// Label typography.
    Label,
}

/// Typography for a [`MaterialTheme`].
pub struct Typography {
    /// Label font size.
    pub headline: TypographyToken,

    /// Label font size.
    pub label: TypographyToken,
}

impl Index<TypographyKind> for Typography {
    type Output = TypographyToken;

    fn index(&self, index: TypographyKind) -> &Self::Output {
        match index {
            TypographyKind::Headline => &self.headline,
            TypographyKind::Label => &self.label,
        }
    }
}

/// Material UI theme.
pub struct MaterialTheme {
    /// Theme colors.
    pub colors: Colors,

    /// Theme typography.
    pub typography: Typography,
}

impl Default for MaterialTheme {
    fn default() -> Self {
        Self {
            colors: Colors {
                primary: Color::srgb_u8(103, 80, 164),
                surface_container: Color::srgb_u8(230, 224, 233),
                text: Color::BLACK,
            },
            typography: Typography {
                headline: TypographyToken {
                    small: TypographyStyle {
                        font_size: 24.,
                        font_weight: 400.,
                        line_height: 32.,
                    },
                    medium: TypographyStyle {
                        font_size: 28.,
                        font_weight: 400.,
                        line_height: 36.,
                    },
                    large: TypographyStyle {
                        font_size: 32.,
                        font_weight: 400.,
                        line_height: 40.,
                    },
                },
                label: TypographyToken {
                    small: TypographyStyle {
                        font_size: 11.,
                        font_weight: 500.,
                        line_height: 16.,
                    },
                    medium: TypographyStyle {
                        font_size: 12.,
                        font_weight: 500.,
                        line_height: 16.,
                    },
                    large: TypographyStyle {
                        font_size: 14.,
                        font_weight: 500.,
                        line_height: 20.,
                    },
                },
            },
        }
    }
}
