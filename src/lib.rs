use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
    fmt,
};

pub mod diagram;
pub use self::diagram::Diagram;

mod query;
pub use self::query::Query;

pub mod system;
pub use self::system::System;

#[derive(Default)]
pub struct World {
    states: HashMap<Id, Box<dyn Any>>,
}

impl World {
    pub fn query<'a, Q: Query<'a>>(&'a mut self) -> Q {
        Q::query(&UnsafeCell::new(self))
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Id {
    type_id: TypeId,
    name: &'static str,
}

impl fmt::Debug for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Id").field(&self.name).finish()
    }
}
