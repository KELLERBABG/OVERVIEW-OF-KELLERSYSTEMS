//! Control-cycle state snapshots.
//!
//! A [`ControlSnapshot`] captures the flight state, derived quantities,
//! pending actuator command, configuration constants, and sensor-quality
//! metadata for a single control cycle, and materializes it as the
//! canonical ground environment used by every formula/evaluator in the
//! verification chain.
//!
//! The snapshot seeds every `mag.*` constant the hazard formulas reference
//! (envelope limits, engine limits, terrain, aircraft) plus `state.*` and
//! `cmd.*` variables — so the gate's standard cases are always decidable.

use crate::dynamics::{AircraftConfig, FlightDerived, FlightState, TerrainModel};
use crate::envelope::ConfigEnvelope;
use crate::formula::VarNS;
use crate::thrust::EngineLimits;
use std::collections::BTreeMap;

/// Normalized actuator command `[-1, 1]` for one control cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ActuatorCommand {
    pub throttle: f64,
    pub elevator: f64,
    pub aileron: f64,
    pub rudder: f64,
}

impl ActuatorCommand {
    pub const NEUTRAL: ActuatorCommand = ActuatorCommand {
        throttle: 0.0,
        elevator: 0.0,
        aileron: 0.0,
        rudder: 0.0,
    };

    pub fn new(throttle: f64, elevator: f64, aileron: f64, rudder: f64) -> Self {
        ActuatorCommand {
            throttle,
            elevator,
            aileron,
            rudder,
        }
    }

    /// Clamp all channels to `[-1, 1]`.
    pub fn clamp_channels(&self) -> ActuatorCommand {
        ActuatorCommand::new(
            self.throttle.clamp(-1.0, 1.0),
            self.elevator.clamp(-1.0, 1.0),
            self.aileron.clamp(-1.0, 1.0),
            self.rudder.clamp(-1.0, 1.0),
        )
    }
}

/// Taint levels for sensor-data provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TaintLevel {
    /// Verified/trusted (internal computational state).
    Trusted,
    /// Single untrusted sensor (e.g., one pitot).
    Suspect,
    /// Conflicting or corrupted multiple-source data.
    Untrusted,
}

/// Readiness of a control surface channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ChannelStatus {
    Operational,
    Degraded,
    Failed,
}

/// Per-channel actuation status.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ChannelInfo {
    pub status: ChannelStatus,
    pub taint: TaintLevel,
}

impl ChannelInfo {
    pub const OPERATIONAL_TRUSTED: ChannelInfo = ChannelInfo {
        status: ChannelStatus::Operational,
        taint: TaintLevel::Trusted,
    };
}

/// Full control-cycle snapshot: everything the verification chain needs.
#[derive(Clone, Debug, PartialEq)]
pub struct ControlSnapshot {
    pub state: FlightState,
    pub derived: FlightDerived,
    pub cmd: ActuatorCommand,
    pub config: AircraftConfig,
    pub envelope: ConfigEnvelope,
    pub engine_limits: EngineLimits,
    pub terrain: TerrainModel,
    /// Flap setting 0..3 (0 = clean, 3 = full).
    pub flap_setting: u32,
    /// Landing gear down.
    pub gear_down: bool,
    /// Thrust reverser armed.
    pub reverser_armed: bool,
    /// Engine fan speed N1 normalized `[0, 1.15]` per engine.
    pub n1: [f64; 2],
    /// Engine core speed N2 normalized `[0, 1.2]` per engine.
    pub n2: [f64; 2],
    /// Exhaust gas temperature (deg C).
    pub egt: [f64; 2],
    /// Oil pressure (psi).
    pub oil_pressure: [f64; 2],
    /// Engine number currently failed/unauthorized.
    pub engine_failed: Option<usize>,
    /// Channel statuses for each actuator lane (0=aileron, 1=elevator,
    /// 2=rudder, 3=throttle).
    pub channels: [ChannelInfo; 4],
    /// Freeze/warning flag set when a command was blocked by the gate.
    pub gate_blocked: bool,
}

impl ControlSnapshot {
    /// Assemble the canonical ground environment for formulas.
    ///
    /// Seeds every magic constant (envelope limits, engine limits,
    /// terrain, aircraft) followed by state/cmd variables.
    pub fn env(&self) -> BTreeMap<String, f64> {
        let mut env = BTreeMap::new();
        self.seed_env(&mut env);
        env
    }

