use std::time::Duration;

use crate::leader::LeaderElection;
use crate::types::{
    BftError, ConsensusMessage, ConsensusPhase, ConsensusState, MessageType, NodeId,
};
use rand::Rng;

#[derive(Debug, Clone, Copy)]
pub struct ConsensusConfig {
    /// Base timeout for view change trigger (ms)
    pub view_change_timeout_ms: u64,
    /// Added jitter range (ms) — actual timeout = base + random(0, jitter_range)
    pub jitter_range_ms: u64,
    /// Exponential backoff multiplier per consecutive view change
    pub backoff_multiplier: f64,
    /// Maximum view change timeout cap (ms)
    pub max_timeout_ms: u64,
    /// Minimum peers required to avoid triggering view change during partition
    pub min_peers_for_quorum: usize,
    /// Number of missed heartbeats before suspecting leader
    pub heartbeat_miss_threshold: u32,
}

impl Default for ConsensusConfig {
    fn default() -> Self {
        Self {
            view_change_timeout_ms: 3000,
            jitter_range_ms: 1000,
            backoff_multiplier: 1.5,
            max_timeout_ms: 60000,
            min_peers_for_quorum: 3,
            heartbeat_miss_threshold: 3,
        }
    }
}

pub struct Consensus {
    pub state: ConsensusState,
    pub node_id: NodeId,
    pub leader_election: LeaderElection,
    pub sk_hex: String,
    pub prepare_count: usize,
    pub commit_count: usize,
    pub config: ConsensusConfig,
    pub consecutive_view_changes: u32,
}

impl Consensus {
    pub fn new(node_id: NodeId, node_ids: Vec<NodeId>, sk_hex: String) -> Self {
        Self::with_config(node_id, node_ids, sk_hex, ConsensusConfig::default())
    }

    pub fn with_config(
        node_id: NodeId,
        node_ids: Vec<NodeId>,
        sk_hex: String,
        config: ConsensusConfig,
    ) -> Self {
        let leader_election = LeaderElection::new(node_ids);
        Self {
            state: ConsensusState::new(),
            node_id,
            leader_election,
            sk_hex,
            config,
            prepare_count: 0,
            commit_count: 0,
            consecutive_view_changes: 0,
        }
    }

    pub fn view_change_timeout(&self) -> Duration {
        let base = self.config.view_change_timeout_ms;
        let jitter: u64 = rand::thread_rng().gen_range(0..=self.config.jitter_range_ms);
        let backoff = if self.consecutive_view_changes > 0 {
            (base as f64
                * self
                    .config
                    .backoff_multiplier
                    .powi(self.consecutive_view_changes as i32 - 1)) as u64
        } else {
            0
        };
        let timeout = (base + jitter + backoff).min(self.config.max_timeout_ms);
        Duration::from_millis(timeout)
    }

    pub fn should_trigger_view_change(&self, active_peer_count: usize) -> bool {
        if active_peer_count < self.config.min_peers_for_quorum {
            return false;
        }
        if self.state.messages.is_empty() && self.prepare_count == 0 {
            return false;
        }
        true
    }

    pub fn record_heartbeat(&mut self, from_leader: bool) {
        if from_leader {
            self.state.leader_heartbeat_misses = 0;
        }
    }

    pub fn miss_heartbeat(&mut self) -> bool {
        self.state.leader_heartbeat_misses += 1;
        self.state.leader_heartbeat_misses >= self.config.heartbeat_miss_threshold
    }

    pub fn propose(&mut self, payload: &[u8]) -> Result<ConsensusMessage, BftError> {
        if !self.leader_election.is_leader(&self.node_id) {
            return Err(BftError::NotLeader);
        }
        if self.state.phase != ConsensusPhase::NewView {
            return Err(BftError::AlreadyPrepared);
        }
        self.state.sequence += 1;
        let msg = ConsensusMessage::new(
            MessageType::PrePrepare,
            self.state.current_view,
            self.state.sequence,
            self.node_id,
            payload,
            &self.sk_hex,
        );
        self.state.messages.push(msg.clone());
        self.state.phase = ConsensusPhase::PrePrepared;
        self.state.prepared_digest = Some(msg.digest.clone());
        Ok(msg)
    }

    pub fn receive_pre_prepare(
        &mut self,
        msg: &ConsensusMessage,
    ) -> Result<ConsensusMessage, BftError> {
        if !self.leader_election.is_leader(&msg.node_id) {
            return Err(BftError::NotLeader);
        }
        if !msg.verify(&self.sk_hex) {
            return Err(BftError::DigestMismatch {
                expected: "valid".to_string(),
                got: "invalid".to_string(),
            });
        }
        self.state.sequence = msg.sequence;
        self.state.current_view = msg.view;
        self.state.prepared_digest = Some(msg.digest.clone());
        self.state.phase = ConsensusPhase::PrePrepared;
        let prepare = ConsensusMessage::new(
            MessageType::Prepare,
            msg.view,
            msg.sequence,
            self.node_id,
            msg.digest.as_bytes(),
            &self.sk_hex,
        );
        self.state.messages.push(prepare.clone());
        self.prepare_count += 1;
        Ok(prepare)
    }

