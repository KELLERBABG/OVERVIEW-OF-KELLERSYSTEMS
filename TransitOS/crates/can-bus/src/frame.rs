//! CAN frame types with CRC validation and sequence number tracking.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CanError {
    #[error("CRC mismatch: expected {expected:#06x}, got {got:#06x}")]
    CrcMismatch { expected: u16, got: u16 },
    #[error("Sequence gap: expected {expected}, got {got}")]
    SequenceGap { expected: u8, got: u8 },
    #[error("Message timeout: no frame received within {timeout_ms}ms")]
    Timeout { timeout_ms: u64 },
    #[error("ISO-TP reassembly failed: {0}")]
    IsoTpError(String),
}

/// A single CAN 2.0 / CAN FD frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanFrame {
    pub arbitration_id: u32,
    pub data: Vec<u8>,
    pub timestamp_us: u64,
    pub is_fd: bool,
}

impl CanFrame {
    pub fn new(id: u32, data: Vec<u8>) -> Self {
        Self {
            arbitration_id: id,
            data,
            timestamp_us: 0,
            is_fd: false,
        }
    }

    pub fn with_timestamp(mut self, ts: u64) -> Self {
        self.timestamp_us = ts;
        self
    }
}

/// Validated CAN frame with CRC, sequence number, and timestamp.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidatedFrame {
    pub frame: CanFrame,
    pub crc: u16,
    pub sequence: u8,
    pub received_at_us: u64,
}

impl ValidatedFrame {
    /// Validate and construct a frame from raw CAN data.
    ///
    /// Expects the last 2 bytes of `data` to contain a CRC-16/CCITT and
    /// the second-to-last byte before that to contain an 8-bit sequence number.
    pub fn validate(frame: CanFrame) -> Result<Self, CanError> {
        if frame.data.len() < 3 {
            return Err(CanError::IsoTpError(
                "frame too short for CRC + sequence".into(),
            ));
        }

        let len = frame.data.len();
        let sequence = frame.data[len - 3];
        let crc_bytes = [frame.data[len - 2], frame.data[len - 1]];
        let payload = &frame.data[..len - 3];
        let received_at_us = frame.timestamp_us;

        let expected_crc = super::crc::crc16_ccitt(payload);
        let actual_crc = u16::from_be_bytes(crc_bytes);

        if expected_crc != actual_crc {
            return Err(CanError::CrcMismatch {
                expected: expected_crc,
                got: actual_crc,
            });
        }

        Ok(Self {
            frame,
            crc: actual_crc,
            sequence,
            received_at_us,
        })
    }
}

/// Tracks CAN message sequences and detects gaps.
#[derive(Debug)]
pub struct SequenceTracker {
    last_sequence: Option<u8>,
    gap_count: u64,
}

impl SequenceTracker {
    pub fn new() -> Self {
        Self {
            last_sequence: None,
            gap_count: 0,
        }
    }

    /// Check whether a sequence number is contiguous with the last seen.
    pub fn check(&mut self, seq: u8) -> Result<(), CanError> {
        if let Some(last) = self.last_sequence {
            let expected = last.wrapping_add(1);
            if seq != expected {
                self.gap_count += 1;
                return Err(CanError::SequenceGap { expected, got: seq });
            }
        }
        self.last_sequence = Some(seq);
        Ok(())
    }

    pub fn gap_count(&self) -> u64 {
        self.gap_count
    }
}

impl Default for SequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

/// State of an ISO-TP multi-frame reassembly.
#[derive(Debug, Clone)]
pub enum IsoTpState {
    Idle,
    Receiving {
        total_length: usize,
        received: Vec<u8>,
        expected_seq: u8,
    },
}

/// ISO-TP (ISO 15765-2) multi-frame reassembler.
#[derive(Debug)]
pub struct IsoTpReassembler {
    state: IsoTpState,
}

impl IsoTpReassembler {
    pub fn new() -> Self {
        Self {
            state: IsoTpState::Idle,
        }
    }

