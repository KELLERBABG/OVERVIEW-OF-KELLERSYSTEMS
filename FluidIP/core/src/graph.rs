//! Evolutionary DAG engine for patent graph mapping.
//!
//! Provides `PatentDag` — a wrapper around `petgraph::stable_graph::StableGraph`
//! with patent-specific semantics: cycle prevention on insert, topological ordering,
//! ancestry queries, LCA, path finding, and Merkle-ized lifecycle event logging.

use crate::error::CoreError;
use crate::models::*;
use petgraph::algo::{has_path_connecting, is_cyclic_directed, toposort};
use petgraph::graph::{DefaultIx, NodeIndex};
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::{Bfs, IntoEdgeReferences, IntoNodeReferences};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet, VecDeque};
use uuid::Uuid;

/// The patent DAG — a directed acyclic graph of `PatentNode` connected by `Edge`.
///
/// Uses `StableDiGraph` so that node/edge indices remain stable across removals
/// (CRDT-friendly: concurrent edits don't invalidate existing references).
#[derive(Clone, Debug)]
pub struct PatentDag {
    /// petgraph stable digraph for O(1) adjacency lookups.
    graph: StableDiGraph<PatentNode, Edge, DefaultIx>,
    /// Map from UUID → petgraph NodeIndex for fast lookup.
    node_index: HashMap<Uuid, NodeIndex<DefaultIx>>,
    /// Map from UUID → petgraph EdgeIndex for fast lookup.
    edge_index: HashMap<Uuid, petgraph::graph::EdgeIndex<DefaultIx>>,
    /// Append-only lifecycle event log (Merkle-ized).
    lifecycle_log: Vec<LifecycleEvent>,
    /// Hash of the most recent lifecycle event (Merkle chain head).
    last_event_hash: Vec<u8>,
}

impl PatentDag {
    /// Create an empty patent DAG.
    pub fn new() -> Self {
        Self {
            graph: StableDiGraph::new(),
            node_index: HashMap::new(),
            edge_index: HashMap::new(),
            lifecycle_log: Vec::new(),
            last_event_hash: Sha256::digest(b"genesis").to_vec(),
        }
    }

    // ─── Node CRUD ──────────────────────────────────────────────────────────

    /// Add a new patent node to the DAG. Returns an error if a node with the
    /// same UUID already exists.
    pub fn add_node(&mut self, node: PatentNode) -> Result<(), CoreError> {
        let id = node.id;
        if self.node_index.contains_key(&id) {
            return Err(CoreError::DuplicateNode(id.to_string()));
        }
        let idx = self.graph.add_node(node);
        self.node_index.insert(id, idx);
        self.push_lifecycle_event(id, LifecycleEventType::NodeCreated);
        Ok(())
    }

    /// Retrieve an immutable reference to a patent node by UUID.
    pub fn get_node(&self, id: &Uuid) -> Result<&PatentNode, CoreError> {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        Ok(&self.graph[*idx])
    }

    /// Retrieve a mutable reference to a patent node by UUID.
    pub fn get_node_mut(&mut self, id: &Uuid) -> Result<&mut PatentNode, CoreError> {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        Ok(&mut self.graph[*idx])
    }

    /// Update an existing node's mutable fields. Logs a `NodeModified` event.
    pub fn update_node<F>(&mut self, id: &Uuid, updater: F) -> Result<(), CoreError>
    where
        F: FnOnce(&mut PatentNode),
    {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        updater(&mut self.graph[*idx]);
        self.push_lifecycle_event(*id, LifecycleEventType::NodeModified);
        Ok(())
    }

