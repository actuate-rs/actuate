use crate::{
    ecs::{Spawn, SystemParamFunction},
    Data,
};
use bevy_color::Color;
use bevy_ecs::prelude::{Bundle, EntityWorldMut, Event, Trigger};
use bevy_picking::prelude::*;
use core::mem;
use std::{cell::Cell, rc::Rc};

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

/// ECS bundle modifier.
#[derive(Clone, Default)]
pub struct Modifier<'a> {
    fns: Vec<Rc<dyn Fn(Spawn<'a>) -> Spawn<'a> + 'a>>,
}

impl<'a> Modifier<'a> {
    /// Apply this modifier.
    pub fn apply(&self, spawn: Spawn<'a>) -> Spawn<'a> {
        self.fns
            .iter()
            .fold(spawn, |spawn, modifier| modifier(spawn))
    }

    /// Append another stack of modifiers to this modifier.
    pub fn append(&mut self, modifier: Self) {
        self.fns.extend(modifier.fns);
    }
}

unsafe impl Data for Modifier<'_> {}

/// Modifiable composable.
pub trait Modify<'a> {
    /// Get a mutable reference to the modifier of this button.
    fn modifier(&mut self) -> &mut Modifier<'a>;

    /// Append a modifier to this composable.
    fn append(mut self, modifier: Modifier<'a>) -> Self
    where
        Self: Sized,
    {
        self.modifier().append(modifier);
        self
    }

    /// Add a function to run when this composable's bundle is spawned.
    fn on_insert<F>(mut self, f: F) -> Self
    where
        Self: Sized,
        F: Fn(EntityWorldMut) + 'a,
    {
        let f = Rc::new(f);
        self.modifier().fns.push(Rc::new(move |spawn| {
            let f = f.clone();
            spawn.on_insert(move |e| f(e))
        }));
        self
    }

    /// Add an observer to the container of this button.
    fn observe<F, E, B, Marker>(mut self, observer: F) -> Self
    where
        Self: Sized,
        F: SystemParamFunction<Marker, In = Trigger<'static, E, B>, Out = ()> + Send + Sync + 'a,
        E: Event,
        B: Bundle,
    {
        let observer_cell = Cell::new(Some(observer));
        let f: Rc<dyn Fn(Spawn) -> Spawn> = Rc::new(move |spawn| {
            let observer = observer_cell.take().unwrap();
            let spawn: Spawn<'a> = unsafe { mem::transmute(spawn) };
            let spawn = spawn.observe(observer);
            let spawn: Spawn = unsafe { mem::transmute(spawn) };
            spawn
        });
        let f: Rc<dyn Fn(Spawn) -> Spawn> = unsafe { mem::transmute(f) };
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
