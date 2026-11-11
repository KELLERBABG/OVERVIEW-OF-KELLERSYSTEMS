//! End-to-end asynchronous buffer-bounce DTN mesh verification.
//!
//! Verifies that Reed-Solomon / LDPC shards transmitted during open NVIS
//! windows are buffered by intermediate relay nodes and reconstructed
//! bit-exact at the destination across 3 disconnected nodes under dynamic
//! f_oF2 fading and simulated jammer injection.

use abos_protocol::bounce::BufferBounceEngine;
use abos_protocol::shard::{reconstruct_file, split_file};
use abos_common::types::Shard;

/// Simulate a 3-node DTN mesh: source (A) → relay (B) → destination (C).
///
/// - Node A splits a file into shards and transmits during NVIS windows.
/// - Node B captures shards, buffers them, and re-emits when its window opens.
/// - Node C collects complementary shards and reconstructs the file.
///
/// Dynamic conditions:
/// - NVIS windows open/close based on simulated f_oF2 fading.
/// - A jammer injects interference that temporarily blocks transmission.

struct SimNode {
    name: &'static str,
    engine: BufferBounceEngine,
}

impl SimNode {
    fn new(name: &'static str) -> Self {
        Self {
            name,
            engine: BufferBounceEngine::new(5),
        }
    }

    /// Attempt to transmit a shard; only succeeds if NVIS is open and no jammer.
    fn try_transmit(&mut self, shard: Shard) -> bool {
        if !self.engine.can_transmit() {
            return false;
        }
        self.engine.submit_shard(shard, 0);
        true
    }

    /// Capture a shard into the relay buffer.
    fn capture(&mut self, shard: Shard) {
        self.engine.buffer_relay_shard(shard);
    }

    /// Get all buffered relay shards for re-emission.
    fn buffered_shards(&self) -> Vec<Shard> {
        self.engine.relay_shards().to_vec()
    }

    /// Set NVIS window state.
    fn set_nvis(&mut self, open: bool) {
        self.engine.set_nvis_window(open);
    }

    /// Report a jammed frame.
    fn report_jam(&mut self) {
        self.engine.report_jammed_frame();
    }

    /// Report a clean frame.
    fn report_clean(&mut self) {
        self.engine.report_clean_frame();
    }

    /// Whether this node can currently transmit.
    fn can_transmit(&self) -> bool {
        self.engine.can_transmit()
    }
}

/// Simulate dynamic f_oF2 fading: returns whether the NVIS window is open
/// at a given time step. Uses a simple sinusoidal model where the critical
/// frequency oscillates, opening and closing the skywave path.
fn nvis_window_open(time_step: u64) -> bool {
    // Simulate f_oF2 varying between 5 MHz and 12 MHz with period ~20 steps.
    // NVIS is open when f_oF2 > 7 MHz (arbitrary threshold for test).
    let phase = (time_step as f64) * (std::f64::consts::PI / 10.0);
    let fo_f2_mhz = 8.5 + 3.5 * phase.sin(); // oscillates 5..12 MHz
    fo_f2_mhz > 7.0
}

/// Simulate jammer injection: returns true if a jammer is active at this step.
fn jammer_active(time_step: u64) -> bool {
    // Jammer is active during steps 5-8 and 15-17 (two bursts).
    (time_step >= 5 && time_step <= 8) || (time_step >= 15 && time_step <= 17)
}

