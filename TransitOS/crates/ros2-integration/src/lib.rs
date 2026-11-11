//! ROS 2 integration for verification-in-the-loop control.
//!
//! Provides a safety node that subscribes to actuator commands and publishes
//! safety verdicts on a separate ROS 2 topic. Defines message types for
//! inter-process communication and bridges between the ROS 2 middleware
//! and Transit OS verification pipeline.
//!
//! # Architecture
//! - [`node`] — SafetyNode with state processing and verdict publishing
//! - [`messages`] — ROS 2 message type definitions
//! - [`bridge`] — Ros2Bridge mapping ROS 2 messages to verification queries

pub mod bridge;
pub mod messages;
pub mod node;
pub mod rclrs_node;

pub use bridge::*;
pub use messages::*;
pub use node::*;
pub use rclrs_node::*;
