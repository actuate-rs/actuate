use core::mem;
use std::cell::Cell;

use crate::{
    ecs::{spawn, Spawn, SystemParamFunction},
    prelude::Compose,
    use_context, Data, Scope, Signal,
};
use bevy_color::Color;
use bevy_ecs::prelude::{Bundle, Event, Trigger};
use bevy_picking::prelude::*;
use bevy_text::TextFont;
use bevy_ui::{
    prelude::Text, AlignItems, BackgroundColor, BorderRadius, BoxShadow, JustifyContent, Node,
    Overflow, UiRect, Val,
};

/// Material UI theme.
pub struct MaterialTheme {
    /// Primary color.
    pub primary: Color,
}

impl Default for MaterialTheme {
    fn default() -> Self {
        Self {
            primary: Color::srgb_u8(103, 80, 164),
        }
    }
}

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

/// TODO
#[derive(Default)]
pub struct Modifier<'a> {
    fns: Vec<Box<dyn Fn(Spawn<'a>) -> Spawn<'a>>>,
}

impl<'a> Modifier<'a> {
    /// TODO
    pub fn apply(&self, spawn: Spawn<'a>) -> Spawn<'a> {
        self.fns
            .iter()
            .fold(spawn, |spawn, modifier| modifier(spawn))
    }
}

unsafe impl Data for Modifier<'_> {}

/// TODO
pub trait Modify<'a> {
    /// TODO
    fn modifier(&mut self) -> &mut Modifier<'a>;

    /// Add an observer to the container of this button.
    fn observe<F, E, B, Marker>(mut self, observer: F) -> Self
    where
        Self: Sized,
        F: SystemParamFunction<Marker, In = Trigger<'static, E, B>, Out = ()> + Send + Sync + 'a,
        E: Event,
        B: Bundle,
    {
        let observer_cell = Cell::new(Some(observer));
        let f: Box<dyn Fn(Spawn) -> Spawn> = Box::new(move |spawn| {
            let observer = observer_cell.take().unwrap();
            let spawn: Spawn<'a> = unsafe { mem::transmute(spawn) };
            let spawn = spawn.observe(observer);
            let spawn: Spawn = unsafe { mem::transmute(spawn) };
            spawn
        });
        let f: Box<dyn Fn(Spawn) -> Spawn> = unsafe { mem::transmute(f) };
        self.modifier().fns.push(f);
        self
    }

    /// Add an click observer to the container of this button.
    fn on_click(self, f: impl Fn() + Send + Sync + 'a) -> Self
    where
        Self: Sized,
    {
        self.observe(move |_: Trigger<Pointer<Click>>| f())
    }
}

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
