//! Actuator drivers for steering, throttle, and brake with safety limits.
//!
//! Each driver enforces physical constraints (max steering angle, max
//! deceleration rate, brake pressure limits) before forwarding commands
//! to hardware, preventing mechanical damage.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ActuatorError {
    #[error("Actuator command exceeds safety limits: {0}")]
    SafetyLimitExceeded(String),
    #[error("Actuator not responding: {0}")]
    NotResponding(String),
    #[error("Actuator fault: {0}")]
    Fault(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActuatorStatus {
    Ready,
    Active,
    Fault,
    Disengaged,
}

/// Unified actuator command with safety validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorCommand {
    pub steering_angle: f64,
    pub throttle: f64,
    pub brake_pressure: f64,
    pub timestamp_us: u64,
}

/// Physical limits for actuator commands.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActuatorLimits {
    pub max_steering_angle: f64,
    pub max_steering_rate: f64, // rad/s
    pub max_throttle: f64,
    pub max_brake_pressure: f64,
    pub max_brake_rate: f64, // kPa/s
}

impl Default for ActuatorLimits {
    fn default() -> Self {
        Self {
            max_steering_angle: 0.6, // ~34 degrees
            max_steering_rate: 0.8,  // rad/s
            max_throttle: 1.0,
            max_brake_pressure: 10000.0, // kPa
            max_brake_rate: 20000.0,
        }
    }
}

impl ActuatorLimits {
    /// Validate a command against physical limits.
    pub fn validate(&self, cmd: &ActuatorCommand) -> Result<(), ActuatorError> {
        if cmd.steering_angle.abs() > self.max_steering_angle {
            return Err(ActuatorError::SafetyLimitExceeded(format!(
                "steering angle {} exceeds max {}",
                cmd.steering_angle, self.max_steering_angle
            )));
        }
        if cmd.throttle < 0.0 || cmd.throttle > self.max_throttle {
            return Err(ActuatorError::SafetyLimitExceeded(format!(
                "throttle {} out of range [0, {}]",
                cmd.throttle, self.max_throttle
            )));
        }
        if cmd.brake_pressure < 0.0 || cmd.brake_pressure > self.max_brake_pressure {
            return Err(ActuatorError::SafetyLimitExceeded(format!(
                "brake pressure {} exceeds max {}",
                cmd.brake_pressure, self.max_brake_pressure
            )));
        }
        Ok(())
    }

    /// Clamp a command to physical limits.
    pub fn clamp(&self, cmd: &mut ActuatorCommand) {
        cmd.steering_angle = cmd
            .steering_angle
            .clamp(-self.max_steering_angle, self.max_steering_angle);
        cmd.throttle = cmd.throttle.clamp(0.0, self.max_throttle);
        cmd.brake_pressure = cmd.brake_pressure.clamp(0.0, self.max_brake_pressure);
    }
}

/// Steering actuator driver.
pub struct SteeringActuator {
    status: ActuatorStatus,
    limits: ActuatorLimits,
    current_angle: f64,
}

impl SteeringActuator {
    pub fn new(limits: ActuatorLimits) -> Self {
        Self {
            status: ActuatorStatus::Disengaged,
            limits,
            current_angle: 0.0,
        }
    }

    pub fn engage(&mut self) {
        self.status = ActuatorStatus::Ready;
    }

    pub fn apply(&mut self, angle: f64, dt: f64) -> Result<f64, ActuatorError> {
        if self.status == ActuatorStatus::Fault {
            return Err(ActuatorError::Fault("steering in fault state".into()));
        }
        let dt = dt.max(0.0);
        let clamped = angle.clamp(
            -self.limits.max_steering_angle,
            self.limits.max_steering_angle,
        );
        let max_change = self.limits.max_steering_rate * dt;
        let delta = (clamped - self.current_angle).clamp(-max_change, max_change);
        self.current_angle += delta;
        self.status = ActuatorStatus::Active;
        Ok(self.current_angle)
    }

    pub fn current_angle(&self) -> f64 {
        self.current_angle
    }

    pub fn status(&self) -> ActuatorStatus {
        self.status
    }
}

/// Brake actuator driver.
pub struct BrakeActuator {
    status: ActuatorStatus,
    limits: ActuatorLimits,
    current_pressure: f64,
}

impl BrakeActuator {
    pub fn new(limits: ActuatorLimits) -> Self {
        Self {
            status: ActuatorStatus::Disengaged,
            limits,
            current_pressure: 0.0,
        }
    }

