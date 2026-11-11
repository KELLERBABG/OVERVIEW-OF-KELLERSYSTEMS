use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use sybil_core::{constants::*, NodeId, SybilError, SybilResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnchorStatus {
    Founding,
    Active { promoted_at: DateTime<Utc> },
    Retired { retired_at: DateTime<Utc> },
}

#[derive(Debug, Clone)]
pub struct Anchor {
    pub node_id: NodeId,
    pub did: String,
    pub status: AnchorStatus,
    pub weight: u32,
    pub added_at: DateTime<Utc>,
}

/// Manages bootstrap anchor set and threshold vouching (cold start mitigation).
pub struct BootstrapManager {
    anchors: HashMap<NodeId, Anchor>,
    min_vouches: u32,
    min_graph_size_for_rotation: usize,
    vouch_records: HashMap<NodeId, HashSet<NodeId>>,
    graph_size: usize,
}

impl BootstrapManager {
    pub fn new(min_vouches: u32, min_graph_size_for_rotation: usize) -> Self {
        Self {
            anchors: HashMap::new(),
            min_vouches,
            min_graph_size_for_rotation,
            vouch_records: HashMap::new(),
            graph_size: 0,
        }
    }

    pub fn initialize(&mut self, anchor_nodes: Vec<(NodeId, String)>) {
        let now = Utc::now();
        for (node_id, did) in anchor_nodes {
            self.anchors.insert(node_id.clone(), Anchor {
                node_id,
                did,
                status: AnchorStatus::Founding,
                weight: 5,
                added_at: now,
            });
        }
    }

    pub fn anchor_vouch(&mut self, anchor_id: &NodeId, subject_id: &NodeId) -> SybilResult<()> {
        if !self.anchors.contains_key(anchor_id) {
            return Err(SybilError::Vouching(format!("{} is not a bootstrap anchor", anchor_id)));
        }
        self.vouch_records.entry(subject_id.clone()).or_default().insert(anchor_id.clone());
        Ok(())
    }

    pub fn can_join(&self, subject_id: &NodeId) -> bool {
        self.vouch_records.get(subject_id).map(|v| v.len() >= self.min_vouches as usize).unwrap_or(false)
    }

    pub fn anchor_vouch_count(&self, subject_id: &NodeId) -> usize {
        self.vouch_records.get(subject_id).map(|v| v.len()).unwrap_or(0)
    }

    pub fn can_rotate_anchors(&self) -> bool {
        self.graph_size >= self.min_graph_size_for_rotation
    }

    pub fn promote_to_anchor(&mut self, node_id: NodeId, did: String) -> SybilResult<()> {
        if !self.can_rotate_anchors() {
            return Err(SybilError::Vouching("Graph too small for anchor rotation".to_string()));
        }
        self.anchors.insert(node_id.clone(), Anchor {
            node_id,
            did,
            status: AnchorStatus::Active { promoted_at: Utc::now() },
            weight: 3,
            added_at: Utc::now(),
        });
        Ok(())
    }

    pub fn retire_anchor(&mut self, node_id: &NodeId) -> SybilResult<()> {
        if !self.can_rotate_anchors() {
            return Err(SybilError::Vouching("Graph too small for anchor rotation".to_string()));
        }
        self.anchors.remove(node_id);
        Ok(())
    }

    pub fn active_anchors(&self) -> Vec<&Anchor> {
        self.anchors.values().filter(|a| matches!(a.status, AnchorStatus::Founding | AnchorStatus::Active { .. })).collect()
    }

    pub fn anchor_count(&self) -> usize {
        self.active_anchors().len()
    }

    pub fn set_graph_size(&mut self, size: usize) { self.graph_size = size; }
    pub fn min_rotation_size(&self) -> usize { self.min_graph_size_for_rotation }
    pub fn clear_vouch_records(&mut self, subject_id: &NodeId) { self.vouch_records.remove(subject_id); }
}

impl Default for BootstrapManager {
    fn default() -> Self { Self::new(MIN_ANCHOR_VOUCHES, 100) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_anchors() {
        let mut m = BootstrapManager::default();
        m.initialize(vec![
            (NodeId::new(), "did:a".into()),
            (NodeId::new(), "did:b".into()),
            (NodeId::new(), "did:c".into()),
        ]);
        assert_eq!(m.anchor_count(), 3);
    }

    #[test]
    fn test_threshold_vouching() {
        let mut m = BootstrapManager::new(3, 100);
        let a1 = NodeId::new(); let a2 = NodeId::new(); let a3 = NodeId::new();
        let alice = NodeId::new();
        m.initialize(vec![(a1.clone(), "a".into()), (a2.clone(), "b".into()), (a3.clone(), "c".into())]);
        m.anchor_vouch(&a1, &alice).unwrap();
        m.anchor_vouch(&a2, &alice).unwrap();
        assert!(!m.can_join(&alice));
        m.anchor_vouch(&a3, &alice).unwrap();
        assert!(m.can_join(&alice));
    }

    #[test]
    fn test_rotation_blocked() {
        let mut m = BootstrapManager::new(3, 100);
        m.set_graph_size(50);
        assert!(m.promote_to_anchor(NodeId::new(), "d:".into()).is_err());
    }

    #[test]
    fn test_rotation_allowed() {
        let mut m = BootstrapManager::new(3, 100);
        m.set_graph_size(200);
        assert!(m.promote_to_anchor(NodeId::new(), "d:".into()).is_ok());
    }
}