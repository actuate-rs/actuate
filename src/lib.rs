#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use core::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    fmt,
};

#[cfg(not(feature = "std"))]
use alloc::boxed::Box;

pub mod control;

pub mod diagram;
pub use self::diagram::Diagram;

mod query;
pub use self::query::Query;

pub mod system;
pub use self::system::System;

pub mod time;

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
