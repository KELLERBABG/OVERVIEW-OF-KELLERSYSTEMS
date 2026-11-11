//! **City interoperability** — V2V/drone/logistics messaging + ad-hoc
//! traffic orchestration.
//!
//! - `V2vMsg`: a MAC-protected interop frame carrying a vehicle, drone, or
//!   logistics message over the City P2P/KELLER transport (32-byte body).
//! - `IntersectionScheduler`: right-of-way negotiation for a simulated
//!   autonomous-vehicle intersection. Vehicles submit arrival intents; the
//!   scheduler grants passage in arrival order up to a per-slot capacity,
//!   and refuses when the queue would exceed the cone-zone limit (the
//!   "ad-hoc traffic orchestration" piece from the plan).

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::mac::{city_mac, MacKey};
use city_types::Bytes32;

pub const INTEROP_MAX_FRAMES: usize = 32;
pub const INTEROP_MAX_QUEUE: usize = 8;
/// Domain label for the interop frame MAC key (host profile).
pub const INTEROP_LABEL: u32 = 0x1A7E_9000;

fn interop_key() -> MacKey {
    MacKey::derive(INTEROP_LABEL)
}

pub const KIND_V2V: u8 = 1;
pub const KIND_DRONE: u8 = 2;
pub const KIND_LOGISTICS: u8 = 3;

/// A verified interop frame (one 32-byte City payload + kind + MAC).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct V2vMsg {
    pub kind: u8,
    pub from: u32,
    pub seq: u32,
    pub body: Bytes32,
    pub mac: u64,
}

impl V2vMsg {
    /// Compute the keyed MAC over (kind, from, seq, body).
    pub fn compute_mac(&self) -> u64 {
        city_mac(
            &interop_key(),
            self.from,
            self.kind as u32,
            self.seq,
            &self.body.0,
        )
    }

    pub fn is_valid(&self) -> bool {
        self.kind == KIND_V2V || self.kind == KIND_DRONE || self.kind == KIND_LOGISTICS
    }

    pub fn signature_ok(&self) -> bool {
        self.is_valid() && self.mac == self.compute_mac()
    }
}

/// A right-of-way request at an intersection.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Arrival {
    pub vehicle: u32,
    pub lane: u8,
    pub arrived_seq: u32,
    pub granted: bool,
}

impl Arrival {
    pub const fn empty() -> Self {
        Self {
            vehicle: 0,
            lane: 0,
            arrived_seq: 0,
            granted: false,
        }
    }
}

/// Intersection scheduler: grants passage in FIFO arrival order with a
/// per-tick capacity ceiling.
#[derive(Copy, Clone, Debug)]
pub struct IntersectionScheduler {
    queue: [Arrival; INTEROP_MAX_QUEUE],
    count: usize,
    next_seq: u32,
    capacity: u32,
}

impl Default for IntersectionScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl IntersectionScheduler {
    pub const fn new() -> Self {
        Self {
            queue: [Arrival::empty(); INTEROP_MAX_QUEUE],
            count: 0,
            next_seq: 1,
            capacity: 2,
        }
    }

    pub const fn with_capacity(capacity: u32) -> Self {
        Self {
            queue: [Arrival::empty(); INTEROP_MAX_QUEUE],
            count: 0,
            next_seq: 1,
            capacity,
        }
    }

    /// Submit an arrival intent. Returns its queue position (None if full).
    pub fn arrive(&mut self, vehicle: u32, lane: u8) -> Option<usize> {
        if self.count >= INTEROP_MAX_QUEUE {
            return None;
        }
        let pos = self.count;
        self.queue[pos] = Arrival {
            vehicle,
            lane,
            arrived_seq: self.next_seq,
            granted: false,
        };
        self.count += 1;
        self.next_seq = self.next_seq.wrapping_add(1);
        Some(pos)
    }

    /// Grant passage to the next `capacity` arrivals (FIFO right-of-way).
    /// Returns the number granted this tick.
    pub fn grant_next(&mut self) -> usize {
        let mut granted = 0usize;
        for i in 0..self.count {
            if granted >= self.capacity as usize {
                break;
            }
            if !self.queue[i].granted {
                self.queue[i].granted = true;
                granted += 1;
            }
        }
        granted
    }

    /// Remove all granted arrivals (they have passed the intersection).
    /// Returns how many vehicles cleared.
    pub fn clear_passed(&mut self) -> usize {
        let before = self.count;
        let mut kept = 0usize;
        for i in 0..self.count {
            if !self.queue[i].granted {
                self.queue[kept] = self.queue[i];
                kept += 1;
            }
        }
        self.count = kept;
        before - kept
    }

    pub fn queued(&self) -> usize {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v2v_message_mac_and_tamper() {
        let body = Bytes32::from_slice(b"right-of-way request").unwrap();
        let mut m = V2vMsg {
            kind: KIND_V2V,
            from: 1,
            seq: 7,
            body,
            mac: 0,
        };
        m.mac = m.compute_mac();
        assert!(m.signature_ok());

        m.kind = KIND_DRONE; // spoofed kind -> invalid signature
        assert!(!m.signature_ok());
        m.kind = KIND_V2V;
        m.mac ^= 1; // tampered MAC -> invalid
        assert!(!m.signature_ok());
    }

    #[test]
    fn intersection_gives_fifo_right_of_way() {
        let mut s = IntersectionScheduler::with_capacity(2);
        // Four vehicles arrive in order.
        assert_eq!(s.arrive(11, 0), Some(0));
        assert_eq!(s.arrive(22, 1), Some(1));
        assert_eq!(s.arrive(33, 0), Some(2));
        assert_eq!(s.arrive(44, 1), Some(3));

        // First tick grants the two oldest arrivals.
        assert_eq!(s.grant_next(), 2);
        assert_eq!(s.clear_passed(), 2);
        assert_eq!(s.queued(), 2);

        // Second tick grants the remaining two.
        assert_eq!(s.grant_next(), 2);
        assert_eq!(s.clear_passed(), 2);
        assert_eq!(s.queued(), 0);
    }

    #[test]
    fn intersection_queue_full_rejects() {
        let mut s = IntersectionScheduler::with_capacity(1);
        for i in 0..INTEROP_MAX_QUEUE {
            assert!(s.arrive(i as u32, 0).is_some());
        }
        assert!(s.arrive(99, 0).is_none());
    }
}
