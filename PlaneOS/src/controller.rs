//! Prototype fly-by-wire control law.
//!
//! Produces candidate normalized actuator commands `[-1, 1]` each control
//! cycle from pilot/simulator input and simple feedback laws. The output is
//! **not** applied directly: it is handed to the verification chain and
//! only the execution gate commits commands that are provably safe.
//!
//! **Mock only** — no certification value, no DO-178C artifacts.

use crate::dynamics::{FlightDerived, Quaternion};
use crate::state::ActuatorCommand;

/// Normalized pilot input `[-1, 1]` per axis.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PilotInput {
    pub pitch: f64,
    pub roll: f64,
    pub yaw: f64,
    pub throttle: f64,
}

impl Default for PilotInput {
    fn default() -> Self {
        PilotInput {
            pitch: 0.0,
            roll: 0.0,
            yaw: 0.0,
            throttle: 0.3,
        }
    }
}

impl PilotInput {
    pub fn new(pitch: f64, roll: f64, yaw: f64, throttle: f64) -> Self {
        PilotInput {
            pitch: pitch.clamp(-1.0, 1.0),
            roll: roll.clamp(-1.0, 1.0),
            yaw: yaw.clamp(-1.0, 1.0),
            throttle: throttle.clamp(-1.0, 1.0),
        }
    }
}

/// Simple PID-style gains for the mock control law.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ControlGains {
    /// Gain on pitch error.
    pub pitch_kp: f64,
    /// Gain on roll error.
    pub roll_kp: f64,
    /// Gain on yaw error.
    pub yaw_kp: f64,
    /// Proportional throttle hold on airspeed error (knots).
    pub speed_kp: f64,
    /// Fraction of manual pitch applied directly (mixing).
    pub direct_mix: f64,
}

impl Default for ControlGains {
    fn default() -> Self {
        ControlGains {
            pitch_kp: 0.5,
            roll_kp: 0.7,
            yaw_kp: 0.3,
            speed_kp: 0.02,
            direct_mix: 0.6,
        }
    }
}

/// The prototype control law.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlyByWire {
    pub gains: ControlGains,
    /// Target airspeed for auto-throttle (knots IAS).
    pub target_ias_knots: f64,
    /// Target altitude (m) for altitude-hold (mock).
    pub target_altitude: f64,
    /// Corner for maximum commanded load factor (uses for g-limiter).
    pub max_nz_cmd: f64,
}

impl Default for FlyByWire {
    fn default() -> Self {
        FlyByWire {
            gains: ControlGains::default(),
            target_ias_knots: 250.0,
            target_altitude: 3_000.0,
            max_nz_cmd: 1.5,
        }
    }
}

impl FlyByWire {
    /// Compute the candidate command from current state + pilot input.
    pub fn compute(
        &self,
        derived: &FlightDerived,
        quat: Quaternion,
        input: PilotInput,
    ) -> ActuatorCommand {
        let (roll, pitch, yaw) = quat.to_euler();

        // Pitch law: hold attitude target from direct input, with
        // proportional pull-up toward level flight.
        let pitch_error = -pitch;
        let elevator = self.gains.direct_mix * input.pitch
            + self.gains.pitch_kp * pitch_error;

        // Roll law: track roll input.
        let aileron = self.gains.roll_kp * (input.roll * 0.5 - roll);

        // Yaw damper: damp sideslip/yaw-rate with input mix.
        let rudder = self.gains.yaw_kp * (-yaw * 0.1) + 0.2 * input.yaw;

        // Auto-throttle: correct airspeed error toward target.
        let speed_err = self.target_ias_knots - derived.ias_knots;
        let throttle = input.throttle + self.gains.speed_kp * speed_err;

        // Simple load-factor limiter: if command would exceed g-limit,
        // neutralize pitch-up elevator.
        let elevator = if derived.nz > self.max_nz_cmd && elevator > 0.0 {
            0.0
        } else {
            elevator
        };

        ActuatorCommand::new(throttle, elevator, aileron, rudder).clamp_channels()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_clamped_to_unit() {
        let fbw = FlyByWire::default();
        let derived = FlightDerived {
            tas: 150.0,
            ias_knots: 245.0,
            mach: 0.5,
            alpha_deg: 3.0,
            nz: 1.0,
            altitude: 3000.0,
            terrain_clearance: 3000.0,
            climb_rate: 0.0,
        };
        let cmd = fbw.compute(
            &derived,
            Quaternion::IDENTITY,
            PilotInput::new(5.0, 5.0, 5.0, 5.0),
        );
        assert!(cmd.throttle <= 1.0);
        assert!(cmd.elevator >= -1.0 && cmd.elevator <= 1.0);
        assert!(cmd.aileron >= -1.0 && cmd.aileron <= 1.0);
        assert!(cmd.rudder >= -1.0 && cmd.rudder <= 1.0);
    }

    #[test]
    fn autothrottle_commands_power() {
        let fbw = FlyByWire::default();
        let derived = FlightDerived {
            tas: 120.0,
            ias_knots: 220.0, // below target 250
            mach: 0.4,
            alpha_deg: 3.0,
            nz: 1.0,
            altitude: 3000.0,
            terrain_clearance: 3000.0,
            climb_rate: 0.0,
        };
        let cmd = fbw.compute(&derived, Quaternion::IDENTITY, PilotInput::default());
        assert!(cmd.throttle > 0.3);
    }

    #[test]
    fn g_limiter_neutralizes_pullup() {
        let fbw = FlyByWire::default();
        let derived = FlightDerived {
            tas: 150.0,
            ias_knots: 245.0,
            mach: 0.5,
            alpha_deg: 3.0,
            nz: 2.0, // above max_nz_cmd
            altitude: 3000.0,
            terrain_clearance: 3000.0,
            climb_rate: 0.0,
        };
        let cmd = fbw.compute(
            &derived,
            Quaternion::IDENTITY,
            PilotInput::new(1.0, 0.0, 0.0, 0.3),
        );
        assert!(cmd.elevator <= 0.0);
    }
}