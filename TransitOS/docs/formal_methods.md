# Formal Verification Methodology

**Version 1.0.0** | **New in this version**: RSS fault model, MRM trigger logic, dynamic CoG estimation

## Abstract Interpretation

Transit OS uses abstract interpretation to soundly over-approximate all
possible vehicle behaviors. Two abstract domains are provided:

### Interval Domain

An interval `[lo, hi]` represents all possible values of a variable.
Operations on intervals produce sound bounds:

- `[a,b] + [c,d] = [a+c, b+d]`
- `[a,b] - [c,d] = [a-d, b-c]`
- `[a,b] * [c,d] = [min(ac,ad,bc,bd), max(ac,ad,bc,bd)]`
- `sin([a,b])` produces `[-1, 1]` if the interval spans a full period

**Limitation — dependency problem**: Pure intervals suffer from `x - x ≠ 0`.
When evaluating chained equations across multiple time steps, interval width
explodes exponentially, leading to excessive false positives. Use the affine
domain for multi-step integration.

### Affine Arithmetic Domain (Recommended for Integration)

Affine forms track linear dependencies between variables:

```
x̂ = x₀ + ε₁·x₁ + ε₂·x₂ + ... + εₙ·xₙ
```

where εᵢ ∈ [-1, 1] are shared noise symbols representing uncertain variables.
When the same noise symbol appears in two terms, their dependencies are
preserved:

- `x̂ - x̂ = 0` (exact cancellation, unlike intervals)
- `x̂ + ŷ` combines noise terms; shared symbols are summed
- `x̂ · ŷ` produces a new affine form with cross-term error bounds

This eliminates spurious over-approximation in multi-step computations.

### Creating Affine Forms

```rust
use transit_abstract_interpretation::affine::AffineForm;

// From an interval [0, 10]
let mut noise_counter = 0;
let x = AffineForm::from_interval(0.0, 10.0, &mut noise_counter);

// Arithmetic preserves dependencies
let result = x.clone().sub(&x);  // == 0 (exact)
let (lo, hi) = result.to_interval();  // [0, 0] not [-10, 10]
```

### Symbolic Expressions

Vehicle physics equations are expressed symbolically:

```rust
// Braking distance with air brake lag: v·t_lag + v²/(2·μ·g)
fn braking_distance_with_lag_expr() -> SymbolicExpr {
    var("velocity") * var("brake_lag")
        + var("velocity").pow(2) / (constant(2.0) * var("friction_coeff") * constant(9.81))
}

// Articulation jackknife risk: |trailer_angle| / 0.7
fn jackknife_risk_expr() -> SymbolicExpr {
    SymbolicExpr::Abs(Box::new(var("trailer_angle"))) / constant(0.7)
}
```

## SMT-Based Collision Detection

Transit OS supports two collision geometries:

### Swept-Circle (Passenger Cars)

For standard vehicles, collision is formulated as a swept-circle intersection:

- **Variables**: time `t`, ego position `(x, y)`, obstacle position `(ox, oy)`
- **Constraints**: kinematic equations over `t`, separation distance formula
- **Query**: is there a `t ∈ [0, horizon]` where ‖p_ego(t) - p_obs(t)‖ ≤ r_sum?

The built-in TCC solver solves the quadratic analytically in O(n) for n obstacles.

### Oriented Rectangle (Trucks & Articulated Vehicles)

For long vehicles, the Separating Axis Theorem (SAT) provides accurate
rectangle-rectangle collision detection:

```rust
RectangleCollision::check_oriented_rectangles(
    ego_center, ego_half, ego_heading,
    obs_center, obs_half, obs_heading,
) -> SolverResult
```

The SAT projects both rectangles onto 8 candidate axes (4 edge normals per
rectangle). If all projections overlap, the rectangles intersect.

## Safety Invariants

Seven safety invariants are proven at each verification step:

| Invariant | Formula | Passenger Car | Heavy Truck | Articulated |
|-----------|---------|--------------|-------------|-------------|
| Braking Distance | `v·t_lag + v²/(2μg) ≤ gap` | configurable | incl. air lag | incl. air lag |
| Lateral Acceleration | `v·ω ≤ rollover` | 6.0 m/s² | 4.0 m/s² | 3.5 m/s² |
| Speed Limit | `speed ≤ max` | 70 m/s | 50 m/s | 50 m/s |
| Slip Angle | `β ≤ max` | 0.4 rad | 0.3 rad | 0.25 rad |
| Yaw Rate | `ψ̇ ≤ max` | 3.0 rad/s | 2.0 rad/s | 1.5 rad/s |
| Road Friction | `√(a_x² + a_y²) ≤ μg` | friction-dep. | friction-dep. | friction-dep. |
| Steering Rate | `Δδ/Δt ≤ max` | actuator-limited | actuator-limited | actuator-limited |

### Additional Truck-Specific Violations

- **RolloverRisk**: lateral acceleration > 80% of `track_width / (2·CoG_height) · g`
- **JackknifeRisk**: trailer angle > 0.4 rad
- **ActuatorLagExceeded**: total brake lag > 250 ms
- **BrakeCommandMismatch**: |brake_commanded - brake_applied| > 0.15

## Air Brake Latency Model

Heavy trucks use pneumatic brakes with significant mechanical lag. The
commanded-to-applied brake force is modelled as a first-order lag:

