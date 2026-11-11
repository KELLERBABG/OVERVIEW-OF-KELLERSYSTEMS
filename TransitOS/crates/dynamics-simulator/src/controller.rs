use transit_state_machine::state::VehicleState;

pub struct PidController {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    integral: f64,
    prev_error: f64,
}

impl PidController {
    pub fn new(kp: f64, ki: f64, kd: f64) -> Self {
        Self {
            kp,
            ki,
            kd,
            integral: 0.0,
            prev_error: 0.0,
        }
    }

    pub fn update(&mut self, error: f64, dt: f64) -> f64 {
        self.integral += error * dt;
        let derivative = (error - self.prev_error) / dt.max(1e-6);
        self.prev_error = error;
        self.kp * error + self.ki * self.integral + self.kd * derivative
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = 0.0;
    }
}

pub struct LaneKeepController {
    pub steer_pid: PidController,
    pub target_speed: f64,
    pub speed_pid: PidController,
}

impl LaneKeepController {
    pub fn new(target_speed: f64) -> Self {
        Self {
            steer_pid: PidController::new(2.0, 0.0, 0.5),
            target_speed,
            speed_pid: PidController::new(0.5, 0.1, 0.05),
        }
    }

    pub fn control(&mut self, state: &VehicleState, lane_offset: f64, dt: f64) -> (f64, f64, f64) {
        let steering = self.steer_pid.update(lane_offset, dt).clamp(-0.5, 0.5);

        let speed_error = self.target_speed - state.dynamics.velocity;
        let throttle = self.speed_pid.update(speed_error, dt).clamp(0.0, 1.0);

        (steering, throttle, 0.0)
    }
}

pub struct CruiseController {
    pub target_speed: f64,
    pub pid: PidController,
    pub following_gap: f64,
}

impl CruiseController {
    pub fn new(target_speed: f64) -> Self {
        Self {
            target_speed,
            pid: PidController::new(1.0, 0.2, 0.1),
            following_gap: 2.0,
        }
    }

    pub fn control(
        &mut self,
        state: &VehicleState,
        lead_distance: Option<f64>,
        dt: f64,
    ) -> (f64, f64, f64) {
        let target = match lead_distance {
            Some(dist) if dist < self.following_gap * state.dynamics.velocity => {
                (dist / self.following_gap).min(self.target_speed)
            }
            _ => self.target_speed,
        };

        let error = target - state.dynamics.velocity;
        let throttle = self.pid.update(error, dt).clamp(-1.0, 1.0);

        if throttle >= 0.0 {
            (0.0, throttle, 0.0)
        } else {
            (0.0, 0.0, (-throttle).min(1.0))
        }
    }
}
