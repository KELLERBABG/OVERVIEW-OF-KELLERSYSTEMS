use transit_state_machine::params::{RoadSurface, TireCondition, VehicleParams};
use transit_state_machine::state::VehicleState;

#[derive(Debug)]
pub struct EdgeCase {
    pub description: &'static str,
    pub condition: EdgeCondition,
    pub severity: f64,
}

#[derive(Debug)]
pub enum EdgeCondition {
    Aquaplaning {
        speed: f64,
        water_depth_mm: f64,
    },
    BlackIce {
        speed: f64,
        temperature_c: f64,
    },
    TireBlowout {
        speed: f64,
        failed_axle: &'static str,
    },
    PanicBrake {
        speed: f64,
        steer_input: f64,
    },
    Oversteer {
        speed: f64,
        steering_rate: f64,
    },
    Understeer {
        speed: f64,
        curvature_radius: f64,
    },
}

impl EdgeCase {
    pub fn check_aquaplaning(state: &VehicleState, water_depth_mm: f64) -> Option<EdgeCase> {
        let v = state.dynamics.velocity;
        let aquaplaning_speed = 50.0 - water_depth_mm * 2.0;
        if v >= aquaplaning_speed && state.road == RoadSurface::Wet {
            Some(EdgeCase {
                description: "Aquaplaning risk",
                condition: EdgeCondition::Aquaplaning {
                    speed: v,
                    water_depth_mm,
                },
                severity: ((v - aquaplaning_speed) / 20.0).clamp(0.0, 1.0),
            })
        } else {
            None
        }
    }

    pub fn check_tire_blowout(
        state: &VehicleState,
        params: &VehicleParams,
        cond: &TireCondition,
    ) -> Option<EdgeCase> {
        let v = state.dynamics.velocity;
        match cond {
            TireCondition::Blown if v > 10.0 => Some(EdgeCase {
                description: "Tire blowout at speed",
                condition: EdgeCondition::TireBlowout {
                    speed: v,
                    failed_axle: if params.front_wt_frac > 0.5 {
                        "front"
                    } else {
                        "rear"
                    },
                },
                severity: (v / 50.0).clamp(0.0, 1.0),
            }),
            _ => None,
        }
    }

    pub fn check_black_ice(state: &VehicleState, temperature_c: f64) -> Option<EdgeCase> {
        let v = state.dynamics.velocity;
        if temperature_c < 0.0 && v > 15.0 && state.road == RoadSurface::Dry {
            Some(EdgeCase {
                description: "Potential black ice on dry-looking road",
                condition: EdgeCondition::BlackIce {
                    speed: v,
                    temperature_c,
                },
                severity: (v / 40.0).clamp(0.0, 1.0),
            })
        } else {
            None
        }
    }

    pub fn check_oversteer(state: &VehicleState) -> Option<EdgeCase> {
        let slip = state.dynamics.side_slip;
        if slip.abs() > 0.15 && state.dynamics.velocity > 10.0 {
            Some(EdgeCase {
                description: "Vehicle oversteering",
                condition: EdgeCondition::Oversteer {
                    speed: state.dynamics.velocity,
                    steering_rate: state.dynamics.yaw_rate,
                },
                severity: (slip.abs() / 0.4).clamp(0.0, 1.0),
            })
        } else {
            None
        }
    }

    pub fn check_understeer(state: &VehicleState) -> Option<EdgeCase> {
        let steering = state.actuators.steering_angle.abs();
        let curvature = state.curvature_radius();
        if steering > 0.15 && curvature > 200.0 && state.dynamics.velocity > 5.0 {
            Some(EdgeCase {
                description: "Vehicle understeering",
                condition: EdgeCondition::Understeer {
                    speed: state.dynamics.velocity,
                    curvature_radius: curvature,
                },
                severity: (steering / 0.7).clamp(0.0, 1.0),
            })
        } else {
            None
        }
    }
}

/// Detects edge cases that can be evaluated from the vehicle state alone
/// (no external environmental sensors required).
pub fn detect_state_edge_cases(state: &VehicleState) -> Vec<EdgeCase> {
    let mut cases = Vec::new();
    if let Some(case) = EdgeCase::check_oversteer(state) {
        cases.push(case);
    }
    if let Some(case) = EdgeCase::check_understeer(state) {
        cases.push(case);
    }
    cases
}

#[cfg(test)]
mod tests {
    use super::*;
    use transit_state_machine::params::{RoadSurface, TireCondition, VehicleParams};

    #[test]
    fn detects_oversteer_from_side_slip() {
        let mut state = VehicleState::default();
        state.dynamics.velocity = 15.0;
        state.dynamics.side_slip = 0.2;
        assert!(EdgeCase::check_oversteer(&state).is_some());
    }

    #[test]
    fn detects_understeer_from_steering_without_turning() {
        let mut state = VehicleState::default();
        state.dynamics.velocity = 10.0;
        state.dynamics.yaw_rate = 0.0;
        state.actuators.steering_angle = 0.3;
        assert!(EdgeCase::check_understeer(&state).is_some());
    }

    #[test]
    fn detects_aquaplaning_on_wet_road() {
        let mut state = VehicleState::default();
        state.dynamics.velocity = 50.0;
        state.road = RoadSurface::Wet;
        assert!(EdgeCase::check_aquaplaning(&state, 2.0).is_some());
    }

    #[test]
    fn detects_black_ice_in_freezing_temperature() {
        let mut state = VehicleState::default();
        state.dynamics.velocity = 20.0;
        state.road = RoadSurface::Dry;
        assert!(EdgeCase::check_black_ice(&state, -2.0).is_some());
    }

    #[test]
    fn detects_tire_blowout_at_speed() {
        let mut state = VehicleState::default();
        state.dynamics.velocity = 20.0;
        assert!(EdgeCase::check_tire_blowout(
            &state,
            &VehicleParams::default(),
            &TireCondition::Blown,
        )
        .is_some());
    }

    #[test]
    fn state_only_detection_aggregates_cases() {
        let mut state = VehicleState::default();
        state.dynamics.velocity = 15.0;
        state.dynamics.side_slip = 0.2;
        assert!(!detect_state_edge_cases(&state).is_empty());
    }
}
