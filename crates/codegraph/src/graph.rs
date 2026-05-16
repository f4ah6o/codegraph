use crate::db::Database;
use crate::types::{Edge, EdgeKind, Node, NodeEdge};
use anyhow::Result;
use serde::Serialize;
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Default, Serialize)]
pub struct Subgraph {
    pub nodes: HashMap<String, Node>,
    pub edges: Vec<Edge>,
    pub roots: Vec<String>,
}

pub struct GraphTraverser<'a> {
    db: &'a Database,
}

impl<'a> GraphTraverser<'a> {
    pub fn new(db: &'a Database) -> Self {
        Self { db }
    }

    pub fn get_callers(&self, node_id: &str, max_depth: usize) -> Result<Vec<NodeEdge>> {
        self.walk_edges(
            node_id,
            max_depth,
            Direction::Incoming,
            &[EdgeKind::Calls, EdgeKind::References, EdgeKind::Imports],
        )
    }

    pub fn get_callees(&self, node_id: &str, max_depth: usize) -> Result<Vec<NodeEdge>> {
        self.walk_edges(
            node_id,
            max_depth,
            Direction::Outgoing,
            &[EdgeKind::Calls, EdgeKind::References, EdgeKind::Imports],
        )
    }

    pub fn get_impact_radius(&self, node_id: &str, max_depth: usize) -> Result<Subgraph> {
        let Some(root) = self.db.get_node(node_id)? else {
            return Ok(Subgraph::default());
        };
        let mut out = Subgraph::default();
        out.roots.push(node_id.to_string());
        out.nodes.insert(root.id.clone(), root);

        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((node_id.to_string(), 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_depth || !visited.insert(current.clone()) {
                continue;
            }

            if let Some(node) = self.db.get_node(&current)? {
                if matches!(
                    node.kind,
                    crate::types::NodeKind::Class
                        | crate::types::NodeKind::Interface
                        | crate::types::NodeKind::Struct
                        | crate::types::NodeKind::Trait
                        | crate::types::NodeKind::Protocol
                        | crate::types::NodeKind::Module
                        | crate::types::NodeKind::Enum
                ) {
                    for edge in self
                        .db
                        .get_outgoing_edges(&current, Some(&[EdgeKind::Contains]))?
                    {
                        if let Some(child) = self.db.get_node(&edge.target)? {
                            out.nodes.insert(child.id.clone(), child.clone());
                            out.edges.push(edge.clone());
                            queue.push_back((child.id, depth));
                        }
                    }
                }
            }

            if depth == max_depth {
                continue;
            }

            for edge in self.db.get_incoming_edges(&current, None)? {
                if let Some(source) = self.db.get_node(&edge.source)? {
                    out.nodes.insert(source.id.clone(), source.clone());
                    out.edges.push(edge);
                    queue.push_back((source.id, depth + 1));
                }
            }
        }

        Ok(out)
    }

    fn walk_edges(
        &self,
        node_id: &str,
        max_depth: usize,
        direction: Direction,
        kinds: &[EdgeKind],
    ) -> Result<Vec<NodeEdge>> {
        let mut out = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((node_id.to_string(), 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth || !visited.insert(current.clone()) {
                continue;
            }

            let edges = match direction {
                Direction::Incoming => self.db.get_incoming_edges(&current, Some(kinds))?,
                Direction::Outgoing => self.db.get_outgoing_edges(&current, Some(kinds))?,
            };

            for edge in edges {
                let next = match direction {
                    Direction::Incoming => &edge.source,
                    Direction::Outgoing => &edge.target,
                };
                if visited.contains(next) {
                    continue;
                }
                if let Some(node) = self.db.get_node(next)? {
                    queue.push_back((node.id.clone(), depth + 1));
                    out.push(NodeEdge { node, edge });
                }
            }
        }

        Ok(out)
    }
}

enum Direction {
    Incoming,
    Outgoing,
}