    /// Feed a validated frame and return the complete payload when assembled.
    pub fn feed(&mut self, frame: &ValidatedFrame) -> Result<Option<Vec<u8>>, CanError> {
        let data = &frame.frame.data;
        if data.is_empty() {
            return Err(CanError::IsoTpError("empty frame".into()));
        }

        let pci_type = (data[0] >> 4) & 0x0F;

        match pci_type {
            0 => {
                // Single frame
                let len = (data[0] & 0x0F) as usize;
                Ok(Some(data[1..=len].to_vec()))
            }
            1 => {
                // First frame — sequence starts at 1 (ISO 15765-2)
                let total_len = (((data[0] & 0x0F) as usize) << 8) | (data[1] as usize);
                let payload = data[2..].to_vec();
                let mut received = payload.clone();
                received.truncate(total_len.min(received.len()));

                self.state = IsoTpState::Receiving {
                    total_length: total_len,
                    received,
                    expected_seq: 1,
                };
                Ok(None)
            }
            2 => {
                // Consecutive frame — enforce sequence ordering
                // Extract values from state to avoid borrow conflicts
                let (total_length, received_len, expected_seq_val) = match &self.state {
                    IsoTpState::Receiving {
                        total_length,
                        received,
                        expected_seq,
                    } => (*total_length, received.len(), *expected_seq),
                    _ => {
                        return Err(CanError::IsoTpError(
                            "consecutive frame without first frame".into(),
                        ));
                    }
                };

                let seq = data[0] & 0x0F;
                if seq != expected_seq_val {
                    self.state = IsoTpState::Idle;
                    return Err(CanError::IsoTpError(format!(
                        "ISO-TP sequence gap: expected {}, got {}",
                        expected_seq_val, seq
                    )));
                }

                let next_seq = expected_seq_val.wrapping_add(1);
                let payload = data[1..].to_vec();
                let remaining = total_length - received_len;
                let take = remaining.min(payload.len());

                // Now update state with new values
                if let IsoTpState::Receiving {
                    ref mut received,
                    ref mut expected_seq,
                    ..
                } = self.state
                {
                    *expected_seq = next_seq;
                    received.extend_from_slice(&payload[..take]);

                    if received.len() >= total_length {
                        let result = received.clone();
                        self.state = IsoTpState::Idle;
                        Ok(Some(result))
                    } else {
                        Ok(None)
                    }
                } else {
                    unreachable!()
                }
            }
            3 => {
                // Flow control — for TX side, currently no-op
                Ok(None)
            }
            _ => Err(CanError::IsoTpError(format!(
                "unknown PCI type: {}",
                pci_type
            ))),
        }
    }
}

impl Default for IsoTpReassembler {
    fn default() -> Self {
        Self::new()
    }
}

/// Actuator command encoded/decoded on CAN bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanActuatorCommand {
    pub steering_angle: f64,
    pub throttle: f64,
    pub brake_pressure: f64,
    pub timestamp_us: u64,
}

impl CanActuatorCommand {
    /// Encode to 8-byte CAN frame (ID 0x100).
    pub fn to_frame(&self) -> CanFrame {
        let mut data = vec![0u8; 8];

        // steering: 2 bytes, scale 0.01 rad, offset -std::f64::consts::PI
        let steer_raw = ((self.steering_angle + std::f64::consts::PI) * 100.0) as u16;
        data[0..2].copy_from_slice(&steer_raw.to_be_bytes());

        // throttle: 1 byte, 0-100%
        data[2] = (self.throttle * 100.0).clamp(0.0, 255.0) as u8; // brake: 2 bytes, 0-10000 kPa
        let brake_raw = (self.brake_pressure * 100.0) as u16;
        data[3..5].copy_from_slice(&brake_raw.to_be_bytes());

        // timestamp: 3 bytes, microseconds truncated.
        // Wraps after ~4.6 hours. For sessions longer than this, use the
        // full u64 `timestamp_us` field on `CanFrame` directly, or extend
        // to CAN FD (64-byte frames) which supports 8-byte timestamps.
        let ts = self.timestamp_us as u32;
        data[5] = ((ts >> 16) & 0xFF) as u8;
        data[6] = ((ts >> 8) & 0xFF) as u8;
        data[7] = (ts & 0xFF) as u8;

        CanFrame::new(0x100, data).with_timestamp(self.timestamp_us)
    }

    /// Decode from 8-byte CAN frame.
    pub fn from_frame(frame: &CanFrame) -> Result<Self, CanError> {
        if frame.data.len() < 8 {
            return Err(CanError::IsoTpError("actuator frame too short".into()));
        }

        let steer_raw = u16::from_be_bytes([frame.data[0], frame.data[1]]);
        let steering_angle = (steer_raw as f64) / 100.0 - std::f64::consts::PI;

        let throttle = frame.data[2] as f64 / 100.0;

        let brake_raw = u16::from_be_bytes([frame.data[3], frame.data[4]]);
        let brake_pressure = brake_raw as f64 / 100.0;

        let ts =
            ((frame.data[5] as u32) << 16) | ((frame.data[6] as u32) << 8) | (frame.data[7] as u32);
        let timestamp_us = ts as u64;

        Ok(Self {
            steering_angle,
            throttle,
            brake_pressure,
            timestamp_us,
        })
    }
}

/// Sensor data frame encoded on CAN bus.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanSensorData {
    pub velocity: f64,
    pub yaw_rate: f64,
    pub lateral_accel: f64,
    pub steering_angle: f64,
    pub timestamp_us: u64,
}

