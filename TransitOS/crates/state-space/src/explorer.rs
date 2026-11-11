use crate::region::StateRegion;
use rand::Rng;

pub enum ExplorationStrategy {
    Grid,
    Random { samples: u32 },
    Border,
    MonteCarlo { samples: u32 },
}

pub struct ExplorerConfig {
    pub strategy: ExplorationStrategy,
    pub v_bins: u32,
    pub a_bins: u32,
    pub s_bins: u32,
    pub b_bins: u32,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            strategy: ExplorationStrategy::Grid,
            v_bins: 10,
            a_bins: 5,
            s_bins: 5,
            b_bins: 3,
        }
    }
}

pub fn explore_region(region: &StateRegion, config: &ExplorerConfig) -> Vec<(f64, f64, f64, f64)> {
    match &config.strategy {
        ExplorationStrategy::Grid => region.discretize(config),
        ExplorationStrategy::Random { samples } => {
            let mut rng = rand::thread_rng();
            let mut points = Vec::with_capacity(*samples as usize);
            for _ in 0..*samples {
                points.push((
                    rng.gen_range(region.velocity_range.0..region.velocity_range.1),
                    rng.gen_range(region.acceleration_range.0..region.acceleration_range.1),
                    rng.gen_range(region.steering_range.0..region.steering_range.1),
                    rng.gen_range(region.brake_range.0..region.brake_range.1),
                ));
            }
            points
        }
        ExplorationStrategy::Border => {
            let mut points = region.discretize(config);
            let inner = StateRegion {
                velocity_range: (
                    region.velocity_range.0 + region.velocity_range.1 * 0.1,
                    region.velocity_range.1 - region.velocity_range.1 * 0.1,
                ),
                acceleration_range: (
                    region.acceleration_range.0 + region.acceleration_range.1 * 0.1,
                    region.acceleration_range.1 - region.acceleration_range.1 * 0.1,
                ),
                steering_range: (
                    region.steering_range.0 + region.steering_range.1 * 0.1,
                    region.steering_range.1 - region.steering_range.1 * 0.1,
                ),
                brake_range: (
                    region.brake_range.0 + region.brake_range.1 * 0.1,
                    region.brake_range.1 - region.brake_range.1 * 0.1,
                ),
            };
            let inner_points = inner.discretize(config);
            points.retain(|p| !inner_points.contains(p));
            points
        }
        ExplorationStrategy::MonteCarlo { samples } => {
            let mut rng = rand::thread_rng();
            let mut points = region.discretize(config);
            for _ in 0..*samples {
                let idx = rng.gen_range(0..points.len());
                points[idx] = (
                    rng.gen_range(region.velocity_range.0..region.velocity_range.1),
                    rng.gen_range(region.acceleration_range.0..region.acceleration_range.1),
                    rng.gen_range(region.steering_range.0..region.steering_range.1),
                    rng.gen_range(region.brake_range.0..region.brake_range.1),
                );
            }
            points
        }
    }
}
