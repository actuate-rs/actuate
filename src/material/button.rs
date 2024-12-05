use super::{MaterialTheme, Modifier, Modify};
use crate::{ecs::spawn, prelude::Compose, use_context, Data, Scope, Signal};
use bevy_color::Color;
use bevy_ui::{
    AlignItems, BackgroundColor, BorderRadius, BoxShadow, JustifyContent, Node, Overflow, UiRect,
    Val,
};

/// Create a material UI button.
pub fn button<'a, C>(content: C) -> Button<'a, C> {
    Button {
        content,
        elevation: 0.,
        height: Val::Px(40.),
        padding: UiRect::left(Val::Px(24.)).with_right(Val::Px(24.)),
        modifier: Modifier::default(),
    }
}

/// Material UI button.
pub struct Button<'a, C> {
    content: C,
    padding: UiRect,
    height: Val,
    elevation: f32,
    modifier: Modifier<'a>,
}

impl<'a, C> Button<'a, C> {
    /// Set the elevation of this button.
    pub fn elevation(mut self, elevation: f32) -> Self {
        self.elevation = elevation;
        self
    }

    /// Set the padding of this button.
    pub fn padding(mut self, padding: UiRect) -> Self {
        self.padding = padding;
        self
    }
}

unsafe impl<C: Data> Data for Button<'_, C> {}

impl<C: Compose> Compose for Button<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let theme = use_context::<MaterialTheme>(&cx)
            .cloned()
            .unwrap_or_default();

        cx.me()
            .modifier
            .apply(spawn((
                Node {
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    padding: cx.me().padding,
                    height: cx.me().height,
                    overflow: Overflow::clip(),
                    ..Default::default()
                },
                BorderRadius::all(Val::Px(10.))
                    .with_left(Val::Px(20.))
                    .with_right(Val::Px(20.)),
                BackgroundColor(theme.primary),
                BoxShadow {
                    color: Color::srgba(0., 0., 0., 0.12 * cx.me().elevation),
                    x_offset: Val::Px(0.),
                    y_offset: Val::Px(1.),
                    spread_radius: Val::Px(0.),
                    blur_radius: Val::Px(3. * cx.me().elevation),
                },
            )))
            .content(unsafe { Signal::map_unchecked(cx.me(), |me| &me.content) })
    }
}

impl<'a, C: Compose> Modify<'a> for Button<'a, C> {
    fn modifier(&mut self) -> &mut Modifier<'a> {
        &mut self.modifier
    }
}
