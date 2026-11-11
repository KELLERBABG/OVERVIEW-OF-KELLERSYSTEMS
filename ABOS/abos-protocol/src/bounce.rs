use abos_common::types::*;
use std::collections::HashMap;

/// Buffer-Bounce engine implementing asynchronous store-and-forward relay
///
/// The atmosphere acts as a distributed relay: shards are transmitted upward
/// without a specific destination. Other Ghost Nodes capture, store, and
/// re-emit shards. The engine waits for ACKs or complementary shards
/// appearing in the spectrum before stopping its own transmit cycle.
pub struct BufferBounceEngine {
    /// Shards we have submitted for transmission (pending ACK)
    pending_shards: HashMap<[u8; 32], (Shard, u64)>,
    /// ACKs we've received from other nodes
    received_acks: Vec<[u8; 32]>,
    /// Complementary shards we've detected in the spectrum
    complementary_shards: Vec<Shard>,
    /// Maximum number of retransmissions per shard
    max_retransmissions: u32,
    /// Current retransmission count per shard
    retransmit_count: HashMap<[u8; 32], u32>,
    /// NVIS window state: tracks whether the ionosphere is open for skywave
    nvis_window_open: bool,
    /// Jammer detection: number of consecutive jammed frames observed
    jammer_consecutive_frames: u32,
    /// Threshold before we declare a jammer present
    jammer_threshold: u32,
    /// Whether a jammer is currently detected
    jammer_detected: bool,
    /// Relay buffer: shards captured from the spectrum for store-and-forward
    relay_buffer: Vec<Shard>,
    /// Maximum relay buffer size
    max_relay_buffer: usize,
}

impl BufferBounceEngine {
    pub fn new(max_retransmissions: u32) -> Self {
        Self {
            pending_shards: HashMap::new(),
            received_acks: Vec::new(),
            complementary_shards: Vec::new(),
            max_retransmissions,
            retransmit_count: HashMap::new(),
            nvis_window_open: true,
            jammer_consecutive_frames: 0,
            jammer_threshold: 3,
            jammer_detected: false,
            relay_buffer: Vec::new(),
            max_relay_buffer: 1024,
        }
    }

    /// Submit a shard for buffer-bounce transmission
    pub fn submit_shard(&mut self, shard: Shard, timestamp: u64) {
        let mut shard_id = [0u8; 32];
        shard_id[..4].copy_from_slice(&shard.shard_index.to_le_bytes());
        shard_id[4..8].copy_from_slice(&shard.file_id[..4]);
        self.pending_shards.insert(shard_id, (shard, timestamp));
        self.retransmit_count.entry(shard_id).or_insert(0);
    }

    /// Process an ACK for a specific shard
    /// Returns true if all complementary shards for this file have been received
    pub fn process_ack(&mut self, shard_id: [u8; 32]) -> bool {
        self.received_acks.push(shard_id);
        self.pending_shards.remove(&shard_id);

        // Check if we have enough complementary shards to reconstruct
        // (simplified: just check if we have received ACKs for all our shards)

        self.pending_shards.is_empty()
    }

    /// Record a complementary shard detected in the spectrum
    pub fn record_complementary_shard(&mut self, shard: Shard) {
        let index = shard.shard_index;
        // Avoid duplicates
        if !self
            .complementary_shards
            .iter()
            .any(|s| s.shard_index == index)
        {
            self.complementary_shards.push(shard);
        }
    }

    /// Check if we should retransmit a shard
    pub fn should_retransmit(&mut self, shard_id: &[u8; 32]) -> bool {
        if let Some(count) = self.retransmit_count.get_mut(shard_id) {
            if *count < self.max_retransmissions {
                *count += 1;
                return true;
            }
        }
        false
    }

    /// Get pending shards that need transmission
    pub fn get_pending_shards(&self) -> Vec<&Shard> {
        self.pending_shards.values().map(|(s, _)| s).collect()
    }

    /// Get the number of complementary shards collected
    pub fn complementary_shard_count(&self) -> usize {
        self.complementary_shards.len()
    }

    /// Clear completed transfers
    pub fn cleanup(&mut self) {
        self.pending_shards.clear();
        self.received_acks.clear();
        self.complementary_shards.clear();
        self.retransmit_count.clear();
    }

    // --- NVIS Window Management ----------------------------------------

    /// Set the NVIS window state (open = skywave propagation possible)
    pub fn set_nvis_window(&mut self, open: bool) {
        self.nvis_window_open = open;
    }

    /// Whether the NVIS window is currently open for skywave transmission
    pub fn nvis_window_open(&self) -> bool {
        self.nvis_window_open
    }

    /// Whether a shard can be transmitted right now (NVIS open AND no jammer)
    pub fn can_transmit(&self) -> bool {
        self.nvis_window_open && !self.jammer_detected
    }

    // --- Jammer Detection ----------------------------------------------

    /// Report a frame with elevated interference. Returns true if a jammer
    /// is now declared (threshold crossed).
    pub fn report_jammed_frame(&mut self) -> bool {
        self.jammer_consecutive_frames += 1;
        if self.jammer_consecutive_frames >= self.jammer_threshold {
            self.jammer_detected = true;
        }
        self.jammer_detected
    }

    /// Report a clean frame (resets consecutive jammed counter).
    pub fn report_clean_frame(&mut self) {
        self.jammer_consecutive_frames = 0;
        if self.jammer_consecutive_frames == 0 && !self.jammer_detected {
            // Already clear
        } else if self.jammer_consecutive_frames == 0 {
            // Jammer cleared after enough clean frames
            self.jammer_detected = false;
        }
    }

    /// Whether a jammer is currently detected
    pub fn jammer_detected(&self) -> bool {
        self.jammer_detected
    }

    /// Clear the jammer state (e.g., after operator intervention)
    pub fn clear_jammer(&mut self) {
        self.jammer_detected = false;
        self.jammer_consecutive_frames = 0;
    }

    // --- Relay Buffer (Store-and-Forward) ----------------------------

    /// Capture a shard into the relay buffer for asynchronous forwarding.
    /// Returns true if the shard was newly buffered, false if duplicate.
    pub fn buffer_relay_shard(&mut self, shard: Shard) -> bool {
        let is_dup = self
            .relay_buffer
            .iter()
            .any(|s| s.shard_index == shard.shard_index && s.file_id == shard.file_id);
        if is_dup {
            return false;
        }
        if self.relay_buffer.len() < self.max_relay_buffer {
            self.relay_buffer.push(shard);
            true
        } else {
            false
        }
    }

    /// Retrieve all buffered relay shards (for re-emission).
    pub fn relay_shards(&self) -> &[Shard] {
        &self.relay_buffer
    }

    /// Number of shards in the relay buffer.
    pub fn relay_buffer_count(&self) -> usize {
        self.relay_buffer.len()
    }

    /// Remove a shard from the relay buffer after successful re-emission.
    pub fn drain_relay_shard(&mut self, shard_index: u32, file_id: &[u8; 32]) -> Option<Shard> {
        if let Some(pos) = self
            .relay_buffer
            .iter()
            .position(|s| s.shard_index == shard_index && s.file_id == *file_id)
        {
            Some(self.relay_buffer.remove(pos))
        } else {
            None
        }
    }

    /// Clear the relay buffer.
    pub fn clear_relay_buffer(&mut self) {
        self.relay_buffer.clear();
    }
}
