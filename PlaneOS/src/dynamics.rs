//! 6-DoF flight dynamics foundation.
//!
//! Provides the rigid-body state representation ([`FlightState`]), attitude
//! math ([`Quaternion`]), vector helpers, an ISA atmosphere model, a
//! [`TerrainModel`], and a lightweight [`FlightDynamics`] integrator used as
//! the domain mock for the verification chain.
//!
//! **Research mock only.** The aerodynamic model uses constant coefficient
//! approximations. It is intentionally simple and clearly mocked so the
//! verification stack (formulas, constraints, invariants, gate) can be
//! exercised without implying certified flight-dynamics fidelity.

use crate::formula::{Expr, Formula, FormulaBuilder};
use std::collections::BTreeMap;

/// 3D vector.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Vec3 {
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Vec3 { x, y, z }
    }

    pub fn dot(self, o: Vec3) -> f64 {
        self.x * o.x + self.y * o.y + self.z * o.z
    }

    pub fn cross(self, o: Vec3) -> Vec3 {
        Vec3 {
            x: self.y * o.z - self.z * o.y,
            y: self.z * o.x - self.x * o.z,
            z: self.x * o.y - self.y * o.x,
        }
    }

    pub fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    pub fn normalized(self) -> Option<Vec3> {
        let n = self.norm();
        if n < 1e-12 {
            None
        } else {
            Some(Vec3::new(self.x / n, self.y / n, self.z / n))
        }
    }

    pub fn scale(self, s: f64) -> Vec3 {
        Vec3::new(self.x * s, self.y * s, self.z * s)
    }
}

impl std::ops::Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl std::ops::Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl std::ops::Mul<f64> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: f64) -> Vec3 {
        self.scale(rhs)
    }
}

impl std::ops::Div<f64> for Vec3 {
    type Output = Vec3;
    fn div(self, rhs: f64) -> Vec3 {
        self.scale(1.0 / rhs)
    }
}

