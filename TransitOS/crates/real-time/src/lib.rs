//! Real-time latency analysis and scheduling for bounded verification.
//!
//! Provides worst-case execution time (WCET) tracking, latency budgeting,
//! and scheduling policy analysis (Rate Monotonic, EDF, PREEMPT_RT).
//! Designed to prove that verification completes within hard real-time
//! deadlines.
//!
//! # Architecture
//! - [`latency`] — LatencyBudget, LatencyMonitor with p99 tracking
//! - [`wcet`] — WcetAnalyzer, VerificationTiming estimates
//! - [`scheduling`] — TaskSpec, SafetyVerificationTask, schedulability analysis

pub mod deterministic;
pub mod latency;
pub mod scheduling;
pub mod wcet;

pub use deterministic::*;
pub use latency::*;
pub use scheduling::*;
pub use wcet::*;
