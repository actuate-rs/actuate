use crate::{Id, World};
use core::{cell::UnsafeCell, fmt, mem};

#[cfg(not(feature = "std"))]
use alloc::{format, string::String, vec::Vec};

#[cfg(feature = "std")]
use std::collections::{HashMap, HashSet};

#[cfg(not(feature = "std"))]
use hashbrown::{HashMap, HashSet};

mod builder;
pub use self::builder::Builder;

mod node;
use self::node::{Node, NodeDebugger};

pub struct Diagram {
    nodes: HashMap<Id, Node>,
    world: World,
    inputs: Vec<(Id, Id)>,
    finished_systems: HashSet<Id>,
}

impl Diagram {
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub fn run(&mut self) {
        let mut queue: Vec<_> = self.inputs.iter().map(|(_, id)| *id).collect();
        while let Some(id) = queue.pop() {
            if self.finished_systems.insert(id) {
                let node = self.nodes.get_mut(&id).unwrap();
                unsafe { node.data.system.run_any(&UnsafeCell::new(&mut self.world)) };
                queue.extend(node.children.iter().map(|(_, id)| *id));
            }
        }
        mem::take(&mut self.finished_systems);
    }

    pub fn visualize(&self) -> String {
        let mut s = String::from("graph TD\n");

        let mut next_id = 'A' as u8;
        let mut graph_ids = HashMap::new();

        for (input_id, node_id) in &self.inputs {
            let graph_id = graph_ids.get(&node_id.type_id).copied().unwrap_or_else(|| {
                let graph_id = next_id as char;
                next_id += 1;
                graph_ids.insert(node_id.type_id, graph_id);
                graph_id
            });
            s.push_str(&format!(
                "  Input[Input] --> |\"{}\"| {}\n",
                input_id.name, graph_id
            ));
        }

        for (_, id) in &self.inputs {
            let node = self.nodes.get(id).unwrap();

            let graph_id = graph_ids.get(&id.type_id).copied().unwrap_or_else(|| {
                let graph_id = next_id as char;
                next_id += 1;
                graph_ids.insert(id.type_id, graph_id);
                graph_id
            });
            s.push_str(&format!("  {}[\"{}\"]\n", graph_id, id.name));

            for (write_id, child_id) in &node.children {
                let child_graph_id =
                    graph_ids
                        .get(&child_id.type_id)
                        .copied()
                        .unwrap_or_else(|| {
                            let graph_id = next_id as char;
                            next_id += 1;
                            graph_ids.insert(id.type_id, graph_id);
                            graph_id
                        });
                s.push_str(&format!(
                    "  {} --> |\"{}\"| {}\n",
                    graph_id, write_id.name, child_graph_id
                ));
            }
        }

        s
    }
}

impl fmt::Debug for Diagram {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
