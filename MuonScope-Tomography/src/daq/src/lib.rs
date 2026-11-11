//! # MuonScope DAQ - High-Throughput Edge Data Acquisition & Tracking Engine

pub mod ring_buffer;
pub mod packet_decoder;
pub mod barycenter;
pub mod track_kalman;
pub mod coordinate_frame;
pub mod poca_core;
pub mod flux_normalizer;
pub mod sam_t_mesh;
pub mod storage;
pub mod calibration;

use serde::{Deserialize, Serialize};

pub type Vector3 = nalgebra::Vector3<f64>;
pub type Point3 = nalgebra::Point3<f64>;
pub type Quaternion = nalgebra::UnitQuaternion<f64>;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct PlaneHit {
    pub plane_id: u8,
    pub x_mm: f64,
    pub y_mm: f64,
    pub z_mm: f64,
    pub charge_adc: u16,
    pub timestamp_ps: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackSegment {
    pub origin_mm: Point3,
    pub direction: Vector3,
    pub chi2_ndf: f64,
    pub timestamp_utc_ns: u64,
    pub hits: Vec<PlaneHit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorrelatedEvent {
    pub event_id: u64,
    pub ingress_origin: Point3,
    pub ingress_dir: Vector3,
    pub egress_origin: Point3,
    pub egress_dir: Vector3,
    pub poca_point: Point3,
    pub scattering_angle_rad: f64,
    pub miss_distance_mm: f64,
    pub timestamp_utc_ns: u64,
}
