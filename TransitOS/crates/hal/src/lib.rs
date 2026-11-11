//! Hardware Abstraction Layer for Transit OS.
//!
//! Provides a platform-independent interface to vehicle sensors (LiDAR,
//! radar, camera, IMU, wheel encoders, GPS/GNSS), actuators (steering,
//! throttle, brake), and communication buses (CAN, Ethernet). Designed
//! for ISO 26262 ASIL-D compliance with built-in self-test and
//! diagnostic hooks.

pub mod actuator;
pub mod bus;
pub mod sensor;

pub use actuator::*;
pub use bus::*;
pub use sensor::*;
