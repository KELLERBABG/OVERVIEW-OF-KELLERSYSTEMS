use crate::types::NodeId;

pub struct LeaderElection {
    pub node_ids: Vec<NodeId>,
    pub current_leader_idx: usize,
}

impl LeaderElection {
    pub fn new(node_ids: Vec<NodeId>) -> Self {
        Self {
            node_ids,
            current_leader_idx: 0,
        }
    }

    pub fn current_leader(&self) -> Option<&NodeId> {
        self.node_ids.get(self.current_leader_idx)
    }

    pub fn is_leader(&self, node_id: &NodeId) -> bool {
        self.current_leader() == Some(node_id)
    }

    pub fn advance_view(&mut self) -> &NodeId {
        self.current_leader_idx = (self.current_leader_idx + 1) % self.node_ids.len();
        self.current_leader().unwrap()
    }

    pub fn leader_for_view(&self, view: u64) -> Option<&NodeId> {
        let idx = view as usize % self.node_ids.len();
        self.node_ids.get(idx)
    }

    pub fn node_count(&self) -> usize {
        self.node_ids.len()
    }

    pub fn fault_tolerance(&self) -> usize {
        (self.node_ids.len() - 1) / 3
    }

    pub fn quorum_size(&self) -> usize {
        self.node_ids.len() - self.fault_tolerance()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_leader_election_round_robin() {
        let ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
        let mut election = LeaderElection::new(ids.clone());
        assert_eq!(election.current_leader(), Some(&ids[0]));
        election.advance_view();
        assert_eq!(election.current_leader(), Some(&ids[1]));
        election.advance_view();
        assert_eq!(election.current_leader(), Some(&ids[2]));
        election.advance_view();
        assert_eq!(election.current_leader(), Some(&ids[3]));
        election.advance_view();
        assert_eq!(election.current_leader(), Some(&ids[0]));
    }

    #[test]
    fn test_leader_for_view() {
        let ids: Vec<NodeId> = (0..3).map(|_| Uuid::new_v4()).collect();
        let election = LeaderElection::new(ids.clone());
        assert_eq!(election.leader_for_view(0), Some(&ids[0]));
        assert_eq!(election.leader_for_view(1), Some(&ids[1]));
        assert_eq!(election.leader_for_view(2), Some(&ids[2]));
        assert_eq!(election.leader_for_view(3), Some(&ids[0]));
    }

    #[test]
    fn test_quorum_size() {
        let ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
        let election = LeaderElection::new(ids);
        assert_eq!(election.fault_tolerance(), 1);
        assert_eq!(election.quorum_size(), 3);
    }
}
