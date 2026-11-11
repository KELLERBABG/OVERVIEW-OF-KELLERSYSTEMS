//! CARLA simulator integration for end-to-end verification.
//!
//! Bridges Transit OS verification with the CARLA autonomous driving
//! simulator. Provides scenario runners, sensor fusion pipelines, and
//! end-to-end test harnesses that run the full verification stack
//! against simulated driving scenarios.
//!
//! # Architecture
//! - [`bridge`] — CarlaBridge with state translation, E2eTestRun
//! - [`scenario_runner`] — ScenarioRunner for executing scenarios against pipeline
//! - [`sensor`] — SensorFusion combining radar, lidar, camera data

pub mod bridge;
pub mod carla_client;
pub mod scenario_runner;
pub mod sensor;

pub use bridge::*;
pub use carla_client::*;
pub use scenario_runner::*;
pub use sensor::*;