    /// Same as [`ControlSnapshot::env`] for a caller-provided mutable map.
    pub fn seed_env(&self, env: &mut BTreeMap<String, f64>) {
        // --- Aircraft magic constants.
        env.insert("mag.aircraft.wing_area".to_string(), self.config.wing_area);
        env.insert("mag.aircraft.mass".to_string(), self.config.mass);
        env.insert(
            "mag.aircraft.max_thrust".to_string(),
            self.config.max_total_thrust(),
        );
        env.insert(
            "mag.aircraft.stall_alpha_deg".to_string(),
            14.0, // mock
        );

        // --- Envelope magic constants.
        self.envelope.seed_env(env);

        // --- Engine magic constants.
        self.engine_limits.seed_env(env);

        // --- Terrain magic constants.
        self.terrain.seed_env(env);

        // --- State variables.
        let st = &self.state;
        let d = &self.derived;
        let mut entries: Vec<(&str, f64)> = vec![
            ("t", st.t),
            ("pos_x", st.pos.x),
            ("pos_y", st.pos.y),
            ("pos_z", st.pos.z),
            ("vel_x", st.vel_body.x),
            ("vel_y", st.vel_body.y),
            ("vel_z", st.vel_body.z),
            ("quat_w", st.quat.w),
            ("quat_x", st.quat.x),
            ("quat_y", st.quat.y),
            ("quat_z", st.quat.z),
            ("omega_x", st.omega.x),
            ("omega_y", st.omega.y),
            ("omega_z", st.omega.z),
            ("tas", d.tas),
            ("ias_knots", d.ias_knots),
            ("mach", d.mach),
            ("alpha_deg", d.alpha_deg),
            ("nz", d.nz),
            ("altitude", d.altitude),
            ("terrain_clearance", d.terrain_clearance),
            ("climb_rate", d.climb_rate),
            // --- Engine parameters.
            ("n1_L", self.n1[0]),
            ("n1_R", self.n1[1]),
            ("n2_L", self.n2[0]),
            ("n2_R", self.n2[1]),
            ("egt_L", self.egt[0]),
            ("egt_R", self.egt[1]),
            ("oil_pressure_L", self.oil_pressure[0]),
            ("oil_pressure_R", self.oil_pressure[1]),
            // --- Channel status booleans (as 0/1 for NRA embedding).
            (
                "aileron_ok",
                (self.channels[0].status == ChannelStatus::Operational) as u8 as f64,
            ),
            (
                "elevator_ok",
                (self.channels[1].status == ChannelStatus::Operational) as u8 as f64,
            ),
            (
                "rudder_ok",
                (self.channels[2].status == ChannelStatus::Operational) as u8 as f64,
            ),
            (
                "throttle_ok",
                (self.channels[3].status == ChannelStatus::Operational) as u8 as f64,
            ),
            ("gate_blocked", self.gate_blocked as u8 as f64),
            ("flap_setting", self.flap_setting as f64),
            ("gear_down", self.gear_down as u8 as f64),
            ("reverser_armed", self.reverser_armed as u8 as f64),
        ];
        for (name, value) in entries.drain(..) {
            env.insert(format!("{}.{}", VarNS::State.prefix(), name), value);
        }

        // --- Pending actuator command (cmd namespace).
        let cmd_entries = [
            ("throttle", self.cmd.throttle),
            ("elevator", self.cmd.elevator),
            ("aileron", self.cmd.aileron),
            ("rudder", self.cmd.rudder),
        ];
        for (name, value) in cmd_entries {
            env.insert(format!("{}.{}", VarNS::Cmd.prefix(), name), value);
        }
    }
}