    /// Remove a node and all its incident edges. Returns the removed node.
    /// Logs a removal event. Note: this invalidates topological orderings that
    /// were previously computed — use with care in CRDT contexts.
    pub fn remove_node(&mut self, id: &Uuid) -> Result<PatentNode, CoreError> {
        let idx = self
            .node_index
            .remove(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        // Remove all incident edges from the index.
        let edges_to_remove: Vec<Uuid> = self
            .edge_index
            .iter()
            .filter(|(_, &ei)| {
                let (a, b) = self.graph.edge_endpoints(ei).unwrap();
                a == idx || b == idx
            })
            .map(|(uid, _)| *uid)
            .collect();
        for eid in &edges_to_remove {
            self.edge_index.remove(eid);
        }
        let node = self.graph.remove_node(idx).unwrap();
        self.push_lifecycle_event(*id, LifecycleEventType::NodeRemoved);
        Ok(node)
    }

    /// Check if a node exists in the DAG.
    pub fn contains_node(&self, id: &Uuid) -> bool {
        self.node_index.contains_key(id)
    }

    /// Return the number of nodes in the DAG.
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    // ─── Edge CRUD ──────────────────────────────────────────────────────────

    /// Add a directed edge from `parent_id` to `child_id`.
    ///
    /// Validates:
    /// - Both nodes exist.
    /// - Contribution weight ∈ [0, 1].
    /// - Adding the edge does NOT create a cycle (DAG invariant).
    ///
    /// Logs an `EdgeAdded` lifecycle event on success.
    pub fn add_edge(
        &mut self,
        parent_id: &Uuid,
        child_id: &Uuid,
        edge: Edge,
    ) -> Result<(), CoreError> {
        let parent_idx = self
            .node_index
            .get(parent_id)
            .ok_or_else(|| CoreError::NodeNotFound(parent_id.to_string()))?;
        let child_idx = self
            .node_index
            .get(child_id)
            .ok_or_else(|| CoreError::NodeNotFound(child_id.to_string()))?;

        if !(0.0..=1.0).contains(&edge.contribution_weight) {
            return Err(CoreError::InvalidContributionWeight(edge.contribution_weight));
        }

        // Temporarily add the edge and check for cycles.
        let edge_idx = self.graph.add_edge(*parent_idx, *child_idx, edge);
        if is_cyclic_directed(&self.graph) {
            self.graph.remove_edge(edge_idx);
            return Err(CoreError::CycleDetected {
                parent: parent_id.to_string(),
                child: child_id.to_string(),
            });
        }

        let edge_id = self.graph[edge_idx].id;
        self.edge_index.insert(edge_id, edge_idx);
        self.push_lifecycle_event(edge_id, LifecycleEventType::EdgeAdded);
        Ok(())
    }

    /// Retrieve a reference to an edge by UUID.
    pub fn get_edge(&self, id: &Uuid) -> Result<&Edge, CoreError> {
        let idx = self
            .edge_index
            .get(id)
            .ok_or_else(|| CoreError::EdgeNotFound(id.to_string()))?;
        Ok(&self.graph[*idx])
    }

    /// Remove an edge by UUID. Logs an `EdgeRemoved` event.
    pub fn remove_edge(&mut self, id: &Uuid) -> Result<Edge, CoreError> {
        let idx = self
            .edge_index
            .remove(id)
            .ok_or_else(|| CoreError::EdgeNotFound(id.to_string()))?;
        let edge = self.graph.remove_edge(idx).unwrap();
        self.push_lifecycle_event(*id, LifecycleEventType::EdgeRemoved);
        Ok(edge)
    }

    /// Return the number of edges in the DAG.
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    // ─── Topological Ordering ────────────────────────────────────────────────

    /// Return nodes in topological order (parents before children).
    /// Returns an error if the graph contains a cycle (should never happen
    /// due to cycle prevention on insertion).
    pub fn topological_order(&self) -> Result<Vec<&PatentNode>, CoreError> {
        let order = toposort(&self.graph, None)
            .map_err(|_| CoreError::Internal("unexpected cycle in DAG".to_string()))?;
        Ok(order.into_iter().map(|idx| &self.graph[idx]).collect())
    }

    /// Return node indices in topological order.
    pub fn topological_order_indices(&self) -> Result<Vec<Uuid>, CoreError> {
        let order = toposort(&self.graph, None)
            .map_err(|_| CoreError::Internal("unexpected cycle in DAG".to_string()))?;
        Ok(order
            .into_iter()
            .map(|idx| self.graph[idx].id)
            .collect())
    }

    // ─── Traversal: Ancestors & Descendants ─────────────────────────────────

    /// Return all ancestor nodes of `id` (inclusive of `id` itself).
    /// Traverses edges in reverse direction (incoming neighbors).
    pub fn ancestors(&self, id: &Uuid) -> Result<Vec<&PatentNode>, CoreError> {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut visited = HashSet::new();
        let mut stack = vec![*idx];
        let mut result = Vec::new();
        while let Some(current) = stack.pop() {
            if visited.insert(current) {
                result.push(&self.graph[current]);
                // Traverse incoming edges (reverse direction) to find ancestors.
                for neighbor in self.graph.neighbors_directed(current, petgraph::Direction::Incoming) {
                    if !visited.contains(&neighbor) {
                        stack.push(neighbor);
                    }
                }
            }
        }
        Ok(result)
    }

    /// Return all descendant nodes reachable from `id` (inclusive).
    /// Traverses edges in forward direction (outgoing neighbors).
    pub fn descendants(&self, id: &Uuid) -> Result<Vec<&PatentNode>, CoreError> {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut bfs = Bfs::new(&self.graph, *idx);
        let mut result = Vec::new();
        while let Some(nx) = bfs.next(&self.graph) {
            result.push(&self.graph[nx]);
        }
        Ok(result)
    }

    /// Return the set of all ancestor UUIDs (inclusive).
    /// Traverses edges in reverse direction (incoming neighbors).
    pub fn ancestor_ids(&self, id: &Uuid) -> Result<HashSet<Uuid>, CoreError> {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut visited = HashSet::new();
        let mut stack = vec![*idx];
        while let Some(current) = stack.pop() {
            if visited.insert(self.graph[current].id) {
                for neighbor in self.graph.neighbors_directed(current, petgraph::Direction::Incoming) {
                    stack.push(neighbor);
                }
            }
        }
        Ok(visited)
    }

    /// Return the set of all descendant UUIDs (inclusive).
    pub fn descendant_ids(&self, id: &Uuid) -> Result<HashSet<Uuid>, CoreError> {
        let idx = self
            .node_index
            .get(id)
            .ok_or_else(|| CoreError::NodeNotFound(id.to_string()))?;
        let mut bfs = Bfs::new(&self.graph, *idx);
        let mut result = HashSet::new();
        while let Some(nx) = bfs.next(&self.graph) {
            result.insert(self.graph[nx].id);
        }
        Ok(result)
    }

    /// Check if there is a directed path from `from` to `to`.
    pub fn has_path(&self, from: &Uuid, to: &Uuid) -> Result<bool, CoreError> {
        let from_idx = self
            .node_index
            .get(from)
            .ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let to_idx = self
            .node_index
            .get(to)
            .ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;
        Ok(has_path_connecting(&self.graph, *from_idx, *to_idx, None))
    }

    // ─── LCA (Lowest Common Ancestor) ────────────────────────────────────────

    /// Find the lowest common ancestor(s) of two nodes.
    /// An LCA is a node that is an ancestor of both `a` and `b` and is farthest
    /// from the root(s). Returns all LCAs (there may be more than one in a DAG).
    pub fn lowest_common_ancestors(
        &self,
        a: &Uuid,
        b: &Uuid,
    ) -> Result<Vec<&PatentNode>, CoreError> {
        let a_ancestors = self.ancestor_ids(a)?;
        let b_ancestors = self.ancestor_ids(b)?;
        let common: HashSet<Uuid> = a_ancestors.intersection(&b_ancestors).copied().collect();

        if common.is_empty() {
            return Ok(Vec::new());
        }

        // Compute topological order and pick the last (deepest) common ancestors.
        let topo = self.topological_order_indices()?;
        let mut lcas: Vec<Uuid> = Vec::new();
        for node_id in &topo {
            if common.contains(node_id) {
                // Check if any descendant of this node is also in common.
                let desc = self.descendant_ids(node_id)?;
                let has_descendant_in_common = desc.iter().any(|d| d != node_id && common.contains(d));
                if !has_descendant_in_common {
                    lcas.push(*node_id);
                }
            }
        }

        Ok(lcas
            .into_iter()
            .map(|id| &self.graph[self.node_index[&id]])
            .collect())
    }

    // ─── Path Finding ────────────────────────────────────────────────────────

    /// Find all simple paths from `from` to `to` (breadth-first, bounded).
    /// `max_paths` limits the number of paths returned to avoid explosion.
    pub fn find_paths(
        &self,
        from: &Uuid,
        to: &Uuid,
        max_paths: usize,
    ) -> Result<Vec<Vec<Uuid>>, CoreError> {
        let from_idx = self
            .node_index
            .get(from)
            .ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let to_idx = self
            .node_index
            .get(to)
            .ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;

        let mut results = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back(vec![*from_idx]);

        while let Some(path) = queue.pop_front() {
            if results.len() >= max_paths {
                break;
            }
            let last = *path.last().unwrap();
            if last == *to_idx {
                results.push(path.into_iter().map(|n| self.graph[n].id).collect());
                continue;
            }
            for neighbor in self.graph.neighbors(last) {
                if !path.contains(&neighbor) {
                    let mut new_path = path.clone();
                    new_path.push(neighbor);
                    queue.push_back(new_path);
                }
            }
        }

        Ok(results)
    }

    // ─── Distance Metrics ────────────────────────────────────────────────────

    /// Compute the shortest path distance (number of edges) between two nodes.
    /// Returns `None` if no path exists.
    pub fn distance(&self, from: &Uuid, to: &Uuid) -> Result<Option<usize>, CoreError> {
        let from_idx = self
            .node_index
            .get(from)
            .ok_or_else(|| CoreError::NodeNotFound(from.to_string()))?;
        let to_idx = self
            .node_index
            .get(to)
            .ok_or_else(|| CoreError::NodeNotFound(to.to_string()))?;

        if !has_path_connecting(&self.graph, *from_idx, *to_idx, None) {
            return Ok(None);
        }

        // BFS for shortest path.
        let mut queue = VecDeque::new();
        let mut visited = HashSet::new();
        queue.push_back((*from_idx, 0usize));
        visited.insert(*from_idx);

        while let Some((current, dist)) = queue.pop_front() {
            if current == *to_idx {
                return Ok(Some(dist));
            }
            for neighbor in self.graph.neighbors(current) {
                if visited.insert(neighbor) {
                    queue.push_back((neighbor, dist + 1));
                }
            }
        }

        Ok(None)
    }

    // ─── Graph Export ────────────────────────────────────────────────────────

    /// Export the full graph state (nodes + edges) for serialization.
    pub fn export_state(&self) -> PatentGraphState {
        let nodes: HashMap<Uuid, PatentNode> = self
            .graph
            .node_references()
            .map(|(_, n)| (n.id, n.clone()))
            .collect();
        let edges: Vec<Edge> = self
            .graph
            .edge_references()
            .map(|e| e.weight().clone())
            .collect();
        PatentGraphState { nodes, edges }
    }

    /// Return a reference to the lifecycle event log.
    pub fn lifecycle_log(&self) -> &[LifecycleEvent] {
        &self.lifecycle_log
    }

    /// Return the most recent lifecycle event hash (Merkle chain head).
    pub fn last_event_hash(&self) -> &[u8] {
        &self.last_event_hash
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    /// Push a lifecycle event and update the Merkle chain.
    fn push_lifecycle_event(&mut self, node_id: Uuid, event_type: LifecycleEventType) {
        let event_id = Uuid::new_v4();
        let timestamp = chrono::Utc::now().timestamp();
        let mut hasher = Sha256::new();
        hasher.update(&self.last_event_hash);
        hasher.update(event_id.as_bytes());
        hasher.update(node_id.as_bytes());
        hasher.update(timestamp.to_le_bytes());
        let merkle_root = hasher.finalize().to_vec();

        let event = LifecycleEvent {
            id: event_id,
            event_type,
            node_id,
            timestamp_unix: timestamp,
            signature: Vec::new(), // to be filled by the signing layer
            parent_events: vec![Uuid::from_slice(&self.last_event_hash[..16]).unwrap_or_default()],
            merkle_root: merkle_root.clone(),
        };
        self.last_event_hash = merkle_root;
        self.lifecycle_log.push(event);
    }
}

impl Default for PatentDag {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node(id: Uuid) -> PatentNode {
        PatentNode {
            id,
            title: "Test Patent".into(),
            patent_type: PatentType::Software,
            authors: vec!["Alice".into()],
            created_at_unix: 0,
            content_hash: vec![],
            invariant: None,
            tags: vec![],
            license: License::Open,
            meta_commitment: vec![],
        }
    }

    fn make_edge(parent: Uuid, child: Uuid, weight: f64) -> Edge {
        Edge {
            id: Uuid::new_v4(),
            parent_id: parent,
            child_id: child,
            kind: EdgeKind::Derivative,
            delta: None,
            contribution_weight: weight,
            signature: vec![],
        }
    }

    #[test]
    fn test_add_and_get_node() {
        let mut dag = PatentDag::new();
        let id = Uuid::new_v4();
        dag.add_node(make_node(id)).unwrap();
        assert!(dag.contains_node(&id));
        assert_eq!(dag.node_count(), 1);
    }

    #[test]
    fn test_duplicate_node_error() {
        let mut dag = PatentDag::new();
        let id = Uuid::new_v4();
        dag.add_node(make_node(id)).unwrap();
        let err = dag.add_node(make_node(id)).unwrap_err();
        assert!(matches!(err, CoreError::DuplicateNode(_)));
    }

    #[test]
    fn test_cycle_prevention() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_node(make_node(c)).unwrap();
        // a → b, b → c, c → a should be rejected.
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        let err = dag.add_edge(&c, &a, make_edge(c, a, 0.5)).unwrap_err();
        assert!(matches!(err, CoreError::CycleDetected { .. }));
    }

    #[test]
    fn test_topological_order() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        let order = dag.topological_order_indices().unwrap();
        // a must come before b before c.
        let pos_a = order.iter().position(|&x| x == a).unwrap();
        let pos_b = order.iter().position(|&x| x == b).unwrap();
        let pos_c = order.iter().position(|&x| x == c).unwrap();
        assert!(pos_a < pos_b);
        assert!(pos_b < pos_c);
    }