/// Quaternion attitude representation `(w, x, y, z)`, units of the
/// scalar-first convention. Normalized whenever constructed through
/// [`Quaternion::from_axis_angle`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Quaternion {
    pub w: f64,
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl Quaternion {
    pub const IDENTITY: Quaternion = Quaternion {
        w: 1.0,
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };

    pub fn new(w: f64, x: f64, y: f64, z: f64) -> Self {
        Quaternion { w, x, y, z }
    }

    /// Build from a rotation about a unit axis by `angle_rad`.
    pub fn from_axis_angle(axis: Vec3, angle_rad: f64) -> Option<Self> {
        let axis = axis.normalized()?;
        let half = angle_rad * 0.5;
        let s = half.sin();
        Some(Quaternion {
            w: half.cos(),
            x: axis.x * s,
            y: axis.y * s,
            z: axis.z * s,
        })
    }

    pub fn norm(self) -> f64 {
        (self.w * self.w + self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalized(self) -> Option<Quaternion> {
        let n = self.norm();
        if n < 1e-12 {
            None
        } else {
            Some(Quaternion::new(
                self.w / n,
                self.x / n,
                self.y / n,
                self.z / n,
            ))
        }
    }

    /// Quaternion product `self ⊗ rhs` (inherent rotation order).
    pub fn mul(self, rhs: Quaternion) -> Quaternion {
        Quaternion::new(
            self.w * rhs.w - self.x * rhs.x - self.y * rhs.y - self.z * rhs.z,
            self.w * rhs.x + self.x * rhs.w + self.y * rhs.z - self.z * rhs.y,
            self.w * rhs.y - self.x * rhs.z + self.y * rhs.w + self.z * rhs.x,
            self.w * rhs.z + self.x * rhs.y - self.y * rhs.x + self.z * rhs.w,
        )
    }

    pub fn conjugate(self) -> Quaternion {
        Quaternion::new(self.w, -self.x, -self.y, -self.z)
    }

    /// Rotate a vector by this quaternion.
    pub fn rotate(self, v: Vec3) -> Vec3 {
        let qv = Vec3::new(self.x, self.y, self.z);
        let t = qv.cross(v).scale(2.0);
        v + t.scale(self.w) + qv.cross(t)
    }

    /// Inverse rotation of a vector.
    pub fn rotate_inv(self, v: Vec3) -> Vec3 {
        self.conjugate().rotate(v)
    }

    /// Extract Euler attitude `(roll, pitch, yaw)` in radians (ZYX order).
    pub fn to_euler(self) -> (f64, f64, f64) {
        let q = self;
        // Roll
        let roll = (2.0 * (q.w * q.x + q.y * q.z))
            .atan2(1.0 - 2.0 * (q.x * q.x + q.y * q.y));
        // Pitch
        let sinp = 2.0 * (q.w * q.y - q.z * q.x);
        let pitch = sinp.clamp(-1.0, 1.0).asin();
        // Yaw
        let yaw = (2.0 * (q.w * q.z + q.x * q.y))
            .atan2(1.0 - 2.0 * (q.y * q.y + q.z * q.z));
        (roll, pitch, yaw)
    }
}

/// ISA (International Standard Atmosphere) helpers.
pub struct Atmosphere;

impl Atmosphere {
    /// Sea-level conditions.
    pub const RHO0: f64 = 1.225; // kg/m^3
    pub const P0: f64 = 101325.0; // Pa
    pub const T0: f64 = 288.15; // K
    pub const G: f64 = 9.80665; // m/s^2
    pub const R: f64 = 287.053; // J/(kg·K)
    pub const LAPSE: f64 = 0.0065; // K/m below tropopause

    /// Temperature (K) at geometric altitude `h` (m), troposphere only.
    pub fn temperature(h: f64) -> f64 {
        Self::T0 - Self::LAPSE * h.max(0.0)
    }

    /// Pressure (Pa) at altitude (troposphere, ISA).
    pub fn pressure(h: f64) -> f64 {
        let t = Self::temperature(h);
        Self::P0 * (t / Self::T0).powf(Self::G / (Self::R * Self::LAPSE))
    }

    /// Density (kg/m^3) at altitude (troposphere, ISA).
    pub fn density(h: f64) -> f64 {
        Self::pressure(h) / (Self::R * Self::temperature(h))
    }

    /// Speed of sound (m/s) at altitude.
    pub fn speed_of_sound(h: f64) -> f64 {
        const GAMMA: f64 = 1.4;
        (GAMMA * Self::R * Self::temperature(h)).sqrt()
    }
}

/// Terrain elevation (m, MSL) at a horizontal position. Simple mock
/// supports a single constant elevation or a plane with slope.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TerrainModel {
    pub base_elevation: f64,
    pub slope_x: f64,
    pub slope_y: f64,
}

impl Default for TerrainModel {
    fn default() -> Self {
        TerrainModel {
            base_elevation: 0.0,
            slope_x: 0.0,
            slope_y: 0.0,
        }
    }
}

impl TerrainModel {
    pub fn sea_level() -> Self {
        TerrainModel::default()
    }

    pub fn elevation_at(&self, pos: Vec3) -> f64 {
        self.base_elevation + self.slope_x * pos.x + self.slope_y * pos.y
    }

    pub fn terrain_clearance(&self, pos: Vec3) -> f64 {
        pos.z - self.elevation_at(pos)
    }

    /// Build a ground-collision hazard formula for the current position.
    ///
    /// `margin` is the minimum terrain clearance in meters; a formula that
    /// is SAT means the aircraft is at/below the terrain + margin.
    pub fn ground_collision_formula(
        &self,
        pos_x: Expr,
        pos_y: Expr,
        alt: Expr,
        margin: f64,
    ) -> Formula {
        let elevation = Expr::magic("terrain.base_elevation")
            + Expr::magic("terrain.slope_x") * pos_x
            + Expr::magic("terrain.slope_y") * pos_y;
        alt.le(elevation + Expr::Const(margin))
    }

    /// Add terrain constants into the canonical environment.
    pub fn seed_env(&self, env: &mut BTreeMap<String, f64>) {
        env.insert("mag.terrain.base_elevation".to_string(), self.base_elevation);
        env.insert("mag.terrain.slope_x".to_string(), self.slope_x);
        env.insert("mag.terrain.slope_y".to_string(), self.slope_y);
    }
}

/// Flight configuration parameters (mocked aerodynamic coefficients).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AircraftConfig {
    /// Wing reference area (m^2).
    pub wing_area: f64,
    /// Wing span (m).
    pub wing_span: f64,
    /// Mean aerodynamic chord (m).
    pub mac: f64,
    /// Aircraft mass (kg).
    pub mass: f64,
    /// Max thrust per engine (N).
    pub max_thrust_per_engine: f64,
    /// Number of engines.
    pub engine_count: u32,
    /// Drag coefficient at zero lift.
    pub cd0: f64,
    /// Lift curve slope per radian (approx).
    pub cl_alpha: f64,
    /// Moment coefficient slope per radian (approx).
    pub cm_alpha: f64,
}

