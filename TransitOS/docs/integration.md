# Integration Guide

**Version 1.0.0** | **New**: MRM fallback, RSS fault detection, dynamic CoG estimation

## Quick Start

Add Transit OS crates to your `Cargo.toml`:

```toml
[dependencies]
transit-state-machine = { path = "crates/state-machine" }
transit-deployment-gate = { path = "crates/deployment-gate" }
transit-verification-invariants = { path = "crates/verification-invariants" }
```

## MRM Fallback & RSS Fault Detection

When the gate blocks a command, it returns a concrete MRM fallback:

```rust
use transit_deployment_gate::mrm::{MrmCommand, MrmController};
use transit_state_machine::params::RoadSurface;
use transit_path_constraints::rss::{RssRule, RssViolatorDetector, FaultParty};

// RSS safe following distance
let safe_gap = RssRule::longitudinal_safe_distance(
    ego_vel, 8.0, lead_vel, 8.0, 0.5
);

// Detect cut-in violator
let (violation, fault, reason) = RssViolatorDetector::check_cut_in(
    ego_vel, ego_heading, other_vel, other_lat_speed,
    lateral_gap, longitudinal_gap, &RoadSurface::Dry, 0.5
);

// Select MRM strategy based on situation
let mrm = MrmController::select(ego_vel, &RoadSurface::Dry, Some(obstacle_dist));
// Returns straight-line brake, pull-over, or gradual decel
println!("MRM: steer={}, throttle={}, brake={}", mrm.steering, mrm.throttle, mrm.brake);
```

## Dynamic CoG Estimation for Trucks

Raw payload sensor readings fluctuate over road bumps. Use the `CogFilter`
to smooth estimates before updating params:

```rust
use transit_state_machine::params::{VehicleParams, CogFilter};

let mut params = VehicleParams::articulated_truck();
let mut filter = CogFilter::new(0.15, params.cog_height, params.mass);

// In each estimation cycle:
let (raw_mass, raw_cog) = VehicleParams::estimate_from_payload(payload_kg, true);
let (smoothed_cog, smoothed_mass) = filter.update(raw_cog, raw_mass);
params.update_cog_height(smoothed_cog);
params.update_mass(smoothed_mass);

// Rollover threshold adapts automatically:
let threshold = params.rollover_threshold(); // smooth, no thrashing
```

## Split-µ Braking (MRM)

On asymmetric friction surfaces, use the split-mu variant:

```rust
use transit_deployment_gate::mrm::MrmController;

// Dry left (μ=1.0), ice right (μ=0.1)
let mrm = MrmController::straight_line_brake_split_mu(
    20.0,  // velocity m/s
    1.0,   // mu_left
    0.1,   // mu_right
);
// mrm.brake_left_bias ≈ 0.1, mrm.brake_right_bias ≈ 1.0
// mrm.steering ≈ 0.05 rad toward left (high-µ side)
```

## Core Verification Loop (Passenger Car)

```rust
use transit_state_machine::{
    machine::StateMachine,
    params::{RoadSurface, VehicleParams},
    state::VehicleState,
    transitions::StateTransition,
};
use transit_deployment_gate::{Interceptor, GateVerdict};
use transit_smt_solver::query::CollisionQuery;
use transit_taint_analysis::taint::TaintedValue;
use transit_verification_invariants::invariants::default_safety_invariants;

fn main() {
    let params = VehicleParams::passenger_car();  // or heavy_truck(), articulated_truck()
    let initial_state = VehicleState::default();
    let mut machine = StateMachine::new(initial_state, params);

    let invariants = default_safety_invariants(RoadSurface::Dry, 50.0, 100.0);
    let mut interceptor = Interceptor::new();

    loop {
        let (steering, throttle, brake) = control_algorithm(&machine.current);
        let cmd: TaintedValue<()> = TaintedValue::clean(());
        let query = CollisionQuery::new(
            (machine.current.pose.x, machine.current.pose.y),
            machine.current.dynamics.velocity,
            machine.current.pose.heading,
            obstacles,
            3.0,
        );
        let decision = interceptor.intercept(&cmd, &query, &invariants);

        match decision.verdict {
            GateVerdict::Allow => {
                machine.step(StateTransition {
                    dt: 0.01, steering_input: steering,
                    throttle_input: throttle, brake_input: brake,
                });
                // Note: air brake lag is applied internally:
                // state.actuators.brake_applied lags behind brake_input
            }
            GateVerdict::Block => {
                log::warn!("Command blocked: {:?}", decision.reason);
            }
        }
    }
}
```

