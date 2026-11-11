//! # Wire Format: Slice Codec, CRC-16 Framing, Link-Layer Fragmentation
//!
//! Implements the on-air representation of [`ProtocolSlice`] frames per the
//! protocol specification (`docs/protocol/protocol-spec.md`):
//!
//! ```text
//! Slice Type (1) | Source (8) | Dest (8) | Hops (1) | SeqNo (2) |
//! Energy (1) | Payload (0..=128) | CRC16 (2)
//! ```
//!
//! Header overhead is 21 bytes (minimum frame 23 bytes).
//!
//! ## Link-layer fragmentation
//!
//! Radio transceivers have small payload limits — nRF24L01+ is **32 bytes**
//! and CC1101 is **64 bytes** — while a full slice frame can reach 151 bytes.
//! [`LinkLayer`] fragments a frame into MTU-sized chunks (5-byte chunk
//! header) and reassembles them on the receiver, so the mesh protocol is
//! independent of the physical radio's payload limit.

use crate::types::{EnergyLevel, NodeId, ProtocolSlice, SliceType, MAX_PAYLOAD_SIZE};
use heapless::Vec;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// CRC-16/CCITT-FALSE initial value.
const CRC16_INIT: u16 = 0xFFFF;

/// Poly for CRC-16/CCITT-FALSE (X.25 variant uses 0x1021 as well).
const CRC16_POLY: u16 = 0x1021;

/// Fixed header size of a slice frame (excluding payload and CRC).
pub const SLICE_HEADER_LEN: usize = 1 + 8 + 8 + 1 + 2 + 1;

/// Minimum frame size (header + empty payload + CRC).
pub const MIN_FRAME_LEN: usize = SLICE_HEADER_LEN + 2;

/// Maximum frame size (header + max payload + CRC).
pub const MAX_FRAME_LEN: usize = SLICE_HEADER_LEN + MAX_PAYLOAD_SIZE + 2;

/// Size of the link-layer chunk header.
pub const CHUNK_HEADER_LEN: usize = 2 + 1 + 1 + 1; // seqno + total + index + data_len

/// Default MTU for the nRF24L01+ (32 bytes).
pub const MTU_NRF24: usize = 32;

/// Default MTU for the CC1101 (64 bytes).
pub const MTU_CC1101: usize = 64;

// ---------------------------------------------------------------------------
// Energy Level Encoding
// ---------------------------------------------------------------------------

impl EnergyLevel {
    /// Wire representation (matches protocol spec §4).
    pub fn to_wire_u8(self) -> u8 {
        match self {
            EnergyLevel::Surplus => 0,
            EnergyLevel::Nominal => 1,
            EnergyLevel::Low => 2,
            EnergyLevel::Critical => 3,
        }
    }

    /// Decode from the wire representation; unknown values map to `Critical`.
    pub fn from_wire_u8(v: u8) -> Self {
        match v {
            0 => EnergyLevel::Surplus,
            1 => EnergyLevel::Nominal,
            2 => EnergyLevel::Low,
            _ => EnergyLevel::Critical,
        }
    }
}

// ---------------------------------------------------------------------------
// CRC-16/CCITT-FALSE
// ---------------------------------------------------------------------------

/// Compute CRC-16/CCITT-FALSE over `data`.
pub fn crc16(data: &[u8]) -> u16 {
    let mut crc = CRC16_INIT;
    for &byte in data {
        crc ^= (byte as u16) << 8;
        for _ in 0..8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ CRC16_POLY;
            } else {
                crc <<= 1;
            }
        }
    }
    crc
}

// ---------------------------------------------------------------------------
// Slice Codec
// ---------------------------------------------------------------------------

