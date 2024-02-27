use super::Diagram;
use crate::{system::AnySystem, Id};
use core::fmt;

#[cfg(not(feature = "std"))]
use alloc::{boxed::Box, vec::Vec};

pub(super) struct NodeData {
    pub(super) system: Box<dyn AnySystem>,
    pub(super) reads: Vec<Id>,
    pub(super) writes: Vec<Id>,
}

pub(super) struct Node {
    pub(super) data: NodeData,
    pub(super) children: Vec<(Id, Id)>,
}

pub(super) struct NodeDebugger<'a> {
    pub(super) id: Id,
    pub(super) node: &'a Node,
    pub(super) diagram: &'a Diagram,
}

impl fmt::Debug for NodeDebugger<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = f.debug_struct(self.id.name);
        s.field(
            "children",
            &NodeChildrenDebugger {
                node: self.node,
                diagram: self.diagram,
            },
        )
        .field("reads", &self.node.data.reads)
        .field("writes", &self.node.data.writes)
        .finish()
    }
}

struct NodeChildrenDebugger<'a> {
    node: &'a Node,
    diagram: &'a Diagram,
}

impl fmt::Debug for NodeChildrenDebugger<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut children = f.debug_list();
        for (_, child_id) in &self.node.children {
            let child = self.diagram.nodes.get(child_id).unwrap();
            children.entry(&NodeDebugger {
                id: *child_id,
                node: child,
                diagram: self.diagram,
            });
        }
        children.finish()
    }
}
