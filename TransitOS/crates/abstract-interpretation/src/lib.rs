//! Abstract interpretation layer for symbolic vehicle physics.
//!
//! Translates vehicle control functions into symbolic representations where
//! physical variables (velocity, brake pressure, steering angle) are replaced
//! by symbolic expressions over interval abstract domains. Enables sound
//! over-approximation of all possible vehicle behaviors.
//!
//! Supports two abstract domains:
//! - **Interval** (`[lo, hi]`): Simple but suffers from the dependency problem
//!   where `x - x ≠ 0` causes width explosion over long computation chains.
//! - **Affine** (`center + Σ εᵢ·xᵢ`): Tracks linear dependencies between
//!   variables, eliminating spurious over-approximation from the dependency
//!   problem. Recommended for multi-step dynamics integration.
//!
//! # Architecture
//! - [`symbolic`] — SymbolicExpr with arithmetic, trig, pow, tanh, atan2
//! - [`domain`] — Interval arithmetic with proper bounds for nonlinear ops
//! - [`affine`] — Affine arithmetic forms tracking variable dependencies
//! - [`interpreter`] — AbstractInterpreter evaluating expr over interval env
//! - [`physics`] — Pre-built symbolic physics expressions (braking, slip, etc.)
//! - [`analysis`] — Safety condition checking over abstract intervals

pub mod affine;
pub mod analysis;
pub mod domain;
pub mod interpreter;
pub mod physics;
pub mod symbolic;

pub use affine::*;
pub use analysis::*;
pub use domain::*;
pub use interpreter::*;
pub use physics::*;
pub use symbolic::*;