/// Serialize a protocol slice into its on-air frame (with CRC16).
///
/// Payload is encoded exactly as carried in the slice (callers that embed
/// structured payloads — route advertisements, broadcast headers, integrity
/// reports — serialize those into the payload first).
pub fn encode_slice(slice: &ProtocolSlice) -> Result<Vec<u8, MAX_FRAME_LEN>, CodecError> {
    let mut frame = Vec::<u8, MAX_FRAME_LEN>::new();

    frame
        .push(slice.slice_type as u8)
        .map_err(|_| CodecError::FrameTooLarge)?;
    frame
        .extend_from_slice(slice.src.as_bytes())
        .map_err(|_| CodecError::FrameTooLarge)?;
    frame
        .extend_from_slice(slice.dst.as_bytes())
        .map_err(|_| CodecError::FrameTooLarge)?;
    frame
        .push(slice.hops)
        .map_err(|_| CodecError::FrameTooLarge)?;
    frame
        .extend_from_slice(&slice.seqno.to_le_bytes())
        .map_err(|_| CodecError::FrameTooLarge)?;
    frame
        .push(slice.energy.to_wire_u8())
        .map_err(|_| CodecError::FrameTooLarge)?;
    frame
        .extend_from_slice(&slice.payload)
        .map_err(|_| CodecError::FrameTooLarge)?;

    let crc = crc16(&frame);
    frame
        .extend_from_slice(&crc.to_le_bytes())
        .map_err(|_| CodecError::FrameTooLarge)?;

    Ok(frame)
}

/// Parse and CRC-verify an on-air frame into a protocol slice.
pub fn decode_slice(frame: &[u8]) -> Result<ProtocolSlice, CodecError> {
    if frame.len() < MIN_FRAME_LEN {
        return Err(CodecError::FrameTooShort);
    }
    if frame.len() > MAX_FRAME_LEN {
        return Err(CodecError::FrameTooLarge);
    }

    let (body, crc_bytes) = frame.split_at(frame.len() - 2);
    let expected_crc = u16::from_le_bytes([crc_bytes[0], crc_bytes[1]]);
    if crc16(body) != expected_crc {
        return Err(CodecError::CrcMismatch);
    }

    let slice_type = SliceType::from_wire_u8(body[0]).ok_or(CodecError::InvalidSliceType)?;
    let src = NodeId::from_bytes(&body[1..9]).ok_or(CodecError::InvalidNodeId)?;
    let dst = NodeId::from_bytes(&body[9..17]).ok_or(CodecError::InvalidNodeId)?;
    let hops = body[17];
    let seqno = u16::from_le_bytes([body[18], body[19]]);
    let energy = EnergyLevel::from_wire_u8(body[20]);
    let payload = Vec::<u8, MAX_PAYLOAD_SIZE>::from_slice(&body[21..])
        .map_err(|_| CodecError::PayloadTooLarge)?;

    Ok(ProtocolSlice {
        slice_type,
        src,
        dst,
        hops,
        seqno,
        payload,
        energy,
    })
}

impl SliceType {
    /// Map a wire byte to a slice type (unknown values → `None`).
    pub fn from_wire_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(SliceType::Beacon),
            0x02 => Some(SliceType::RouteAdv),
            0x03 => Some(SliceType::DataFragment),
            0x04 => Some(SliceType::DataAck),
            0x05 => Some(SliceType::IntegrityReport),
            0x06 => Some(SliceType::Broadcast),
            0x07 => Some(SliceType::Command),
            0x08 => Some(SliceType::FwUpdate),
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Link-Layer Fragmentation
// ---------------------------------------------------------------------------

/// Fragments oversized frames into MTU-sized chunks for small-payload radios.
#[derive(Debug, Clone)]
pub struct LinkLayer {
    mtu: usize,
    seqno: u16,
}

/// A single MTU-sized link-layer chunk: `[seqno:2][total:1][index:1][len:1][data]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub seqno: u16,
    pub total: u8,
    pub index: u8,
    pub data: Vec<u8, { MTU_NRF24 - CHUNK_HEADER_LEN }>,
}

