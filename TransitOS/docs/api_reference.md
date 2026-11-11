# API Reference

**Version 1.0.0** | All 16 crates at consistent version

## transit-state-machine

### New/Updated Types

- **`VehicleClass`** — `PassengerCar`, `HeavyTruck`, `ArticulatedTruck`
- **`VehicleParams`** — `{ class, mass, wheelbase, track_width, cog_height, front_wt_frac,
  max_steering, max_brake_force, max_throttle_force, air_brake, actuator_delay,
  trailer_wheelbase, hitch_to_rear_axle }`
- **`AirBrakeParams`** — `{ build_up_time_s, release_time_s, max_pressure_bar, line_delay_s }`
- **`ActuatorDelay`** — `{ steering_lag_s, brake_lag_s, throttle_lag_s, can_bus_latency_s }`
- **`VehicleState`** — `{ pose, dynamics, articulation, actuators, tires, road }`
- **`ArticulationState`** — `{ trailer_angle, trailer_yaw_rate, trailer_velocity,
  hitch_force_lateral, hitch_force_longitudinal, jackknife_risk }`
- **`ActuatorState`** — Now includes `brake_commanded` and `brake_applied` (distinguishes
  commanded from pneumatically-lagged applied brake force)
- **`RoadSurface`** — Dry, Wet, Snow, Ice (with friction coefficient)
- **`TireCondition`** — New, Worn, Blown
- **`StateMachine`** — step(), replay(), set_tire_condition()
- **`StateTransition`** — { dt, steering_input, throttle_input, brake_input }
- **`Dynamics`** — { velocity, lateral_velocity, acceleration, yaw_rate, slip_angle, side_slip }
- **`TireState`** — Forces and slip angles for front/rear

### New State Violations

- `RolloverRisk { threshold, actual }` — lateral acceleration exceeds 80% of CoG-based limit
- `JackknifeRisk { trailer_angle }` — articulation angle > 0.4 rad
- `ActuatorLagExceeded { max_lag_s, brake_lag_s }` — total brake lag exceeds 250 ms
- `BrakeCommandMismatch { commanded, applied }` — gap > 0.15 between commanded and applied

### Key Functions

```rust
VehicleParams::passenger_car() -> VehicleParams  // mass 1500, CoG 0.5m
VehicleParams::heavy_truck() -> VehicleParams    // mass 15000, CoG 1.5m, air brake
VehicleParams::articulated_truck() -> VehicleParams // mass 25000, articulated
VehicleParams::rollover_threshold() -> f64       // (track_width / (2*CoG_height)) * g
VehicleParams::yaw_inertia() -> f64
VehicleParams::is_articulated() -> bool
VehicleState::speed() -> f64
VehicleState::is_stationary() -> bool
VehicleState::braking_distance(&VehicleParams) -> f64  // includes air brake lag
VehicleState::curvature_radius() -> f64
VehicleState::is_jackknifed() -> bool
ActuatorState::effective_brake_force() -> f64
ActuatorDelay::truck_default() -> Self  // brake_lag_s=0.35, steering_lag_s=0.05
```

## transit-abstract-interpretation

### Affine Arithmetic Domain (New)

- **`AffineForm`** — `{ center, noise_terms: Vec<(usize, f64)>, error: f64 }`
- Tracks linear dependencies between variables via shared noise symbols
- Eliminates the dependency problem (`x - x = 0` exactly)

```rust
AffineForm::new(center) -> AffineForm
AffineForm::with_noise(center, noise_id, half_range) -> AffineForm
AffineForm::from_interval(lo, hi, &mut noise_counter) -> AffineForm
AffineForm::to_interval(&self) -> (f64, f64)
AffineForm::add(&self, other) -> AffineForm
AffineForm::sub(&self, other) -> AffineForm
AffineForm::mul(&self, other) -> AffineForm
AffineForm::neg(&self) -> AffineForm
AffineForm::sqrt(&self) -> AffineForm
```

### Symbolic Expressions

- **`SymbolicExpr`** — Add, Sub, Mul, Div, Neg, Pow, Sin, Cos, Atan2, Sqrt, Abs, Tanh
- Operators: `+`, `-`, `*`, `/`, `.pow(i32)`
- `var(name)` / `constant(val)` — constructors

### Interval Domain

- **`Interval`** — `[lo, hi]` with arithmetic, trig, sqrt, pow, atan2
- **`AbstractDomain`** — Interval, Bottom, Top

