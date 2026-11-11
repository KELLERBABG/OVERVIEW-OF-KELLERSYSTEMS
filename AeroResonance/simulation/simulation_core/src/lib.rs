//! simulation-core — no_std core for Atmospheric Resonance Energy Harvesting.
//!
//! This crate contains all hardware-independent logic that can be compiled
//! for both the simulation CLI (with std) and bare-metal embedded targets
//! (without std, e.g. Keller OS on STM32/ESP32).
//!
//! Modules:
//!   - physics::     Core physical constants & models (Schumann, TEG, piezo, PMIC)
//!   - cap_aging::   Supercapacitor degradation model C(T, cycles), I_leak(T)
//!   - entropy::     TRNG bit extraction (von Neumann + BLAKE3 sponge)
//!   - crypto::      ChaCha20-Poly1305 packet framing for Ghost-Net telemetry
//!   - scheduler::   Deterministic task tier scheduler (T_-1 through T_3)

#![no_std]

pub mod cap_aging;
pub mod crypto;
pub mod entropy;
pub mod physics;
pub mod scheduler;
