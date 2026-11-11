//! City OS shared types — the common vocabulary used by every City service.
//!
//! These types are aligned with the KELLER OS kernel ABI so that the same
//! structs can be serialized over KELLER IPC channels (32-byte payloads),
//! stored in the Keller Vault (AES-XTS sectors), or gossiped over City P2P
//! links. Everything is `no_std` — City services run on top of the KELLER
//! microkernel with no heap, exactly like the kernel itself.
#![no_std]

// Keep `std` unit-testing available on the host without polluting the
// no-std consumers.
#[cfg(test)]
extern crate std;

/// Maximum payload size of one KELLER IPC message (matches `IPC_MSG_LEN`).
pub const MSG_LEN: usize = 32;

/// Maximum number of KELLER IPC channels (matches `IPC_MAX_CHANNELS`).
pub const MAX_CHANNELS: usize = 8;

/// Node ids are 32-bit (KELLER process ids / capability owners).
pub type NodeId = u32;

/// Geo tags quantize a district cell: high 16 bits = lat, low 16 bits = lon.
/// A tag belongs to a district if it falls inside that district's bounding box.
pub type GeoTag = u32;

/// A capability slot id (see `city-caps` for the KELLER-aligned table).
pub type CapSlot = u32;

/// A temporal resource slot (KELLER fixed-slot scheduler uses 4).
pub const SLOT_COUNT: u32 = 4;

/// A fixed 32-byte wire buffer — the universal City message body.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Bytes32(pub [u8; MSG_LEN]);

impl Bytes32 {
    pub const EMPTY: Self = Self([0; MSG_LEN]);

    pub const fn new() -> Self {
        Self([0; MSG_LEN])
    }

    /// Copy a payload into the buffer (zero-padding; rejects oversized input).
    pub fn from_slice(s: &[u8]) -> Option<Self> {
        if s.len() > MSG_LEN {
            return None;
        }
        let mut b = [0u8; MSG_LEN];
        b[..s.len()].copy_from_slice(s);
        Some(Self(b))
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn is_zero(&self) -> bool {
        self.0 == [0; MSG_LEN]
    }
}

impl Default for Bytes32 {
    fn default() -> Self {
        Self::EMPTY
    }
}

/// Concrete KELLER IPC message type ids used on City channels.
/// (Same numbering convention as the kernel `ipc::send(channel, sender, type, …)`.)
pub mod msg_type {
    pub const TXN: u32 = 1;
    pub const GOSSIP: u32 = 2;
    pub const DROP: u32 = 3;
    pub const SHARD: u32 = 4;
    pub const RESOURCE: u32 = 5;
    pub const AUTH: u32 = 6;
    pub const HEARTBEAT: u32 = 7;
}

/// Universal KELLER-aligned message (32-byte body + typed envelope).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CityMsg {
    pub sender: NodeId,
    pub msg_type: u32,
    pub body: Bytes32,
}

impl CityMsg {
    pub const fn new(sender: NodeId, msg_type: u32) -> Self {
        Self {
            sender,
            msg_type,
            body: Bytes32::EMPTY,
        }
    }

    pub fn with_body(mut self, body: &[u8]) -> Option<Self> {
        self.body = Bytes32::from_slice(body)?;
        Some(self)
    }
}

/// One temporal slot measurement (used by the resource allocator and the
/// node dashboard to prove timing isolation on KELLER).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SlotBudget {
    pub slot: u32,
    pub budget_cycles: u32,
    pub used_cycles: u32,
    pub exceeded: bool,
}

impl SlotBudget {
    pub fn ok(&self) -> bool {
        !self.exceeded && self.used_cycles <= self.budget_cycles
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;

    #[test]
    fn bytes32_pads_and_rejects() {
        let b = Bytes32::from_slice(&[1, 2, 3]).unwrap();
        assert_eq!(b.0[0], 1);
        assert_eq!(b.0[3], 0);
        assert!(Bytes32::from_slice(&[0u8; 33]).is_none());
    }

    #[test]
    fn city_msg_roundtrip() {
        let mut v: Vec<u8> = Vec::new();
        v.extend_from_slice(&[0u8; 32]);
        let m = CityMsg::new(1, msg_type::TXN).with_body(&v[..20]).unwrap();
        assert_eq!(m.sender, 1);
        assert_eq!(m.body.as_slice()[19], 0);
    }
}