impl LinkLayer {
    pub fn new(mtu: usize) -> Result<Self, CodecError> {
        if mtu <= CHUNK_HEADER_LEN {
            return Err(CodecError::MtuTooSmall);
        }
        Ok(Self { mtu, seqno: 0 })
    }

    /// Split a frame into chunks for transmission.
    pub fn fragment(&mut self, frame: &[u8]) -> Vec<Chunk, 16> {
        let seqno = self.seqno;
        self.seqno = self.seqno.wrapping_add(1);

        let payload_per_chunk = self.mtu - CHUNK_HEADER_LEN;
        let total = frame.len().div_ceil(payload_per_chunk) as u8;
        let mut chunks: Vec<Chunk, 16> = Vec::new();

        for (index, part) in frame.chunks(payload_per_chunk).enumerate() {
            let mut data: Vec<u8, { MTU_NRF24 - CHUNK_HEADER_LEN }> = Vec::new();
            if data.extend_from_slice(part).is_ok() {
                let _ = chunks.push(Chunk {
                    seqno,
                    total,
                    index: index as u8,
                    data,
                });
            }
        }
        chunks
    }

    /// Current chunk sequence number.
    pub fn seqno(&self) -> u16 {
        self.seqno
    }
}

/// Reassembles link-layer chunks back into a frame.
#[derive(Debug, Clone)]
pub struct Reassembler {
    /// In-progress reassembly buffers keyed by chunk seqno.
    buffers: Vec<Buffer, 4>,
}

#[derive(Debug, Clone)]
struct Buffer {
    seqno: u16,
    chunks: Vec<Option<Vec<u8, { MTU_NRF24 - CHUNK_HEADER_LEN }>>, 16>,
}

impl Default for Reassembler {
    fn default() -> Self {
        Self::new()
    }
}

impl Reassembler {
    pub fn new() -> Self {
        Self {
            buffers: Vec::new(),
        }
    }

    /// Feed one chunk. Returns `Some(frame)` when the whole frame is present.
    pub fn push(&mut self, chunk: Chunk) -> Result<Option<Vec<u8, MAX_FRAME_LEN>>, CodecError> {
        let total = chunk.total as usize;
        let index = chunk.index as usize;
        if index >= total {
            return Err(CodecError::InvalidChunkIndex);
        }

        // Find or create the buffer for this seqno.
        let buf_idx = match self.buffers.iter().position(|b| b.seqno == chunk.seqno) {
            Some(i) => i,
            None => {
                if self.buffers.len() >= 4 {
                    self.buffers.remove(0); // Drop oldest buffer
                }
                let mut buf: Vec<Option<Vec<u8, { MTU_NRF24 - CHUNK_HEADER_LEN }>>, 16> =
                    Vec::new();
                for _ in 0..total {
                    let _ = buf.push(None);
                }
                let _ = self.buffers.push(Buffer {
                    seqno: chunk.seqno,
                    chunks: buf,
                });
                self.buffers.len() - 1
            }
        };

        let buffer = &mut self.buffers[buf_idx];
        buffer.chunks[index] = Some(chunk.data);

        if buffer.chunks.iter().all(|c| c.is_some()) {
            // Assemble
            let mut frame = Vec::<u8, MAX_FRAME_LEN>::new();
            for part in buffer.chunks.iter().flatten() {
                if frame.extend_from_slice(part).is_err() {
                    return Err(CodecError::FrameTooLarge);
                }
            }
            // Recompute total from first chunk? The frame is opaque here; the
            // caller (decode_slice) validates length + CRC.
            let _ = self.buffers.remove(buf_idx);
            if frame.len() < MIN_FRAME_LEN || frame.len() > MAX_FRAME_LEN {
                return Err(CodecError::FrameTooLarge);
            }
            Ok(Some(frame))
        } else {
            Ok(None)
        }
    }

