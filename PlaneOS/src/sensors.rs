//! Taint-coded sensor stream audit.
//!
//! Implements the taint-tracking requirement from the development plan:
//! data flowing from untrusted sensors (pitot, IMU, GPS, ADS-B, weather
//! radar) to critical flight-control actuation is continuously audited.
//!
//! A [`SensorTaintGraph`] records which flight-path quantities were derived
//! from which sensors and at what taint level. The audit produces:
//!
//! * a taint level for every actuation channel,
//! * a symbolic `TaintHazard` formula that is SAT when a critical channel
//!   depends on untrusted data, which the execution gate treats as a
//!   blocker.

use crate::formula::{Expr, Formula, FormulaBuilder};
use crate::state::{ChannelStatus, TaintLevel};
use std::collections::BTreeMap;

/// Sensor identifiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum SensorId {
    PitotLeft,
    PitotRight,
    PitotCenter,
    StaticPort,
    Iru,
    Gps,
    AdsbIn,
    AdsbOut,
    WeatherRadar,
    AoAVane,
    Internal,
}

impl SensorId {
    pub const ALL: [SensorId; 11] = [
        SensorId::PitotLeft,
        SensorId::PitotRight,
        SensorId::PitotCenter,
        SensorId::StaticPort,
        SensorId::Iru,
        SensorId::Gps,
        SensorId::AdsbIn,
        SensorId::AdsbOut,
        SensorId::WeatherRadar,
        SensorId::AoAVane,
        SensorId::Internal,
    ];

    pub fn name(self) -> &'static str {
        match self {
            SensorId::PitotLeft => "pitot_left",
            SensorId::PitotRight => "pitot_right",
            SensorId::PitotCenter => "pitot_center",
            SensorId::StaticPort => "static_port",
            SensorId::Iru => "iru",
            SensorId::Gps => "gps",
            SensorId::AdsbIn => "adsb_in",
            SensorId::AdsbOut => "adsb_out",
            SensorId::WeatherRadar => "weather_radar",
            SensorId::AoAVane => "aoa_vane",
            SensorId::Internal => "internal",
        }
    }

    /// Default trust level per sensor type.
    pub fn default_taint(self) -> TaintLevel {
        match self {
            SensorId::Internal => TaintLevel::Trusted,
            SensorId::Iru | SensorId::StaticPort => TaintLevel::Suspect,
            _ => TaintLevel::Untrusted,
        }
    }
}

/// A flight-path quantity that can be tainted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Quantity {
    Airspeed,
    Altitude,
    Attitude,
    Position,
    AngleOfAttack,
    VerticalSpeed,
    Heading,
    Weather,
}

impl Quantity {
    pub fn name(self) -> &'static str {
        match self {
            Quantity::Airspeed => "airspeed",
            Quantity::Altitude => "altitude",
            Quantity::Attitude => "attitude",
            Quantity::Position => "position",
            Quantity::AngleOfAttack => "aoa",
            Quantity::VerticalSpeed => "vertical_speed",
            Quantity::Heading => "heading",
            Quantity::Weather => "weather",
        }
    }
}

/// The taint state of one quantity.
#[derive(Clone, Debug, PartialEq)]
pub struct TaintRecord {
    pub quantity: Quantity,
    pub level: TaintLevel,
    /// Contributing sensors.
    pub sources: Vec<SensorId>,
}

/// Per-sensor status.
#[derive(Clone, Debug, PartialEq)]
pub struct SensorStatus {
    pub sensor: SensorId,
    pub taint: TaintLevel,
    pub available: bool,
    pub value: Option<f64>,
}

impl SensorStatus {
    pub fn trusted(sensor: SensorId, value: f64) -> Self {
        SensorStatus {
            sensor,
            taint: TaintLevel::Trusted,
            available: true,
            value: Some(value),
        }
    }

    pub fn failing(sensor: SensorId) -> Self {
        SensorStatus {
            sensor,
            taint: sensor.default_taint(),
            available: false,
            value: None,
        }
    }
}

/// Dependencies: which sensors feed each flight-path quantity.
#[derive(Clone, Debug, PartialEq)]
pub struct SensorTaintGraph {
    /// Per-quantity taint records.
    pub records: BTreeMap<Quantity, TaintRecord>,
    /// Per-sensor status.
    pub sensors: BTreeMap<SensorId, SensorStatus>,
    /// Flag set by the audit when a critical channel depended on tainted
    /// data at the last cycle.
    pub alert_triggered: bool,
}

impl Default for SensorTaintGraph {
    fn default() -> Self {
        SensorTaintGraph {
            records: BTreeMap::new(),
            sensors: BTreeMap::new(),
            alert_triggered: false,
        }
    }
}

