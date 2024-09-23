use slab::Slab;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    ops::Deref,
};

#[derive(Clone, Copy)]
pub struct Entity {
    id: usize,
}

#[derive(Default)]
pub struct World {
    entities: Slab<HashMap<TypeId, Box<dyn Any>>>,
    reads: Vec<(Entity, TypeId)>,
}

impl World {
    pub fn spawn(&mut self) -> EntityMut {
        let id = self.entities.insert(HashMap::new());
        EntityMut {
            id: Entity { id },
            world: self,
        }
    }

    pub fn query<'w, Q: QueryData<'w>>(&'w mut self, entity: Entity) -> Q {
        unsafe {
            QueryData::query_data(
                UnsafeWorldCell {
                    ptr: self as _,
                    _marker: PhantomData,
                },
                entity,
            )
        }
    }
}

pub struct EntityMut<'a> {
    id: Entity,
    world: &'a mut World,
}

impl EntityMut<'_> {
    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn insert(&mut self, component: impl Any) -> &mut Self {
        self.world.entities[self.id.id].insert(component.type_id(), Box::new(component));
        self
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.world.entities[self.id.id]
            .get(&TypeId::of::<T>())?
            .downcast_ref()
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.world.entities[self.id.id]
            .get_mut(&TypeId::of::<T>())?
            .downcast_mut()
    }
}

#[derive(Copy, Clone)]
pub struct UnsafeWorldCell<'w> {
    ptr: *mut World,
    _marker: PhantomData<&'w World>,
}

pub trait QueryData<'w> {
    unsafe fn query_data(world: UnsafeWorldCell<'w>, entity: Entity) -> Self;
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

impl<'w, T: 'static> QueryData<'w> for Ref<'w, T> {
    unsafe fn query_data(world: UnsafeWorldCell<'w>, entity: Entity) -> Self {
        Ref {
            world,
            value: (&mut *world.ptr).entities[entity.id]
                .get(&TypeId::of::<T>())
                .and_then(|x| x.downcast_ref())
                .unwrap(),
            entity,
        }
    }
}