    pub fn engage(&mut self) {
        self.status = ActuatorStatus::Ready;
    }

    pub fn apply(&mut self, pressure: f64, dt: f64) -> Result<f64, ActuatorError> {
        if self.status == ActuatorStatus::Fault {
            return Err(ActuatorError::Fault("brake in fault state".into()));
        }
        let dt = dt.max(0.0);
        let clamped = pressure.clamp(0.0, self.limits.max_brake_pressure);
        let max_change = self.limits.max_brake_rate * dt;
        let delta = (clamped - self.current_pressure).clamp(-max_change, max_change);
        self.current_pressure += delta;
        self.status = ActuatorStatus::Active;
        Ok(self.current_pressure)
    }

    pub fn current_pressure(&self) -> f64 {
        self.current_pressure
    }

    pub fn status(&self) -> ActuatorStatus {
        self.status
    }
}

/// Throttle actuator driver.
pub struct ThrottleActuator {
    status: ActuatorStatus,
    limits: ActuatorLimits,
    current_position: f64,
}

impl ThrottleActuator {
    pub fn new(limits: ActuatorLimits) -> Self {
        Self {
            status: ActuatorStatus::Disengaged,
            limits,
            current_position: 0.0,
        }
    }

    pub fn engage(&mut self) {
        self.status = ActuatorStatus::Ready;
    }

    pub fn apply(&mut self, position: f64) -> Result<f64, ActuatorError> {
        if self.status == ActuatorStatus::Fault {
            return Err(ActuatorError::Fault("throttle in fault state".into()));
        }
        self.current_position = position.clamp(0.0, self.limits.max_throttle);
        self.status = ActuatorStatus::Active;
        Ok(self.current_position)
    }

    pub fn current_position(&self) -> f64 {
        self.current_position
    }

    pub fn status(&self) -> ActuatorStatus {
        self.status
    }
}

/// Combined actuator system for the vehicle.
pub struct VehicleActuators {
    pub steering: SteeringActuator,
    pub brake: BrakeActuator,
    pub throttle: ThrottleActuator,
}

impl VehicleActuators {
    pub fn new(limits: ActuatorLimits) -> Self {
        Self {
            steering: SteeringActuator::new(limits.clone()),
            brake: BrakeActuator::new(limits.clone()),
            throttle: ThrottleActuator::new(limits),
        }
    }

    pub fn engage_all(&mut self) {
        self.steering.engage();
        self.brake.engage();
        self.throttle.engage();
    }

    /// Apply a validated command, returning the actual applied values.
    pub fn apply(
        &mut self,
        cmd: &ActuatorCommand,
        dt: f64,
    ) -> Result<ActuatorCommand, ActuatorError> {
        let steering = self.steering.apply(cmd.steering_angle, dt)?;
        let brake = self.brake.apply(cmd.brake_pressure, dt)?;
        let throttle = self.throttle.apply(cmd.throttle)?;
        Ok(ActuatorCommand {
            steering_angle: steering,
            throttle,
            brake_pressure: brake,
            timestamp_us: cmd.timestamp_us,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn limits_validate_ok() {
        let limits = ActuatorLimits::default();
        let cmd = ActuatorCommand {
            steering_angle: 0.3,
            throttle: 0.5,
            brake_pressure: 2000.0,
            timestamp_us: 0,
        };
        assert!(limits.validate(&cmd).is_ok());
    }

    #[test]
    fn limits_reject_oversteer() {
        let limits = ActuatorLimits::default();
        let cmd = ActuatorCommand {
            steering_angle: 1.0,
            throttle: 0.0,
            brake_pressure: 0.0,
            timestamp_us: 0,
        };
        assert!(limits.validate(&cmd).is_err());
    }

    #[test]
    fn steering_ramp() {
        let limits = ActuatorLimits::default();
        let max_angle = limits.max_steering_angle;
        let mut steering = SteeringActuator::new(limits);
        steering.engage();
        let angle = steering.apply(1.0, 0.1).unwrap(); // should clamp to max
        assert!(angle.abs() <= max_angle);
    }

    #[test]
    fn brake_ramp_rate_limit() {
        let limits = ActuatorLimits::default();
        let max_pressure = limits.max_brake_pressure;
        let mut brake = BrakeActuator::new(limits);
        brake.engage();
        let pressure = brake.apply(50000.0, 0.1).unwrap(); // way above max
        assert!(pressure <= max_pressure);
    }
}