/// Build a reasonable in-flight snapshot from a state + command.
pub fn build_snapshot(
    state: &FlightState,
    cmd: ActuatorCommand,
    config: &AircraftConfig,
    terrain: &TerrainModel,
) -> ControlSnapshot {
    let derived = FlightDerived::from_state(state, terrain);
    ControlSnapshot {
        state: *state,
        derived,
        cmd: cmd.clamp_channels(),
        config: *config,
        envelope: ConfigEnvelope::default(),
        engine_limits: EngineLimits::default(),
        terrain: *terrain,
        flap_setting: 0,
        gear_down: false,
        reverser_armed: false,
        n1: [0.75; 2],
        n2: [0.85; 2],
        egt: [520.0; 2],
        oil_pressure: [55.0; 2],
        engine_failed: None,
        channels: [
            ChannelInfo::OPERATIONAL_TRUSTED,
            ChannelInfo::OPERATIONAL_TRUSTED,
            ChannelInfo::OPERATIONAL_TRUSTED,
            ChannelInfo::OPERATIONAL_TRUSTED,
        ],
        gate_blocked: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dynamics::{FlightState, Vec3};

    fn key_list(env: &BTreeMap<String, f64>) -> Vec<String> {
        env.keys().cloned().collect::<Vec<_>>()
    }

    #[test]
    fn snapshot_env_is_complete_for_core_formulas() {
        let state = FlightState::level(Vec3::new(0.0, 0.0, 1000.0), 150.0, 0.0);
        let snap = build_snapshot(
            &state,
            ActuatorCommand::new(0.5, 0.1, -0.2, 0.0),
            &AircraftConfig::default(),
            &TerrainModel::sea_level(),
        );
        let env = snap.env();
        let keys = key_list(&env);
        // Core canonical names exist.
        for key in [
            "state.ias_knots",
            "state.alpha_deg",
            "state.nz",
            "state.altitude",
            "cmd.throttle",
            "cmd.elevator",
            "state.terrain_clearance",
            "state.aileron_ok",
            "state.n1_R",
            "mag.envelope.vne_knots",
            "mag.engine.n1_max",
            "mag.terrain.base_elevation",
        ] {
            assert!(
                env.contains_key(key),
                "missing {}; env keys: {:?}",
                key,
                keys
            );
        }
        assert_eq!(env["cmd.throttle"], 0.5);
        assert_eq!(env["mag.envelope.vne_knots"], 340.0);
        assert_eq!(env["state.aileron_ok"], 1.0);
    }

    #[test]
    fn command_clamping() {
        let c = ActuatorCommand::new(2.0, -3.0, 0.5, 0.1).clamp_channels();
        assert_eq!(c.throttle, 1.0);
        assert_eq!(c.elevator, -1.0);
        assert_eq!(c.aileron, 0.5);
    }

    #[test]
    fn engine_failure_marked() {
        let state = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 140.0, 0.0);
        let mut snap = build_snapshot(
            &state,
            ActuatorCommand::NEUTRAL,
            &AircraftConfig::default(),
            &TerrainModel::sea_level(),
        );
        snap.engine_failed = Some(1);
        snap.n1[1] = 0.0;
        snap.n2[1] = 0.0;
        let env = snap.env();
        let keys = key_list(&env);
        assert!(
            env.contains_key("state.n1_R"),
            "state.n1_R missing; keys: {:?}",
            keys
        );
        assert_eq!(env["state.n1_R"], 0.0);
    }

    #[test]
    fn envelope_magic_constants_are_seeded() {
        let state = FlightState::level(Vec3::new(0.0, 0.0, 3000.0), 150.0, 0.0);
        let snap = build_snapshot(
            &state,
            ActuatorCommand::NEUTRAL,
            &AircraftConfig::default(),
            &TerrainModel::sea_level(),
        );
        let env = snap.env();
        let keys = key_list(&env);
        // The gate's standard cases reference these; without them every
        // hazard would be UNKNOWN and the gate would always block.
        for key in [
            "mag.envelope.vne_knots",
            "mag.envelope.vs_clean_knots",
            "mag.envelope.alpha_max_deg",
            "mag.envelope.nz_max",
            "mag.envelope.nz_min",
            "mag.envelope.mmo",
            "mag.envelope.max_altitude",
            "mag.envelope.min_terrain_clearance",
            "mag.engine.n1_max",
            "mag.engine.n2_max",
            "mag.engine.egt_max",
            "mag.engine.oil_pressure_min",
            "mag.engine.n1_reverse_max",
            "mag.terrain.base_elevation",
        ] {
            assert!(
                env.contains_key(key),
                "missing magic constant {}; keys: {:?}",
                key,
                keys
            );
        }
    }
}