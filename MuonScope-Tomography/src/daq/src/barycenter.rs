pub struct TriangularBarycenter {
    pub strip_width_mm: f64,
    pub num_strips: usize,
    pub half_detector_width_mm: f64,
}

impl TriangularBarycenter {
    pub fn new(strip_width_mm: f64, num_strips: usize) -> Self {
        let half_detector_width_mm = (num_strips as f64 * strip_width_mm) / 2.0;
        Self {
            strip_width_mm,
            num_strips,
            half_detector_width_mm,
        }
    }

    pub fn calculate_hit_pos(
        &self,
        strip_idx_1: usize,
        charge_1: f64,
        strip_idx_2: Option<usize>,
        charge_2: Option<f64>,
    ) -> f64 {
        let center_1 = (strip_idx_1 as f64 + 0.5) * self.strip_width_mm;

        if let (Some(s2), Some(q2)) = (strip_idx_2, charge_2) {
            if q2 > 0.0 && (charge_1 + q2) > 0.0 {
                let center_2 = (s2 as f64 + 0.5) * self.strip_width_mm;
                let weighted_x = (charge_1 * center_1 + q2 * center_2) / (charge_1 + q2);
                return weighted_x - self.half_detector_width_mm;
            }
        }

        center_1 - self.half_detector_width_mm
    }
}
