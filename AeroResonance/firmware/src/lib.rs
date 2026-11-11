//! # EHAR Field Node — firmware core (host-testable, no_std)
//!
//! This crate is the hardware-independent logic of the **EHAR Field Node MVP**
//! (see `hardware/` for the build). It reuses the `eh-sim-core` crate — the
//! exact same scheduler, entropy pipeline, crypto and supercapacitor model the
//! desktop simulator uses — so the simulation acts as a **digital twin** of
//! the device: what the sim predicts, the node should reproduce.
//!
//! Everything in this crate compiles and runs on the host (`cargo test`) and
//! on bare metal (`riscv32imc-unknown-none-elf`, ESP32-C3). The MCU-specific
//! glue lives in `src/bin/ehar_node.rs` (ESP32-C3, target-only crate).
//!
//! Modules map 1:1 to the simulation:
//! - [`EnergyModel`]        ↔ sim `physics::pmic_*` + `subsystems::SolarPhotovoltaic`
//! - [`node_tier`]          ↔ sim `eh_sim_core::scheduler` (task tier by V_cap)
//! - [`TelemetryFrame`]     ↔ sim Ghost-Net packet framing (`eh_sim_core::crypto`)
//! - [`NodeEntropy`]        ↔ sim `eh_sim_core::entropy::EntropyPipeline` (ADC TRNG)
//! - [`CapHealth`]          ↔ sim `eh_sim_core::cap_aging::CapState`

#![no_std]

extern crate alloc;

pub mod bme280;
pub mod energy;
pub mod entropy;
pub mod telemetry;

pub use bme280::{compensate, adc20, CalibData, Sample, CHIP_ID, I2C_ADDR};

pub use energy::{
    build_cycle_plan, cap_energy_j, cap_voltage_for_energy_j, count_transitions,
    estimate_daily_harvest_wh, node_tier, plan_duty, run_hysteresis_trace, tier_code,
    ColdStartHysteresis, ColdStartState, CyclePlan, CycleReading, DutyPlan, HysteresisConfig,
    LoadProfile, TierConfig,
};
pub use entropy::NodeEntropy;
pub use telemetry::{
    build_packet, decode_payload, packet_len, parse_packet, TelemetryFrame,
    FRAME_MAGIC, FRAME_VERSION, PAYLOAD_LEN,
};

/// Version of the field-node firmware (kept in step with the repo).
pub const FIRMWARE_VERSION: &str = env!("CARGO_PKG_VERSION");