impl CanSensorData {
    /// Encode to 8-byte CAN frame (ID 0x200).
    pub fn to_frame(&self) -> CanFrame {
        let mut data = vec![0u8; 8];

        let vel_raw = (self.velocity * 10.0) as u16;
        data[0..2].copy_from_slice(&vel_raw.to_be_bytes());

        let yaw_raw = ((self.yaw_rate + 10.0) * 100.0) as u16;
        data[2..4].copy_from_slice(&yaw_raw.to_be_bytes());

        let lat_raw = ((self.lateral_accel + 20.0) * 50.0) as u16;
        data[4..6].copy_from_slice(&lat_raw.to_be_bytes());

        let steer_raw = ((self.steering_angle + std::f64::consts::PI) * 100.0) as u16;
        data[6..8].copy_from_slice(&steer_raw.to_be_bytes());

        CanFrame::new(0x200, data).with_timestamp(self.timestamp_us)
    }

    /// Decode from 8-byte CAN frame.
    pub fn from_frame(frame: &CanFrame) -> Result<Self, CanError> {
        if frame.data.len() < 8 {
            return Err(CanError::IsoTpError("sensor frame too short".into()));
        }

        let vel_raw = u16::from_be_bytes([frame.data[0], frame.data[1]]);
        let velocity = vel_raw as f64 / 10.0;

        let yaw_raw = u16::from_be_bytes([frame.data[2], frame.data[3]]);
        let yaw_rate = yaw_raw as f64 / 100.0 - 10.0;

        let lat_raw = u16::from_be_bytes([frame.data[4], frame.data[5]]);
        let lateral_accel = lat_raw as f64 / 50.0 - 20.0;

        let steer_raw = u16::from_be_bytes([frame.data[6], frame.data[7]]);
        let steering_angle = steer_raw as f64 / 100.0 - std::f64::consts::PI;

        Ok(Self {
            velocity,
            yaw_rate,
            lateral_accel,
            steering_angle,
            timestamp_us: frame.timestamp_us,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sequence_tracker_detects_gap() {
        let mut tracker = SequenceTracker::new();
        assert!(tracker.check(0).is_ok());
        assert!(tracker.check(1).is_ok());
        assert!(tracker.check(3).is_err()); // gap: 2 skipped
        assert_eq!(tracker.gap_count(), 1);
    }

    #[test]
    fn sequence_tracker_wraps_around() {
        let mut tracker = SequenceTracker::new();
        assert!(tracker.check(254).is_ok());
        assert!(tracker.check(255).is_ok());
        assert!(tracker.check(0).is_ok()); // wrapping
    }

    #[test]
    fn actuator_roundtrip() {
        let cmd = CanActuatorCommand {
            steering_angle: 0.5,
            throttle: 0.8,
            brake_pressure: 500.0,
            timestamp_us: 12345,
        };
        let frame = cmd.to_frame();
        let decoded = CanActuatorCommand::from_frame(&frame).unwrap();
        assert!((decoded.steering_angle - 0.5).abs() < 0.02);
        assert!((decoded.throttle - 0.8).abs() < 0.02);
        assert!((decoded.brake_pressure - 500.0).abs() < 1.0);
    }

    #[test]
    fn sensor_roundtrip() {
        let data = CanSensorData {
            velocity: 25.5,
            yaw_rate: 0.12,
            lateral_accel: -1.2,
            steering_angle: 0.3,
            timestamp_us: 99999,
        };
        let frame = data.to_frame();
        let decoded = CanSensorData::from_frame(&frame).unwrap();
        assert!((decoded.velocity - 25.5).abs() < 0.2);
        assert!((decoded.yaw_rate - 0.12).abs() < 0.02);
    }

    #[test]
    fn isotp_detects_sequence_gap() {
        use super::{IsoTpReassembler, ValidatedFrame};

        let mut reassembler = IsoTpReassembler::new();

        // First frame: 10 bytes total, PCI type 1
        let first = ValidatedFrame {
            frame: CanFrame {
                arbitration_id: 0x100,
                data: vec![0x10, 0x0A, 0xAA, 0xBB], // PCI=1, len=10
                timestamp_us: 0,
                is_fd: false,
            },
            crc: 0,
            sequence: 0,
            received_at_us: 0,
        };
        assert!(reassembler.feed(&first).unwrap().is_none());

        // Consecutive frame with WRONG sequence (should be 1, got 3)
        let bad_seq = ValidatedFrame {
            frame: CanFrame {
                arbitration_id: 0x100,
                data: vec![0x23, 0xCC, 0xDD], // PCI=2, seq=3 (wrong!)
                timestamp_us: 0,
                is_fd: false,
            },
            crc: 0,
            sequence: 0,
            received_at_us: 0,
        };
        let err = reassembler.feed(&bad_seq);
        assert!(err.is_err());
        match err.unwrap_err() {
            super::CanError::IsoTpError(msg) => {
                assert!(msg.contains("sequence gap"));
            }
            other => panic!("expected IsoTpError, got {:?}", other),
        }
    }
}
