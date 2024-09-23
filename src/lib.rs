use slab::Slab;
use std::{
    any::{Any, TypeId},
    collections::HashMap,
    marker::PhantomData,
    mem,
    ops::{Deref, DerefMut},
};

#[derive(Clone, Copy)]
pub struct Entity {
    id: usize,
}

#[derive(Clone, Copy)]
pub struct SystemId {
    id: usize,
}

struct ComponentData {
    value: Box<dyn Any>,
    readers: Vec<SystemId>,
}

#[derive(Default)]
pub struct World {
    entities: Slab<HashMap<TypeId, ComponentData>>,
    reads: Vec<(Entity, TypeId)>,
    systems: Slab<Box<dyn System>>,
    queued_system_ids: Vec<SystemId>,
    current_system_id: Option<SystemId>,
}

impl World {
    pub fn add_system<'w, Marker>(&mut self, system: impl IntoSystem<Marker>) -> SystemId {
        let id = self.systems.insert(Box::new(system.into_system()));
        self.queued_system_ids.push(SystemId { id });
        SystemId { id }
    }

    pub fn run_system(&mut self, id: SystemId) {
        let ptr = self as _;
        let system = self.systems[id.id].as_mut();
        system.run(UnsafeWorldCell {
            ptr,
            _marker: PhantomData,
        });

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

pub struct EntityMut<'a> {
    id: Entity,
    world: &'a mut World,
}

impl EntityMut<'_> {
    pub fn id(&self) -> Entity {
        self.id
    }

    pub fn insert(&mut self, component: impl Any) -> &mut Self {
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

#[derive(Copy, Clone)]
pub struct UnsafeWorldCell<'w> {
    ptr: *mut World,
    _marker: PhantomData<&'w World>,
}

pub trait QueryData {
    type Data<'w>;

    unsafe fn query_data<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Self::Data<'w>;
}

pub struct Query<'w, D> {
    world: UnsafeWorldCell<'w>,
    _marker: PhantomData<D>,
}

impl<'w, D> Query<'w, D> {
    pub fn get(&self, entity: Entity) -> D::Data<'w>
    where
        D: QueryData,
    {
        unsafe { D::query_data(self.world, entity) }
    }
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

        world.queued_system_ids.extend_from_slice(&component_data.readers);

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

pub trait SystemParam {
    type Item<'w>;

    fn system_param<'w>(world: UnsafeWorldCell<'w>) -> Self::Item<'w>;
}

impl<'a, D: QueryData> SystemParam for Query<'a, D> {
    type Item<'w> = Query<'w, D>;

    fn system_param<'w>(world: UnsafeWorldCell<'w>) -> Self::Item<'w> {
        Query {
            world,
            _marker: PhantomData,
        }
    }
}

pub trait System: 'static {
    fn run<'w>(&mut self, world: UnsafeWorldCell<'w>);
}

pub trait IntoSystem<Marker> {
    type System: System;

    fn into_system(self) -> Self::System;
}

impl<Marker: 'static, F: SystemParamFunction<Marker> + 'static> IntoSystem<Marker> for F {
    type System = FunctionSystem<F, Marker>;

    fn into_system(self) -> Self::System {
        FunctionSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

pub struct FunctionSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<Marker: 'static, F: SystemParamFunction<Marker> + 'static> System
    for FunctionSystem<F, Marker>
{
    fn run<'w>(&mut self, world: UnsafeWorldCell<'w>) {
        self.f.run(F::Param::system_param(world));
    }
}

pub trait SystemParamFunction<Marker> {
    type Param: SystemParam;

    fn run(&mut self, param: <Self::Param as SystemParam>::Item<'_>);
}

impl<P, F> SystemParamFunction<fn(P)> for F
where
    P: SystemParam,
    F: FnMut(P) + FnMut(P::Item<'_>),
{
    type Param = P;

    fn run(&mut self, param: <Self::Param as SystemParam>::Item<'_>) {
        self(param)
    }
}
