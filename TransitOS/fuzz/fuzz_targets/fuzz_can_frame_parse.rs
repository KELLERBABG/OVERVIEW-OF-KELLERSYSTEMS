#![no_main]
use libfuzzer_sys::fuzz_target;
use transit_can_bus::frame::{CanFrame, ValidatedFrame, CanActuatorCommand, CanSensorData};

fuzz_target!(|data: &[u8]| {
    // Fuzz CanFrame creation and validation
    if data.len() >= 4 {
        let id = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let payload = if data.len() > 4 { &data[4..] } else { &[] };
        
        let frame = CanFrame::new(id, payload.to_vec());
        
        // Try to validate the frame (may fail with CRC mismatch, which is fine)
        let _ = ValidatedFrame::validate(frame);
    }
    
    // Fuzz actuator command decoding
    if data.len() >= 8 {
        let frame = CanFrame::new(0x100, data[..8].to_vec());
        let _ = CanActuatorCommand::from_frame(&frame);
    }
    
    // Fuzz sensor data decoding
    if data.len() >= 8 {
        let frame = CanFrame::new(0x200, data[..8].to_vec());
        let _ = CanSensorData::from_frame(&frame);
    }
});
