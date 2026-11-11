use byteorder::{BigEndian, ReadBytesExt};
use crc32fast::Hasher;
use std::io::Cursor;
use thiserror::Error;

pub const PACKET_SYNC_WORD: u32 = 0xAA55AA55;

#[derive(Error, Debug)]
pub enum PacketError {
    #[error("Invalid synchronization header word: {0:#X}")]
    InvalidSync(u32),
    #[error("CRC32 mismatch: expected {expected:#X}, computed {computed:#X}")]
    CrcMismatch { expected: u32, computed: u32 },
    #[error("Payload truncated or buffer underflow")]
    Underflow,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone)]
pub struct RawFpgaEvent {
    pub sequence_id: u32,
    pub timestamp_utc_ns: u64,
    pub station_id: u8,
    pub trigger_pattern: u16,
    pub channel_hits: Vec<(u8, u16, u16)>,
}

pub fn decode_fpga_packet(buffer: &[u8]) -> Result<RawFpgaEvent, PacketError> {
    if buffer.len() < 24 {
        return Err(PacketError::Underflow);
    }

    let mut cursor = Cursor::new(buffer);

    let sync = cursor.read_u32::<BigEndian>()?;
    if sync != PACKET_SYNC_WORD {
        return Err(PacketError::InvalidSync(sync));
    }

    let sequence_id = cursor.read_u32::<BigEndian>()?;
    let timestamp_utc_ns = cursor.read_u64::<BigEndian>()?;
    let station_id = cursor.read_u8()?;
    let _reserved = cursor.read_u8()?;
    let trigger_pattern = cursor.read_u16::<BigEndian>()?;
    let num_hits = cursor.read_u16::<BigEndian>()? as usize;

    let mut hits = Vec::with_capacity(num_hits);
    for _ in 0..num_hits {
        let ch = cursor.read_u8()?;
        let _flags = cursor.read_u8()?;
        let tdc = cursor.read_u16::<BigEndian>()?;
        let adc = cursor.read_u16::<BigEndian>()?;
        hits.push((ch, tdc, adc));
    }

    let expected_crc = cursor.read_u32::<BigEndian>()?;
    let payload_len = cursor.position() as usize - 4;
    let mut hasher = Hasher::new();
    hasher.update(&buffer[..payload_len]);
    let computed_crc = hasher.finalize();

    if expected_crc != computed_crc {
        return Err(PacketError::CrcMismatch {
            expected: expected_crc,
            computed: computed_crc,
        });
    }

    Ok(RawFpgaEvent {
        sequence_id,
        timestamp_utc_ns,
        station_id,
        trigger_pattern,
        channel_hits: hits,
    })
}
