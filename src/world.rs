use crate::{Entity, IntoSystem, Query, QueryData, System, SystemId};
use slab::Slab;
use std::{
    any::{Any, TypeId},
    collections::{HashMap, HashSet},
    marker::PhantomData,
    mem,
};

pub(crate) struct ComponentData {
    pub(crate) value: Box<dyn Any>,
    pub(crate) readers: Vec<SystemId>,
}

#[derive(Default)]
pub struct World {
    pub(crate) entities: Slab<HashMap<TypeId, ComponentData>>,
    pub(crate) reads: Vec<(Entity, TypeId)>,
    pub(crate) systems: Slab<Box<dyn System>>,
    pub(crate) queued_system_ids: HashSet<SystemId>,
    pub(crate) current_system_id: Option<SystemId>,
    pub(crate) query_system_ids: HashMap<TypeId, Vec<SystemId>>,
}

impl World {
    pub fn add_system<'w, Marker>(&mut self, system: impl IntoSystem<Marker>) -> SystemId {
        let id = self.systems.insert(Box::new(system.into_system()));
        self.queued_system_ids.insert(SystemId { id });
        SystemId { id }
    }

    pub fn run_system(&mut self, id: SystemId) {
        let ptr = self as _;
        let system = self.systems[id.id].as_mut();

        self.current_system_id = Some(id);
        system.run(UnsafeWorldCell {
            ptr,
            _marker: PhantomData,
        });
        self.current_system_id = None;

        for (entity, type_id) in mem::take(&mut self.reads) {
            self.entities[entity.id]
                .get_mut(&type_id)
                .unwrap()
                .readers
                .push(id);
        }
    }

    pub fn run(&mut self) {
        for system_id in mem::take(&mut self.queued_system_ids) {
            self.run_system(system_id);
        }
    }

    pub fn spawn(&mut self) -> EntityMut {
        let id = self.entities.insert(HashMap::new());
        EntityMut {
            id: Entity { id },
            world: self,
        }
    }

    pub fn query<'w, D: QueryData>(&'w mut self) -> Query<D> {
        Query {
            world: UnsafeWorldCell {
                ptr: self,
                _marker: PhantomData,
            },
            _marker: PhantomData,
        }
    }
}

#[derive(Copy, Clone)]
pub struct UnsafeWorldCell<'w> {
    pub(crate) ptr: *mut World,
    pub(crate) _marker: PhantomData<&'w World>,
}

pub struct EntityMut<'a> {
    pub(crate) id: Entity,
    pub(crate) world: &'a mut World,
}

impl EntityMut<'_> {
    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn insert(&mut self, component: impl Any) -> &mut Self {
        if let Some(ids) = self.world.query_system_ids.get(&component.type_id()) {
            for id in ids {
                self.world.queued_system_ids.insert(*id);
            }
        }

        self.world.entities[self.id.id].insert(
            component.type_id(),
            ComponentData {
                value: Box::new(component),
                readers: Vec::new(),
            },
        );
        self
    }

    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.world.entities[self.id.id]
            .get(&TypeId::of::<T>())?
            .value
            .downcast_ref()
    }

    pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.world.entities[self.id.id]
            .get_mut(&TypeId::of::<T>())?
            .value
            .downcast_mut()
    }
}
