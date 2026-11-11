//! # Embedded Platform (STM32G031J6)
//!
//! Hardware bring-up layer for the compute node:
//!
//! - **Clocks** — HSI16 via `RCC.constrain()` (HAL).
//! - **GPIO** — PA5 drives the status LED (blinks once per tick).
//! - **ADC** — PA0 senses the supercapacitor voltage through the R3/R4
//!   divider (schematic §5: V_storage = 2 × V_adc) and feeds the real energy
//!   model every tick; raw ADC reads also feed the entropy pool.
//! - **Time** — the firmware clock is advanced per tick so
//!   `TimeMs::now()` works in `no_std` (see `smart_matter_core::types`).
//!
//! ## Bench bring-up (peripheral drivers)
//!
//! The LIS3DH accelerometer (I2C1), nRF24L01+/CC1101 radio (SPI1) and
//! DS18B20 (1-Wire) are intentionally **not yet wired** — each `SensorError`
//! / `RadioError` below marks exactly where the driver plugs in. The rest of
//! the firmware (protocol, mesh, storage, crypto, integrity, energy) is
//! fully functional and exercised by the host simulator and test suite.
//!
//! ## Key provisioning
//!
//! The deployment secret is baked at compile time via
//! `SMART_MATTER_DEPLOYMENT_SECRET` (64 hex chars); per-node keys are
//! derived from it with the node ID. Without the env var a demo secret is
//! used (fine for benches, **not** for deployment).

use smart_matter_core::energy::{LDO_OUTPUT_MV, MCU_UVLO_MV, PIEZO_NOMINAL_MV};
use smart_matter_core::mesh::{Radio, RadioError};
use smart_matter_core::sensor::{Accelerometer, AdcNoiseSource, SensorError, TemperatureSensor};
use smart_matter_core::types::{
    EnergyLevel, NodeId, ProtocolSlice, TimeMs, VibrationSample, VoltageReading,
};
use stm32g0xx_hal::analog::adc::Adc;
use stm32g0xx_hal::hal::adc::Channel;
use stm32g0xx_hal::prelude::*;

use crate::node::Node;

/// Node unique identifier (set during manufacturing).
const NODE_ID_BYTES: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01];

/// Demo deployment secret (override at build time with
/// `SMART_MATTER_DEPLOYMENT_SECRET`, 64 hex chars).
const DEMO_SECRET: [u8; 32] = [0x42; 32];

// ---------------------------------------------------------------------------
// Sensors / radio
// ---------------------------------------------------------------------------

/// LIS3DH placeholder — plug the I2C1 driver in here.
struct LIS3DH;

impl Accelerometer for LIS3DH {
    fn read_sample(&mut self) -> Result<VibrationSample, SensorError> {
        Err(SensorError::NotReady) // TODO(bring-up): I2C1 LIS3DH driver
    }
    fn set_rate(&mut self, _hz: u32) -> Result<(), SensorError> {
        Err(SensorError::NotReady)
    }
    fn noise_floor(&self) -> f32 {
        220.0 // μg/√Hz (LIS3DH datasheet)
    }
}

/// Real ADC on PA0: raw reads feed the entropy pool, scaled reads feed the
/// energy model (V_storage = 2 × V_adc through the 100k/100k divider).
struct AdcVoltage<PIN> {
    adc: Adc,
    pin: PIN,
}

impl<PIN: Channel<Adc, ID = u8>> AdcVoltage<PIN> {
    fn raw(&mut self) -> u16 {
        stm32g0xx_hal::block!(self.adc.read(&mut self.pin)).unwrap_or(0)
    }

    /// Storage supercapacitor voltage in mV (12-bit ADC, divider ×2).
    fn storage_mv(&mut self) -> u32 {
        let v_adc_mv =
            stm32g0xx_hal::block!(self.adc.read_voltage(&mut self.pin)).unwrap_or(0) as u32;
        (v_adc_mv * 2).min(3600)
    }
}

impl<PIN: Channel<Adc, ID = u8>> AdcNoiseSource for AdcVoltage<PIN> {
    fn read_raw(&mut self) -> Result<u32, SensorError> {
        Ok(self.raw() as u32)
    }
}

/// DS18B20 placeholder — plug the 1-Wire driver in here.
struct DS18B20;

impl TemperatureSensor for DS18B20 {
    fn read_millicelsius(&mut self) -> Result<i32, SensorError> {
        Err(SensorError::NotReady) // TODO(bring-up): 1-Wire DS18B20 driver
    }
}

/// nRF24L01+/CC1101 placeholder — plug the SPI1 driver in here.
struct SubGhzRadio;

impl Radio for SubGhzRadio {
    fn send(&mut self, _slice: &ProtocolSlice) -> Result<(), RadioError> {
        Err(RadioError::Busy) // TODO(bring-up): SPI1 radio driver
    }
    fn receive(&mut self, _timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError> {
        Ok(None)
    }
    fn last_rssi(&self) -> i16 {
        -127
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

pub fn run() -> ! {
    let dp = stm32g0xx_hal::pac::Peripherals::take().expect("peripherals");
    let mut rcc = dp.RCC.constrain();
    let gpioa = dp.GPIOA.split(&mut rcc);

    let mut led = gpioa.pa5.into_push_pull_output();
    let vbat_pin = gpioa.pa0.into_analog();
    let adc_voltage = AdcVoltage {
        adc: Adc::new(dp.ADC, &mut rcc),
        pin: vbat_pin,
    };

    // Deployment secret: compile-time env or demo default.
    let secret = match option_env!("SMART_MATTER_DEPLOYMENT_SECRET") {
        Some(hex) => decode_baked_secret(hex),
        None => DEMO_SECRET,
    };

    let node_id = NodeId(NODE_ID_BYTES);
    let mut node = Node::new(node_id, &secret, SubGhzRadio, LIS3DH, adc_voltage, DS18B20);

    // Firmware clock: one loop iteration ≈ 1 ms.
    let mut clock_ms: u64 = 0;

    loop {
        let report = node.tick();
        clock_ms += 1;
        TimeMs::set_embedded_clock(clock_ms);

        // Feed the real energy model from the ADC voltage sense.
        let storage_mv = node.adc_mut().storage_mv();
        node.update_voltage(VoltageReading {
            harvester_mv: PIEZO_NOMINAL_MV,
            storage_mv,
            vdd_mv: if storage_mv >= MCU_UVLO_MV {
                LDO_OUTPUT_MV
            } else {
                0
            },
        });

        // Status LED: blink fast in Critical, slow otherwise.
        if report.energy_level == EnergyLevel::Critical || report.tick % 5 == 0 {
            led.toggle();
        }

        cortex_m::asm::wfi();
    }
}

/// Decode a baked 64-char hex deployment secret; falls back to the demo
/// secret on malformed input.
fn decode_baked_secret(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    let hex = hex.trim().trim_start_matches("0x");
    if hex.len() != 64 {
        return DEMO_SECRET;
    }
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16).unwrap_or(0);
        let lo = (chunk[1] as char).to_digit(16).unwrap_or(0);
        out[i] = (hi * 16 + lo) as u8;
    }
    out
}