    /// Number of in-progress buffers.
    pub fn pending_buffers(&self) -> usize {
        self.buffers.len()
    }
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodecError {
    FrameTooShort,
    FrameTooLarge,
    PayloadTooLarge,
    CrcMismatch,
    InvalidSliceType,
    InvalidNodeId,
    MtuTooSmall,
    InvalidChunkIndex,
    ChunkTooLarge,
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn test_node_id(id: u8) -> NodeId {
        let mut bytes = [0u8; 8];
        bytes[0] = id;
        NodeId(bytes)
    }

    fn sample_slice() -> ProtocolSlice {
        ProtocolSlice {
            slice_type: SliceType::DataFragment,
            src: test_node_id(1),
            dst: test_node_id(2),
            hops: 3,
            seqno: 0x1234,
            payload: Vec::from_slice(b"dead-drop payload").unwrap(),
            energy: EnergyLevel::Nominal,
        }
    }

    #[test]
    fn test_crc16_known_vector() {
        // CRC-16/CCITT-FALSE of "123456789" is 0x29B1.
        assert_eq!(crc16(b"123456789"), 0x29B1);
    }

    #[test]
    fn test_slice_roundtrip() {
        let slice = sample_slice();
        let frame = encode_slice(&slice).unwrap();
        let decoded = decode_slice(&frame).unwrap();

        assert_eq!(decoded.slice_type, slice.slice_type);
        assert_eq!(decoded.src, slice.src);
        assert_eq!(decoded.dst, slice.dst);
        assert_eq!(decoded.hops, slice.hops);
        assert_eq!(decoded.seqno, slice.seqno);
        assert_eq!(decoded.energy, slice.energy);
        assert_eq!(&decoded.payload[..], &slice.payload[..]);
    }

    #[test]
    fn test_frame_size_bounds() {
        let empty = ProtocolSlice {
            payload: Vec::new(),
            ..sample_slice()
        };
        assert_eq!(encode_slice(&empty).unwrap().len(), MIN_FRAME_LEN);

        let full_payload = Vec::from_slice(&[0xABu8; MAX_PAYLOAD_SIZE]).unwrap();
        let full = ProtocolSlice {
            payload: full_payload,
            ..sample_slice()
        };
        assert_eq!(encode_slice(&full).unwrap().len(), MAX_FRAME_LEN);
    }

    #[test]
    fn test_crc_tamper_detected() {
        let slice = sample_slice();
        let mut frame = encode_slice(&slice).unwrap();
        frame[0] ^= 0xFF; // Corrupt the slice type byte
        assert!(matches!(decode_slice(&frame), Err(CodecError::CrcMismatch)));
    }

    #[test]
    fn test_truncated_frame_rejected() {
        let slice = sample_slice();
        let frame = encode_slice(&slice).unwrap();
        // Cut below the minimum frame size (header + CRC).
        let truncated = &frame[..MIN_FRAME_LEN - 1];
        assert!(matches!(
            decode_slice(truncated),
            Err(CodecError::FrameTooShort)
        ));
    }

    #[test]
    fn test_energy_level_wire_encoding() {
        assert_eq!(EnergyLevel::Surplus.to_wire_u8(), 0);
        assert_eq!(EnergyLevel::Nominal.to_wire_u8(), 1);
        assert_eq!(EnergyLevel::Low.to_wire_u8(), 2);
        assert_eq!(EnergyLevel::Critical.to_wire_u8(), 3);
        assert_eq!(EnergyLevel::from_wire_u8(0), EnergyLevel::Surplus);
        assert_eq!(EnergyLevel::from_wire_u8(99), EnergyLevel::Critical);
    }

    #[test]
    fn test_slice_type_wire_encoding() {
        for (wire, ty) in [
            (0x01, SliceType::Beacon),
            (0x02, SliceType::RouteAdv),
            (0x03, SliceType::DataFragment),
            (0x04, SliceType::DataAck),
            (0x05, SliceType::IntegrityReport),
            (0x06, SliceType::Broadcast),
            (0x07, SliceType::Command),
            (0x08, SliceType::FwUpdate),
        ] {
            assert_eq!(SliceType::from_wire_u8(wire), Some(ty));
            assert_eq!(ty as u8, wire);
        }
        assert_eq!(SliceType::from_wire_u8(0x00), None);
    }

    #[test]
    fn test_link_layer_fragmentation_nrf24() {
        let mut link = LinkLayer::new(MTU_NRF24).unwrap();

        let slice = ProtocolSlice {
            payload: Vec::from_slice(&[0xCDu8; 100]).unwrap(),
            ..sample_slice()
        };
        let frame = encode_slice(&slice).unwrap();
        assert!(frame.len() > MTU_NRF24); // Must need fragmentation

        let chunks = link.fragment(&frame);
        assert!(chunks.len() > 1);
        assert!(chunks
            .iter()
            .all(|c| c.data.len() + CHUNK_HEADER_LEN <= MTU_NRF24));

        // Reassemble
        let mut reassembler = Reassembler::new();
        let mut result = None;
        for chunk in chunks {
            if let Some(frame) = reassembler.push(chunk).unwrap() {
                result = Some(frame);
            }
        }
        let result = result.expect("frame should reassemble");
        let decoded = decode_slice(&result).unwrap();
        assert_eq!(decoded.payload.len(), 100);
    }

    #[test]
    fn test_link_layer_single_chunk() {
        let mut link = LinkLayer::new(MTU_NRF24).unwrap();
        // A small slice (empty payload → 23-byte frame) fits one chunk.
        let frame = encode_slice(&ProtocolSlice {
            payload: Vec::new(),
            ..sample_slice()
        })
        .unwrap();
        assert_eq!(frame.len(), MIN_FRAME_LEN);
        let chunks = link.fragment(&frame);
        assert_eq!(chunks.len(), 1);

        let mut reassembler = Reassembler::new();
        let reassembled = reassembler
            .push(chunks.into_iter().next().unwrap())
            .unwrap();
        assert_eq!(decode_slice(&reassembled.unwrap()).unwrap().seqno, 0x1234);
    }

    #[test]
    fn test_reassembler_rejects_bad_index() {
        let mut link = LinkLayer::new(MTU_NRF24).unwrap();
        let frame = encode_slice(&sample_slice()).unwrap();
        let mut chunks = link.fragment(&frame);
        chunks[0].index = 99;
        let mut reassembler = Reassembler::new();
        assert_eq!(
            reassembler.push(chunks.remove(0)),
            Err(CodecError::InvalidChunkIndex)
        );
    }

    #[test]
    fn test_mtu_too_small_rejected() {
        assert!(matches!(LinkLayer::new(5), Err(CodecError::MtuTooSmall)));
    }

    #[test]
    fn route_adv_fits_slice_payload() {
        use crate::types::{RouteAdvertisement, RouteEntry, RouteFlags};
        let mut entries = heapless::Vec::<RouteEntry, 32>::new();
        for i in 0..5u8 {
            let mut id = [0u8; 8];
            id[0] = i;
            entries
                .push(RouteEntry {
                    destination: crate::types::NodeId(id),
                    next_hop: crate::types::NodeId(id),
                    metric: 100 + i as u16,
                    flags: RouteFlags::empty(),
                    age_sec: 0,
                })
                .ok();
        }
        let adv = RouteAdvertisement {
            origin: crate::types::NodeId([1, 0, 0, 0, 0, 0, 0, 0]),
            seqno: 0,
            flags: RouteFlags::empty(),
            entries,
        };
        // Five entries (the per-slice budget) must fit the 128-byte payload.
        let mut buf = [0u8; 128];
        let written = postcard::to_slice(&adv, &mut buf).expect("fits in 128 bytes");
        assert!(written.len() <= MAX_PAYLOAD_SIZE);
    }
}
