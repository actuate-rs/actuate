use crate::{
    ecs::{spawn, use_world, Modifier, Modify},
    prelude::Compose,
    use_mut, Scope, Signal, SignalMut,
};
use actuate_macros::Data;
use bevy_ecs::prelude::*;
use bevy_input::{
    mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bevy_picking::prelude::*;
use bevy_ui::prelude::*;
use std::mem;

#[cfg(feature = "material")]
#[cfg_attr(docsrs, doc(cfg(feature = "material")))]
/// Material UI.
pub mod material;

/// Create a scroll view.
pub fn scroll_view<'a, C: Compose>(content: C) -> ScrollView<'a, C> {
    ScrollView {
        content,
        line_size: 30.,
        modifier: Modifier::default(),
    }
}

#[derive(Data)]
#[actuate(path = "crate")]
/// Scroll view composable.
pub struct ScrollView<'a, C> {
    content: C,
    line_size: f32,
    modifier: Modifier<'a>,
}

impl<C> ScrollView<'_, C> {
    /// Set the line size to scroll (default: 30).
    pub fn line_size(mut self, size: f32) -> Self {
        self.line_size = size;
        self
    }
}

impl<C: Compose> Compose for ScrollView<'_, C> {
    fn compose(cx: Scope<Self>) -> impl Compose {
        let is_hovered = use_mut(&cx, || false);

        let entity_cell = use_mut(&cx, || None);

        use_world(
            &cx,
            move |mut mouse_wheel_events: EventReader<MouseWheel>,
                  mut scrolled_node_query: Query<&mut ScrollPosition>,
                  keyboard_input: Res<ButtonInput<KeyCode>>| {
                for mouse_wheel_event in mouse_wheel_events.read() {
                    dbg!(mouse_wheel_event);
                    let (mut dx, mut dy) = match mouse_wheel_event.unit {
                        MouseScrollUnit::Line => (
                            mouse_wheel_event.x * cx.me().line_size,
                            mouse_wheel_event.y * cx.me().line_size,
                        ),
                        MouseScrollUnit::Pixel => (mouse_wheel_event.x, mouse_wheel_event.y),
                    };

                    if keyboard_input.pressed(KeyCode::ControlLeft)
                        || keyboard_input.pressed(KeyCode::ControlRight)
                    {
                        std::mem::swap(&mut dx, &mut dy)
                    }

                    if *is_hovered {
                        if let Some(entity) = *entity_cell {
                            if let Ok(mut scroll_position) = scrolled_node_query.get_mut(entity) {
                                scroll_position.offset_x -= dx;
                                scroll_position.offset_y -= dy;
                            }
                        }
                    }
                }
            },
        );

        let modifier = &cx.me().modifier;
        let modifier: &Modifier = unsafe { mem::transmute(modifier) };

        modifier
            .apply(
                spawn(Node {
                    height: Val::Percent(100.),
                    flex_direction: FlexDirection::Column,
                    overflow: Overflow::scroll_y(),
                    ..Default::default()
                })
                .on_spawn(move |entity| SignalMut::set(entity_cell, Some(entity.id())))
                .observe(move |_: Trigger<Pointer<Over>>| SignalMut::set(is_hovered, true))
                .observe(move |_: Trigger<Pointer<Out>>| SignalMut::set(is_hovered, false)),
            )
            .content(unsafe { Signal::map_unchecked(cx.me(), |me| &me.content) })
    }
}

impl<'a, C: Compose> Modify<'a> for ScrollView<'a, C> {
    fn modifier(&mut self) -> &mut Modifier<'a> {
        &mut self.modifier
    }
}