impl SensorTaintGraph {
    /// Build a graph from sensor statuses, inferring per-quantity taint
    /// from the contributing sensors.
    pub fn from_sensors(sensors: Vec<SensorStatus>) -> Self {
        let mut graph = SensorTaintGraph::default();
        for s in sensors {
            graph.sensors.insert(s.sensor, s);
        }
        graph.recompute();
        graph
    }

    /// Recompute the per-quantity taint records from sensor statuses.
    pub fn recompute(&mut self) {
        let mut records = BTreeMap::new();
        for q in [
            Quantity::Airspeed,
            Quantity::Altitude,
            Quantity::Attitude,
            Quantity::Position,
            Quantity::AngleOfAttack,
            Quantity::VerticalSpeed,
            Quantity::Heading,
            Quantity::Weather,
        ] {
            let contributing = self.contributing_sensors(q);
            let sources = contributing
                .iter()
                .filter_map(|s| self.sensors.get(s))
                .filter(|st| st.available)
                .map(|st| st.sensor)
                .collect::<Vec<_>>();

            let level = self.combined_taint(q);
            records.insert(
                q,
                TaintRecord {
                    quantity: q,
                    level,
                    sources,
                },
            );
        }
        self.records = records;
    }

    /// Sensors that nominally contribute to a quantity.
    fn contributing_sensors(&self, q: Quantity) -> Vec<SensorId> {
        match q {
            Quantity::Airspeed => vec![
                SensorId::PitotLeft,
                SensorId::PitotRight,
                SensorId::PitotCenter,
                SensorId::StaticPort,
            ],
            Quantity::Altitude => vec![SensorId::StaticPort, SensorId::Gps],
            Quantity::Attitude => vec![SensorId::Iru],
            Quantity::Position => vec![SensorId::Gps, SensorId::AdsbIn],
            Quantity::AngleOfAttack => vec![SensorId::AoAVane, SensorId::Iru],
            Quantity::VerticalSpeed => vec![SensorId::StaticPort, SensorId::Iru],
            Quantity::Heading => vec![SensorId::Iru, SensorId::Gps],
            Quantity::Weather => vec![SensorId::WeatherRadar],
        }
    }

    /// Combined taint level for a quantity: worst taint among available
    /// contributing sensors; if none available -> Untrusted (biased).
    fn combined_taint(&self, q: Quantity) -> TaintLevel {
        let mut worst = TaintLevel::Trusted;
        let mut any_available = false;
        for s in self.contributing_sensors(q) {
            if let Some(status) = self.sensors.get(&s) {
                if status.available {
                    any_available = true;
                    if taint_rank(status.taint) > taint_rank(worst) {
                        worst = status.taint;
                    }
                }
            }
        }
        if !any_available {
            TaintLevel::Untrusted
        } else {
            worst
        }
    }

    /// Taint level of a single sensor.
    pub fn sensor_taint(&self, s: SensorId) -> TaintLevel {
        self.sensors
            .get(&s)
            .map(|st| st.taint)
            .unwrap_or(TaintLevel::Untrusted)
    }

    /// Taint level for a quantity.
    pub fn quantity_taint(&self, q: Quantity) -> TaintLevel {
        self.records
            .get(&q)
            .map(|r| r.level)
            .unwrap_or(TaintLevel::Untrusted)
    }

    /// Is a quantity considered tainted (Suspect or worse)?
    pub fn is_tainted(&self, q: Quantity) -> bool {
        self.quantity_taint(q) != TaintLevel::Trusted
    }

    /// Audit: set alert if any critical quantity (airspeed, attitude,
    /// altitude, angle-of-attack) is untrusted.
    pub fn audit_critical(&mut self) -> bool {
        self.alert_triggered = [
            Quantity::Airspeed,
            Quantity::Altitude,
            Quantity::Attitude,
            Quantity::AngleOfAttack,
            Quantity::Position,
        ]
        .iter()
        .any(|q| self.quantity_taint(*q) == TaintLevel::Untrusted);
        self.alert_triggered
    }

    /// Map quantity taint to an actuation channel.
    ///
    /// Returns the channel index and its resulting status.
    /// Channels: 0=aileron, 1=elevator, 2=rudder, 3=throttle.
    pub fn channel_taint(&self, channel: usize) -> (ChannelStatus, TaintLevel) {
        let outcome = |lvl: TaintLevel| {
            let status = match lvl {
                TaintLevel::Untrusted => ChannelStatus::Failed,
                TaintLevel::Suspect => ChannelStatus::Degraded,
                TaintLevel::Trusted => ChannelStatus::Operational,
            };
            (status, lvl)
        };

        match channel {
            0 => outcome(self.quantity_taint(Quantity::Position)),
            1 => {
                // Elevator mainly depends on pitch/attitude + AOA.
                let att = self.quantity_taint(Quantity::Attitude);
                let aoa = self.quantity_taint(Quantity::AngleOfAttack);
                let lvl = if taint_rank(att) > taint_rank(aoa) {
                    att
                } else {
                    aoa
                };
                outcome(lvl)
            }
            2 => outcome(self.quantity_taint(Quantity::Heading)),
            3 => {
                // Throttle doesn't depend on tainted sensors in this
                // mock; keep trusted unless engine data is missing.
                (ChannelStatus::Operational, TaintLevel::Trusted)
            }
            _ => (ChannelStatus::Operational, TaintLevel::Trusted),
        }
    }

