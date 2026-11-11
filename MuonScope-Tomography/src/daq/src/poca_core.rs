//! Multithreaded Point of Closest Approach (PoCA) Solver

use crate::{CorrelatedEvent, Point3, Vector3};

/// Calculates PoCA point, miss distance, and 3D deflection angle between two rays.
pub fn calculate_poca_intersection(
    p_in: &Point3,
    d_in: &Vector3,
    p_out: &Point3,
    d_out: &Vector3,
) -> (Point3, f64, f64) {
    let w0 = p_in - p_out;
    let a = d_in.dot(d_in);
    let b = d_in.dot(d_out);
    let c = d_out.dot(d_out);
    let d = d_in.dot(&w0);
    let e = d_out.dot(&w0);

    let denom = a * c - b * b;
    if denom.abs() < 1e-9 {
        let mid = (p_in.coords + p_out.coords) * 0.5;
        return (Point3::from(mid), 0.0, 0.0);
    }

    let t_min = (b * e - c * d) / denom;
    let s_min = (a * e - b * d) / denom;

    let pt_in = p_in + d_in * t_min;
    let pt_out = p_out + d_out * s_min;

    let poca = Point3::from((pt_in.coords + pt_out.coords) * 0.5);
    let miss_dist = (pt_in - pt_out).norm();

    let cos_th = (d_in.dot(d_out) / (d_in.norm() * d_out.norm())).clamp(-1.0, 1.0);
    let theta = cos_th.acos();

    (poca, miss_dist, theta)
}

/// In-memory 3D Voxel Grid Accumulator for live streaming telemetry
pub struct LiveVoxelAccumulator {
    nx: usize,
    ny: usize,
    nz: usize,
    bounds_min: Point3,
    bounds_max: Point3,
    variance_sum: Vec<f32>,
    hit_counts: Vec<u32>,
}

impl LiveVoxelAccumulator {
    pub fn new(
        grid_size: (usize, usize, usize),
        bounds_min: Point3,
        bounds_max: Point3,
    ) -> Self {
        let total = grid_size.0 * grid_size.1 * grid_size.2;
        Self {
            nx: grid_size.0,
            ny: grid_size.1,
            nz: grid_size.2,
            bounds_min,
            bounds_max,
            variance_sum: vec![0.0; total],
            hit_counts: vec![0; total],
        }
    }

    pub fn deposit_event(&mut self, event: &CorrelatedEvent) -> bool {
        if event.miss_distance_mm > 30.0 || event.scattering_angle_rad < 0.001 {
            return false;
        }

        let p = &event.poca_point;
        if p.x < self.bounds_min.x || p.x >= self.bounds_max.x
            || p.y < self.bounds_min.y || p.y >= self.bounds_max.y
            || p.z < self.bounds_min.z || p.z >= self.bounds_max.z
        {
            return false;
        }

        let gx = (p.x - self.bounds_min.x) / (self.bounds_max.x - self.bounds_min.x) * (self.nx as f64);
        let gy = (p.y - self.bounds_min.y) / (self.bounds_max.y - self.bounds_min.y) * (self.ny as f64);
        let gz = (p.z - self.bounds_min.z) / (self.bounds_max.z - self.bounds_min.z) * (self.nz as f64);

        let ix = gx as usize;
        let iy = gy as usize;
        let iz = gz as usize;

        if ix < self.nx && iy < self.ny && iz < self.nz {
            let idx = ix * (self.ny * self.nz) + iy * self.nz + iz;
            let score = (event.scattering_angle_rad * event.scattering_angle_rad) as f32;
            self.variance_sum[idx] += score;
            self.hit_counts[idx] += 1;
            return true;
        }

        false
    }
}
