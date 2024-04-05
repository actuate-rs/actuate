use slotmap::{DefaultKey, SlotMap};
use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

pub trait Component {}

pub trait ComponentBuilder {
    fn build(self, entity: &mut Entity);
}

impl<C: Component + 'static> ComponentBuilder for C {
    fn build(self, entity: &mut Entity) {
        entity.components.insert(self.type_id(), Box::new(self));
    }
}

#[derive(Default)]
pub struct Entity {
    components: HashMap<TypeId, Box<dyn Any>>,
}

#[derive(Default)]
pub struct World {
    entities: SlotMap<DefaultKey, Entity>,
}

impl World {
    pub fn spawn(&mut self, component: impl ComponentBuilder) -> EntityHandle {
        let mut entity = Entity::default();
        component.build(&mut entity);
        let key = self.entities.insert(entity);
        EntityHandle { key }
    }
}

#[derive(Clone, Copy)]
pub struct EntityHandle {
    key: DefaultKey,
}

impl EntityHandle {
    pub fn query<Q: Query>(self, world: &mut World) -> Q::Output<'_> {
        let entity = &mut world.entities[self.key];
        Q::query(entity)
    }
}

pub trait Query {
    type Output<'e>;

    fn query(entity: &mut Entity) -> Self::Output<'_>;
}

impl<T: 'static> Query for &T {
    type Output<'e> = &'e T;

    fn query(entity: &mut Entity) -> Self::Output<'_> {
        entity
            .components
            .get(&TypeId::of::<T>())
            .and_then(|any| any.downcast_ref())
            .unwrap()
    }
}
