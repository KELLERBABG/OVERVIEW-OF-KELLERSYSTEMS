#![no_main]
use libfuzzer_sys::fuzz_target;
use transit_can_bus::frame::{CanFrame, ValidatedFrame, IsoTpReassembler};

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }
    
    let mut reassembler = IsoTpReassembler::new();
    
    // Split input into chunks and feed to reassembler
    for chunk in data.chunks(8) {
        let frame = CanFrame::new(0x100, chunk.to_vec());
        let validated = ValidatedFrame {
            frame,
            crc: 0,
            sequence: 0,
            received_at_us: 0,
        };
        
        // Feed may return errors (sequence gaps, etc.) - that's fine for fuzzing
        let _ = reassembler.feed(&validated);
    }
});
