use slotmap::{DefaultKey, SlotMap};
use std::{
    any::{self, TypeId},
    marker::PhantomData,
};

#[derive(Clone, Copy)]
pub struct Id {
    name: &'static str,
    type_id: TypeId,
}

impl Id {
    pub fn new<T: 'static>() -> Self {
        Self {
            name: any::type_name::<T>(),
            type_id: TypeId::of::<T>(),
        }
    }
}

pub trait System: 'static {
    fn inputs(&self) -> Vec<Id>;

    fn outputs(&self) -> Vec<Id>;
}

pub trait IntoSystem<Marker> {
    type System: System;

    fn into_system(self) -> Self::System;
}

pub struct FnSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<F, I1, O1> System for FnSystem<F, (I1, O1)>
where
    F: Fn(I1) -> O1 + 'static,
    I1: 'static,
    O1: 'static,
{
    fn inputs(&self) -> Vec<Id> {
        vec![Id::new::<I1>()]
    }

    fn outputs(&self) -> Vec<Id> {
        vec![Id::new::<O1>()]
    }
}

impl<F, I1, O1> IntoSystem<(I1, O1)> for F
where
    F: Fn(I1) -> O1 + 'static,
    I1: 'static,
    O1: 'static,
{
    type System = FnSystem<F, (I1, O1)>;

    fn into_system(self) -> Self::System {
        FnSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

struct Binding {
    id: Id,
    system_key: Option<DefaultKey>,
}

struct Node {
    system: Box<dyn System>,
    inputs: Vec<Binding>,
    outputs: Vec<Binding>,
}

#[derive(Default)]
pub struct Builder {
    systems: SlotMap<DefaultKey, Node>,
}

impl Builder {
    pub fn add_system<Marker>(&mut self, system: impl IntoSystem<Marker>) -> DefaultKey {
        let system = system.into_system();
        let node = Node {
            inputs: system
                .inputs()
                .iter()
                .map(|id| Binding {
                    id: *id,
                    system_key: None,
                })
                .collect(),
            outputs: system
                .outputs()
                .iter()
                .map(|id| Binding {
                    id: *id,
                    system_key: None,
                })
                .collect(),
            system: Box::new(system),
        };
        self.systems.insert(node)
    }
}

pub struct Diagram {}

impl Diagram {
    pub fn builder() -> Builder {
        Builder::default()
    }
}
