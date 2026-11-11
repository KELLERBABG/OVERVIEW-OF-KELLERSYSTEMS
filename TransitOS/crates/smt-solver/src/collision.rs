use crate::query::{Obstacle, SolverResult};

pub struct SweptPathCollision;

impl SweptPathCollision {
    pub fn check_swept_circle(
        ego_start: (f64, f64),
        ego_end: (f64, f64),
        ego_radius: f64,
        obs_start: (f64, f64),
        obs_end: (f64, f64),
        obs_radius: f64,
    ) -> SolverResult {
        let r_sum = ego_radius + obs_radius;

        let dx = ego_end.0 - ego_start.0;
        let dy = ego_end.1 - ego_start.1;
        let dox = obs_end.0 - obs_start.0;
        let doy = obs_end.1 - obs_start.1;

        let pdx = dx - dox;
        let pdy = dy - doy;

        let sx = ego_start.0 - obs_start.0;
        let sy = ego_start.1 - obs_start.1;

        let a = pdx * pdx + pdy * pdy;
        let b = 2.0 * (sx * pdx + sy * pdy);
        let c = sx * sx + sy * sy - r_sum * r_sum;

        if a < 1e-10 {
            return if c <= 0.0 {
                SolverResult::Sat
            } else {
                SolverResult::Unsat
            };
        }

        let disc = b * b - 4.0 * a * c;
        if disc < 0.0 {
            return SolverResult::Unsat;
        }

        let sqrt_disc = disc.sqrt();
        let t1 = (-b - sqrt_disc) / (2.0 * a);
        let t2 = (-b + sqrt_disc) / (2.0 * a);

        if (0.0..=1.0).contains(&t1) || (0.0..=1.0).contains(&t2) || (t1 < 0.0 && t2 > 1.0) {
            SolverResult::Sat
        } else {
            SolverResult::Unsat
        }
    }

    pub fn check_multiple(
        ego_path: &[(f64, f64)],
        obstacles: &[Obstacle],
        ego_radius: f64,
    ) -> Vec<bool> {
        let mut results = Vec::new();
        let start = ego_path.first().copied().unwrap_or((0.0, 0.0));
        let end = ego_path.last().copied().unwrap_or((0.0, 0.0));
        for obs in obstacles {
            let obs_end = (
                obs.position.0 + obs.velocity.0,
                obs.position.1 + obs.velocity.1,
            );
            let result = Self::check_swept_circle(
                start,
                end,
                ego_radius,
                obs.position,
                obs_end,
                obs.half_length.hypot(obs.half_width),
            );
            results.push(matches!(result, SolverResult::Sat));
        }
        results
    }
}

pub struct RectangleCollision;

impl RectangleCollision {
    pub fn check_oriented_rectangles(
        ego_center: (f64, f64),
        ego_half: (f64, f64),
        ego_heading: f64,
        obs_center: (f64, f64),
        obs_half: (f64, f64),
        obs_heading: f64,
    ) -> SolverResult {
        let corners_ego = Self::oriented_corners(ego_center, ego_half, ego_heading);
        let corners_obs = Self::oriented_corners(obs_center, obs_half, obs_heading);

        if Self::sat_overlap(&corners_ego, &corners_obs) {
            SolverResult::Sat
        } else {
            SolverResult::Unsat
        }
    }

    fn oriented_corners(center: (f64, f64), half: (f64, f64), heading: f64) -> Vec<(f64, f64)> {
        let (cx, cy) = center;
        let (hw, hh) = half;
        let cos_h = heading.cos();
        let sin_h = heading.sin();
        let offsets = [(-hw, -hh), (hw, -hh), (hw, hh), (-hw, hh)];
        offsets
            .iter()
            .map(|&(dx, dy)| {
                let rx = dx * cos_h - dy * sin_h;
                let ry = dx * sin_h + dy * cos_h;
                (cx + rx, cy + ry)
            })
            .collect()
    }

    fn project(vertices: &[(f64, f64)], axis: (f64, f64)) -> (f64, f64) {
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        let len = (axis.0 * axis.0 + axis.1 * axis.1).sqrt();
        if len < 1e-10 {
            return (0.0, 0.0);
        }
        let ax = axis.0 / len;
        let ay = axis.1 / len;
        for &(vx, vy) in vertices {
            let proj = vx * ax + vy * ay;
            min = min.min(proj);
            max = max.max(proj);
        }
        (min, max)
    }

    fn sat_overlap(rect_a: &[(f64, f64)], rect_b: &[(f64, f64)]) -> bool {
        let axes = Self::get_axes(rect_a, rect_b);
        for axis in axes {
            let (a_min, a_max) = Self::project(rect_a, axis);
            let (b_min, b_max) = Self::project(rect_b, axis);
            if a_max < b_min || b_max < a_min {
                return false;
            }
        }
        true
    }

    fn get_axes(rect_a: &[(f64, f64)], rect_b: &[(f64, f64)]) -> Vec<(f64, f64)> {
        let mut axes = Vec::new();
        for i in 0..4 {
            let j = (i + 1) % 4;
            let dx = rect_a[j].0 - rect_a[i].0;
            let dy = rect_a[j].1 - rect_a[i].1;
            axes.push((-dy, dx));
            let dx = rect_b[j].0 - rect_b[i].0;
            let dy = rect_b[j].1 - rect_b[i].1;
            axes.push((-dy, dx));
        }
        axes
    }
}
