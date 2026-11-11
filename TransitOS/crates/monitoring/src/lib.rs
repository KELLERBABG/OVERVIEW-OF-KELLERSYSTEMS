//! Real-time safety monitoring dashboard.
//!
//! Collects metrics from the verification pipeline and presents a live
//! dashboard of safety status, violation history, sensor integrity, and
//! latency statistics. Used for both development debugging and production
//! monitoring.
//!
//! # Architecture
//! - [`metrics`] — SafetyMetrics, SafetyStatus (Safe/Warning/Critical)
//! - [`dashboard`] — Dashboard with snapshots and uptime tracking
//! - [`history`] — ViolationHistory with frequency analysis
//! - [`integrity`] — SensorIntegrityReport per-sensor status

pub mod dashboard;
pub mod history;
pub mod integrity;
pub mod metrics;

pub use dashboard::*;
pub use history::*;
pub use integrity::*;
pub use metrics::*;
