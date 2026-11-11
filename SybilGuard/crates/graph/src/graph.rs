use petgraph::graph::{NodeIndex, UnGraph};
use petgraph::visit::{EdgeRef, IntoEdgeReferences};
use sybil_core::{Edge, EdgeMetadata, EdgeType, NodeId, NodeMetadata, SocialGraph, SybilError, SybilResult};

/// Internal petgraph-based social graph.
#[derive(Debug, Clone)]
pub struct PoPGraph {
    graph: UnGraph<NodeMetadata, f64>,
    node_index_map: std::collections::HashMap<NodeId, NodeIndex>,
}

impl PoPGraph {
    pub fn new() -> Self {
        Self {
            graph: UnGraph::new_undirected(),
            node_index_map: std::collections::HashMap::new(),
        }
    }

    pub fn from_social_graph(sg: &SocialGraph) -> SybilResult<Self> {
        let mut pg = Self::new();
        for node in &sg.nodes {
            pg.add_node(node.clone())?;
        }
        for edge in &sg.edges {
            pg.add_edge(edge)?;
        }
        Ok(pg)
    }

    pub fn to_social_graph(&self) -> SocialGraph {
        let now = chrono::Utc::now();
        let nodes: Vec<NodeMetadata> = self
            .graph
            .node_indices()
            .map(|i| self.graph[i].clone())
            .collect();
        let edges: Vec<Edge> = self
            .graph
            .edge_references()
            .map(|e| {
                let source = self.graph[e.source()].node_id.clone();
                let target = self.graph[e.target()].node_id.clone();
                Edge {
                    source,
                    target,
                    weight: *e.weight(),
                    metadata: EdgeMetadata {
                        created_at: now,
                        edge_type: EdgeType::Social,
                        is_active: true,
                    },
                }
            })
            .collect();
        SocialGraph {
            nodes,
            edges,
            directed: false,
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_node(&mut self, metadata: NodeMetadata) -> SybilResult<NodeIndex> {
        let node_id = metadata.node_id.clone();
        if self.node_index_map.contains_key(&node_id) {
            return Err(SybilError::Graph(format!("node {} already exists", node_id)));
        }
        let idx = self.graph.add_node(metadata);
        self.node_index_map.insert(node_id, idx);
        Ok(idx)
    }

    pub fn remove_node(&mut self, node_id: &NodeId) -> SybilResult<()> {
        let idx = self.node_index_map.remove(node_id)
            .ok_or_else(|| SybilError::NodeNotFound(node_id.to_string()))?;
        self.graph.remove_node(idx);
        Ok(())
    }

    pub fn node_index(&self, node_id: &NodeId) -> SybilResult<NodeIndex> {
        self.node_index_map.get(node_id).copied()
            .ok_or_else(|| SybilError::NodeNotFound(node_id.to_string()))
    }

    pub fn add_edge(&mut self, edge: &Edge) -> SybilResult<()> {
        let source_idx = self.node_index(&edge.source)?;
        let target_idx = self.node_index(&edge.target)?;
        self.graph.add_edge(source_idx, target_idx, edge.weight);
        Ok(())
    }

    pub fn remove_edge(&mut self, source: &NodeId, target: &NodeId) -> SybilResult<()> {
        let source_idx = self.node_index(source)?;
        let target_idx = self.node_index(target)?;
        let edge_idx = self.graph.find_edge(source_idx, target_idx)
            .ok_or_else(|| SybilError::Graph(format!("no edge between {} and {}", source, target)))?;
        self.graph.remove_edge(edge_idx);
        Ok(())
    }

    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Get the degree of a node (number of neighbors).
    pub fn degree(&self, node_id: &NodeId) -> SybilResult<usize> {
        let idx = self.node_index(node_id)?;
        Ok(self.graph.neighbors(idx).count())
    }

    pub fn all_node_ids(&self) -> Vec<NodeId> {
        self.graph.node_indices()
            .map(|i| self.graph[i].node_id.clone())
            .collect()
    }

    pub fn neighbors(&self, node_id: &NodeId) -> SybilResult<Vec<NodeId>> {
        let idx = self.node_index(node_id)?;
        let neighbors: Vec<NodeId> = self.graph.neighbors(idx)
            .map(|n| self.graph[n].node_id.clone())
            .collect();
        Ok(neighbors)
    }

    pub fn inner(&self) -> &UnGraph<NodeMetadata, f64> {
        &self.graph
    }

    pub fn inner_mut(&mut self) -> &mut UnGraph<NodeMetadata, f64> {
        &mut self.graph
    }
}

impl Default for PoPGraph {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_node_meta(id: NodeId) -> NodeMetadata {
        NodeMetadata {
            node_id: id,
            did: None,
            reputation_score: 0.0,
            clustering_coefficient: None,
            centrality_score: None,
            embedding_depth: 0,
            joined_at: chrono::Utc::now(),
            last_active: chrono::Utc::now(),
            is_sybil: None,
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn test_add_and_remove_node() {
        let mut pg = PoPGraph::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        pg.add_node(make_node_meta(n1.clone())).unwrap();
        pg.add_node(make_node_meta(n2.clone())).unwrap();
        assert_eq!(pg.node_count(), 2);
        pg.remove_node(&n1).unwrap();
        assert_eq!(pg.node_count(), 1);
    }

    #[test]
    fn test_add_edge() {
        let mut pg = PoPGraph::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        pg.add_node(make_node_meta(n1.clone())).unwrap();
        pg.add_node(make_node_meta(n2.clone())).unwrap();
        let edge = Edge {
            source: n1.clone(),
            target: n2.clone(),
            weight: 1.0,
            metadata: EdgeMetadata {
                created_at: chrono::Utc::now(),
                edge_type: EdgeType::Social,
                is_active: true,
            },
        };
        pg.add_edge(&edge).unwrap();
        assert_eq!(pg.edge_count(), 1);
        assert_eq!(pg.degree(&n1).unwrap(), 1);
    }

    #[test]
    fn test_node_not_found() {
        let pg = PoPGraph::new();
        let n1 = NodeId::new();
        assert!(matches!(pg.degree(&n1), Err(SybilError::NodeNotFound(_))));
    }

    #[test]
    fn test_duplicate_node_error() {
        let mut pg = PoPGraph::new();
        let n1 = NodeId::new();
        pg.add_node(make_node_meta(n1.clone())).unwrap();
        assert!(pg.add_node(make_node_meta(n1)).is_err());
    }

    #[test]
    fn test_social_graph_roundtrip() {
        let mut pg = PoPGraph::new();
        let n1 = NodeId::new();
        let n2 = NodeId::new();
        pg.add_node(make_node_meta(n1.clone())).unwrap();
        pg.add_node(make_node_meta(n2.clone())).unwrap();
        let edge = Edge {
            source: n1.clone(),
            target: n2.clone(),
            weight: 1.0,
            metadata: EdgeMetadata {
                created_at: chrono::Utc::now(),
                edge_type: EdgeType::Social,
                is_active: true,
            },
        };
        pg.add_edge(&edge).unwrap();
        let sg = pg.to_social_graph();
        let pg2 = PoPGraph::from_social_graph(&sg).unwrap();
        assert_eq!(pg2.node_count(), 2);
        assert_eq!(pg2.edge_count(), 1);
    }
}