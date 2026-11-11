//! Taint data flow analysis for sensor-to-actuator integrity.
//!
//! Tracks untrusted external sensor data (radar, camera, LIDAR, GPS, V2X)
//! through the control pipeline to critical actuator commands. Detects
//! sensor spoofing and prevents tainted data from reaching actuators.
//!
//! # Architecture
//! - [`taint`] — TaintedValue with multi-source tracking
//! - [`sensors`] — SensorType enum with trust levels and spoofability
//! - [`track`] — Propagation, combination, sanitization operators
//! - [`flow`] — DataFlowGraph with taint path tracing

pub mod flow;
pub mod sensors;
pub mod taint;
pub mod track;

pub use flow::*;
pub use sensors::*;
pub use taint::*;
pub use track::*;
