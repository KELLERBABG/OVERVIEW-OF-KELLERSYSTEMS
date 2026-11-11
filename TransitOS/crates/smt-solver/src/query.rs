use crate::collision::{RectangleCollision, SweptPathCollision};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SolverResult {
    Sat,
    Unsat,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CollisionModel {
    /// Swept-circle geometry, suitable for passenger cars.
    #[default]
    Circle,
    /// Oriented-rectangle (Separating Axis Theorem) geometry for long vehicles.
    Rectangle,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Obstacle {
    pub position: (f64, f64),
    pub velocity: (f64, f64),
    pub half_length: f64,
    pub half_width: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollisionQuery {
    pub ego_position: (f64, f64),
    pub ego_velocity: f64,
    pub ego_heading: f64,
    pub ego_half_length: f64,
    pub ego_half_width: f64,
    pub obstacles: Vec<Obstacle>,
    pub time_horizon: f64,
    pub model: CollisionModel,
}

impl CollisionQuery {
    pub fn new(
        ego_pos: (f64, f64),
        ego_vel: f64,
        ego_heading: f64,
        obstacles: Vec<Obstacle>,
        horizon: f64,
    ) -> Self {
        Self {
            ego_position: ego_pos,
            ego_velocity: ego_vel,
            ego_heading,
            ego_half_length: 2.0,
            ego_half_width: 1.0,
            obstacles,
            time_horizon: horizon,
            model: CollisionModel::Circle,
        }
    }

    pub fn with_geometry(
        mut self,
        half_length: f64,
        half_width: f64,
        model: CollisionModel,
    ) -> Self {
        self.ego_half_length = half_length;
        self.ego_half_width = half_width;
        self.model = model;
        self
    }

    pub fn will_collide(&self) -> bool {
        match self.model {
            CollisionModel::Circle => self.swept_circle_collision(),
            CollisionModel::Rectangle => self.rectangle_collision(),
        }
    }

    fn ego_end(&self) -> (f64, f64) {
        (
            self.ego_position.0 + self.ego_velocity * self.ego_heading.cos() * self.time_horizon,
            self.ego_position.1 + self.ego_velocity * self.ego_heading.sin() * self.time_horizon,
        )
    }

    fn swept_circle_collision(&self) -> bool {
        let ego_end = self.ego_end();
        let ego_radius = self.ego_half_length.hypot(self.ego_half_width);
        for obs in &self.obstacles {
            let obs_end = (
                obs.position.0 + obs.velocity.0 * self.time_horizon,
                obs.position.1 + obs.velocity.1 * self.time_horizon,
            );
            let obs_radius = obs.half_length.hypot(obs.half_width);
            if matches!(
                SweptPathCollision::check_swept_circle(
                    self.ego_position,
                    ego_end,
                    ego_radius,
                    obs.position,
                    obs_end,
                    obs_radius,
                ),
                SolverResult::Sat
            ) {
                return true;
            }
        }
        false
    }

    fn rectangle_collision(&self) -> bool {
        let ego_end = self.ego_end();
        for obs in &self.obstacles {
            let obs_end = (
                obs.position.0 + obs.velocity.0 * self.time_horizon,
                obs.position.1 + obs.velocity.1 * self.time_horizon,
            );
            for (ego_pos, obs_pos) in [(self.ego_position, obs.position), (ego_end, obs_end)] {
                if matches!(
                    RectangleCollision::check_oriented_rectangles(
                        ego_pos,
                        (self.ego_half_length, self.ego_half_width),
                        self.ego_heading,
                        obs_pos,
                        (obs.half_length, obs.half_width),
                        0.0,
                    ),
                    SolverResult::Sat
                ) {
                    return true;
                }
            }
        }
        false
    }
}

pub struct SatModel {
    pub variable_values: Vec<(String, f64)>,
}

#[cfg(test)]
mod tests {
    use super::{CollisionModel, CollisionQuery, Obstacle};

    fn obstacle_at(x: f64, y: f64) -> Obstacle {
        Obstacle {
            position: (x, y),
            velocity: (0.0, 0.0),
            half_length: 2.0,
            half_width: 1.0,
        }
    }

    #[test]
    fn circle_model_detects_head_on_collision() {
        let query = CollisionQuery::new((0.0, 0.0), 20.0, 0.0, vec![obstacle_at(50.0, 0.0)], 3.0);
        assert!(query.will_collide());
    }

    #[test]
    fn circle_model_clears_non_colliding_obstacle() {
        let query = CollisionQuery::new((0.0, 0.0), 20.0, 0.0, vec![obstacle_at(0.0, 50.0)], 3.0);
        assert!(!query.will_collide());
    }

    #[test]
    fn rectangle_model_uses_sat_for_trucks() {
        let overlapping =
            CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![obstacle_at(4.0, 0.0)], 3.0)
                .with_geometry(7.5, 1.25, CollisionModel::Rectangle);
        assert!(overlapping.will_collide());

        let clear = CollisionQuery::new((0.0, 0.0), 0.0, 0.0, vec![obstacle_at(0.0, 50.0)], 3.0)
            .with_geometry(7.5, 1.25, CollisionModel::Rectangle);
        assert!(!clear.will_collide());
    }

    #[test]
    fn rectangle_model_detects_future_collision() {
        let query = CollisionQuery::new((0.0, 0.0), 10.0, 0.0, vec![obstacle_at(25.0, 0.0)], 3.0)
            .with_geometry(7.5, 1.25, CollisionModel::Rectangle);
        assert!(query.will_collide());
    }
}
