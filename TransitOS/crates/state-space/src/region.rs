use crate::explorer::ExplorerConfig;

#[derive(Debug, Clone)]
pub struct StateRegion {
    pub velocity_range: (f64, f64),
    pub acceleration_range: (f64, f64),
    pub steering_range: (f64, f64),
    pub brake_range: (f64, f64),
}

impl StateRegion {
    pub fn nominal_driving() -> Self {
        Self {
            velocity_range: (0.0, 50.0),
            acceleration_range: (-5.0, 5.0),
            steering_range: (-0.5, 0.5),
            brake_range: (0.0, 1.0),
        }
    }

    pub fn extreme_conditions() -> Self {
        Self {
            velocity_range: (0.0, 80.0),
            acceleration_range: (-15.0, 10.0),
            steering_range: (-0.7, 0.7),
            brake_range: (0.0, 1.0),
        }
    }

    pub fn highway() -> Self {
        Self {
            velocity_range: (20.0, 60.0),
            acceleration_range: (-3.0, 2.0),
            steering_range: (-0.2, 0.2),
            brake_range: (0.0, 0.6),
        }
    }

    pub fn parking() -> Self {
        Self {
            velocity_range: (-3.0, 3.0),
            acceleration_range: (-2.0, 2.0),
            steering_range: (-0.7, 0.7),
            brake_range: (0.0, 0.5),
        }
    }

    pub fn discretize(&self, config: &ExplorerConfig) -> Vec<(f64, f64, f64, f64)> {
        let mut points = Vec::new();
        let v_step = if config.v_bins > 0 {
            (self.velocity_range.1 - self.velocity_range.0) / config.v_bins as f64
        } else {
            0.0
        };
        let a_step = if config.a_bins > 0 {
            (self.acceleration_range.1 - self.acceleration_range.0) / config.a_bins as f64
        } else {
            0.0
        };
        let s_step = if config.s_bins > 0 {
            (self.steering_range.1 - self.steering_range.0) / config.s_bins as f64
        } else {
            0.0
        };
        let b_step = if config.b_bins > 0 {
            (self.brake_range.1 - self.brake_range.0) / config.b_bins as f64
        } else {
            0.0
        };

        for vi in 0..config.v_bins {
            for ai in 0..config.a_bins {
                for si in 0..config.s_bins {
                    for bi in 0..config.b_bins {
                        points.push((
                            self.velocity_range.0 + vi as f64 * v_step,
                            self.acceleration_range.0 + ai as f64 * a_step,
                            self.steering_range.0 + si as f64 * s_step,
                            self.brake_range.0 + bi as f64 * b_step,
                        ));
                    }
                }
            }
        }
        points
    }
}
