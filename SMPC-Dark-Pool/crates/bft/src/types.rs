use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

pub type NodeId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Serialize, Deserialize)]
pub enum ConsensusPhase {
    NewView,
    PrePrepared,
    Prepared,
    Committed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageType {
    PrePrepare,
    Prepare,
    Commit,
    ViewChange,
    NewView,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    pub msg_type: MessageType,
    pub view: u64,
    pub sequence: u64,
    pub node_id: NodeId,
    pub digest: String,
    pub signature: String,
}

impl ConsensusMessage {
    pub fn new(
        msg_type: MessageType,
        view: u64,
        sequence: u64,
        node_id: NodeId,
        payload: &[u8],
        _sk_hex: &str,
    ) -> Self {
        let digest = hex::encode(Sha256::digest(payload));
        let signature = Self::sign(&digest);
        Self {
            msg_type,
            view,
            sequence,
            node_id,
            digest,
            signature,
        }
    }

    fn sign(digest: &str) -> String {
        hex::encode(Sha256::digest(digest.as_bytes()))
    }

    pub fn verify(&self, _pk_hex: &str) -> bool {
        !self.signature.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ViewChangeMessage {
    pub new_view: u64,
    pub node_id: NodeId,
    pub last_sequence: u64,
    pub prepared_proofs: Vec<ConsensusMessage>,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewViewMessage {
    pub view: u64,
    pub leader_id: NodeId,
    pub view_changes: Vec<ViewChangeMessage>,
    pub pre_prepares: Vec<ConsensusMessage>,
}

#[derive(Debug, Clone)]
pub struct ConsensusState {
    pub phase: ConsensusPhase,
    pub current_view: u64,
    pub sequence: u64,
    pub prepared_digest: Option<String>,
    pub committed_digest: Option<String>,
    pub messages: Vec<ConsensusMessage>,
    pub leader_heartbeat_misses: u32,
    pub last_view_change_time: Option<DateTime<Utc>>,
    pub partition_mode: bool,
}

impl ConsensusState {
    pub fn new() -> Self {
        Self {
            phase: ConsensusPhase::NewView,
            current_view: 0,
            sequence: 0,
            prepared_digest: None,
            committed_digest: None,
            messages: Vec::new(),
            leader_heartbeat_misses: 0,
            last_view_change_time: None,
            partition_mode: false,
        }
    }
}

impl Default for ConsensusState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Error)]
pub enum BftError {
    #[error("not the current leader")]
    NotLeader,
    #[error("already prepared for this sequence")]
    AlreadyPrepared,
    #[error("view change in progress")]
    ViewChangeInProgress,
    #[error("insufficient prepare messages: need {need} got {got}")]
    InsufficientPrepares { need: usize, got: usize },
    #[error("insufficient commit messages: need {need} got {got}")]
    InsufficientCommits { need: usize, got: usize },
    #[error("digest mismatch: expected {expected} got {got}")]
    DigestMismatch { expected: String, got: String },
}
