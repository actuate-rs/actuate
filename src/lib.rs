use std::{
    any::TypeId,
    ops::{Deref, DerefMut},
};

mod world;
pub use self::world::{ComponentMut, EntityMut, UnsafeWorldCell, World};

mod system;
pub use self::system::{FunctionSystem, IntoSystem, System, SystemParam, SystemParamFunction};

mod query;
pub use self::query::{Query, QueryData};

#[derive(Clone, Copy)]
pub struct Entity {
    id: usize,
}

#[derive(Clone, Copy, Hash, PartialEq, Eq)]
pub struct SystemId {
    id: usize,
}

pub struct Ref<'w, T> {
    world: UnsafeWorldCell<'w>,
    value: &'w T,
    entity: Entity,
}

impl<T: 'static> Deref for Ref<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let world = unsafe { &mut *self.world.ptr };
        world.reads.push((self.entity, TypeId::of::<T>()));
        self.value
    }
}

impl<'a, T: 'static> QueryData for Ref<'a, T> {
    type Data<'w> = Ref<'w, T>;

    unsafe fn query_data<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Self::Data<'w> {
        if let Some(id) = (&mut *world.ptr).current_system_id {
            if let Some(ids) = (&mut *world.ptr)
                .query_system_ids
                .get_mut(&TypeId::of::<T>())
            {
                ids.push(id);
            } else {
                (&mut *world.ptr)
                    .query_system_ids
                    .insert(TypeId::of::<T>(), vec![id]);
            }
        }

        Ref {
            world,
            value: (&mut *world.ptr).entities[entity.id]
                .get(&TypeId::of::<T>())
                .and_then(|x| x.value.downcast_ref())
                .unwrap(),
            entity,
        }
    }
}

pub struct Mut<'w, T> {
    world: UnsafeWorldCell<'w>,
    value: &'w mut T,
    entity: Entity,
}

impl<T: 'static> Deref for Mut<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        let world = unsafe { &mut *self.world.ptr };
        world.reads.push((self.entity, TypeId::of::<T>()));
        self.value
    }
}

impl<T: 'static> DerefMut for Mut<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        let world = unsafe { &mut *self.world.ptr };
        world.reads.push((self.entity, TypeId::of::<T>()));

        let component_data = world.entities[self.entity.id]
            .get_mut(&TypeId::of::<T>())
            .unwrap();

        if let Some(id) = world.current_system_id {
            component_data.readers.push(id);
        }

        for id in &component_data.readers {
            world.queued_system_ids.insert(*id);
        }

        if let Some(ids) = world.query_system_ids.get(&TypeId::of::<T>()) {
            for id in ids {
                world.queued_system_ids.insert(*id);
            }
        }

        self.value
    }
}

impl<'a, T: 'static> QueryData for Mut<'a, T> {
    type Data<'w> = Mut<'w, T>;

    unsafe fn query_data<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Self::Data<'w> {
        Mut {
            world,
            value: (&mut *world.ptr).entities[entity.id]
                .get_mut(&TypeId::of::<T>())
                .and_then(|x| x.value.downcast_mut())
                .unwrap(),
            entity,
        }
    }
}

pub trait Component: Sized {
    fn start(me: &mut ComponentMut<Self>) {
        let _ = me;
    }
}
