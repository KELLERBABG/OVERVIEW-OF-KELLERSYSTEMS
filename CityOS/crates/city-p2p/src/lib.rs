//! **City P2P transport** — fixed-slot gossip between city nodes.
//!
//! Maps directly onto the KELLER networking model: the kernel's `net.rs`
//! runs isolated, MAC-protected loopback packets on fixed temporal slots, and
//! `ipc.rs` provides keyed-MAC channels. City P2P provides the *node-level*
//! counterpart: a fixed neighborhood table and a small **verified frame** that
//! carries a City `Bytes32` payload + a keyed SipHash-2-4 MAC for
//! anti-replay/anti-forgery.
//!
//! All state is static (no heap), matching the kernel constraint; on a real
//! node these frames ride KELLER IPC or the e1000 NIC (`e1000.rs`).

#![no_std]

#[cfg(test)]
extern crate std;

pub mod hw;

use city_crypto::mac::{city_mac, verify_mac, MacKey};
use city_types::{Bytes32, NodeId};

pub const P2P_MAX_PEERS: usize = 8;
pub const P2P_MAX_FRAMES: usize = 32;
/// Domain label for the P2P network key (host profile; see `MacKey::derive`).
pub const P2P_LABEL: u32 = 0xC17DECAF;

/// The P2P network key (host profile: label-derived). On a real node this
/// is the district's vault-held network key.
pub fn network_key() -> MacKey {
    MacKey::derive(P2P_LABEL)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Peer {
    pub id: NodeId,
    pub channel: usize,
    pub active: bool,
}

impl Peer {
    pub const fn empty() -> Self {
        Self {
            id: 0,
            channel: 0,
            active: false,
        }
    }
}

/// A wire frame: a City message body + envelope + MAC for verification.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Frame {
    pub seq: u32,
    pub kind: u32,
    pub from: NodeId,
    pub body: Bytes32,
    pub mac: u64,
}

impl Frame {
    pub fn compute_mac(&self) -> u64 {
        city_mac(&network_key(), self.from, self.kind, self.seq, &self.body.0)
    }

    pub fn is_valid(&self) -> bool {
        verify_mac(
            &network_key(),
            self.from,
            self.kind,
            self.seq,
            &self.body.0,
            self.mac,
        )
    }
}

/// Fixed-capacity gossip neighborhood + frame buffer.
#[derive(Copy, Clone, Debug)]
pub struct P2pNet {
    peers: [Peer; P2P_MAX_PEERS],
    peer_count: usize,
    frames: [Frame; P2P_MAX_FRAMES],
    frame_count: usize,
}

impl Default for P2pNet {
    fn default() -> Self {
        Self::new()
    }
}

impl P2pNet {
    pub const fn new() -> Self {
        Self {
            peers: [Peer::empty(); P2P_MAX_PEERS],
            peer_count: 0,
            frames: [Frame {
                seq: 0,
                kind: 0,
                from: 0,
                body: Bytes32::EMPTY,
                mac: 0,
            }; P2P_MAX_FRAMES],
            frame_count: 0,
        }
    }

    pub fn peer_count(&self) -> usize {
        self.peer_count
    }

    pub fn frame_count(&self) -> usize {
        self.frame_count
    }

    /// Register a neighbor. Returns false if already tracked or table full.
    pub fn add_peer(&mut self, id: NodeId, channel: usize) -> bool {
        if self.peers.iter().any(|p| p.active && p.id == id) {
            return false;
        }
        if self.peer_count >= P2P_MAX_PEERS {
            return false;
        }
        self.peers[self.peer_count] = Peer {
            id,
            channel,
            active: true,
        };
        self.peer_count += 1;
        true
    }

    pub fn has_peer(&self, id: NodeId) -> bool {
        self.peers.iter().any(|p| p.active && p.id == id)
    }

