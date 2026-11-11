//! 6-DoF mock simulator integrating the verification chain.
//!
//! Ties together the components from the development plan:
//!
//! 1. [`FlyByWire`] computes a candidate command from pilot input.
//! 2. The candidate is frozen into a [`ControlSnapshot`].
//! 3. The [`ExecutionGate`] checks every lethal hazard on the snapshot's
//!    canonical environment (with horizon prediction).
//! 4. Only approved commands reach the [`FlightDynamics`] integrator.
//!    Blocked commands freeze the aircraft (fail-safe).
//! 5. Each verdict is appended to a hash-chained [`SpecChain`] for the
//!    flight recorder.
//!
//! **Mock only.** Not a certified avionics product.

use crate::constraints::PathConstraints;
use crate::controller::{FlyByWire, PilotInput};
use crate::dynamics::{AircraftConfig, FlightDynamics, FlightState, TerrainModel};
use crate::envelope::ConfigEnvelope;
use crate::gate::{ExecutionGate, GateReport, StandardCases};
use crate::sensors::{SensorId, SensorStatus, SensorTaintGraph};
use crate::spec::SpecChain;
use crate::state::{build_snapshot, ActuatorCommand, ControlSnapshot};
use crate::thrust::EngineLimits;
use std::collections::BTreeMap;

/// Build all-trusted sensor statuses.
pub fn trusted_sensors() -> Vec<SensorStatus> {
    SensorId::ALL
        .iter()
        .map(|id| SensorStatus::trusted(*id, 0.0))
        .collect()
}

/// The integrated mock flight simulator.
#[derive(Clone, Debug)]
pub struct PlaneSimulator {
    pub dynamics: FlightDynamics,
    pub fbw: FlyByWire,
    pub envelope: ConfigEnvelope,
    pub engine_limits: EngineLimits,
    pub terrain: TerrainModel,
    pub gate: ExecutionGate,
    pub path: PathConstraints,
    pub spec: SpecChain,
    pub sensors: SensorTaintGraph,
    /// Current live flight state.
    pub state: FlightState,
    /// Last committed actuator command (empty for blocked cycles).
    pub last_command: Option<ActuatorCommand>,
    /// Number of simulation cycles executed.
    pub cycle: u64,
    /// If `true`, gates are bypassed (for robustness tests only).
    pub bypass_gate: bool,
    /// Persistent env overrides applied on every cycle (test hook).
    env_overrides: BTreeMap<String, f64>,
}

impl PlaneSimulator {
    /// Construct a simulator from an initial state + trusted sensors.
    pub fn new(initial: FlightState, config: AircraftConfig, terrain: TerrainModel) -> Self {
        let dynamics = FlightDynamics {
            config,
            ..FlightDynamics::default()
        };
        let gate = ExecutionGate::stub(StandardCases::all());
        let sensors = SensorTaintGraph::from_sensors(trusted_sensors());
        PlaneSimulator {
            dynamics,
            fbw: FlyByWire::default(),
            envelope: ConfigEnvelope::default(),
            engine_limits: EngineLimits::default(),
            terrain,
            gate,
            path: PathConstraints::default(),
            spec: SpecChain::new(),
            sensors,
            state: initial,
            last_command: None,
            cycle: 0,
            bypass_gate: false,
            env_overrides: BTreeMap::new(),
        }
    }

    /// Run one control cycle with the given pilot input.
    pub fn step(&mut self, input: PilotInput) -> GateReport {
        self.cycle += 1;

        // 1) Compute candidate command from the control law.
        let derived = crate::dynamics::FlightDerived::from_state(&self.state, &self.terrain);
        let candidate = self.fbw.compute(&derived, self.state.quat, input);

        // 2) Build the snapshot + apply the sensor taint audit to the
        //    actuation channels.
        let mut snapshot =
            build_snapshot(&self.state, candidate, &self.dynamics.config, &self.terrain);
        for channel in 0..4 {
            let (status, _taint) = self.sensors.channel_taint(channel);
            snapshot.channels[channel].status = status;
        }

        // 2b) The snapshot env already contains all canonical magic +
        //     state + cmd variables.
        let env = snapshot.env();

        // 2c) Test-hook overrides + horizon prediction.
        let mut full_env = env.clone();
        for (k, v) in &self.env_overrides {
            full_env.insert(k.clone(), *v);
        }
        full_env.extend(self.predict_env(&snapshot));

        // 3) Verify with the gate.
        let report = self.gate.evaluate(&full_env, self.cycle);

        // 4) Apply the command only if approved.
        if report.approved {
            let cmd = candidate.clamp_channels();
            self.state = self.dynamics.step(
                &self.state,
                cmd.throttle.max(0.0),
                cmd.elevator,
                cmd.aileron,
                cmd.rudder,
            );
            self.last_command = Some(cmd);
        } else {
            // Blocked: freeze the aircraft.
            self.last_command = None;
        }

        // 5) Record the spec + verdict in the hash chain.
        self.spec.push(&report, self.state.t, full_env);

        report
    }

