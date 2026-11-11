# Standardized Driving Scenarios

**Version 1.0.0** | **New**: truck/articulated scenarios added in this version

## Passenger Car Scenarios

### 1. Emergency Brake

Brake from constant speed to a full stop.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 20 m/s | Initial velocity |
| Road | Dry | Surface friction |
| Vehicle | PassengerCar | Car params (mass 1500 kg, CoG 0.5 m) |

**Constraints**: stopping distance < obstacle distance, deceleration ≤ road
friction limit, slip angle within stable region.

### 2. Lane Change

Lateral displacement at constant speed.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 15 m/s | Longitudinal velocity |
| Lateral offset | 3.5 m | Lane width |
| Road | Dry | Surface friction |

**Constraints**: lateral acceleration ≤ 4 m/s², steering angle ≤ 0.5 rad,
longitudinal gap ≥ safe distance.

### 3. Lane Keeping

Straight-line driving with PID lateral control.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Target speed | 15 m/s | Cruise speed |
| Lane offset | 0 m | Initial lateral error |
| Road | Dry | Surface friction |

### 4. Intersection Crossing

Approaching and crossing a junction.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Approach speed | 10 m/s | Velocity entering intersection |
| Visibility | Full | Sight lines |
| Road | Dry | Surface friction |

**Constraints**: speed ≤ 15 m/s, stopping distance ≤ 20 m.

### 5. Aquaplaning Test

High-speed driving on wet surface.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 30 m/s | Velocity |
| Water depth | 3 mm | Standing water |
| Road | Wet | Low friction |

**Edge case detection**: The edge case analyzer flags aquaplaning risk
when speed exceeds `50 - 2 * water_depth_mm` on wet surfaces.

### 6. Curve Negotiation

Steady-state cornering.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Curve radius | 50 m | Road curvature |
| Speed | friction-limited | Max safe = `√(μ·g·r)` |
| Road | Dry | Surface friction |

**Constraints**: speed ≤ friction-limited max, lateral acceleration ≤ μ·g.

### 7. Acceleration Lane Merge

Merging onto a highway from an on-ramp.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Target speed | 30 m/s | Highway speed |
| Merge gap | 30 m | Required following gap |
| Road | Dry | Surface friction |

**Constraints**: speed ≥ 0.9×target, rear gap ≥ merge gap.

## Heavy Commercial Vehicle Scenarios

### 8. Truck Emergency Brake

Heavy truck braking with air brake latency.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 20 m/s | Initial velocity |
| Brake lag | 0.35 s | Pneumatic build-up time |
| Vehicle | HeavyTruck | 15,000 kg, CoG 1.5 m |
| Road | Dry | Surface friction |

**Key difference from passenger car**: The braking distance formula includes
an air lag term: `v · t_lag + v² / (2 · μ · g)`. With t_lag = 0.35 s,
the truck travels an additional 7 meters before full braking at 20 m/s.

### 9. Articulated Truck Emergency Brake

Articulated semi-truck braking — tests trailer stability during hard stop.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 20 m/s | Initial velocity |
| Vehicle | ArticulatedTruck | 25,000 kg, 6m + 9m trailer |
| Road | Dry | Surface friction |

**Critical checks**: Brake command mismatch (air lag), trailer angle stability,
jackknife risk.

### 10. Articulated Lane Change

Lane change at speed — tests trailer off-tracking and jackknife risk.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 15 m/s | Longitudinal velocity |
| Vehicle | ArticulatedTruck | 25,000 kg |
| Road | Dry | Surface friction |

**Key difference from passenger car**: The trailer's rear axle follows a
different path than the tractor (off-tracking). At speed, aggressive steering
inputs can cause the trailer angle to exceed 0.4 rad, triggering the
jackknife violation.

### 11. Jackknife Recovery

Initial trailer angle with corrective steering — tests stability control.

| Parameter | Default | Description |
|-----------|---------|-------------|
| Speed | 10 m/s | Longitudinal velocity |
| Initial trailer angle | 0.5 rad | Near-jackknife condition |
| Vehicle | ArticulatedTruck | 25,000 kg |
| Road | Dry | Surface friction |

**Edge case**: The state machine's articulation dynamics model the hitch
forces that self-correct or worsen the jackknife depending on steering
input.

## Writing Custom Scenarios

Scenarios are constructed using the `Scenario` struct. For truck scenarios,
use the appropriate `VehicleParams` constructor:

```rust
use transit_state_machine::params::VehicleParams;
use transit_dynamics_simulator::scenarios::Scenario;
use transit_dynamics_simulator::simulator::Simulator;

// Heavy truck scenario
let truck_params = VehicleParams::heavy_truck();

// Articulated truck with custom trailer angle
let mut state = VehicleState::default();
state.articulation.trailer_angle = 0.3; // initial articulation

let scenario = Scenario {
    initial_state: state,
    params: truck_params,
    description: "Custom truck scenario",
};

let mut sim = Simulator::new(scenario.initial_state, scenario.params);
sim.run_for(5.0, |state| {
    // control function: returns (steering, throttle, brake)
    // brake commands are subject to air brake lag internally
    (0.0, 0.0, 0.8)
});
```
