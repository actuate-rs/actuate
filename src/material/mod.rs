use crate::{
    ecs::{Spawn, SystemParamFunction},
    Data,
};
use bevy_color::Color;
use bevy_ecs::prelude::{Bundle, Event, Trigger};
use bevy_picking::prelude::*;
use core::mem;
use std::cell::Cell;

mod button;
pub use self::button::{button, Button};

mod radio;
pub use self::radio::{radio_button, RadioButton};

/// Text composables.
pub mod text;

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

/// ECS bundle modifier.
#[derive(Default)]
pub struct Modifier<'a> {
    fns: Vec<Box<dyn Fn(Spawn<'a>) -> Spawn<'a>>>,
}

impl<'a> Modifier<'a> {
    /// Apply this modifier.
    pub fn apply(&self, spawn: Spawn<'a>) -> Spawn<'a> {
        self.fns
            .iter()
            .fold(spawn, |spawn, modifier| modifier(spawn))
    }
}

unsafe impl Data for Modifier<'_> {}

/// Modifiable composable.
pub trait Modify<'a> {
    /// Get a mutable reference to the modifier of this button.
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