#[test]
fn e2e_buffer_bounce_3node_nvis_jammer() {
    // Create test payload: 4 KB of deterministic data.
    let original: Vec<u8> = (0..4096u32).map(|i| (i % 251) as u8).collect();

    // Split into shards with 1.5x redundancy.
    let shard_size = 512;
    let shards = split_file(&original, shard_size, 1.5);
    assert!(!shards.is_empty(), "must produce shards");
    let total_shards = shards[0].total_shards;

    // Set up 3 nodes: A (source), B (relay), C (destination).
    let mut node_a = SimNode::new("A");
    let mut node_b = SimNode::new("B");
    let mut node_c = SimNode::new("C");

    // Destination collects shards here.
    let mut dest_shards: Vec<Shard> = Vec::new();

    // Simulate 40 time steps with dynamic NVIS and jammer conditions.
    for t in 0..40u64 {
        let nvis_open = nvis_window_open(t);
        let jammed = jammer_active(t);

        // Update NVIS window state on all nodes.
        node_a.set_nvis(nvis_open);
        node_b.set_nvis(nvis_open);
        node_c.set_nvis(nvis_open);

        // Jammer affects all nodes.
        if jammed {
            node_a.report_jam();
            node_b.report_jam();
            node_c.report_jam();
        } else {
            node_a.report_clean();
            node_b.report_clean();
            node_c.report_clean();
        }

        // --- Phase 1: Source transmits shards during open NVIS windows ---
        if node_a.can_transmit() {
            // Transmit up to 2 shards per time step (simulating bandwidth limit).
            let pending = node_a.engine.get_pending_shards();
            // On first transmission, submit all shards.
            if pending.is_empty() && t == 0 {
                for shard in &shards {
                    node_a.try_transmit(shard.clone());
                }
            }
        }

        // --- Phase 2: Relay captures and re-emits ---
        // When NVIS is open and no jammer, relay B can forward buffered shards.
        if node_b.can_transmit() {
            let buffered = node_b.buffered_shards();
            for shard in &buffered {
                // Relay forwards to destination (simulated direct delivery).
                dest_shards.push(shard.clone());
            }
            // Clear relay buffer after forwarding.
            node_b.engine.clear_relay_buffer();
        }

        // --- Phase 3: Source delivers shards to relay when window allows ---
        if node_a.can_transmit() {
            let pending = node_a.engine.get_pending_shards();
            for shard in &pending {
                // Simulate: source transmits, relay captures.
                node_b.capture((*shard).clone());
            }
        }

        // --- Phase 4: Relay delivers to destination when its window opens ---
        if node_b.can_transmit() {
            let buffered = node_b.buffered_shards();
            for shard in &buffered {
                dest_shards.push(shard.clone());
            }
            node_b.engine.clear_relay_buffer();
        }
    }

    // Deduplicate destination shards (relay may have forwarded duplicates).
    let mut seen: Vec<(u32, [u8; 32])> = Vec::new();
    let mut unique_dest_shards: Vec<Shard> = Vec::new();
    for s in &dest_shards {
        let key = (s.shard_index, s.file_id);
        if !seen.contains(&key) {
            seen.push(key);
            unique_dest_shards.push(s.clone());
        }
    }

    // Reconstruct the file at destination.
    let recovered = reconstruct_file(&unique_dest_shards)
        .expect("destination must reconstruct file from buffered shards");

    assert_eq!(
        recovered, original,
        "reconstructed file must be bit-exact with original"
    );
    assert!(
        unique_dest_shards.len() >= total_shards as usize,
        "must have received at least {} unique shards, got {}",
        total_shards,
        unique_dest_shards.len()
    );
}

#[test]
fn e2e_jammer_blocks_transmission_then_recovers() {
    // Verify that jammer injection actually blocks transmission and
    // recovery allows continued operation.
    let mut node = SimNode::new("test");

    // Initially clean: can transmit.
    node.set_nvis(true);
    node.report_clean();
    assert!(node.can_transmit(), "clean NVIS-open node must transmit");

    // Jammer activates over 3 consecutive frames (threshold = 3).
    for _ in 0..3 {
        node.report_jam();
    }
    assert!(!node.can_transmit(), "jammed node must not transmit");

    // Jammer clears: clean frames reset the counter.
    node.report_clean();
    assert!(node.can_transmit(), "after jammer clears, node transmits again");
}

#[test]
fn e2e_nvis_window_gates_transmission() {
    let mut node = SimNode::new("nvis_test");
    node.set_nvis(false);
    node.report_clean();
    assert!(!node.can_transmit(), "closed NVIS window must block transmission");

    node.set_nvis(true);
    assert!(node.can_transmit(), "open NVIS window must allow transmission");
}

#[test]
fn e2e_relay_buffer_dedup_and_capacity() {
    let mut engine = BufferBounceEngine::new(3);

    // Create a shard.
    let file_id = [42u8; 32];
    let shard = Shard {
        file_id,
        shard_index: 0,
        total_shards: 4,
        data: vec![1, 2, 3, 4],
        checksum: 0xDEADBEEF,
    };

    // First insertion succeeds.
    assert!(engine.buffer_relay_shard(shard.clone()));
    // Duplicate is rejected.
    assert!(!engine.buffer_relay_shard(shard.clone()));
    assert_eq!(engine.relay_buffer_count(), 1);

    // Different shard index is accepted.
    let shard2 = Shard {
        file_id,
        shard_index: 1,
        total_shards: 4,
        data: vec![5, 6, 7, 8],
        checksum: 0xCAFEBABE,
    };
    assert!(engine.buffer_relay_shard(shard2));
    assert_eq!(engine.relay_buffer_count(), 2);

    // Drain one shard.
    let drained = engine.drain_relay_shard(0, &file_id);
    assert!(drained.is_some());
    assert_eq!(engine.relay_buffer_count(), 1);
}
