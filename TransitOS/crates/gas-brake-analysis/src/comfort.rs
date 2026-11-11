pub struct ComfortLimits;

impl ComfortLimits {
    pub fn max_comfortable_accel() -> f64 {
        2.5
    }

    pub fn max_comfortable_decel() -> f64 {
        3.5
    }

    pub fn max_comfortable_jerk() -> f64 {
        2.0
    }

    pub fn max_comfortable_lateral() -> f64 {
        2.0
    }

    pub fn is_comfortable(accel: f64, jerk: f64, lateral: f64) -> bool {
        accel.abs() <= Self::max_comfortable_accel()
            && jerk.abs() <= Self::max_comfortable_jerk()
            && lateral.abs() <= Self::max_comfortable_lateral()
    }
}
