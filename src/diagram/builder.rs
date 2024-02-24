use super::{
    node::{Node, NodeData},
    Diagram,
};
use crate::{
    system::{AnySystem, IntoSystem},
    Id, World,
};
use std::{
    any::{self, Any},
    collections::{HashMap, HashSet},
    mem,
};

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
                system.writes_any(&mut writes);

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

                queue.extend(node.writes.iter().copied());
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
            world: World {
                states: mem::take(&mut self.states),
            },
            inputs,
            outputs: mem::take(&mut self.outputs),
            finished_systems: HashSet::new(),
        }
    }
}