### Physics Shortcuts

```rust
physics::braking_distance_expr() -> SymbolicExpr
physics::lateral_accel_expr() -> SymbolicExpr
physics::centripetal_force_expr() -> SymbolicExpr
physics::slip_angle_expr(wheelbase_frac) -> SymbolicExpr
physics::tire_force_expr() -> SymbolicExpr
```

## transit-dynamics-simulator

### New/Updated Types

- **`ArticulatedDynamics`** — Trailer dynamics computation
- **`AirBrakeSim`** — Air brake lag and lag-aware stopping distance
- **`Simulator`** — Time-stepped with closure-based control
- **`BicycleModel`** — Load transfer, now class-parameterized
- **`TireModel`** — Lateral force (stiffness varies by vehicle class)
- **`PidController`** / **`LaneKeepController`** / **`CruiseController`**

### New Scenario Factories

```rust
truck_emergency_brake(speed, road) -> Scenario
articulated_truck_emergency_brake(speed, road) -> Scenario
articulated_lane_change(speed, road) -> Scenario
jackknife_recovery(speed, initial_trailer_angle, road) -> Scenario
```

## transit-smt-solver

### Rectangle Collision (New)

- **`RectangleCollision`** — SAT-based oriented rectangle intersection

```rust
RectangleCollision::check_oriented_rectangles(
    ego_center: (f64, f64),
    ego_half: (f64, f64),
    ego_heading: f64,
    obs_center: (f64, f64),
    obs_half: (f64, f64),
    obs_heading: f64,
) -> SolverResult
```

## transit-path-constraints

### RSS Fault Model (New in v1.0.0)

```rust
FaultParty::Ego | FaultParty::Other | FaultParty::Unavoidable

RssRule::longitudinal_safe_distance(ego_vel, ego_max_dec, lead_vel, lead_max_dec, reaction_time) -> f64
RssRule::lateral_safe_distance(ego_vel, lateral_jerk_limit) -> f64
RssRule::cut_in_violation(ego_vel, ego_max_dec, gap, reaction_time, road) -> (bool, FaultParty)
RssRule::rear_end_fault(ego_vel, ego_max_dec, lead_vel, lead_max_dec, gap) -> FaultParty

RssViolatorDetector::check_cut_in(...) -> (bool, FaultParty, Option<&str>)
RssViolatorDetector::should_trigger_mrm(fault, speed, obstacle_dist, max_dec) -> bool
```

## transit-deployment-gate

### MRM Controller (v1.0.0)

```rust
MrmCommand {
    steering: f64, throttle: f64, brake: f64,
    brake_left_bias: f64,   // [0,1] left wheel brake scaling
    brake_right_bias: f64,  // [0,1] right wheel brake scaling
    reason: String,
}

// Construction helpers:
MrmCommand::symmetric(steering, throttle, brake, reason) -> MrmCommand
MrmCommand::asymmetric(steering, throttle, brake, left_bias, right_bias, reason) -> MrmCommand

// Split-μ: per-wheel brake distribution for asymmetric friction:
MrmController::compute_split_mu_biases(mu_left, mu_right) -> (f64, f64)
MrmController::straight_line_brake_split_mu(velocity, mu_left, mu_right) -> MrmCommand

MrmController::straight_line_brake(velocity, road) -> MrmCommand
MrmController::pull_over(velocity, road) -> MrmCommand
MrmController::gradual_decel(velocity, road) -> MrmCommand
MrmController::select(velocity, road, obstacle_dist) -> MrmCommand

// GateDecision includes MRM fallback:
GateDecision { verdict, reason, timestamp, check_id, mrm_fallback: Option<MrmCommand> }
GateDecision::block_with_mrm(check_id, reason, mrm) -> GateDecision
```

## transit-state-machine

### Dynamic CoG Estimation (v1.0.0)

```rust
// Runtime updates (apply CogFilter first for noise rejection):
VehicleParams::update_mass(estimated_mass)
VehicleParams::update_cog_height(estimated_cog)
VehicleParams::estimate_from_payload(payload_kg, is_loaded) -> (f64, f64)

// EMA low-pass filter for payload sensor noise:
CogFilter { alpha: f64, estimated_cog: f64, estimated_mass: f64, samples: u64 }
CogFilter::new(alpha, initial_cog, initial_mass) -> CogFilter
CogFilter::update(&mut self, raw_cog, raw_mass) -> (f64, f64)
CogFilter::reset(&mut self, cog, mass)
```
