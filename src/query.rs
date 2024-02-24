use std::{
    any::{self, TypeId},
    cell::UnsafeCell,
};

use crate::{Id, World};

pub trait Query<'a> {
    fn reads(ids: &mut Vec<Id>);

    fn writes(ids: &mut Vec<Id>);

    fn query(world: &UnsafeCell<&'a mut World>) -> Self;
}

impl<'a, T: 'static> Query<'a> for &'a T {
    fn reads(ids: &mut Vec<Id>) {
        ids.push(Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        })
    }

    fn writes(_ids: &mut Vec<Id>) {}

    fn query(world: &UnsafeCell<&'a mut World>) -> Self {
        let world = unsafe { &mut *world.get() };
        let id = Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        };
        world.states.get(&id).unwrap().downcast_ref().unwrap()
    }
}

impl<'a, T: 'static> Query<'a> for &'a mut T {
    fn reads(_ids: &mut Vec<Id>) {}

    fn writes(ids: &mut Vec<Id>) {
        ids.push(Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        })
    }

    fn query(world: &UnsafeCell<&'a mut World>) -> Self {
        let world = unsafe { &mut *world.get() };
        let id = Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        };
        world.states.get_mut(&id).unwrap().downcast_mut().unwrap()
    }
}

impl<'a, Q1: Query<'a>, Q2: Query<'a>> Query<'a> for (Q1, Q2) {
    fn reads(ids: &mut Vec<Id>) {
        Q1::reads(ids);
        Q2::reads(ids)
    }

    fn writes(ids: &mut Vec<Id>) {
        Q1::writes(ids);
        Q2::writes(ids)
    }

    fn query(world: &UnsafeCell<&'a mut World>) -> Self {
        // TODO: check for overlaps
        (Q1::query(world), Q2::query(world))
    }
}
