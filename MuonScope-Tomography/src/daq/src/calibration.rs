pub struct ChannelDeskewCalibrator {
    pub num_channels: usize,
    pub delay_taps: Vec<u8>,
    pub target_skew_ps: f64,
    pub residual_skews_ps: Vec<f64>,
}

impl ChannelDeskewCalibrator {
    pub fn new(num_channels: usize) -> Self {
        Self {
            num_channels,
            delay_taps: vec![0; num_channels],
            target_skew_ps: 30.0,
            residual_skews_ps: vec![0.0; num_channels],
        }
    }

    pub fn compute_optimal_taps(&mut self, measured_delays_ps: &[f64]) -> &[u8] {
        if measured_delays_ps.len() < self.num_channels {
            return &self.delay_taps;
        }

        let max_delay = measured_delays_ps
            .iter()
            .take(self.num_channels)
            .cloned()
            .fold(f64::NEG_INFINITY, f64::max);

        let tap_resolution_ps = 78.0; // 78 ps per Artix-7 IDELAYE2 tap

        for i in 0..self.num_channels {
            let delta = max_delay - measured_delays_ps[i];
            let taps = (delta / tap_resolution_ps).round().clamp(0.0, 31.0) as u8;
            self.delay_taps[i] = taps;

            let residual = delta - (taps as f64 * tap_resolution_ps);
            self.residual_skews_ps[i] = residual;
        }

        &self.delay_taps
    }

    pub fn is_calibration_passed(&self) -> bool {
        self.residual_skews_ps
            .iter()
            .all(|&skew| skew.abs() <= self.target_skew_ps)
    }
}