impl Default for AircraftConfig {
    fn default() -> Self {
        // Medium-range transport-like configuration.
        AircraftConfig {
            wing_area: 122.6,
            wing_span: 34.1,
            mac: 4.2,
            mass: 72_000.0,
            max_thrust_per_engine: 120_000.0,
            engine_count: 2,
            cd0: 0.02,
            cl_alpha: 5.2,
            cm_alpha: -0.45,
        }
    }
}

impl AircraftConfig {
    pub fn max_total_thrust(&self) -> f64 {
        self.max_thrust_per_engine * self.engine_count as f64
    }

    pub fn weight(&self) -> f64 {
        self.mass * Atmosphere::G
    }
}

/// 6-DoF flight state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightState {
    /// Simulation time (s).
    pub t: f64,
    /// Position in Earth axes (m). NED for z; +z up for terrain math.
    pub pos: Vec3,
    /// Body velocity (m/s) in body frame.
    pub vel_body: Vec3,
    /// Attitude quaternion (body ← earth).
    pub quat: Quaternion,
    /// Body angular velocity (rad/s).
    pub omega: Vec3,
}

impl FlightState {
    pub fn new(pos: Vec3, vel_body: Vec3, quat: Quaternion, omega: Vec3) -> Self {
        FlightState {
            t: 0.0,
            pos,
            vel_body,
            quat,
            omega,
        }
    }

    pub fn level(pos: Vec3, speed_ms: f64, heading_rad: f64) -> Self {
        let quat = Quaternion::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), heading_rad)
            .expect("heading axis is unit");
        let vel_body = Vec3::new(speed_ms, 0.0, 0.0);
        FlightState::new(pos, vel_body, quat, Vec3::ZERO)
    }

    /// Altitude (m, MSL).
    pub fn altitude(&self) -> f64 {
        self.pos.z
    }

    /// True airspeed (m/s) — magnitude of body velocity.
    pub fn tas(&self) -> f64 {
        self.vel_body.norm()
    }

    /// Velocity in earth axes (m/s).
    pub fn velocity_earth(&self) -> Vec3 {
        self.quat.rotate(self.vel_body)
    }

    /// Mach number.
    pub fn mach(&self) -> f64 {
        let a = Atmosphere::speed_of_sound(self.altitude().max(0.0));
        self.tas() / a
    }

    /// Normal acceleration `n_z` (g). Mock: 1.0 g at level flight.
    pub fn nz(&self) -> f64 {
        let accel = self.quat.rotate_inv(Vec3::new(0.0, 0.0, Atmosphere::G));
        accel.z / Atmosphere::G
    }

    /// Angle of attack (rad), computed from body velocity components.
    pub fn alpha(&self) -> f64 {
        let u = self.vel_body.x.max(1e-6);
        (self.vel_body.z / u).atan()
    }

    /// Angle of attack in degrees.
    pub fn alpha_deg(&self) -> f64 {
        self.alpha().to_degrees()
    }

    /// Indirect airspeed (knots).
    pub fn ias_knots(&self) -> f64 {
        // Approximate equivalent airspeed from TAS and density ratio.
        let rho = Atmosphere::density(self.altitude().max(0.0));
        let eas = self.tas() * (rho / Atmosphere::RHO0).sqrt();
        eas * 1.94384
    }
}