    /// Build a verified gossip frame (does not send).
    pub fn build_frame(&self, seq: u32, kind: u32, from: NodeId, body: &Bytes32) -> Frame {
        let mut f = Frame {
            seq,
            kind,
            from,
            body: *body,
            mac: 0,
        };
        f.mac = f.compute_mac();
        f
    }

    /// Buffer an incoming frame after verifying its MAC.
    /// Returns false if the MAC is wrong, the frame is a replay, or the
    /// buffer is full.
    pub fn ingest(&mut self, frame: &Frame) -> bool {
        if !frame.is_valid() {
            return false;
        }
        // Replay / duplicate check.
        if self
            .frames
            .iter()
            .take(self.frame_count)
            .any(|f| f.from == frame.from && f.seq == frame.seq)
        {
            return false;
        }
        if self.frame_count >= P2P_MAX_FRAMES {
            return false;
        }
        self.frames[self.frame_count] = *frame;
        self.frame_count += 1;
        true
    }

    /// Fan-out a frame to every peer over the real `city-ipc` transport.
    /// Each peer is mapped to one of the 8 KELLER-style IPC channels; the
    /// frame body (32 bytes) is sent as a verified City message. Returns the
    /// number of peers that accepted the frame.
    pub fn broadcast(&self, frame: &mut Frame) -> usize {
        frame.mac = frame.compute_mac();
        self.peers
            .iter()
            .filter(|p| p.active)
            .filter(|p| {
                // Send the frame body as one IPC message: sender(frame.from),
                // kind as msg_type. city-ipc reserves the last 8 bytes of a
                // 32-byte message for its own SipHash tag, so the body is
                // truncated to the 24-byte payload window.
                let mut payload = [0u8; 24];
                let n = core::cmp::min(24, frame.body.0.len());
                payload[..n].copy_from_slice(&frame.body.0[..n]);
                city_ipc::send(p.channel, frame.from, frame.kind, &payload[..n])
            })
            .count()
    }

    /// Drain buffered verified frames.
    pub fn drain(&mut self) -> [Frame; P2P_MAX_FRAMES] {
        let out = self.frames;
        self.frames = [Frame {
            seq: 0,
            kind: 0,
            from: 0,
            body: Bytes32::EMPTY,
            mac: 0,
        }; P2P_MAX_FRAMES];
        self.frame_count = 0;
        out
    }
}

/// Convenience: build the standard "announce" payload used at district boot.
pub fn announce_payload(node: NodeId, district: u16) -> Bytes32 {
    let mut b = Bytes32::EMPTY;
    b.0[..4].copy_from_slice(&node.to_le_bytes());
    b.0[4..6].copy_from_slice(&district.to_le_bytes());
    b
}

/// Standard message kinds (aligned with `city_types::msg_type`).
pub use city_types::msg_type as kind;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_and_gossip_roundtrip() {
        // broadcast() now sends over the real city-ipc transport.
        city_ipc::init();
        let mut net = P2pNet::new();
        assert!(net.add_peer(7, 2));
        assert!(net.add_peer(9, 3));
        assert!(!net.add_peer(7, 4)); // duplicate
        assert_eq!(net.peer_count(), 2);

        let body = announce_payload(1, 42);
        let mut f = net.build_frame(1, kind::GOSSIP, 1, &body);
        assert!(net.ingest(&f));
        assert!(!net.ingest(&f)); // replay

        // Tamper the payload -> MAC fails on ingest.
        f.body.0[0] ^= 0xFF;
        assert!(!net.ingest(&f));

        // Broadcast accepts to both peers (2 channels).
        let mut g = net.build_frame(2, kind::HEARTBEAT, 1, &body);
        assert_eq!(net.broadcast(&mut g), 2);
    }

    #[test]
    fn invalid_mac_is_rejected() {
        let mut net = P2pNet::new();
        let body = Bytes32::from_slice(b"node-hello").unwrap();
        let mut f = net.build_frame(1, kind::AUTH, 5, &body);
        f.mac ^= 0x0000_0001;
        assert!(!net.ingest(&f));
        assert_eq!(net.frame_count(), 0);
    }
}