    pub fn receive_prepare(
        &mut self,
        msg: &ConsensusMessage,
    ) -> Result<Option<ConsensusMessage>, BftError> {
        if !msg.verify(&self.sk_hex) {
            return Err(BftError::DigestMismatch {
                expected: "valid".to_string(),
                got: "invalid".to_string(),
            });
        }
        self.state.messages.push(msg.clone());
        self.prepare_count += 1;
        let quorum = self.leader_election.quorum_size();
        if self.prepare_count >= quorum
            && (self.state.phase as u8) < (ConsensusPhase::Prepared as u8)
        {
            self.state.phase = ConsensusPhase::Prepared;
            let commit = ConsensusMessage::new(
                MessageType::Commit,
                self.state.current_view,
                self.state.sequence,
                self.node_id,
                msg.digest.as_bytes(),
                &self.sk_hex,
            );
            self.state.messages.push(commit.clone());
            self.commit_count += 1;
            return Ok(Some(commit));
        }
        Ok(None)
    }

    pub fn receive_commit(&mut self, msg: &ConsensusMessage) -> Result<bool, BftError> {
        if !msg.verify(&self.sk_hex) {
            return Err(BftError::DigestMismatch {
                expected: "valid".to_string(),
                got: "invalid".to_string(),
            });
        }
        self.state.messages.push(msg.clone());
        self.commit_count += 1;
        let quorum = self.leader_election.quorum_size();
        if self.commit_count >= quorum
            && (self.state.phase as u8) < (ConsensusPhase::Committed as u8)
        {
            self.state.phase = ConsensusPhase::Committed;
            self.state.committed_digest = self.state.prepared_digest.clone();
            return Ok(true);
        }
        Ok(false)
    }

    pub fn is_committed(&self) -> bool {
        self.state.phase == ConsensusPhase::Committed
    }

    pub fn committed_digest(&self) -> Option<&str> {
        self.state.committed_digest.as_deref()
    }

    pub fn reset(&mut self) {
        self.state.phase = ConsensusPhase::NewView;
        self.prepare_count = 0;
        self.commit_count = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn setup_consensus(count: usize) -> Vec<Consensus> {
        let ids: Vec<NodeId> = (0..count).map(|_| Uuid::new_v4()).collect();
        ids.iter()
            .map(|id| Consensus::new(*id, ids.clone(), "test_sk".to_string()))
            .collect()
    }

    #[test]
    fn test_leader_proposes() {
        let mut nodes = setup_consensus(4);
        let result = nodes[0].propose(b"trade_42");
        assert!(result.is_ok());
        let msg = result.unwrap();
        assert_eq!(msg.msg_type, MessageType::PrePrepare);
    }

    #[test]
    fn test_non_leader_cannot_propose() {
        let mut nodes = setup_consensus(4);
        let result = nodes[1].propose(b"trade_42");
        assert!(matches!(result, Err(BftError::NotLeader)));
    }

    #[test]
    fn test_full_consensus_flow() {
        let mut nodes = setup_consensus(4);
        let proposal = nodes[0].propose(b"trade_42").unwrap();
        let prepares: Vec<_> = (1..4)
            .map(|i| nodes[i].receive_pre_prepare(&proposal).unwrap())
            .collect();
        let mut commits = Vec::new();
        for prepare in &prepares {
            for node in nodes.iter_mut() {
                if let Some(commit) = node.receive_prepare(prepare).unwrap() {
                    commits.push(commit);
                }
            }
        }
        for commit in &commits {
            for node in nodes.iter_mut() {
                let _ = node.receive_commit(commit);
            }
        }
        for (i, node) in nodes.iter().enumerate() {
            assert!(node.is_committed(), "node {} should be committed", i);
        }
    }

    #[test]
    fn test_view_change_timeout_computes_with_jitter() {
        let nodes = setup_consensus(4);
        let timeout = nodes[0].view_change_timeout();
        let ms = timeout.as_millis() as u64;
        // consecutive=0: base=3000 + jitter(0..1000) = 3000..4000
        assert!(ms >= 3000, "timeout {} should be >= 3000", ms);
        assert!(ms <= 4000, "timeout {} should be <= 4000", ms);
    }

    #[test]
    fn test_should_not_trigger_view_change_during_partition() {
        let nodes = setup_consensus(4);
        // With only 1 active peer (below min_peers_for_quorum = 3)
        assert!(!nodes[0].should_trigger_view_change(1));
        // With 0 active peers
        assert!(!nodes[0].should_trigger_view_change(0));
    }

    #[test]
    fn test_should_trigger_view_change_with_enough_peers() {
        let mut nodes = setup_consensus(4);
        // Simulate an active node that has sent prepares
        nodes[0].prepare_count = 3;
        assert!(nodes[0].should_trigger_view_change(3));
    }

    #[test]
    fn test_view_change_backoff_increases_timeout() {
        let mut nodes = setup_consensus(4);
        // baseline
        nodes[0].consecutive_view_changes = 0;
        let t0 = nodes[0].view_change_timeout();
        // after 1 view change
        nodes[0].consecutive_view_changes = 1;
        let t1 = nodes[0].view_change_timeout();
        // t1 should be >= t0 (backoff multiplier 1.5 adds at least 4500ms base portion)
        assert!(
            t1.as_millis() >= t0.as_millis(),
            "t1={:?} should be >= t0={:?}",
            t1,
            t0
        );
    }

    #[test]
    fn test_heartbeat_miss_tracking() {
        let mut node = setup_consensus(4).into_iter().next().unwrap();
        assert!(!node.miss_heartbeat());
        assert!(!node.miss_heartbeat());
        assert!(node.miss_heartbeat());
        node.record_heartbeat(true);
        // After reset, needs 3 more misses
        assert!(!node.miss_heartbeat());
        assert!(!node.miss_heartbeat());
        assert!(node.miss_heartbeat());
    }
}
