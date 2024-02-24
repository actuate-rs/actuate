#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::boxed::Box;
use core::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    fmt,
};

pub mod control;

pub mod diagram;
pub use self::diagram::Diagram;

mod query;
pub use self::query::Query;

pub mod system;
pub use self::system::System;

#[cfg(feature = "std")]
use std::collections::HashMap;

#[cfg(not(feature = "std"))]
use hashbrown::HashMap;

#[derive(Default)]
pub struct World {
    states: HashMap<Id, Box<dyn Any>>,
}

impl World {
    pub fn query<'a, 'w, Q>(&'w mut self) -> Q::Output<'w>
    where
        Q: Query<'a>,
    {
        Q::query(&UnsafeCell::new(self))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Id {
    type_id: TypeId,
    name: &'static str,
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Id").field(&self.name).finish()
    }
}

pub trait Plugin {
    fn build(self, diagram: &mut diagram::Builder);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time(pub u64);

#[cfg(feature = "std")]
mod time_std {
    use crate::{diagram, Plugin, Time};
    use std::time::{SystemTime, UNIX_EPOCH};

    pub struct TimePlugin;

    impl Plugin for TimePlugin {
        fn build(self, diagram: &mut diagram::Builder) {
            diagram.add_state(Time(0)).add_system(time_system);
        }
    }

    pub fn time_system(Time(time): &mut Time) {
        *time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as _;
    }
}

#[cfg(feature = "std")]
pub use self::time_std::{time_system, TimePlugin};
