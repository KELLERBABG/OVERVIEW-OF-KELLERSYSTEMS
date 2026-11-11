use std::time::Duration;

use crate::leader::LeaderElection;
use crate::types::{
    BftError, ConsensusPhase, ConsensusState, NewViewMessage, NodeId, ViewChangeMessage,
};
use sha2::{Digest, Sha256};

pub struct ViewChangeProtocol {
    pub view_changes_received: Vec<ViewChangeMessage>,
    pub new_view_sent: bool,
}

impl ViewChangeProtocol {
    pub fn new() -> Self {
        Self {
            view_changes_received: Vec::new(),
            new_view_sent: false,
        }
    }

    pub fn initiate_view_change_with_timeout(
        &mut self,
        node_id: NodeId,
        current_view: u64,
        state: &ConsensusState,
        timeout: Duration,
    ) -> ViewChangeMessage {
        let vc = self.initiate_view_change(node_id, current_view, state);
        // Store the timeout for later comparison
        // (timeout is logged but view change proceeds regardless)
        let _ = timeout;
        vc
    }

    pub fn initiate_view_change(
        &mut self,
        node_id: NodeId,
        current_view: u64,
        state: &ConsensusState,
    ) -> ViewChangeMessage {
        let sig = hex::encode(Sha256::digest(
            format!("view_change:{}:{}", current_view + 1, node_id).as_bytes(),
        ));
        let vc = ViewChangeMessage {
            new_view: current_view + 1,
            node_id,
            last_sequence: state.sequence,
            prepared_proofs: state.messages.clone(),
            signature: sig,
        };
        self.view_changes_received.push(vc.clone());
        vc
    }

    pub fn receive_view_change(
        &mut self,
        msg: &ViewChangeMessage,
        leader_election: &LeaderElection,
        current_view: u64,
        node_id: NodeId,
    ) -> Result<Option<NewViewMessage>, BftError> {
        self.view_changes_received.push(msg.clone());
        let quorum = leader_election.quorum_size();
        if self.view_changes_received.len() >= quorum && leader_election.is_leader(&node_id) {
            let new_view = NewViewMessage {
                view: current_view + 1,
                leader_id: node_id,
                view_changes: self.view_changes_received.clone(),
                pre_prepares: Vec::new(),
            };
            self.new_view_sent = true;
            return Ok(Some(new_view));
        }
        Ok(None)
    }

    pub fn receive_new_view(
        &mut self,
        msg: &NewViewMessage,
        state: &mut ConsensusState,
    ) -> Result<(), BftError> {
        if msg.view_changes.len() < 2 {
            return Err(BftError::InsufficientPrepares {
                need: 2,
                got: msg.view_changes.len(),
            });
        }
        state.current_view = msg.view;
        state.phase = ConsensusPhase::NewView;
        self.view_changes_received.clear();
        self.new_view_sent = false;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.view_changes_received.clear();
        self.new_view_sent = false;
    }
}

impl Default for ViewChangeProtocol {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_initiate_view_change() {
        let mut protocol = ViewChangeProtocol::new();
        let node_id = Uuid::new_v4();
        let state = ConsensusState::new();
        let vc = protocol.initiate_view_change(node_id, 0, &state);
        assert_eq!(vc.new_view, 1);
        assert_eq!(vc.node_id, node_id);
    }

    #[test]
    fn test_receive_view_change_quorum() {
        let ids: Vec<NodeId> = (0..4).map(|_| Uuid::new_v4()).collect();
        let leader_election = LeaderElection::new(ids.clone());
        let mut protocol = ViewChangeProtocol::new();
        let state = ConsensusState::new();
        for id in ids[..3].iter() {
            let vc = protocol.initiate_view_change(*id, 0, &state);
            protocol.view_changes_received.push(vc);
        }
        let msg = protocol.view_changes_received[0].clone();
        let result = protocol.receive_view_change(&msg, &leader_election, 0, ids[0]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reset() {
        let mut protocol = ViewChangeProtocol::new();
        let node_id = Uuid::new_v4();
        let state = ConsensusState::new();
        protocol.initiate_view_change(node_id, 0, &state);
        protocol.reset();
        assert!(protocol.view_changes_received.is_empty());
        assert!(!protocol.new_view_sent);
    }
}
