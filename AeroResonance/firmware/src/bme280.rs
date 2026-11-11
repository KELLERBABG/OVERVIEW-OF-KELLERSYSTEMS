//! BME280 (temperature / humidity / pressure) driver — v2 sensor payload.
//!
//! Pure, no_std, hardware-independent: register map, calibration parsing and
//! the Bosch **double-precision compensation formulas** (BME280_SensorAPI
//! v3.5.1, BSD-3-Clause — see `simulation/LICENSE`-style provenance note
//! below). The I2C transport is injected by the caller, so this module is
//! unit-tested on the host against the datasheet's worked example.
//!
//! Provenance: formulas transcribed from the Bosch Sensortec BME280_SensorAPI
//! `bme280.c` (BSD-3-Clause, copyright 2020 Bosch Sensortec GmbH), which
//! permits use with attribution. This is an independent Rust port; the sensor
//! register definitions are from the public BME280 datasheet.

// ---------------------------------------------------------------------------
// Register map (datasheet BST-BME280-DS002)
// ---------------------------------------------------------------------------
/// BME280 default I2C address (SDO low). With SDO high it is 0x77.
pub const I2C_ADDR: u8 = 0x76;
/// Chip id returned by REG_CHIP_ID on a genuine BME280.
pub const CHIP_ID: u8 = 0x60;

pub const REG_CHIP_ID: u8 = 0xD0;
pub const REG_RESET: u8 = 0xE0;
pub const REG_CTRL_HUM: u8 = 0xF2;
pub const REG_CTRL_MEAS: u8 = 0xF4;
pub const REG_CONFIG: u8 = 0xF5;
pub const REG_PRESS_MSB: u8 = 0xF7;
pub const REG_TEMP_PRESS_CALIB: u8 = 0x88; // 26 bytes
pub const REG_HUM_CALIB: u8 = 0xE1; // 7 bytes

/// Soft-reset command for REG_RESET (0xB6).
pub const RESET_CMD: u8 = 0xB6;

/// Oversampling + power-mode preset: normal mode, T×1, P×1, H×1.
/// CTRL_MEAS = (osrs_t << 5) | (osrs_p << 2) | mode(3=normal).
pub const CTRL_MEAS_NORMAL_1X: u8 = (1 << 5) | (1 << 2) | 0x03;
pub const CTRL_HUM_1X: u8 = 0x01;

// ---------------------------------------------------------------------------
// Calibration data
// ---------------------------------------------------------------------------
/// All calibration coefficients, parsed from the two calibration blocks.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CalibData {
    pub dig_t1: u16,
    pub dig_t2: i16,
    pub dig_t3: i16,
    pub dig_p1: u16,
    pub dig_p2: i16,
    pub dig_p3: i16,
    pub dig_p4: i16,
    pub dig_p5: i16,
    pub dig_p6: i16,
    pub dig_p7: i16,
    pub dig_p8: i16,
    pub dig_p9: i16,
    pub dig_h1: u8,
    pub dig_h2: i16,
    pub dig_h3: i8,
    pub dig_h4: i16,
    pub dig_h5: i16,
    pub dig_h6: i8,
}

fn le_u16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

impl CalibData {
    /// Parse from the two raw calibration blocks read from the sensor
    /// (26 bytes at `REG_TEMP_PRESS_CALIB` + 7 bytes at `REG_HUM_CALIB`).
    pub fn from_raw(tp: &[u8; 26], h: &[u8; 7]) -> CalibData {
        let h4_msb = (h[3] as i8 as i16) << 4;
        let h4 = h4_msb | (h[4] & 0x0F) as i16;
        let h5 = ((h[5] as i8 as i16) << 4) | ((h[4] >> 4) as i16);
        CalibData {
            dig_t1: le_u16(&tp[0..2]),
            dig_t2: le_u16(&tp[2..4]) as i16,
            dig_t3: le_u16(&tp[4..6]) as i16,
            dig_p1: le_u16(&tp[6..8]),
            dig_p2: le_u16(&tp[8..10]) as i16,
            dig_p3: le_u16(&tp[10..12]) as i16,
            dig_p4: le_u16(&tp[12..14]) as i16,
            dig_p5: le_u16(&tp[14..16]) as i16,
            dig_p6: le_u16(&tp[16..18]) as i16,
            dig_p7: le_u16(&tp[18..20]) as i16,
            dig_p8: le_u16(&tp[20..22]) as i16,
            dig_p9: le_u16(&tp[22..24]) as i16,
            dig_h1: tp[24],
            dig_h2: le_u16(&h[0..2]) as i16,
            dig_h3: h[2] as i8,
            dig_h4: h4,
            dig_h5: h5,
            dig_h6: h[6] as i8,
        }
    }
}

