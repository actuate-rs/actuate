use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    fmt,
    time::{SystemTime, UNIX_EPOCH},
};

pub mod diagram;
pub use self::diagram::Diagram;

mod query;
pub use self::query::Query;

mod gain;
pub use self::gain::Gain;

mod pid;
pub use self::pid::PidController;

pub mod system;
pub use self::system::System;

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time(pub u64);

pub trait Plugin {
    fn build(self, diagram: &mut diagram::Builder);
}

fn time_system(Time(time): &mut Time) {
    *time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as _;
}

pub struct TimePlugin;

impl Plugin for TimePlugin {
    fn build(self, diagram: &mut diagram::Builder) {
        diagram.add_state(Time(0)).add_system(time_system);
    }
}
