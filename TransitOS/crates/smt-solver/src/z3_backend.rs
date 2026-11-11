//! Z3 SMT solver backend for formal collision verification.
//!
//! Encodes the collision query as an SMT-LIB2 constraint system and uses Z3
//! to find satisfying models (SAT = collision possible, UNSAT = safe).

use crate::query::{CollisionQuery, SatModel, SolverResult};
use crate::solver::SolverBackend;
use z3::ast::Real;
use z3::{SatResult, Solver};

/// Maximum f64 value that can be safely multiplied by SCALE and cast to i64.
/// i64::MAX / 1000 ≈ 9.22e15 — beyond this, the 1000× scaling overflows.
const SAFE_SCALE_LIMIT: f64 = 9.22e15;

/// Scale factor for converting f64 coordinates to Z3 rationals.
const SCALE: i64 = 1000;

/// Safely convert an f64 to a Z3 Real using string-based rational construction.
/// This avoids i64 overflow for large coordinates (e.g., UTM in meters).
fn f64_to_z3_real(value: f64) -> Real {
    if value.abs() < SAFE_SCALE_LIMIT {
        // Fast path: use integer arithmetic
        Real::from_rational((value * SCALE as f64) as i64, SCALE)
    } else {
        // Slow path: use string representation to avoid overflow
        // Format as "numerator / denominator" with enough precision
        let s = format!("{:.6}", value);
        Real::from_rational_str(&s, "1").unwrap_or_else(|| {
            // Fallback: clamp to safe range
            let clamped = value.clamp(-SAFE_SCALE_LIMIT, SAFE_SCALE_LIMIT);
            Real::from_rational((clamped * SCALE as f64) as i64, SCALE)
        })
    }
}

pub struct Z3Backend;

impl Z3Backend {
    pub fn new() -> Self {
        Self
    }

    fn check_sat(query: &CollisionQuery) -> SolverResult {
        let solver = Solver::new();

        // Create variables (z3 0.20.2 uses thread-local context)
        let t = Real::new_const("t");
        let obs_x = Real::new_const("obs_x");
        let obs_y = Real::new_const("obs_y");
        let obs_vx = Real::new_const("obs_vx");
        let obs_vy = Real::new_const("obs_vy");

        // Time constraint: 0 <= t <= horizon
        solver.assert(t.ge(Real::from_rational(0, 1)));
        solver.assert(t.le(f64_to_z3_real(query.time_horizon)));

        // Ego trajectory constants
        let cos_h = query.ego_heading.cos();
        let sin_h = query.ego_heading.sin();

        let ex = f64_to_z3_real(query.ego_position.0);
        let ey = f64_to_z3_real(query.ego_position.1);
        let ev = f64_to_z3_real(query.ego_velocity);

        let ev_cos = f64_to_z3_real(cos_h);
        let ev_sin = f64_to_z3_real(sin_h);

        // ego_x = ex + ev*cos(h)*t
        let ev_cos_ev = Real::mul(&[&ev_cos, &ev]);
        let ego_x = Real::add(&[&ex, &Real::mul(&[&ev_cos_ev, &t])]);
        // ego_y = ey + ev*sin(h)*t
        let ev_sin_ev = Real::mul(&[&ev_sin, &ev]);
        let ego_y = Real::add(&[&ey, &Real::mul(&[&ev_sin_ev, &t])]);

        // Obstacle position at time t
        let obs_pos_x = Real::add(&[&obs_x, &Real::mul(&[&obs_vx, &t])]);
        let obs_pos_y = Real::add(&[&obs_y, &Real::mul(&[&obs_vy, &t])]);

        // Distance squared < (r_ego + r_obs)^2
        let dx = Real::sub(&[&ego_x, &obs_pos_x]);
        let dy = Real::sub(&[&ego_y, &obs_pos_y]);
        let dist_sq = Real::add(&[&Real::mul(&[&dx, &dx]), &Real::mul(&[&dy, &dy])]);

        let ego_r_val = query.ego_half_length.hypot(query.ego_half_width);
        let ego_r = f64_to_z3_real(ego_r_val);

        let r_sum = if let Some(obs) = query.obstacles.first() {
            let obs_r_val = obs.half_length.hypot(obs.half_width);
            let obs_r = f64_to_z3_real(obs_r_val);
            Real::add(&[&ego_r, &obs_r])
        } else {
            ego_r
        };

        solver.assert(dist_sq.lt(Real::mul(&[&r_sum, &r_sum])));

        // Add obstacle constraints
        for obs in &query.obstacles {
            solver.assert(obs_x.eq(f64_to_z3_real(obs.position.0)));
            solver.assert(obs_y.eq(f64_to_z3_real(obs.position.1)));
            solver.assert(obs_vx.eq(f64_to_z3_real(obs.velocity.0)));
            solver.assert(obs_vy.eq(f64_to_z3_real(obs.velocity.1)));
        }

        match solver.check() {
            SatResult::Sat => SolverResult::Sat,
            SatResult::Unsat => SolverResult::Unsat,
            SatResult::Unknown => SolverResult::Unknown,
        }
    }
}

impl Default for Z3Backend {
    fn default() -> Self {
        Self::new()
    }
}

impl SolverBackend for Z3Backend {
    fn check(&self, query: &CollisionQuery) -> SolverResult {
        Self::check_sat(query)
    }

    fn get_model(&self) -> Option<SatModel> {
        None
    }
}

/// Quick helper: check a query using Z3.
pub fn z3_check(query: &CollisionQuery) -> SolverResult {
    Z3Backend::check_sat(query)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::query::{CollisionQuery, Obstacle};

    #[test]
    fn z3_detects_head_on_collision() {
        let query = CollisionQuery::new(
            (0.0, 0.0),
            20.0,
            0.0,
            vec![Obstacle {
                position: (50.0, 0.0),
                velocity: (0.0, 0.0),
                half_length: 2.0,
                half_width: 1.0,
            }],
            3.0,
        );
        assert_eq!(z3_check(&query), SolverResult::Sat);
    }

    #[test]
    fn z3_clears_non_colliding() {
        let query = CollisionQuery::new(
            (0.0, 0.0),
            10.0,
            0.0,
            vec![Obstacle {
                position: (0.0, 100.0),
                velocity: (0.0, 0.0),
                half_length: 2.0,
                half_width: 1.0,
            }],
            3.0,
        );
        assert_eq!(z3_check(&query), SolverResult::Unsat);
    }

    #[test]
    fn z3_handles_large_utm_coordinates() {
        // UTM coordinates can be >1e6 meters — this must not overflow
        let query = CollisionQuery::new(
            (500000.0, 4000000.0), // typical UTM
            20.0,
            0.0,
            vec![Obstacle {
                position: (500050.0, 4000000.0),
                velocity: (0.0, 0.0),
                half_length: 2.0,
                half_width: 1.0,
            }],
            3.0,
        );
        assert_eq!(z3_check(&query), SolverResult::Sat);
    }

    #[test]
    fn z3_handles_negative_coordinates() {
        // Head-on collision with negative coordinates
        let query = CollisionQuery::new(
            (-100.0, -200.0),
            20.0,
            0.0, // heading straight along x-axis
            vec![Obstacle {
                position: (-50.0, -200.0), // 50m ahead
                velocity: (0.0, 0.0),
                half_length: 2.0,
                half_width: 1.0,
            }],
            3.0,
        );
        assert_eq!(z3_check(&query), SolverResult::Sat);
    }
}
