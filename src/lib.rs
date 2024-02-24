use std::{
    any::{Any, TypeId},
    cell::UnsafeCell,
    collections::HashMap,
};

#[derive(Default)]
pub struct World {
    states: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn query<'a, Q: Query<'a>>(&'a mut self) -> Q {
        Q::query(&UnsafeCell::new(self))
    }
}

pub trait Query<'a> {
    fn query(world: & UnsafeCell<&'a mut World>) -> Self;
}

impl<'a, T: 'static> Query<'a> for &'a T {
    fn query(world: &UnsafeCell<&'a mut World>) -> Self {
        let world = unsafe { &mut *world.get() };
        world
            .states
            .get(&TypeId::of::<T>())
            .unwrap()
            .downcast_ref()
            .unwrap()
    }
}