## Articulated Truck Setup

```rust
use transit_state_machine::params::VehicleParams;
use transit_state_machine::state::VehicleState;

// Configure an articulated semi-truck
let params = VehicleParams::articulated_truck();
// Params: mass=25,000 kg, CoG=1.5 m, wheelbase=6m+9m trailer
// Air brake build-up: 350 ms
// Actuator lag: 50 ms steering, 350 ms brake, 10 ms CAN

let mut state = VehicleState::default();
// Optionally set initial articulation angle (radians)
state.articulation.trailer_angle = 0.2;

let mut machine = StateMachine::new(state, params);
// machine.step() now models:
//   1. First-order steering lag
//   2. Air brake pressure build-up
//   3. Trailer articulation dynamics
//   4. Hitch force feedback
//   5. Jackknife risk computation
```

## Interpreting Brake State

The `ActuatorState` distinguishes commanded vs. applied brake:

```rust
// After a step:
let state = machine.current;

// The control system's requested brake (0..1):
let commanded = state.actuators.brake_commanded;

// The actual brake force achieved after pneumatic lag (0..1):
let applied = state.actuators.brake_applied;

// The verification gate checks this mismatch:
if (commanded - applied).abs() > 0.15 {
    // BrakeCommandMismatch violation is raised
}
```

## Affine Arithmetic (for Multi-Step Verification)

```rust
use transit_abstract_interpretation::affine::AffineForm;

// For computations across multiple time steps, affine arithmetic
// preserves variable dependencies and avoids interval explosion:
let mut counter = 0;
let v = AffineForm::from_interval(0.0, 30.0, &mut counter);
let mu = AffineForm::from_interval(0.7, 1.0, &mut counter);

// v² / (2·μ·g) — accurate bounds even after long chains
let stop_dist = v.mul(&v).div(&AffineForm::new(2.0).mul(&mu).mul(&AffineForm::new(9.81)));
let (lo, hi) = stop_dist.to_interval();
// lo and hi are tighter than interval arithmetic because
// shared noise symbols track variable dependencies
```

## Rectangle Collision (for Trucks)

```rust
use transit_smt_solver::collision::RectangleCollision;
use transit_smt_solver::query::SolverResult;

let result = RectangleCollision::check_oriented_rectangles(
    (0.0, 0.0),     // ego center
    (6.0, 1.25),    // ego half-dimensions (long truck)
    0.0,            // ego heading
    (50.0, 0.0),    // obstacle center
    (2.0, 1.0),     // obstacle half-dimensions
    0.0,            // obstacle heading
);

if matches!(result, SolverResult::Sat) {
    // Rectangle overlap detected — more accurate than swept-circle
    // for long/wide vehicles at arbitrary angles
}
```

## ROS 2 Integration

```rust
use transit_ros2_integration::bridge::Ros2Bridge;
use transit_ros2_integration::messages::ActuatorCommandMsg;
use transit_deployment_gate::pipeline::VerificationPipeline;
use transit_deployment_gate::policy::GatePolicy;

let invariants = default_safety_invariants(RoadSurface::Dry, 50.0, 100.0);
let pipeline = VerificationPipeline::new(GatePolicy::Normal, invariants);
let mut bridge = Ros2Bridge::new(pipeline);

fn on_actuator_command(cmd: ActuatorCommandMsg) {
    let (verdict, _decision) = bridge.on_actuator_command(
        &cmd, &current_obstacles, ego_position, ego_heading,
    );
    publish_safety_verdict(verdict);
}
```

