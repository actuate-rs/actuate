use std::{
    any::{self, Any, TypeId},
    cell::UnsafeCell,
    collections::{HashMap, HashSet},
    fmt,
    marker::PhantomData,
    mem,
};

#[derive(Default)]
pub struct World {
    states: HashMap<TypeId, Box<dyn Any>>,
}

impl World {
    pub fn query<'a, Q: Query<'a>>(&'a mut self) -> Q {
        Q::query(&UnsafeCell::new(self))
    }
}

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
        world
            .states
            .get(&TypeId::of::<T>())
            .unwrap()
            .downcast_ref()
            .unwrap()
    }
}

pub trait System<'a>: 'static {
    type Query: Query<'a>;

    fn run(&self, query: Self::Query);
}

pub struct FnSystem<F, Marker> {
    f: F,
    _marker: PhantomData<Marker>,
}

impl<'a, F, Q> System<'a> for FnSystem<F, Q>
where
    F: Fn(Q) + 'static,
    Q: Query<'a> + 'static,
{
    type Query = Q;

    fn run(&self, query: Self::Query) {
        (self.f)(query)
    }
}

pub trait IntoSystem<'a, Marker> {
    type System: System<'a>;

    fn into_system(self) -> Self::System;
}

impl<'a, F, Q> IntoSystem<'a, Q> for F
where
    F: Fn(Q) + 'static,
    Q: Query<'a> + 'static,
{
    type System = FnSystem<F, Q>;

    fn into_system(self) -> Self::System {
        FnSystem {
            f: self,
            _marker: PhantomData,
        }
    }
}

trait AnySystem {
    fn reads_any(&self, ids: &mut Vec<Id>);

    fn writes_any(&self, ids: &mut Vec<Id>);

    unsafe fn run_any(&self, world: &UnsafeCell<&mut World>);
}

impl<'a, S: System<'a>> AnySystem for S {
    fn reads_any(&self, ids: &mut Vec<Id>) {
        S::Query::reads(ids)
    }

    fn writes_any(&self, ids: &mut Vec<Id>) {
        S::Query::writes(ids)
    }

    unsafe fn run_any(&self, world: &UnsafeCell<&mut World>) {
        let world = unsafe { mem::transmute(world) };
        let query = S::Query::query(world);
        self.run(query)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Id {
    type_id: TypeId,
    name: &'static str,
}

#[derive(Default)]
pub struct Builder {
    states: HashMap<Id, Box<dyn Any>>,
    inputs: HashSet<Id>,
    outputs: HashSet<Id>,
    systems: HashMap<Id, Box<dyn AnySystem>>,
}

impl Builder {
    pub fn add_system<'a, Marker>(&mut self, system: impl IntoSystem<'a, Marker>) -> &mut Self
    where
        Self: 'a,
    {
        let s = system.into_system();
        let id = Id {
            type_id: s.type_id(),
            name: any::type_name_of_val(&s),
        };
        self.systems.insert(id, Box::new(s));
        self
    }

    pub fn add_state(&mut self, state: impl Any) -> &mut Self {
        let id = Id {
            type_id: state.type_id(),
            name: any::type_name_of_val(&state),
        };
        self.states.insert(id, Box::new(state));
        self
    }

    pub fn add_input(&mut self, input: impl Any) -> &mut Self {
        self.inputs.insert(Id {
            type_id: input.type_id(),
            name: any::type_name_of_val(&input),
        });
        self.add_state(input)
    }

    pub fn add_output(&mut self, input: impl Any) -> &mut Self {
        self.outputs.insert(Id {
            type_id: input.type_id(),
            name: any::type_name_of_val(&input),
        });
        self.add_state(input)
    }

    pub fn build(&mut self) -> Diagram {
        let mut node_datas: HashMap<_, _> = mem::take(&mut self.systems)
            .into_iter()
            .map(|(system_id, system)| {
                let mut reads = Vec::new();
                system.reads_any(&mut reads);

                let mut writes = Vec::new();
                system.reads_any(&mut writes);

                let node = NodeData {
                    reads,
                    writes,
                    system,
                };
                (system_id, node)
            })
            .collect();

        let mut nodes = HashMap::new();
        let mut inputs = Vec::new();
        let mut queue: Vec<_> = self.inputs.iter().copied().collect();

        while let Some(input_id) = queue.pop() {
            let mut readers = Vec::new();
            for (id, node) in &node_datas {
                if node.reads.contains(&input_id) {
                    readers.push(*id);

                    if self.inputs.contains(&input_id) {
                        inputs.push((input_id, *id))
                    }
                }
            }

            for id in readers {
                let node = node_datas.remove(&id).unwrap();
                let mut children = Vec::new();
                for (other_id, other_node) in &node_datas {
                    for write_id in &node.writes {
                        if other_node.reads.contains(&write_id) {
                            children.push(*other_id);
                        }
                    }
                }
                nodes.insert(
                    id,
                    Node {
                        data: node,
                        children,
                    },
                );
            }
        }

        Diagram {
            nodes,
            states: mem::take(&mut self.states),
            inputs,
            outputs: mem::take(&mut self.outputs),
        }
    }
}

struct NodeData {
    system: Box<dyn AnySystem>,
    reads: Vec<Id>,
    writes: Vec<Id>,
}

struct Node {
    data: NodeData,
    children: Vec<Id>,
}

struct NodeDebugger<'a> {
    id: Id,
    node: &'a Node,
    diagram: &'a Diagram,
}

impl fmt::Debug for NodeDebugger<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let children = {
            let mut children = f.debug_list();
            for child_id in &self.node.children {
                let child = self.diagram.nodes.get(child_id).unwrap();
                children.entry(&NodeDebugger {
                    id: *child_id,
                    node: child,
                    diagram: self.diagram,
                });
            }
            children.finish()?
        };

        let mut s = f.debug_struct(self.id.name);
        s.field("children", &children);
        s.finish()
    }
}

pub struct Diagram {
    nodes: HashMap<Id, Node>,
    states: HashMap<Id, Box<dyn Any>>,
    inputs: Vec<(Id, Id)>,
    outputs: HashSet<Id>,
}

impl fmt::Debug for Diagram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut list = f.debug_list();

        for (_input_id, id) in &self.inputs {
            let node = self.nodes.get(id).unwrap();
            list.entry(&NodeDebugger {
                id: *id,
                node,
                diagram: self,
            });
        }

        list.finish()?;

        Ok(())
    }
}
