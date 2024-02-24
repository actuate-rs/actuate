use crate::{Id, World};
use alloc::vec::Vec;
use core::{cell::UnsafeCell, fmt, mem};

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
                queue.extend(node.children.iter().copied());
            }
        }
        mem::take(&mut self.finished_systems);
    }
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
