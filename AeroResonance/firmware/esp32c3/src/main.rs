//! EHAR Field Node MVP — ESP32-C3 firmware (v1: USB-serial telemetry).
//!
//! The complete hardware-independent logic lives in `ehar-firmware-core`
//! (host-tested, `cargo test` in `firmware/`); this binary is the thin
//! ESP32-C3 glue: ADC sampling → entropy feed → tier/duty plan → encrypted
//! Ghost-Net frame → print over USB.
//!
//! ## Build & flash
//!
//! ```bash
//! cd firmware/esp32c3
//! rustup target add riscv32imc-unknown-none-elf
//! cargo build --release --target riscv32imc-unknown-none-elf
//! cargo run --release --target riscv32imc-unknown-none-elf   # needs espflash
//! ```
//!
//! ## Wiring (see hardware/ASSEMBLY.md)
//!
//! - GPIO0  ← V_cap divider (2:1, e.g. 100kΩ/100kΩ) — supercapacitor voltage
//! - GPIO1  ← PV panel voltage divider
//! - GPIO2  ← ELF preamplifier output (AC-coupled)
//! - GPIO3  ← piezo amplifier output (AC-coupled)
//!
//! GPIO0/1/2/3 are ADC1 channels on the ESP32-C3. Adjust `PIN_*` consts to
//! match your exact board (SuperMini variants differ).

#![no_std]
#![no_main]

use ehar_firmware_core::{
    adc20, bme280, build_cycle_plan, build_packet, cap_energy_j, compensate,
    estimate_daily_harvest_wh, tier_code, CalibData, CycleReading, LoadProfile, NodeEntropy,
    TierConfig, FIRMWARE_VERSION,
};
use esp_alloc::heap_allocator;
use esp_backtrace as _;
use esp_hal::{
    analog::adc::{Adc, AdcConfig, Attenuation},
    delay::Delay,
    i2c::master::{Config as I2cConfig, I2c},
    main,
    time::Instant,
    Config,
};

// 64 KiB heap for the alloc-based telemetry/entropy code (C3 has 400 KiB SRAM);
// esp_alloc::HEAP is the #[global_allocator] (default feature). The macro
// expands to a block, so it must run inside main().

// --- Board pin mapping (ADC1: GPIO0..GPIO4 on ESP32-C3) ---------------------
// GPIO0 = V_cap divider, GPIO1 = PV divider, GPIO2 = ELF preamp, GPIO3 = piezo.
// GPIO4 = I2C SDA, GPIO5 = I2C SCL (BME280).
// Adjust the `peripherals.GPIO*` literals below to match your exact board.

// --- Measurement constants (calibrate against a multimeter, see BRINGUP.md) -
/// Approx. full-scale voltage at 11 dB attenuation (12-bit ADC), [V].
const ADC_11DB_REF_V: f64 = 2.5;
/// Divider ratio for V_cap: V_adc = V_cap * R2/(R1+R2) with R1=R2 → 0.5.
const V_CAP_DIVIDER: f64 = 0.5;
/// Divider ratio for PV: R1=220k, R2=100k -> R2/(R1+R2) = 100/320 = 0.3125.
/// Protects ADC against open-circuit voltages up to 8.0 V.
const PV_DIVIDER: f64 = 0.3125;
/// Supercapacitor bank capacitance [F] (2 × 50 F series ≈ 25 F).
const CAP_F: f64 = 25.0;
/// Conservative 24 h average harvest for the duty planner [W] (2 W panel,
/// poor conditions). Replaced by the node's own measured log in v2.
const HARVEST_AVG_W: f64 = 0.02;
/// Session key for Ghost-Net framing.
///
/// TEST-ONLY dev key (mirrors the sim's pre-WP1 practice). A production node
/// provisions a key once out-of-band and stores it in NVS; the entropy-derived
/// session key path (`NodeEntropy::session_key`) is implemented for that upgrade.
const GHOST_KEY: [u8; 32] = [0x42; 32];

fn adc_to_volts(raw: u16, divider: f64) -> f64 {
    raw as f64 / 4095.0 * ADC_11DB_REF_V / divider
}

struct Bme280Sensor {
    addr: u8,
    calib: CalibData,
}

fn init_bme280(i2c: &mut I2c<'_, esp_hal::Blocking>) -> Option<Bme280Sensor> {
    for &addr in &[bme280::I2C_ADDR, 0x77] {
        let mut id = [0u8; 1];
        if i2c.write_read(addr, &[bme280::REG_CHIP_ID], &mut id).is_ok() {
            if id[0] == bme280::CHIP_ID || id[0] == 0x58 {
                let mut tp_buf = [0u8; 26];
                let mut hum_buf = [0u8; 7];
                if i2c.write_read(addr, &[bme280::REG_TEMP_PRESS_CALIB], &mut tp_buf).is_ok()
                    && i2c.write_read(addr, &[bme280::REG_HUM_CALIB], &mut hum_buf).is_ok()
                {
                    let calib = CalibData::from_raw(&tp_buf, &hum_buf);
                    let _ = i2c.write(addr, &[bme280::REG_CTRL_HUM, bme280::CTRL_HUM_1X]);
                    let _ = i2c.write(addr, &[bme280::REG_CTRL_MEAS, bme280::CTRL_MEAS_NORMAL_1X]);
                    esp_println::println!(
                        "BME280 detected on I2C address 0x{:02x} (chip_id=0x{:02x})",
                        addr,
                        id[0]
                    );
                    return Some(Bme280Sensor { addr, calib });
                }
            }
        }
    }
    esp_println::println!("BME280 not found on I2C (GPIO4/5) — running in ADC-only mode");
    None
}