    /// Build a taint-hazard formula using the canonical environment.
    ///
    /// The formula is SAT when any critical channel is untrusted, i.e.,
    /// the corresponding `state.<channel>_ok` flag is 0 (failed), and a
    /// non-zero command is pending on that channel. This formula
    /// integrates with the execution gate.
    ///
    /// Note: non-zero is expressed as `abs(cmd) > 0` because the `ne`
    /// builder would collide with `PartialEq::ne`.
    pub fn taint_hazard_formula() -> Formula {
        let non_zero = |name: &str| {
            let cmd = Expr::cmd(name);
            Expr::Abs(Box::new(cmd)).gt(Expr::Const(0.0))
        };
        Formula::any([
            Formula::and(
                Expr::state("aileron_ok").lt(Expr::Const(0.5)),
                non_zero("aileron"),
            ),
            Formula::and(
                Expr::state("elevator_ok").lt(Expr::Const(0.5)),
                non_zero("elevator"),
            ),
            Formula::and(
                Expr::state("rudder_ok").lt(Expr::Const(0.5)),
                non_zero("rudder"),
            ),
            Formula::and(
                Expr::state("throttle_ok").lt(Expr::Const(0.5)),
                non_zero("throttle"),
            ),
        ])
    }
}

fn taint_rank(t: TaintLevel) -> u8 {
    match t {
        TaintLevel::Trusted => 0,
        TaintLevel::Suspect => 1,
        TaintLevel::Untrusted => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_trusted() -> Vec<SensorStatus> {
        SensorId::ALL
            .iter()
            .map(|id| SensorStatus::trusted(*id, 0.0))
            .collect()
    }

    #[test]
    fn trusted_graph_no_alerts() {
        let mut g = SensorTaintGraph::from_sensors(all_trusted());
        assert!(!g.audit_critical());
        assert_eq!(g.quantity_taint(Quantity::Airspeed), TaintLevel::Trusted);
    }

    #[test]
    fn failed_pitots_taint_airspeed() {
        let mut sensors = all_trusted();
        // Fail all pitot + static.
        for id in [
            SensorId::PitotLeft,
            SensorId::PitotRight,
            SensorId::PitotCenter,
            SensorId::StaticPort,
        ] {
            sensors.retain(|s| s.sensor != id);
            sensors.push(SensorStatus::failing(id));
        }
        let mut g = SensorTaintGraph::from_sensors(sensors);
        assert_eq!(
            g.quantity_taint(Quantity::Airspeed),
            TaintLevel::Untrusted
        );
        assert!(g.audit_critical());
    }

    #[test]
    fn missing_imu_degrades_elevator() {
        let mut sensors = all_trusted();
        sensors.retain(|s| s.sensor != SensorId::Iru);
        sensors.push(SensorStatus::failing(SensorId::Iru));
        let g = SensorTaintGraph::from_sensors(sensors);
        let (status, lvl) = g.channel_taint(1);
        assert_eq!(lvl, TaintLevel::Untrusted);
        assert_eq!(status, ChannelStatus::Failed);
    }

    #[test]
    fn taint_formula_sat_on_failed_elevator_cmd() {
        use crate::formula::VarNS;
        use crate::solver::{namespaced_env, SMTBackend, StubSolver};
        let f = SensorTaintGraph::taint_hazard_formula();
        let mut env = namespaced_env(
            VarNS::State,
            &[
                ("aileron_ok", 1.0),
                ("elevator_ok", 0.0),
                ("rudder_ok", 1.0),
                ("throttle_ok", 1.0),
            ],
        );
        env.insert("cmd.elevator".to_string(), 0.3);
        assert!(StubSolver::new().check(&f, &env).is_sat());
    }

    #[test]
    fn taint_formula_unsat_on_zero_cmd() {
        use crate::formula::VarNS;
        use crate::solver::{namespaced_env, SMTBackend, StubSolver};
        let f = SensorTaintGraph::taint_hazard_formula();
        let mut env = namespaced_env(
            VarNS::State,
            &[
                ("aileron_ok", 1.0),
                ("elevator_ok", 0.0),
                ("rudder_ok", 1.0),
                ("throttle_ok", 1.0),
            ],
        );
        // Failed elevator but zero command -> safe.
        env.insert("cmd.elevator".to_string(), 0.0);
        assert!(StubSolver::new().check(&f, &env).is_unsat());
    }
}