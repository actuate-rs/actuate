use crate::{Component, Entity, IntoSystem, Query, QueryData, System, SystemId};
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
    pub(crate) system_ids: HashSet<SystemId>,
}

#[derive(Default)]
pub struct World {
    pub(crate) entities: Slab<HashMap<TypeId, ComponentData>>,
    pub(crate) reads: Vec<(Entity, TypeId)>,
    pub(crate) systems: Slab<Box<dyn System>>,
    pub(crate) queued_system_ids: HashSet<SystemId>,
    pub(crate) current_system_id: Option<SystemId>,
    pub(crate) query_system_ids: HashMap<TypeId, Vec<SystemId>>,
    pub(crate) initialized_systems: HashMap<TypeId, usize>,
}

impl World {
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

    pub fn insert<T: Component + 'static>(&mut self, component: T) -> ComponentMut<T> {
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
                system_ids: HashSet::new(),
            },
        );

        T::start(&mut ComponentsMut {
            world: self.world,
            _marker: PhantomData,
        });

        ComponentMut {
            id: self.id,
            world: self.world,
            _marker: PhantomData,
        }
    }

    pub fn component_mut<T>(&mut self) -> ComponentMut<T> {
        ComponentMut {
            id: self.id,
            world: self.world,
            _marker: PhantomData,
        }
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

pub struct ComponentsMut<'a, T> {
    pub(crate) world: &'a mut World,
    _marker: PhantomData<T>,
}

impl<T: 'static> ComponentsMut<'_, T> {
    pub fn add_system<'w, Marker>(&mut self, system: impl IntoSystem<Marker>) {
        if let Some(count) = self.world.initialized_systems.get_mut(&TypeId::of::<T>()) {
            *count += 1;
            return;
        }
        self.world.initialized_systems.insert(TypeId::of::<T>(), 1);

        let id = self.world.systems.insert(Box::new(system.into_system()));
        self.world.queued_system_ids.insert(SystemId { id });

    }
}

pub struct ComponentMut<'a, T> {
    pub(crate) id: Entity,
    pub(crate) world: &'a mut World,
    _marker: PhantomData<T>,
}

impl<'a, T> ComponentMut<'a, T>
where
    T: 'static,
{
    pub fn entity(&mut self) -> EntityMut {
        EntityMut {
            id: self.id,
            world: self.world,
        }
    }

    pub fn get(&self) -> &T {
        self.world.entities[self.id.id]
            .get(&TypeId::of::<T>())
            .unwrap()
            .value
            .downcast_ref()
            .unwrap()
    }

    pub fn get_mut(&mut self) -> &mut T {
        self.world.entities[self.id.id]
            .get_mut(&TypeId::of::<T>())
            .unwrap()
            .value
            .downcast_mut()
            .unwrap()
    }

    pub fn remove(&mut self) {
        let data = self.world.entities[self.id.id]
            .remove(&TypeId::of::<T>())
            .unwrap();

        if let Some(count) = self.world.initialized_systems.get_mut(&TypeId::of::<T>()) {
            *count -= 1;

            if *count == 0 {
                for id in data.system_ids {
                    self.world.systems.remove(id.id);
                    self.world.queued_system_ids.remove(&id);
                }
            }
        }
    }
}