    #[test]
    fn test_ancestors_and_descendants() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        let anc = dag.ancestor_ids(&c).unwrap();
        assert!(anc.contains(&a));
        assert!(anc.contains(&b));
        assert!(anc.contains(&c));
        let desc = dag.descendant_ids(&a).unwrap();
        assert!(desc.contains(&a));
        assert!(desc.contains(&b));
        assert!(desc.contains(&c));
    }

    #[test]
    fn test_lca() {
        let mut dag = PatentDag::new();
        let root = Uuid::new_v4();
        let left = Uuid::new_v4();
        let right = Uuid::new_v4();
        dag.add_node(make_node(root)).unwrap();
        dag.add_node(make_node(left)).unwrap();
        dag.add_node(make_node(right)).unwrap();
        dag.add_edge(&root, &left, make_edge(root, left, 0.5)).unwrap();
        dag.add_edge(&root, &right, make_edge(root, right, 0.5)).unwrap();
        let lcas = dag.lowest_common_ancestors(&left, &right).unwrap();
        assert_eq!(lcas.len(), 1);
        assert_eq!(lcas[0].id, root);
    }

    #[test]
    fn test_distance() {
        let mut dag = PatentDag::new();
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let c = Uuid::new_v4();
        dag.add_node(make_node(a)).unwrap();
        dag.add_node(make_node(b)).unwrap();
        dag.add_node(make_node(c)).unwrap();
        dag.add_edge(&a, &b, make_edge(a, b, 0.5)).unwrap();
        dag.add_edge(&b, &c, make_edge(b, c, 0.5)).unwrap();
        assert_eq!(dag.distance(&a, &c).unwrap(), Some(2));
        assert!(dag.distance(&c, &a).unwrap().is_none());
    }

    #[test]
    fn test_lifecycle_log() {
        let mut dag = PatentDag::new();
        let id = Uuid::new_v4();
        dag.add_node(make_node(id)).unwrap();
        assert_eq!(dag.lifecycle_log().len(), 1);
        assert_eq!(dag.lifecycle_log()[0].event_type, LifecycleEventType::NodeCreated);
        assert!(!dag.last_event_hash().is_empty());
    }
#[test]
    fn test_remove_node_logs_removal() {
        let mut dag = PatentDag::new();
        let id = Uuid::new_v4();
        dag.add_node(make_node(id)).unwrap();
        assert_eq!(dag.lifecycle_log().len(), 1);

        // Removing the node must log a NodeRemoved event (not NodeCreated).
        dag.remove_node(&id).unwrap();
        assert_eq!(dag.lifecycle_log().len(), 2);
        assert_eq!(dag.lifecycle_log()[1].event_type, LifecycleEventType::NodeRemoved);
        assert!(!dag.contains_node(&id));
    }
}