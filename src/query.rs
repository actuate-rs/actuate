use crate::{Id, World};
use alloc::vec::Vec;
use core::{
    any::{self, TypeId},
    cell::UnsafeCell,
};

pub trait Query<'a> {
    type Output<'w>;

    fn reads(ids: &mut Vec<Id>);

    fn writes(ids: &mut Vec<Id>);

    fn query<'w>(world: &UnsafeCell<&'w mut World>) -> Self::Output<'w>;
}

impl<'a, T: 'static> Query<'a> for &'a T {
    type Output<'w> = &'w T;

    fn reads(ids: &mut Vec<Id>) {
        ids.push(Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        })
    }

    fn writes(_ids: &mut Vec<Id>) {}

    fn query<'w>(world: &UnsafeCell<&'w mut World>) -> Self::Output<'w> {
        let world = unsafe { &mut *world.get() };
        let id = Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        };
        world.states.get(&id).unwrap().downcast_ref().unwrap()
    }
}

impl<'a, T: 'static> Query<'a> for &'a mut T {
    type Output<'w> = &'w mut T;

    fn reads(ids: &mut Vec<Id>) {
        ids.push(Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        })
    }

    fn writes(ids: &mut Vec<Id>) {
        ids.push(Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        })
    }

    fn query<'w>(world: &UnsafeCell<&'w mut World>) -> Self::Output<'w> {
        let world = unsafe { &mut *world.get() };
        let id = Id {
            type_id: TypeId::of::<T>(),
            name: any::type_name::<T>(),
        };
        world.states.get_mut(&id).unwrap().downcast_mut().unwrap()
    }
}

macro_rules! impl_query_for_tuple {
    ($($t:tt),*) => {
        impl<'a, $($t: Query<'a>),*> Query<'a> for ($($t),*) {
            type Output<'w> = ($($t::Output<'w>),*);

            fn reads(ids: &mut Vec<Id>) {
                $($t::reads(ids));*
            }

            fn writes(ids: &mut Vec<Id>) {
                $($t::writes(ids));*
            }

            fn query<'w>(world: &UnsafeCell<&'w mut World>) -> Self::Output<'w> {
                ($($t::query(world)),*)
            }
        }
    };
}

impl_query_for_tuple!(Q1, Q2);
impl_query_for_tuple!(Q1, Q2, Q3);
impl_query_for_tuple!(Q1, Q2, Q3, Q4);
impl_query_for_tuple!(Q1, Q2, Q3, Q4, Q5);
impl_query_for_tuple!(Q1, Q2, Q3, Q4, Q5, Q6);
impl_query_for_tuple!(Q1, Q2, Q3, Q4, Q5, Q6, Q7);
impl_query_for_tuple!(Q1, Q2, Q3, Q4, Q5, Q6, Q7, Q8);
