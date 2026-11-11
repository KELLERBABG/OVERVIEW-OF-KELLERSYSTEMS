//! RTK-GNSS & 9-DoF IMU Georeferencing Transformations

use crate::{Point3, Vector3, Quaternion};

/// Represents absolute geodetic and attitude state of a MuonScope panel station.
#[derive(Debug, Clone, Copy)]
pub struct StationAttitude {
    pub latitude_deg: f64,
    pub longitude_deg: f64,
    pub altitude_m: f64,
    pub orientation: Quaternion, // Rotation from Local Detector Frame -> Global UTM Frame
    pub utm_translation_m: Vector3,
}

impl StationAttitude {
    pub fn new(
        latitude_deg: f64,
        longitude_deg: f64,
        altitude_m: f64,
        yaw_deg: f64,
        pitch_deg: f64,
        roll_deg: f64,
    ) -> Self {
        let yaw = yaw_deg.to_radians();
        let pitch = pitch_deg.to_radians();
        let roll = roll_deg.to_radians();

        let q = Quaternion::from_euler_angles(roll, pitch, yaw);

        Self {
            latitude_deg,
            longitude_deg,
            altitude_m,
            orientation: q,
            utm_translation_m: Vector3::new(0.0, 0.0, altitude_m),
        }
    }

    /// Transforms a local detector point (in mm) into the global UTM coordinate frame (in meters).
    pub fn transform_local_to_global(&self, local_point_mm: &Point3) -> Point3 {
        let local_m = Vector3::new(
            local_point_mm.x / 1000.0,
            local_point_mm.y / 1000.0,
            local_point_mm.z / 1000.0,
        );

        let rotated = self.orientation.transform_vector(&local_m);
        Point3::from(rotated + self.utm_translation_m)
    }

    /// Transforms a local unit direction vector into the global frame.
    pub fn transform_direction_to_global(&self, local_dir: &Vector3) -> Vector3 {
        self.orientation.transform_vector(local_dir)
    }
}
