use nalgebra::{Matrix4, Vector4};
use crate::{PlaneHit, TrackSegment, Point3, Vector3};

pub struct KalmanTrackFitter {
    pub state: Vector4<f64>,
    pub covariance: Matrix4<f64>,
    pub chi2: f64,
    pub ndf: usize,
}

impl KalmanTrackFitter {
    pub fn new() -> Self {
        Self {
            state: Vector4::zeros(),
            covariance: Matrix4::identity() * 100.0,
            chi2: 0.0,
            ndf: 0,
        }
    }

    pub fn fit_hits(&mut self, hits: &[PlaneHit]) -> Option<TrackSegment> {
        if hits.len() < 2 {
            return None;
        }

        let h0 = &hits[0];
        let h_last = &hits[hits.len() - 1];
        let dz = h_last.z_mm - h0.z_mm;

        if dz.abs() < 1e-3 {
            return None;
        }

        let tx = (h_last.x_mm - h0.x_mm) / dz;
        let ty = (h_last.y_mm - h0.y_mm) / dz;

        self.state = Vector4::new(h0.x_mm, h0.y_mm, tx, ty);
        self.chi2 = 0.0;
        self.ndf = hits.len().saturating_sub(2);

        for h in hits {
            let pred_x = self.state[0] + self.state[2] * (h.z_mm - h0.z_mm);
            let pred_y = self.state[1] + self.state[3] * (h.z_mm - h0.z_mm);
            let sigma_pos = 0.8;

            let rx = (h.x_mm - pred_x) / sigma_pos;
            let ry = (h.y_mm - pred_y) / sigma_pos;
            self.chi2 += rx * rx + ry * ry;
        }

        let chi2_ndf = if self.ndf > 0 { self.chi2 / (self.ndf as f64) } else { 0.0 };
        if chi2_ndf > 2.5 {
            return None;
        }

        let z_dir = if dz < 0.0 { -1.0 } else { 1.0 };
        let dir_unnorm = Vector3::new(self.state[2] * z_dir, self.state[3] * z_dir, z_dir);
        let direction = dir_unnorm.normalize();
        let origin = Point3::new(self.state[0], self.state[1], h0.z_mm);

        Some(TrackSegment {
            origin_mm: origin,
            direction,
            chi2_ndf,
            timestamp_utc_ns: h0.timestamp_ps / 1000,
            hits: hits.to_vec(),
        })
    }
}