/// Derived quantities for one control cycle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlightDerived {
    pub tas: f64,
    pub ias_knots: f64,
    pub mach: f64,
    pub alpha_deg: f64,
    pub nz: f64,
    pub altitude: f64,
    pub terrain_clearance: f64,
    pub climb_rate: f64,
}

impl FlightDerived {
    pub fn from_state(state: &FlightState, terrain: &TerrainModel) -> Self {
        FlightDerived {
            tas: state.tas(),
            ias_knots: state.ias_knots(),
            mach: state.mach(),
            alpha_deg: state.alpha_deg(),
            nz: state.nz(),
            altitude: state.altitude(),
            terrain_clearance: terrain.terrain_clearance(state.pos),
            climb_rate: state.velocity_earth().z,
        }
    }
}

/// First-order mock dynamics for a single control cycle.
///
/// Integrates the body state with a very simple point-mass-plus-attitude
/// approximation. **Mock only.** No certification value.
#[derive(Clone, Copy, Debug)]
pub struct FlightDynamics {
    pub config: AircraftConfig,
    /// Time step per cycle (s).
    pub dt: f64,
    /// Simple force scaling to make the mock fly.
    pub thrust_scale: f64,
    /// Simple pitch-rate scaling for elevator input.
    pub elevator_scale: f64,
}

impl Default for FlightDynamics {
    fn default() -> Self {
        FlightDynamics {
            config: AircraftConfig::default(),
            dt: 0.05,
            thrust_scale: 1.0,
            elevator_scale: 0.8,
        }
    }
}

impl FlightDynamics {
    /// Advance one control step given normalized inputs in `[-1, 1]`:
    /// `throttle`, `elevator` (+nosedown), `aileron`, `rudder`.
    pub fn step(
        &self,
        state: &FlightState,
        throttle: f64,
        elevator: f64,
        aileron: f64,
        rudder: f64,
    ) -> FlightState {
        let dt = self.dt;
        let cfg = &self.config;
        let rho = Atmosphere::density(state.altitude().max(0.0));

        // --- Body-axis velocity rates (point-mass + thrust).
        let tas = state.tas();
        let q = 0.5 * rho * tas * tas;
        let cl = cfg.cl_alpha * state.alpha().clamp(-0.5, 0.5);
        let cd = cfg.cd0 + 0.05 * cl * cl;

        let lift = q * cfg.wing_area * cl;
        let drag = q * cfg.wing_area * cd;
        let thrust = throttle.clamp(-1.0, 1.0).max(0.0)
            * cfg.max_total_thrust()
            * self.thrust_scale;

        let g_body = state.quat.rotate_inv(Vec3::new(0.0, 0.0, Atmosphere::G));

        let d_u = (-g_body.x + thrust / cfg.mass
            - drag / cfg.mass * (state.vel_body.x / tas.max(1e-6)))
            * dt;
        let d_v = (-g_body.y) * dt;
        let d_w = (-g_body.z + lift / cfg.mass) * dt;

        // --- Angular rates (mock pitch/roll/yaw coupling).
        let d_pitch =
            elevator.clamp(-1.0, 1.0) * 3.0 * self.elevator_scale
                + (-cfg.cm_alpha * state.alpha()).clamp(-2.0, 2.0);
        let d_roll = aileron.clamp(-1.0, 1.0) * 2.0;
        let d_yaw = rudder.clamp(-1.0, 1.0) * 0.5;

        let omega = state.omega + Vec3::new(d_roll, d_pitch, d_yaw) * dt;

        // --- Attitude update via quaternion derivative.
        let dq = Quaternion::new(0.0, omega.x, omega.y, omega.z)
            .mul(state.quat)
            .scale(0.5);
        let quat_new = (state.quat + dq).normalized().unwrap_or(state.quat);

        // --- Position update in earth frame.
        let vel_new = state.vel_body + Vec3::new(d_u, d_v, d_w);
        let vel_earth_new = quat_new.rotate(vel_new);
        let pos_new = state.pos + vel_earth_new.scale(dt);

        FlightState {
            t: state.t + dt,
            pos: pos_new,
            vel_body: vel_new,
            quat: quat_new,
            omega,
        }
    }
}