#[main]
fn main() -> ! {
    heap_allocator!(size: 65536);

    esp_println::println!(
        "EHAR Field Node v{} — ESP32-C3 (USB telemetry)",
        FIRMWARE_VERSION
    );

    let peripherals = esp_hal::init(Config::default());
    let delay = Delay::new();

    // ADC1 with four pins (ELF + piezo channels are only sampled, not used
    // for power — they are the M1.3 instrumentation payload).
    let mut adc_cfg = AdcConfig::default();
    let mut pin_v_cap = adc_cfg.enable_pin(peripherals.GPIO0, Attenuation::_11dB);
    let mut pin_pv = adc_cfg.enable_pin(peripherals.GPIO1, Attenuation::_11dB);
    let mut pin_elf = adc_cfg.enable_pin(peripherals.GPIO2, Attenuation::_11dB);
    let mut pin_piezo = adc_cfg.enable_pin(peripherals.GPIO3, Attenuation::_11dB);
    let mut adc = Adc::new(peripherals.ADC1, adc_cfg);

    // I2C0 on GPIO4 (SDA) and GPIO5 (SCL) for BME280 climate telemetry.
    let mut i2c = I2c::new(peripherals.I2C0, I2cConfig::default())
        .unwrap()
        .with_sda(peripherals.GPIO4)
        .with_scl(peripherals.GPIO5);
    let bme_sensor = init_bme280(&mut i2c);

    let mut entropy = NodeEntropy::default();
    let mut seq: u32 = 0;
    let tiers = TierConfig::default();
    // v1 is a busy-wait loop (no light-sleep yet), so the "sleep" draw is
    // ~20 mA, not 0.8 mA. The duty planner then reports honest autonomy for
    // the code that actually runs. Switch to 0.0008 once light-sleep lands (v2).
    let load = LoadProfile {
        active_current_a: 0.080,
        sleep_current_a: 0.020,
        burst_time_s: 5.0,
    };

    esp_println::println!(
        "heap ok — CAP={}F harvest_est={}mW daily_est={:.0}mWh/day (2W panel, 4h, 50% eff)",
        CAP_F as u32,
        (HARVEST_AVG_W * 1000.0) as u32,
        estimate_daily_harvest_wh(2.0, 4.0, 0.5) * 1000.0
    );

    loop {
        let cycle_start = Instant::now();
        // --- sample ---------------------------------------------------------
        let raw_v_cap: u16 = nb::block!(adc.read_oneshot(&mut pin_v_cap)).unwrap();
        let raw_pv: u16 = nb::block!(adc.read_oneshot(&mut pin_pv)).unwrap();
        let raw_elf: u16 = nb::block!(adc.read_oneshot(&mut pin_elf)).unwrap();
        let raw_piezo: u16 = nb::block!(adc.read_oneshot(&mut pin_piezo)).unwrap();

        let v_cap = adc_to_volts(raw_v_cap, V_CAP_DIVIDER);
        let pv_v = adc_to_volts(raw_pv, PV_DIVIDER);
        let elf_mv = raw_elf as f64 / 4095.0 * 1000.0; // normalized ADC value
        let piezo_mv = raw_piezo as f64 / 4095.0 * 1000.0;

        // Sample BME280 over I2C if present
        let (temp_c, humidity_pct, pressure_hpa) = if let Some(ref bme) = bme_sensor {
            let mut data = [0u8; 8];
            if i2c.write_read(bme.addr, &[bme280::REG_PRESS_MSB], &mut data).is_ok() {
                let raw_p = adc20(data[0], data[1], data[2]);
                let raw_t = adc20(data[3], data[4], data[5]);
                let raw_h = u16::from_be_bytes([data[6], data[7]]);
                let s = compensate(&bme.calib, raw_t, raw_p, raw_h);
                (s.temp_c, s.humidity_pct, s.pressure_hpa)
            } else {
                (0.0, 0.0, 0.0)
            }
        } else {
            (0.0, 0.0, 0.0)
        };

        // --- entropy (two independent noisy channels) -----------------------
        entropy.feed(raw_pv, raw_elf);

        // --- plan -----------------------------------------------------------
        let reading = CycleReading {
            v_cap,
            temp_c,
            humidity_pct,
            pressure_hpa,
            pv_voltage_v: pv_v,
            elf_mv,
            piezo_mv,
            adc_entropy_bytes: entropy.input_bytes().min(255) as u8,
            harvest_mw: 0.0,
        };
        let plan = build_cycle_plan(v_cap, CAP_F, HARVEST_AVG_W, reading, &tiers, &load);

        // --- telemetry frame (encrypted, Ghost-Net) -------------------------
        let tier_code = tier_code(plan.tier);
        let packet = build_packet(&GHOST_KEY, seq, tier_code, &reading, 0);

        // Print: plaintext header (for humans) + hex of the encrypted packet.
        esp_println::println!(
            "[seq={}] tier={} vcap={}mV pv={}mV elf={} piezo={} E={}J interval={}s autonomy={}s",
            seq,
            plan.tier.name(),
            (v_cap * 1000.0) as u32,
            (pv_v * 1000.0) as u32,
            elf_mv as u32,
            piezo_mv as u32,
            cap_energy_j(CAP_F, v_cap) as u32,
            plan.duty.interval_s as u32,
            plan.duty.autonomy_s as u32,
        );
        esp_println::println!("  GHOST {:02x?}", &packet);

        seq = seq.wrapping_add(1);

        // --- sleep ----------------------------------------------------------
        let interval_ms = (plan.duty.interval_s * 1000.0) as u64;
        while cycle_start.elapsed().as_millis() < interval_ms {
            // v1: busy-wait. v2: esp-hal light-sleep (cuts 20 mA → ~1 mA).
            delay.delay_millis(250);
        }
    }
}
