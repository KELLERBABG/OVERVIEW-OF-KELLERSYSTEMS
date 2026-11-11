//! Mathematical state machine mapping for autonomous vehicle control.
//!
//! Supports passenger cars, heavy trucks, and articulated semi-trucks with
//! full air brake latency, actuator lag, and trailer articulation dynamics.
//! Provides a formal model of all physically possible vehicle states using
//! a bicycle dynamics model with tire forces, load transfer, and road surface
//! friction. This is the foundational crate — every other module depends on
//! its state representation.
//!
//! # Architecture
//! - [`state`] — VehicleState with pose, dynamics, articulation, actuators, tires
//! - [`machine`] — StateMachine for step-by-step state transitions
//! - [`transitions`] — Bicycle model with air brake lag, articulated trailer dynamics
//! - [`params`] — VehicleClass, VehicleParams, AirBrakeParams, ActuatorDelay, RoadSurface, dynamic CoG estimation (v1.0.0)
//! - [`constraints`] — Rollover stability, jackknife detection, brake lag, state bounds

pub mod constraints;
pub mod machine;
pub mod params;
pub mod state;
pub mod transitions;

pub use constraints::*;
pub use machine::*;
pub use params::*;
pub use state::*;
pub use transitions::*;
