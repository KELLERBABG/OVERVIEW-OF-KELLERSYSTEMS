//! CRC-16/CCITT for CAN frame integrity validation.
//!
//! Polynomial: x^16 + x^12 + x^5 + 1 (0x1021), init: 0xFFFF

/// CRC-16/CCITT lookup table.
const CRC_TABLE: [u16; 256] = {
    let mut table = [0u16; 256];
    let mut i = 0u16;
    while i < 256 {
        let mut crc = i << 8;
        let mut j = 0u8;
        while j < 8 {
            if crc & 0x8000 != 0 {
                crc = (crc << 1) ^ 0x1021;
            } else {
                crc <<= 1;
            }
            j += 1;
        }
        table[i as usize] = crc;
        i += 1;
    }
    table
};

/// Compute CRC-16/CCITT over a byte slice.
pub fn crc16_ccitt(data: &[u8]) -> u16 {
    let mut crc: u16 = 0xFFFF;
    for &byte in data {
        crc = (crc << 8) ^ CRC_TABLE[((crc >> 8) ^ byte as u16) as usize];
    }
    crc
}

/// Verify a CRC over data + appended CRC bytes.
pub fn verify_crc16(data_with_crc: &[u8]) -> bool {
    if data_with_crc.len() < 2 {
        return false;
    }
    let payload = &data_with_crc[..data_with_crc.len() - 2];
    let expected = u16::from_be_bytes([
        data_with_crc[data_with_crc.len() - 2],
        data_with_crc[data_with_crc.len() - 1],
    ]);
    crc16_ccitt(payload) == expected
}

/// Append CRC-16/CCITT to a data buffer.
pub fn append_crc(data: &[u8]) -> Vec<u8> {
    let mut result = data.to_vec();
    let crc = crc16_ccitt(data);
    result.extend_from_slice(&crc.to_be_bytes());
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_vector() {
        let data = b"123456789";
        assert_eq!(crc16_ccitt(data), 0x29B1);
    }

    #[test]
    fn empty_data() {
        assert_eq!(crc16_ccitt(b""), 0xFFFF);
    }

    #[test]
    fn append_and_verify() {
        let data = b"hello world";
        let with_crc = append_crc(data);
        assert!(verify_crc16(&with_crc));
    }

    #[test]
    fn detect_corruption() {
        let data = b"hello world";
        let mut with_crc = append_crc(data);
        with_crc[0] ^= 0xFF; // corrupt
        assert!(!verify_crc16(&with_crc));
    }
}
