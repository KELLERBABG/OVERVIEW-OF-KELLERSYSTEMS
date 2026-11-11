use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum VehicleClass {
    PassengerCar,
    HeavyTruck,
    ArticulatedTruck,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct AirBrakeParams {
    pub build_up_time_s: f64,
    pub release_time_s: f64,
    pub max_pressure_bar: f64,
    pub line_delay_s: f64,
}

impl Default for AirBrakeParams {
    fn default() -> Self {
        Self {
            build_up_time_s: 0.35,
            release_time_s: 0.20,
            max_pressure_bar: 8.0,
            line_delay_s: 0.05,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct ActuatorDelay {
    pub steering_lag_s: f64,
    pub brake_lag_s: f64,
    pub throttle_lag_s: f64,
    pub can_bus_latency_s: f64,
}

impl Default for ActuatorDelay {
    fn default() -> Self {
        Self {
            steering_lag_s: 0.02,
            brake_lag_s: 0.01,
            throttle_lag_s: 0.01,
            can_bus_latency_s: 0.005,
        }
    }
}

impl ActuatorDelay {
    pub fn truck_default() -> Self {
        Self {
            steering_lag_s: 0.05,
            brake_lag_s: 0.35,
            throttle_lag_s: 0.05,
            can_bus_latency_s: 0.010,
        }
    }

    pub fn total_brake_lag(&self) -> f64 {
        self.brake_lag_s + self.can_bus_latency_s
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct VehicleParams {
    pub class: VehicleClass,
    pub mass: f64,
    pub wheelbase: f64,
    pub track_width: f64,
    pub cog_height: f64,
    pub front_wt_frac: f64,
    pub max_steering: f64,
    pub max_brake_force: f64,
    pub max_throttle_force: f64,
    pub air_brake: AirBrakeParams,
    pub actuator_delay: ActuatorDelay,
    pub trailer_wheelbase: f64,
    pub hitch_to_rear_axle: f64,
}

impl Default for VehicleParams {
    fn default() -> Self {
        VehicleParams::passenger_car()
    }
}

impl VehicleParams {
    pub fn update_mass(&mut self, estimated_mass: f64) {
        self.mass = estimated_mass;
    }

    pub fn update_cog_height(&mut self, estimated_cog: f64) {
        self.cog_height = estimated_cog;
    }

    pub fn estimate_from_payload(payload_kg: f64, is_loaded: bool) -> (f64, f64) {
        let base_mass = 15000.0;
        let base_cog = 1.5;
        if is_loaded {
            let total_mass = base_mass + payload_kg;
            let cog = (base_mass * base_cog + payload_kg * 1.8) / total_mass;
            (total_mass, cog)
        } else {
            let total_mass = base_mass + payload_kg * 0.3;
            let cog = (base_mass * base_cog + payload_kg * 0.3 * 1.0) / total_mass;
            (total_mass, cog)
        }
    }

    pub fn passenger_car() -> Self {
        Self {
            class: VehicleClass::PassengerCar,
            mass: 1500.0,
            wheelbase: 2.8,
            track_width: 1.8,
            cog_height: 0.5,
            front_wt_frac: 0.55,
            max_steering: 0.7,
            max_brake_force: 12000.0,
            max_throttle_force: 6000.0,
            air_brake: AirBrakeParams::default(),
            actuator_delay: ActuatorDelay::default(),
            trailer_wheelbase: 0.0,
            hitch_to_rear_axle: 0.0,
        }
    }

    pub fn heavy_truck() -> Self {
        Self {
            class: VehicleClass::HeavyTruck,
            mass: 15000.0,
            wheelbase: 6.0,
            track_width: 2.5,
            cog_height: 1.5,
            front_wt_frac: 0.45,
            max_steering: 0.5,
            max_brake_force: 80000.0,
            max_throttle_force: 30000.0,
            air_brake: AirBrakeParams::default(),
            actuator_delay: ActuatorDelay::truck_default(),
            trailer_wheelbase: 0.0,
            hitch_to_rear_axle: 0.0,
        }
    }

    pub fn articulated_truck() -> Self {
        Self {
            class: VehicleClass::ArticulatedTruck,
            mass: 25000.0,
            wheelbase: 6.0,
            track_width: 2.5,
            cog_height: 1.5,
            front_wt_frac: 0.40,
            max_steering: 0.5,
            max_brake_force: 120000.0,
            max_throttle_force: 35000.0,
            air_brake: AirBrakeParams::default(),
            actuator_delay: ActuatorDelay::truck_default(),
            trailer_wheelbase: 9.0,
            hitch_to_rear_axle: 1.5,
        }
    }

    pub fn rollover_threshold(&self) -> f64 {
        let g = 9.81;
        let t = self.track_width;
        let h = self.cog_height;
        if h <= 0.0 {
            return f64::INFINITY;
        }
        (t / (2.0 * h)) * g
    }

    pub fn yaw_inertia(&self) -> f64 {
        self.mass * self.wheelbase * self.wheelbase / 12.0
    }

    pub fn is_articulated(&self) -> bool {
        matches!(self.class, VehicleClass::ArticulatedTruck)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub struct CogFilter {
    pub alpha: f64,
    pub estimated_cog: f64,
    pub estimated_mass: f64,
    pub samples: u64,
}

impl CogFilter {
    pub fn new(alpha: f64, initial_cog: f64, initial_mass: f64) -> Self {
        Self {
            alpha: alpha.clamp(0.01, 0.99),
            estimated_cog: initial_cog,
            estimated_mass: initial_mass,
            samples: 1,
        }
    }

    pub fn update(&mut self, raw_cog: f64, raw_mass: f64) -> (f64, f64) {
        self.samples += 1;
        let a = if self.samples < 5 {
            1.0 / self.samples as f64
        } else {
            self.alpha
        };
        self.estimated_cog = (1.0 - a) * self.estimated_cog + a * raw_cog;
        self.estimated_mass = (1.0 - a) * self.estimated_mass + a * raw_mass;
        (self.estimated_cog, self.estimated_mass)
    }

    pub fn reset(&mut self, cog: f64, mass: f64) {
        self.estimated_cog = cog;
        self.estimated_mass = mass;
        self.samples = 1;
    }
}

impl Default for CogFilter {
    fn default() -> Self {
        Self::new(0.15, 1.5, 15000.0)
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq)]
pub enum RoadSurface {
    #[default]
    Dry,
    Wet,
    Snow,
    Ice,
}

impl RoadSurface {
    pub fn friction_coeff(&self) -> f64 {
        match self {
            RoadSurface::Dry => 1.0,
            RoadSurface::Wet => 0.7,
            RoadSurface::Snow => 0.3,
            RoadSurface::Ice => 0.1,
        }
    }

    pub fn max_deceleration(&self) -> f64 {
        self.friction_coeff() * 9.81
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum TireCondition {
    New,
    Worn,
    Blown,
}
