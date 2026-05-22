use crate::db::Database;
use crate::types::{Edge, EdgeKind, GraphPath, Node, NodeEdge};
use anyhow::Result;
use serde::Serialize;
use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

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
        let mut seen_edges = BTreeSet::new();
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
                            push_unique_edge(&mut out.edges, &mut seen_edges, edge.clone());
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
                    push_unique_edge(&mut out.edges, &mut seen_edges, edge);
                    queue.push_back((source.id, depth + 1));
                }
            }
        }

        out.edges.sort_by(edge_sort_key);
        Ok(out)
    }

    pub fn find_paths(
        &self,
        from_node_id: &str,
        to_node_id: &str,
        max_depth: usize,
        max_paths: usize,
    ) -> Result<Vec<GraphPath>> {
        if max_depth == 0 || max_paths == 0 {
            return Ok(Vec::new());
        }
        let Some(root) = self.db.get_node(from_node_id)? else {
            return Ok(Vec::new());
        };
        if self.db.get_node(to_node_id)?.is_none() {
            return Ok(Vec::new());
        }

        let path_kinds = [
            EdgeKind::Calls,
            EdgeKind::References,
            EdgeKind::Imports,
            EdgeKind::Extends,
            EdgeKind::Implements,
        ];
        let mut out = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(PathState {
            node_id: from_node_id.to_string(),
            nodes: vec![root],
            edges: Vec::new(),
            visited: BTreeSet::from([from_node_id.to_string()]),
        });

        while let Some(state) = queue.pop_front() {
            if state.edges.len() >= max_depth {
                continue;
            }

            let mut outgoing = self
                .db
                .get_outgoing_edges(&state.node_id, Some(&path_kinds))?;
            outgoing.sort_by(edge_sort_key);
            for edge in outgoing {
                if state.visited.contains(&edge.target) {
                    continue;
                }
                let Some(next_node) = self.db.get_node(&edge.target)? else {
                    continue;
                };
                let mut nodes = state.nodes.clone();
                nodes.push(next_node.clone());
                let mut edges = state.edges.clone();
                edges.push(edge.clone());
                if edge.target == to_node_id {
                    out.push(GraphPath { nodes, edges });
                    if out.len() >= max_paths {
                        return Ok(out);
                    }
                    continue;
                }
                let mut visited = state.visited.clone();
                visited.insert(edge.target.clone());
                queue.push_back(PathState {
                    node_id: edge.target,
                    nodes,
                    edges,
                    visited,
                });
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
        let mut emitted = BTreeSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((node_id.to_string(), 0usize));

        while let Some((current, depth)) = queue.pop_front() {
            if depth >= max_depth || !visited.insert(current.clone()) {
                continue;
            }

            let mut edges = match direction {
                Direction::Incoming => self.db.get_incoming_edges(&current, Some(kinds))?,
                Direction::Outgoing => self.db.get_outgoing_edges(&current, Some(kinds))?,
            };
            edges.sort_by(edge_sort_key);

            for edge in edges {
                let next = match direction {
                    Direction::Incoming => &edge.source,
                    Direction::Outgoing => &edge.target,
                };
                if visited.contains(next) || emitted.contains(next) {
                    continue;
                }
                if let Some(node) = self.db.get_node(next)? {
                    queue.push_back((node.id.clone(), depth + 1));
                    emitted.insert(node.id.clone());
                    out.push(NodeEdge {
                        node,
                        edge,
                        depth: depth + 1,
                    });
                }
            }
        }

        out.sort_by(node_edge_sort);
        Ok(out)
    }
}

struct PathState {
    node_id: String,
    nodes: Vec<Node>,
    edges: Vec<Edge>,
    visited: BTreeSet<String>,
}

enum Direction {
    Incoming,
    Outgoing,
}

fn push_unique_edge(edges: &mut Vec<Edge>, seen: &mut BTreeSet<String>, edge: Edge) {
    let key = edge_key(&edge);
    if seen.insert(key) {
        edges.push(edge);
    }
}

fn edge_key(edge: &Edge) -> String {
    format!(
        "{}\0{}\0{}\0{:?}\0{:?}",
        edge.source,
        edge.target,
        edge.kind.as_str(),
        edge.line,
        edge.col
    )
}

fn edge_sort_key(a: &Edge, b: &Edge) -> std::cmp::Ordering {
    edge_key(a).cmp(&edge_key(b))
}

fn node_edge_sort(a: &NodeEdge, b: &NodeEdge) -> std::cmp::Ordering {
    a.depth
        .cmp(&b.depth)
        .then_with(|| a.node.file_path.cmp(&b.node.file_path))
        .then_with(|| a.node.start_line.cmp(&b.node.start_line))
        .then_with(|| a.node.kind.as_str().cmp(b.node.kind.as_str()))
        .then_with(|| a.node.name.cmp(&b.node.name))
}
