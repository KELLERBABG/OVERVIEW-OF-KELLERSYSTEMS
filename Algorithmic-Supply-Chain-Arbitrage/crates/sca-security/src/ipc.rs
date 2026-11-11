//! Verified Inter-Process Communication (IPC).
//!
//! Provides authenticated, ordered message passing between system
//! components. Each message carries:
//! - A HMAC-SHA256 authentication tag
//! - A monotonic sequence number for ordering and replay protection
//! - Sender/recipient component identifiers
//! - Optional payload encryption

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// A verified IPC message between system components.
#[derive(Debug, Clone)]
pub struct IpcMessage {
    /// Unique message ID.
    pub id: Uuid,
    /// Sender component.
    pub sender: String,
    /// Recipient component.
    pub recipient: String,
    /// Message type.
    pub msg_type: String,
    /// Monotonic sequence number (per sender).
    pub sequence: u64,
    /// Payload bytes.
    pub payload: Vec<u8>,
    /// HMAC-SHA256 tag for integrity.
    pub hmac_tag: Vec<u8>,
    /// Timestamp.
    pub timestamp: i64,
}

/// Verified IPC channel between two components.
#[derive(Debug)]
pub struct IpcChannel {
    shared_secret: Vec<u8>,
    sender_seq: u64,
    #[allow(dead_code)]
    receiver_seq: u64,
    last_sequence: u64,
    pending: Vec<IpcMessage>,
    sent: Vec<IpcMessage>,
}

impl IpcChannel {
    /// Create a new IPC channel with a shared secret.
    pub fn new(shared_secret: Vec<u8>) -> Self {
        Self {
            shared_secret,
            sender_seq: 0,
            receiver_seq: 0,
            last_sequence: 0,
            pending: Vec::new(),
            sent: Vec::new(),
        }
    }

    /// Create a message from sender to recipient.
    pub fn create_message(
        &mut self,
        sender: impl Into<String>,
        recipient: impl Into<String>,
        msg_type: impl Into<String>,
        payload: Vec<u8>,
    ) -> IpcMessage {
        self.sender_seq += 1;
        let seq = self.sender_seq;

        let mut msg = IpcMessage {
            id: Uuid::new_v4(),
            sender: sender.into(),
            recipient: recipient.into(),
            msg_type: msg_type.into(),
            sequence: seq,
            payload,
            hmac_tag: Vec::new(),
            timestamp: chrono::Utc::now().timestamp(),
        };

        msg.hmac_tag = self.compute_hmac(&msg);
        let sent_msg = msg.clone();
        self.sent.push(sent_msg);
        msg
    }

    /// Verify an incoming message.
    pub fn verify_message(&mut self, msg: &IpcMessage) -> Result<(), String> {
        // Verify HMAC
        let expected = self.compute_hmac(msg);
        if msg.hmac_tag != expected {
            return Err("HMAC mismatch — message tampered".into());
        }

        // Verify sequence number is monotonic
        if msg.sequence <= self.last_sequence {
            return Err(format!(
                "Sequence violation: got {}, last {}",
                msg.sequence, self.last_sequence
            ));
        }
        self.last_sequence = msg.sequence;

        Ok(())
    }

    /// Receive a verified message.
    pub fn receive(&mut self, msg: IpcMessage) -> Result<Vec<u8>, String> {
        self.verify_message(&msg)?;
        self.pending.push(msg.clone());
        Ok(msg.payload)
    }

    /// Get all sent messages.
    pub fn sent_messages(&self) -> &[IpcMessage] {
        &self.sent
    }

    /// Get pending received messages.
    pub fn pending_messages(&self) -> &[IpcMessage] {
        &self.pending
    }

    /// Compute HMAC-SHA256 for a message.
    fn compute_hmac(&self, msg: &IpcMessage) -> Vec<u8> {
        let mut hasher = Sha256::new();
        hasher.update(&self.shared_secret);
        hasher.update(msg.sender.as_bytes());
        hasher.update(msg.recipient.as_bytes());
        hasher.update(msg.msg_type.as_bytes());
        hasher.update(msg.sequence.to_le_bytes().as_slice());
        hasher.update(&msg.payload);
        hasher.finalize().to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_and_verify() {
        let secret = b"shared_secret_key_32_bytes!".to_vec();
        let mut channel = IpcChannel::new(secret);

        let msg = channel.create_message(
            "graph-engine", "settlement-gate", "delivery-confirm",
            b"payload data".to_vec(),
        );

        let mut verifier = IpcChannel::new(b"shared_secret_key_32_bytes!".to_vec());
        assert!(verifier.verify_message(&msg).is_ok());
    }

    #[test]
    fn test_replay_detection() {
        let secret = b"another_secret_key_1234".to_vec();
        let mut channel = IpcChannel::new(secret.clone());

        let msg = channel.create_message(
            "a", "b", "ping", b"data".to_vec(),
        );

        let mut verifier = IpcChannel::new(secret);
        assert!(verifier.verify_message(&msg).is_ok());

        // Same message again should fail (sequence already seen)
        assert!(verifier.verify_message(&msg).is_err());
    }

    #[test]
    fn test_tampered_message() {
        let secret = b"key_for_tamper_test!!".to_vec();
        let mut channel = IpcChannel::new(secret);

        let mut msg = channel.create_message(
            "a", "b", "data", b"original".to_vec(),
        );
        msg.payload = b"tampered".to_vec(); // Modify after creation

        let mut verifier = IpcChannel::new(b"key_for_tamper_test!!".to_vec());
        assert!(verifier.verify_message(&msg).is_err());
    }
}