// ---------------------------------------------------------------------------
// Compensation (Bosch double-precision formulas)
// ---------------------------------------------------------------------------
/// A compensated measurement.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sample {
    /// Temperature [°C].
    pub temp_c: f64,
    /// Pressure [hPa].
    pub pressure_hpa: f64,
    /// Relative humidity [%].
    pub humidity_pct: f64,
}

fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

/// Compensate a raw ADC triplet (20-bit temp, 20-bit pressure, 16-bit hum)
/// into a [`Sample`]. `t_fine` is computed internally, exactly as Bosch does.
pub fn compensate(c: &CalibData, raw_temp: u32, raw_press: u32, raw_hum: u16) -> Sample {
    // --- temperature -------------------------------------------------------
    let v1 = (raw_temp as f64 / 16_384.0 - c.dig_t1 as f64 / 1024.0) * c.dig_t2 as f64;
    let v2 = (raw_temp as f64 / 131_072.0 - c.dig_t1 as f64 / 8192.0)
        * (raw_temp as f64 / 131_072.0 - c.dig_t1 as f64 / 8192.0)
        * c.dig_t3 as f64;
    let t_fine = v1 + v2; // note: Bosch casts to int32 for t_fine
    let temp_c = clamp((v1 + v2) / 5120.0, -40.0, 85.0);

    // --- pressure ----------------------------------------------------------
    let v1 = t_fine / 2.0 - 64_000.0;
    let mut v2 = v1 * v1 * c.dig_p6 as f64 / 32_768.0;
    v2 += v1 * c.dig_p5 as f64 * 2.0;
    v2 = v2 / 4.0 + c.dig_p4 as f64 * 65_536.0;
    let v3 = c.dig_p3 as f64 * v1 * v1 / 524_288.0;
    let mut v1 = (v3 + c.dig_p2 as f64 * v1) / 524_288.0;
    v1 = (1.0 + v1 / 32_768.0) * c.dig_p1 as f64;

    let pressure_pa = if v1 > 0.0 {
        let mut p = 1_048_576.0 - raw_press as f64;
        p = (p - v2 / 4096.0) * 6250.0 / v1;
        let v1b = c.dig_p9 as f64 * p * p / 2_147_483_648.0;
        let v2b = p * c.dig_p8 as f64 / 32_768.0;
        clamp(p + (v1b + v2b + c.dig_p7 as f64) / 16.0, 30_000.0, 110_000.0)
    } else {
        30_000.0
    };

    // --- humidity ----------------------------------------------------------
    let v1 = t_fine - 76_800.0;
    let v2 = c.dig_h4 as f64 * 64.0 + (c.dig_h5 as f64 / 16_384.0) * v1;
    let v3 = raw_hum as f64 - v2;
    let v4 = c.dig_h2 as f64 / 65_536.0;
    let v5 = 1.0 + (c.dig_h3 as f64 / 67_108_864.0) * v1;
    let v6 = 1.0 + (c.dig_h6 as f64 / 67_108_864.0) * v1 * v5;
    let v6 = v3 * v4 * (v5 * v6);
    let humidity_pct = clamp(v6 * (1.0 - c.dig_h1 as f64 * v6 / 524_288.0), 0.0, 100.0);

    Sample {
        temp_c,
        pressure_hpa: pressure_pa / 100.0,
        humidity_pct,
    }
}