## CARLA Simulation

```rust
use transit_carla_integration::{
    bridge::CarlaBridge, scenario_runner::ScenarioRunner,
};
use transit_dynamics_simulator::scenarios::{
    emergency_brake, articulated_truck_emergency_brake, jackknife_recovery,
};
use transit_deployment_gate::pipeline::VerificationPipeline;
use transit_deployment_gate::policy::GatePolicy;

let invariants = CarlaBridge::build_default_invariants(RoadSurface::Dry);
let pipeline = VerificationPipeline::new(GatePolicy::Strict, invariants);
let bridge = Ros2Bridge::new(pipeline);
let mut runner = ScenarioRunner::new(bridge);

// Test passenger car emergency brake
let result = runner.run_scenario(&emergency_brake(20.0, RoadSurface::Dry));
println!("Car brake: {}", result.all_verdicts_allowed());

// Test articulated truck with air brake lag
let result = runner.run_scenario(
    &articulated_truck_emergency_brake(20.0, RoadSurface::Dry)
);
println!("Truck brake: {}", result.all_verdicts_allowed());

// Test jackknife recovery
let result = runner.run_scenario(
    &jackknife_recovery(10.0, 0.5, RoadSurface::Dry)
);
println!("Jackknife: {}", result.all_verdicts_allowed());
```

## Monitoring Dashboard

```rust
use transit_monitoring::dashboard::Dashboard;

let mut dashboard = Dashboard::new();
dashboard.record_check(true, 100.0, vec![]);
let snapshot = dashboard.snapshot();
println!("Block rate: {:.2}%", snapshot.metrics.block_rate * 100.0);
```

## Real-Time Configuration

```rust
use transit_real_time::{
    latency::LatencyBudget,
    scheduling::SafetyVerificationTask,
};
let budget = LatencyBudget::default();
let task = SafetyVerificationTask::new();
assert!(budget.meets_deadline(task.spec.wcet_us));
```

## Full Verification Pipeline

```rust
use transit_deployment_gate::{
    pipeline::VerificationPipeline, policy::GatePolicy,
};
let invariants = default_safety_invariants(RoadSurface::Dry, 50.0, 100.0);
let mut pipeline = VerificationPipeline::new(GatePolicy::Strict, invariants);
let decision = pipeline.verify(&actuator_cmd, &collision_query);
println!("Block rate: {:.2}%", pipeline.block_rate() * 100.0);
```

## Crate-by-Crate Dependency Map

| Crate | Depends On | Purpose |
|-------|-----------|---------|
| state-machine | — | Vehicle state, bicycle + articulated truck dynamics, air brake lag |
| abstract-interpretation | state-machine | Symbolic analysis, interval + affine domains |
| dynamics-simulator | state-machine, abstract-interpretation | Simulation, truck dynamics, air brake model |
| gas-brake-analysis | state-machine, abstract-interpretation | Energy verification |
| path-constraints | state-machine, abstract-interpretation | Constraint generation |
| state-space | state-machine | State exploration |
| smt-solver | path-constraints | Collision detection (circle + rectangle SAT) |
| verification-invariants | state-machine, abstract-interpretation | Safety proofs, rollover, jackknife |
| taint-analysis | — | Sensor integrity |
| spec-generator | state-machine | Spec mining |
| deployment-gate | smt-solver, verification-invariants, taint-analysis | Safety gate |
| ros2-integration | deployment-gate, verification-invariants | ROS2 bridge |
| carla-integration | state-machine, dynamics-simulator, ros2-integration | CARLA bridge |
| real-time | — | Latency analysis |
| monitoring | deployment-gate, verification-invariants | Dashboard |
