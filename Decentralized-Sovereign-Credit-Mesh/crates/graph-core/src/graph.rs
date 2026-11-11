use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::{Bfs, EdgeRef};
use std::collections::HashMap;

use crate::types::*;

pub struct SocialGraph {
    graph: DiGraph<NodeData, EdgeData>,
    node_indices: HashMap<NodeId, NodeIndex>,
}

impl SocialGraph {
    pub fn new() -> Self {
        SocialGraph {
            graph: DiGraph::new(),
            node_indices: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, data: NodeData) -> NodeIndex {
        if let Some(&idx) = self.node_indices.get(&data.id) {
            self.graph[idx] = data;
            idx
        } else {
            let idx = self.graph.add_node(data.clone());
            self.node_indices.insert(data.id, idx);
            idx
        }
    }

    pub fn add_edge(&mut self, source: &str, target: &str, data: EdgeData) -> Result<(), String> {
        let src_idx = self
            .node_indices
            .get(source)
            .ok_or_else(|| format!("source node '{}' not found", source))?;
        let tgt_idx = self
            .node_indices
            .get(target)
            .ok_or_else(|| format!("target node '{}' not found", target))?;
        self.graph.add_edge(*src_idx, *tgt_idx, data);
        Ok(())
    }

    pub fn get_node(&self, id: &str) -> Option<&NodeData> {
        self.node_indices
            .get(id)
            .and_then(|&idx| self.graph.node_weight(idx))
    }

    pub fn contains_node(&self, id: &str) -> bool {
        self.node_indices.contains_key(id)
    }

    pub fn neighbors(&self, id: &str) -> Vec<(NodeId, &EdgeData)> {
        if let Some(&idx) = self.node_indices.get(id) {
            self.graph
                .edges_directed(idx, petgraph::Direction::Outgoing)
                .map(|e| {
                    let target_idx = e.target();
                    let target_id = self.graph[target_idx].id.clone();
                    (target_id, e.weight())
                })
                .collect()
        } else {
            vec![]
        }
    }

    pub fn predecessors(&self, id: &str) -> Vec<(NodeId, &EdgeData)> {
        if let Some(&idx) = self.node_indices.get(id) {
            self.graph
                .edges_directed(idx, petgraph::Direction::Incoming)
                .map(|e| {
                    let source_idx = e.source();
                    let source_id = self.graph[source_idx].id.clone();
                    (source_id, e.weight())
                })
                .collect()
        } else {
            vec![]
        }
    }

    pub fn all_nodes(&self) -> Vec<&NodeData> {
        self.graph.node_weights().collect()
    }

    pub fn all_edges(&self) -> Vec<(NodeId, NodeId, &EdgeData)> {
        self.graph
            .edge_references()
            .map(|e| {
                let source = self.graph[e.source()].id.clone();
                let target = self.graph[e.target()].id.clone();
                (source, target, e.weight())
            })
            .collect()
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    pub fn bfs(&self, start: &str) -> Vec<NodeId> {
        let mut visited = Vec::new();
        if let Some(&start_idx) = self.node_indices.get(start) {
            let mut bfs = Bfs::new(&self.graph, start_idx);
            while let Some(idx) = bfs.next(&self.graph) {
                visited.push(self.graph[idx].id.clone());
            }
        }
        visited
    }

    pub fn shortest_trust_path(&self, from: &str, to: &str) -> Option<Vec<NodeId>> {
        use std::collections::VecDeque;

        let from_idx = self.node_indices.get(from)?;
        let to_idx = self.node_indices.get(to)?;

        let mut visited = HashMap::new();
        let mut queue = VecDeque::new();
        visited.insert(*from_idx, None);
        queue.push_back(*from_idx);

        while let Some(current) = queue.pop_front() {
            if current == *to_idx {
                let mut path = Vec::new();
                let mut step = Some(current);
                while let Some(idx) = step {
                    path.push(self.graph[idx].id.clone());
                    step = visited[&idx];
                }
                path.reverse();
                return Some(path);
            }
            for edge in self
                .graph
                .edges_directed(current, petgraph::Direction::Outgoing)
            {
                let next = edge.target();
                if let std::collections::hash_map::Entry::Vacant(e) = visited.entry(next) {
                    e.insert(Some(current));
                    queue.push_back(next);
                }
            }
        }
        None
    }

    pub fn to_snapshot(&self) -> GraphSnapshot {
        let nodes = self.all_nodes().into_iter().cloned().collect();
        let edges = self
            .all_edges()
            .into_iter()
            .map(|(s, t, e)| (s, t, e.clone()))
            .collect();
        GraphSnapshot { nodes, edges }
    }

    pub fn from_snapshot(snapshot: GraphSnapshot) -> Self {
        let mut graph = SocialGraph::new();
        for node in snapshot.nodes {
            graph.add_node(node);
        }
        for (source, target, edge) in snapshot.edges {
            let _ = graph.add_edge(&source, &target, edge);
        }
        graph
    }
}

impl Default for SocialGraph {
    fn default() -> Self {
        Self::new()
    }
}