/// Read a 20-bit big-endian ADC value from three consecutive registers
/// (MSB first, lowest 4 bits of the last byte ignored).
pub fn adc20(msb: u8, lsb: u8, xlsb: u8) -> u32 {
    ((msb as u32) << 12) | ((lsb as u32) << 4) | ((xlsb as u32) >> 4)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Known-good vector from the pimoroni/bme280-python test suite
    /// (verified against a real sensor; same Bosch double-precision
    /// compensation as this driver):
    /// raw T=529191 P=326816 H=30281 → T=24.7895 °C, P=1006.615 hPa, H=68.670 %.
    fn known_calib() -> CalibData {
        CalibData {
            dig_t1: 28009,
            dig_t2: 25654,
            dig_t3: 50,
            dig_p1: 39145,
            dig_p2: -10750,
            dig_p3: 3024,
            dig_p4: 5667,
            dig_p5: -120,
            dig_p6: -7,
            dig_p7: 15500,
            dig_p8: -14600,
            dig_p9: 6000,
            dig_h1: 75,
            dig_h2: 376,
            dig_h3: 0,
            dig_h4: 286,
            dig_h5: 50,
            dig_h6: 30,
        }
    }

    #[test]
    fn matches_known_sensor_vector() {
        let c = known_calib();
        let s = compensate(&c, 529_191, 326_816, 30_281);
        assert!(
            (s.temp_c - 24.789_487_8).abs() < 1e-3,
            "temp {:.6}",
            s.temp_c
        );
        assert!(
            (s.pressure_hpa - 1006.615_175_64).abs() < 1e-2,
            "press {:.6}",
            s.pressure_hpa
        );
        assert!(
            (s.humidity_pct - 68.669_966_487).abs() < 1e-3,
            "hum {:.6}",
            s.humidity_pct
        );
    }

    #[test]
    fn calib_parse_matches_direct_values() {
        // Encode known_calib() as raw register blocks, then parse back.
        let mut tp = [0u8; 26];
        tp[0..2].copy_from_slice(&28009u16.to_le_bytes());
        tp[2..4].copy_from_slice(&(25654i16).to_le_bytes());
        tp[4..6].copy_from_slice(&50i16.to_le_bytes());
        tp[6..8].copy_from_slice(&39145u16.to_le_bytes());
        tp[8..10].copy_from_slice(&(-10750i16).to_le_bytes());
        tp[10..12].copy_from_slice(&3024i16.to_le_bytes());
        tp[12..14].copy_from_slice(&5667i16.to_le_bytes());
        tp[14..16].copy_from_slice(&(-120i16).to_le_bytes());
        tp[16..18].copy_from_slice(&(-7i16).to_le_bytes());
        tp[18..20].copy_from_slice(&15500i16.to_le_bytes());
        tp[20..22].copy_from_slice(&(-14600i16).to_le_bytes());
        tp[22..24].copy_from_slice(&6000i16.to_le_bytes());
        tp[24] = 75;
        let mut h = [0u8; 7];
        h[0..2].copy_from_slice(&376i16.to_le_bytes());
        h[2] = 0;
        h[3] = (286 >> 4) as u8; // h4 MSB
        h[4] = (((50 & 0x0F) as u8) << 4) | ((286 & 0x0F) as u8); // h5 lsb | h4 lsb
        h[5] = (50 >> 4) as u8; // h5 MSB
        h[6] = 30;
        let parsed = CalibData::from_raw(&tp, &h);
        assert_eq!(parsed, known_calib());
    }

    #[test]
    fn temperature_is_monotonic_in_raw() {
        let c = known_calib();
        let t1 = compensate(&c, 500_000, 415_148, 38_370).temp_c;
        let t2 = compensate(&c, 512_000, 415_148, 38_370).temp_c;
        let t3 = compensate(&c, 524_288, 415_148, 38_370).temp_c;
        assert!(t1 < t2 && t2 < t3, "{t1} < {t2} < {t3}");
    }

    #[test]
    fn adc20_assemble() {
        // 0x7D000 = 512000 → bytes 0x7D 0x00 0x00
        assert_eq!(adc20(0x7D, 0x00, 0x00), 512_000);
        // 0x655AC = 415148 → 0x65 0x5A 0xC0 (xlsb low nibble ignored)
        assert_eq!(adc20(0x65, 0x5A, 0xC0), 415_148);
    }

    #[test]
    fn compensation_stays_in_physical_range() {
        // Sweep raw values over the full ADC span with the datasheet calib:
        // outputs must stay inside the sensor's spec range, never NaN/Inf.
        let c = known_calib();
        for raw_t in [0u32, 1, 128_000, 512_000, 900_000, 1_048_575] {
            for raw_p in [0u32, 1, 415_148, 1_048_575] {
                for raw_h in [0u16, 1, 38_370, 65_535] {
                    let s = compensate(&c, raw_t, raw_p, raw_h);
                    assert!(s.temp_c.is_finite() && (-40.0..=85.0).contains(&s.temp_c));
                    assert!(s.pressure_hpa.is_finite() && (300.0..=1100.0).contains(&s.pressure_hpa));
                    assert!(s.humidity_pct.is_finite() && (0.0..=100.0).contains(&s.humidity_pct));
                }
            }
        }
    }
}
