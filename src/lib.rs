use std::{
    any::{Any, TypeId},
    collections::HashMap,
};
use slab::Slab;

pub struct Entity {
    id: usize,
}

#[derive(Default)]
pub struct World {
    entities: Slab<HashMap<TypeId, Box<dyn Any>>>,
}

impl World {
    pub fn spawn(&mut self) -> EntityMut {
        let id = self.entities.insert(HashMap::new());
        EntityMut {
            id: Entity { id },
            world: self,
        }
    }
}

pub struct EntityMut<'a> {
    id: Entity,
    world: &'a mut World,
}

impl EntityMut<'_> {
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
