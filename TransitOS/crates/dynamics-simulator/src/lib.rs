//! Vehicle dynamics simulator with bicycle model, articulated truck dynamics,
//! air brake latency simulation, and controllers.
//!
//! Provides a time-stepped simulation environment for testing maneuvers and
//! control algorithms. Includes a library of standardized driving scenarios
//! for both passenger cars and heavy commercial vehicles.
//!
//! # Architecture
//! - [`simulator`] — Simulator with closure-based control loop
//! - [`scenarios`] — Scenario definitions (emergency brake, lane change, truck)
//! - [`bicycle`] — Bicycle model dynamics with load transfer computation
//! - [`tire`] — Tire force model with slip angles and friction limits
//! - [`truck`] — Articulated trailer dynamics, air brake lag simulation
//! - [`controller`] — PID, LaneKeep, and Cruise controllers

pub mod bicycle;
pub mod controller;
pub mod scenarios;
pub mod simulator;
pub mod tire;
pub mod truck;

pub use bicycle::*;
pub use controller::*;
pub use scenarios::*;
pub use simulator::*;
pub use tire::*;
pub use truck::*;