    /// Predict the flight path for the next N cycles from the current
    /// state + candidate command, producing horizon-environment values.
    fn predict_env(&self, snapshot: &ControlSnapshot) -> BTreeMap<String, f64> {
        let mut env = BTreeMap::new();
        let cmd = snapshot.cmd;
        let mut pred = self.state;
        for k in 1..=self.path.horizon {
            pred = self.dynamics.step(
                &pred,
                cmd.throttle.max(0.0),
                cmd.elevator,
                cmd.aileron,
                cmd.rudder,
            );
            let derived = crate::dynamics::FlightDerived::from_state(&pred, &self.terrain);
            for (name, value) in [
                ("tas", derived.tas),
                ("ias_knots", derived.ias_knots),
                ("mach", derived.mach),
                ("alpha_deg", derived.alpha_deg),
                ("nz", derived.nz),
                ("altitude", derived.altitude),
                ("terrain_clearance", derived.terrain_clearance),
                ("climb_rate", derived.climb_rate),
            ] {
                env.insert(
                    format!("{}.{}_{}", crate::formula::VarNS::Horizon.prefix(), name, k),
                    value,
                );
            }
        }
        env
    }

    /// Last committed command or neutral.
    pub fn committed_command(&self) -> ActuatorCommand {
        self.last_command.unwrap_or(ActuatorCommand::NEUTRAL)
    }

    /// Number of approved vs blocked cycles.
    pub fn stats(&self) -> (u64, u64) {
        (self.gate.approved_cycles, self.gate.blocked_cycles)
    }

    /// Validate the accumulated spec chain.
    pub fn spec_valid(&self) -> bool {
        self.spec.validate()
    }

    // --- Debug/test injection helpers --------------------------------

    /// Override a canonical environment variable for all future cycles
    /// (test hook). Passing `None` clears an override.
    pub fn set_env_override(&mut self, key: &str, value: Option<f64>) {
        match value {
            Some(v) => {
                self.env_overrides.insert(key.to_string(), v);
            }
            None => {
                self.env_overrides.remove(key);
            }
        }
    }

    /// Override an engine's EGT (0 = left, 1 = right) for fault-injection
    /// tests.
    pub fn egt_debug_override(&mut self, engine: usize, value: f64) {
        let key = if engine == 0 {
            "state.egt_L".to_string()
        } else {
            "state.egt_R".to_string()
        };
        self.set_env_override(&key, Some(value));
    }

    /// Fail the elevator channel (IRU loss) for taint-injection tests.
    pub fn sensors_debug_fail_elevator(&mut self) {
        let mut list = trusted_sensors();
        list.retain(|s| s.sensor != SensorId::Iru);
        list.push(SensorStatus::failing(SensorId::Iru));
        self.sensors = SensorTaintGraph::from_sensors(list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::{FlightState, Vec3};

    fn test_plane() -> PlaneSimulator {
        let initial = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 150.0, 0.0);
        PlaneSimulator::new(initial, AircraftConfig::default(), TerrainModel::sea_level())
    }

    #[test]
    fn normal_flight_commits_and_records() {
        let mut sim = test_plane();
        let report = sim.step(PilotInput::default());
        assert!(report.approved);
        assert!(sim.last_command.is_some());
        assert_eq!(sim.cycle, 1);
        assert!(sim.spec_valid());
    }

    #[test]
    fn overspeed_candidate_gets_blocked() {
        let mut sim = test_plane();
        // Force an overspeed state: 182 m/s at sea level => IAS ≈ 354 kt
        // (above V_NE = 340 kt). At 3000 m the same TAS would only be
        // ~301 kt IAS, so we move to sea level to make the hazard real.
        sim.state.pos = Vec3::new(0.0, 0.0, 0.0);
        sim.state.vel_body = Vec3::new(182.0, 0.0, 0.0);
        let report = sim.step(PilotInput::default());
        assert!(report.blocked(), "expected blocking report");
        assert!(sim.last_command.is_none());
        assert!(sim.gate.blocked_cycles >= 1);
        assert!(sim.spec_valid());
    }

    #[test]
    fn multiple_cycles_spec_chain_grows() {
        let mut sim = test_plane();
        for _ in 0..5 {
            sim.step(PilotInput::default());
        }
        assert_eq!(sim.cycle, 5);
        assert_eq!(sim.spec.records.len(), 5);
        assert!(sim.spec_valid());
    }

    #[test]
    fn sensor_taint_blocks_elevator_command() {
        let mut sim = test_plane();
        sim.sensors_debug_fail_elevator();
        let report = sim.step(PilotInput::new(0.5, 0.0, 0.0, 0.3));
        assert!(report.blocked(), "taint hazard must block the command");
        assert_eq!(report.blocked_reason, Some("taint_hazard"));
    }
}