```rust
fn air_brake_lag(commanded, current_applied, dt, build_up_time) -> f64 {
    if commanded > current_applied {
        // building up: rate-limited by pneumatic pressure rise
        min(current_applied + dt / build_up_time, commanded)
    } else {
        // releasing: rate-limited by exhaust
        max(current_applied - dt / release_time, commanded)
    }
}
```

Typical build-up time: **350 ms** for full brake force. The stopping distance
calculation accounts for this:

```
stopping_distance = v · t_lag + v² / (2 · μ · g)
```

## Articulated Trailer Dynamics

For articulated trucks, the trailer is modelled as a second bicycle segment
connected at a hitch point. Key equations:

```
trailer_yaw_rate = (v_y_hitch · cos(θ) - v_x_hitch · sin(θ)) / L_trailer
dθ/dt = yaw_rate_tractor - yaw_rate_trailer
```

where θ is the articulation angle. Jackknife risk is computed as:

```
risk = clamp(|θ| / 0.7, 0, 1)
```

## State Space Exploration

Exhaustive exploration uses discretized state regions with vehicle class-specific
bounds. Exploration strategies include grid sampling, random sampling,
border-focused, and Monte Carlo perturbation.

Edge cases detected automatically:
- **Aquaplaning**: high speed on wet surface
- **Black ice**: sub-zero temperature on dry-looking road
- **Tire blowout**: blown tire at speed
- **Oversteer**: side slip exceeding stability threshold
- **Jackknife**: trailer angle exceeding 0.4 rad

## Responsibility-Sensitive Safety (RSS) Fault Model

RSS rules [14] formalize "who is at fault" in any collision scenario:

### Longitudinal Safe Distance

The minimum following distance that guarantees no rear-end collision if the
lead vehicle brakes maximally:

```
d_min = v_ego · t_reaction + v_ego²/(2·a_ego_max) - v_lead²/(2·a_lead_max)
```

where the result is clamped to ≥ 2 m. If the actual gap falls below d_min
and ego collides with the lead vehicle, ego is at fault.

### Cut-In Violation Detection

When an external vehicle enters ego's lane, the system checks:

1. **Lateral clearance**: Is the lateral gap ≥ `RssRule::lateral_safe_distance()`?
2. **Longitudinal gap**: Is the longitudinal gap ≥ ego's stopping distance?

If both conditions fail, the fault is assigned to `Other` (the cutting-in
vehicle), and ego's gate triggers an MRM rather than blocking ego's own
command as unsafe.

### MRM Trigger Condition

An MRM (Minimum Risk Maneuver) is triggered when:

- Fault = `Other` or `Unavoidable`, AND
- Obstacle distance < 1.5× ego's minimum stopping distance

#### Split-µ Surface Handling

When braking on surfaces with asymmetric friction (e.g., dry left, ice right),
symmetric brake pressure induces a destabilizing yaw torque. The
`straight_line_brake_split_mu()` method computes per-wheel brake biases:

```rust
let (bias_left, bias_right) = MrmController::compute_split_mu_biases(mu_left, mu_right);
// Low-µ side gets proportionally less brake force
// A small steering correction counteracts residual yaw torque
```

The bias is the ratio of the lower friction coefficient to the higher one,
clamped to `[0, 1]`. A steering correction up to 0.05 rad is applied toward
the high-µ side.

The MRM controller selects between three strategies:
- **Straight-line brake**: full friction-limited deceleration (obstacle < 20 m)
- **Pull over**: gentle braking with slight steering offset (obstacle 20–50 m)
- **Gradual decel**: comfortable 0.3× friction deceleration (obstacle > 50 m)

### Fault Assignment Summary

| Scenario | Fault | Gate Action |
|----------|-------|-------------|
| Ego following too close → rear-end | Ego | Block + MRM |
| Vehicle cuts in below safe distance | Other | MRM (not Block) |
| Sudden obstacle, safe following distance | Unavoidable | MRM (not Block) |
| All checks pass, no threat | — | Allow |

## Dynamic Cargo Weight & CoG Estimation

Heavy truck mass and center of gravity change drastically with payload:

| Payload State | Total Mass | CoG Height | Rollover Threshold |
|--------------|-----------|-----------|-------------------|
| Empty trailer | 15 t | 1.5 m | 8.2 m/s² |
| Half-loaded | 25 t | 1.65 m | 7.4 m/s² |
| Fully loaded | 40 t | 1.8 m | 6.8 m/s² |

Raw payload sensor readings fluctuate over road bumps and inclines. A
`CogFilter` provides exponential moving average smoothing:

```rust
let mut filter = CogFilter::new(0.15, current_cog, current_mass);
let (smoothed_cog, smoothed_mass) = filter.update(raw_cog, raw_mass);
params.update_cog_height(smoothed_cog);
```

The filter uses a warm-up phase (first 4 samples average equally) then
switches to EMA with `alpha = 0.15` to prevent threshold thrashing.

The system supports runtime updates via:
```rust
params.update_mass(estimated_mass);
params.update_cog_height(estimated_cog);
let (mass, cog) = VehicleParams::estimate_from_payload(payload_kg, is_loaded);
```

After updating, `params.rollover_threshold()` returns the new geometry-based
limit, and the `RolloverRisk` constraint adapts automatically.

## Temporal Logic Specifications

LTL specifications can be mined from driving trajectories:

- `AlwaysWithin(var, lo, hi)` — variable stays in bounds forever
- `EventuallyBelow(var, threshold)` — variable eventually drops below threshold
- `StableWithin { var, lo, hi, duration }` — variable stabilizes in a region