impl Quaternion {
    fn scale(self, s: f64) -> Quaternion {
        Quaternion::new(self.w * s, self.x * s, self.y * s, self.z * s)
    }
}

impl std::ops::Add for Quaternion {
    type Output = Quaternion;
    fn add(self, rhs: Quaternion) -> Quaternion {
        Quaternion::new(
            self.w + rhs.w,
            self.x + rhs.x,
            self.y + rhs.y,
            self.z + rhs.z,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quaternion_rotates_axis() {
        let q = Quaternion::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), std::f64::consts::FRAC_PI_2)
            .unwrap();
        let v = q.rotate(Vec3::new(1.0, 0.0, 0.0));
        assert!((v.x - 0.0).abs() < 1e-9);
        assert!((v.y - 1.0).abs() < 1e-9);
    }

    #[test]
    fn quaternion_normalized_unit() {
        let q = Quaternion::new(1.0, 2.0, 3.0, 4.0).normalized().unwrap();
        assert!((q.norm() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn euler_roundtrip_level() {
        let q = Quaternion::from_axis_angle(Vec3::new(0.0, 0.0, 1.0), 0.7).unwrap();
        let (roll, pitch, yaw) = q.to_euler();
        assert!(roll.abs() < 1e-9);
        assert!(pitch.abs() < 1e-9);
        assert!((yaw - 0.7).abs() < 1e-9);
    }

    #[test]
    fn atmosphere_density_at_sea_level() {
        let rho = Atmosphere::density(0.0);
        assert!((rho - 1.225).abs() < 1e-3);
    }

    #[test]
    fn terrain_clearance() {
        let t = TerrainModel {
            base_elevation: 100.0,
            slope_x: 0.01,
            slope_y: 0.0,
        };
        let pos = Vec3::new(1000.0, 0.0, 150.0);
        assert!((t.terrain_clearance(pos) - 40.0).abs() < 1e-9);
    }

    #[test]
    fn dynamics_step_advances() {
        let d = FlightDynamics::default();
        let s0 = FlightState::level(Vec3::new(0.0, 0.0, 1000.0), 150.0, 0.0);
        let s1 = d.step(&s0, 0.5, 0.0, 0.0, 0.0);
        assert!(s1.t > s0.t);
        assert!((s1.pos.x - s0.pos.x).abs() > 0.0);
    }

    #[test]
    fn derived_quantities_populated() {
        let s = FlightState::level(Vec3::new(0.0, 0.0, 1000.0), 150.0, 0.0);
        let d = FlightDerived::from_state(&s, &TerrainModel::sea_level());
        assert!(d.ias_knots > 200.0);
        assert!(d.mach > 0.4);
        assert!(d.terrain_clearance > 900.0);
    }

    #[test]
    fn vec3_scalar_math() {
        let v = Vec3::new(1.0, 2.0, 3.0) * 2.0;
        assert_eq!(v, Vec3::new(2.0, 4.0, 6.0));
        let w = Vec3::new(4.0, 4.0, 4.0) / 2.0;
        assert_eq!(w, Vec3::new(2.0, 2.0, 2.0));
    }
}