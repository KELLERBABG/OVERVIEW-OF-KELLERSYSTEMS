//! Sensor drivers with self-test, calibration, and health monitoring.
//!
//! **IMPORTANT: This module is a framework, not an implementation.**
//!
//! Each driver struct (LidarDriver, RadarDriver, etc.) provides:
//! - Constructor with hardware configuration parameters
//! - `SensorDriver` trait implementation with health checking
//! - Getter methods for hardware-specific configuration
//!
//! However, the `read()` methods return **empty data** — they are templates
//! that must be filled in by the user for their specific hardware. To use
//! Transit OS on real hardware:
//!
//! 1. **Implement your own sensor driver** that reads from your hardware
//!    (SPI, I2C, USB, network socket, etc.)
//! 2. **Return real sensor data** in the `read()` method
//! 3. **Configure trust levels** appropriately for your sensor suite
//! 4. **Wire the driver** into the verification pipeline via `SensorFusionUnit`
//!
//! The provided drivers are suitable for:
//! - Unit testing and integration testing
//! - Simulation environments
//! - Hardware-in-the-loop (HIL) testing with mocked sensor data
//! - Learning the API before implementing real drivers
//!
//! # Example (custom driver)
//!
//! ```ignore
//! use transit_hal::sensor::{SensorDriver, SensorReading, HealthStatus, SensorError};
//! use transit_taint_analysis::sensors::{SensorSource, SensorType};
//!
//! pub struct MyLidarDriver {
//!     port: u16,
//!     health: HealthStatus,
//! }
//!
//! impl SensorDriver for MyLidarDriver {
//!     type Reading = Vec<MyLidarPoint>;
//!
//!     fn init(&mut self) -> Result<(), SensorError> {
//!         // Connect to your LiDAR hardware
//!         self.health = HealthStatus::Healthy;
//!         Ok(())
//!     }
//!
//!     fn read(&mut self) -> Result<SensorReading<Vec<MyLidarPoint>>, SensorError> {
//!         // Read from your actual hardware
//!         let points = self.read_from_hardware()?;
//!         Ok(SensorReading {
//!             value: points,
//!             timestamp_us: std::time::SystemTime::now()
//!                 .duration_since(std::time::UNIX_EPOCH)
//!                 .unwrap()
//!                 .as_micros() as u64,
//!             health: self.health,
//!             source: SensorSource {
//!                 sensor: SensorType::Lidar,
//!                 id: "my_lidar_0".into(),
//!             },
//!         })
//!     }
//!
//!     // ... other trait methods
//! }
//! ```

use serde::{Deserialize, Serialize};
use thiserror::Error;
use transit_taint_analysis::sensors::{SensorSource, SensorType};

#[derive(Error, Debug)]
pub enum SensorError {
    #[error("Sensor not responding: {0}")]
    NotResponding(String),
    #[error("Calibration failed: {0}")]
    CalibrationFailed(String),
    #[error("Data out of range: {detail}")]
    OutOfRange { detail: String },
    #[error("Self-test failed: {0}")]
    SelfTestFailed(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Failed,
    NotInitialized,
}

/// Unified sensor reading with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorReading<T> {
    pub value: T,
    pub timestamp_us: u64,
    pub health: HealthStatus,
    pub source: SensorSource,
}

/// Trait for all hardware sensor drivers.
///
/// **This is the interface users must implement for their hardware.**
///
/// The provided implementations in this crate are **templates** that return
/// empty data. Users must override `read()` with actual hardware access.
pub trait SensorDriver {
    type Reading;

    /// Initialize the sensor hardware.
    fn init(&mut self) -> Result<(), SensorError>;

    /// Read a measurement from the sensor.
    ///
    /// **NOTE: The provided implementations return empty data.**
    /// Override this method with your actual hardware read logic.
    fn read(&mut self) -> Result<SensorReading<Self::Reading>, SensorError>;

    /// Run built-in self-test (BIST).
    fn self_test(&mut self) -> Result<HealthStatus, SensorError>;

    /// Return the sensor type for taint tracking.
    fn sensor_type(&self) -> SensorType;

    /// Return the sensor's trust level (0.0 – 1.0).
    fn trust_level(&self) -> f64;

    /// Return current health status.
    fn health(&self) -> HealthStatus;
}

// ─── LiDAR ────────────────────────────────────────────────────────

/// LiDAR sensor driver (template — implement read() for your hardware).
///
/// Trust level: 0.95 (highest among sensors).
pub struct LidarDriver {
    health: HealthStatus,
    id: String,
    fov_deg: f64,
    max_range: f64,
}

impl LidarDriver {
    pub fn new(id: &str, fov_deg: f64, max_range: f64) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
            fov_deg,
            max_range,
        }
    }

    pub fn fov_deg(&self) -> f64 {
        self.fov_deg
    }

    pub fn max_range(&self) -> f64 {
        self.max_range
    }
}

impl SensorDriver for LidarDriver {
    type Reading = Vec<LidarPoint>;

    fn init(&mut self) -> Result<(), SensorError> {
        tracing::info!(sensor = %self.id, "Initializing LiDAR");
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<Vec<LidarPoint>>, SensorError> {
        // TEMPLATE: Replace with actual hardware read
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }
        Ok(SensorReading {
            value: Vec::new(),
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Lidar,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        tracing::info!(sensor = %self.id, "Running LiDAR self-test");
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Lidar
    }

    fn trust_level(&self) -> f64 {
        0.95
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

// ─── Radar ────────────────────────────────────────────────────────

/// Radar sensor driver (template — implement read() for your hardware).
///
/// Trust level: 0.90.
pub struct RadarDriver {
    health: HealthStatus,
    id: String,
    max_range: f64,
    update_rate_hz: f64,
}

impl RadarDriver {
    pub fn new(id: &str, max_range: f64, update_rate_hz: f64) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
            max_range,
            update_rate_hz,
        }
    }

    pub fn max_range(&self) -> f64 {
        self.max_range
    }

    pub fn update_rate_hz(&self) -> f64 {
        self.update_rate_hz
    }
}

impl SensorDriver for RadarDriver {
    type Reading = Vec<RadarTarget>;

    fn init(&mut self) -> Result<(), SensorError> {
        tracing::info!(sensor = %self.id, "Initializing radar");
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<Vec<RadarTarget>>, SensorError> {
        // TEMPLATE: Replace with actual hardware read
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }
        Ok(SensorReading {
            value: Vec::new(),
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Radar,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        tracing::info!(sensor = %self.id, "Running radar self-test");
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Radar
    }

    fn trust_level(&self) -> f64 {
        0.90
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

// ─── Camera ───────────────────────────────────────────────────────

/// Camera sensor driver (template — implement read() for your hardware).
///
/// Trust level: 0.75 (lowest among sensors due to spoofing risk).
pub struct CameraDriver {
    health: HealthStatus,
    id: String,
    resolution: (u32, u32),
    fov_deg: f64,
}

impl CameraDriver {
    pub fn new(id: &str, width: u32, height: u32, fov_deg: f64) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
            resolution: (width, height),
            fov_deg,
        }
    }

    pub fn fov_deg(&self) -> f64 {
        self.fov_deg
    }

    pub fn resolution(&self) -> (u32, u32) {
        self.resolution
    }
}

impl SensorDriver for CameraDriver {
    type Reading = Vec<CameraDetection>;

    fn init(&mut self) -> Result<(), SensorError> {
        tracing::info!(
            sensor = %self.id,
            width = self.resolution.0,
            height = self.resolution.1,
            "Initializing camera"
        );
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<Vec<CameraDetection>>, SensorError> {
        // TEMPLATE: Replace with actual hardware read
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }
        Ok(SensorReading {
            value: Vec::new(),
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Camera,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        tracing::info!(sensor = %self.id, "Running camera self-test");
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Camera
    }

    fn trust_level(&self) -> f64 {
        0.75
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

// ─── IMU ──────────────────────────────────────────────────────────

/// IMU sensor driver (template — implement read() for your hardware).
///
/// Trust level: 0.92.
pub struct ImuDriver {
    health: HealthStatus,
    id: String,
}

impl ImuDriver {
    pub fn new(id: &str) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
        }
    }
}

impl SensorDriver for ImuDriver {
    type Reading = ImuReading;

    fn init(&mut self) -> Result<(), SensorError> {
        tracing::info!(sensor = %self.id, "Initializing IMU");
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<ImuReading>, SensorError> {
        // TEMPLATE: Replace with actual hardware read
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }
        Ok(SensorReading {
            value: ImuReading {
                acceleration: [0.0; 3],
                gyroscope: [0.0; 3],
                magnetic_field: [0.0; 3],
            },
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Imu,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        tracing::info!(sensor = %self.id, "Running IMU self-test");
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Imu
    }

    fn trust_level(&self) -> f64 {
        0.92
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

// ─── GPS/GNSS ─────────────────────────────────────────────────────

/// GPS/GNSS sensor driver (template — implement read() for your hardware).
///
/// Trust level: 0.85.
pub struct GpsDriver {
    health: HealthStatus,
    id: String,
}

impl GpsDriver {
    pub fn new(id: &str) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
        }
    }
}

impl SensorDriver for GpsDriver {
    type Reading = GpsReading;

    fn init(&mut self) -> Result<(), SensorError> {
        tracing::info!(sensor = %self.id, "Initializing GPS/GNSS");
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<GpsReading>, SensorError> {
        // TEMPLATE: Replace with actual hardware read
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }
        Ok(SensorReading {
            value: GpsReading {
                latitude: 0.0,
                longitude: 0.0,
                altitude: 0.0,
                heading: 0.0,
                speed: 0.0,
                satellites: 0,
                hdop: 99.9,
            },
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Gps,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Gps
    }

    fn trust_level(&self) -> f64 {
        0.85
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

// ─── Wheel Encoder ────────────────────────────────────────────────

/// Wheel encoder sensor driver (template — implement read() for your hardware).
///
/// Trust level: 0.93.
pub struct WheelEncoderDriver {
    health: HealthStatus,
    id: String,
    wheel_id: u8,
    ticks_per_rev: u32,
}

impl WheelEncoderDriver {
    pub fn new(id: &str, wheel_id: u8, ticks_per_rev: u32) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
            wheel_id,
            ticks_per_rev,
        }
    }

    pub fn ticks_per_rev(&self) -> u32 {
        self.ticks_per_rev
    }

    pub fn wheel_id(&self) -> u8 {
        self.wheel_id
    }
}

impl SensorDriver for WheelEncoderDriver {
    type Reading = WheelEncoderReading;

    fn init(&mut self) -> Result<(), SensorError> {
        tracing::info!(sensor = %self.id, wheel = self.wheel_id, "Initializing wheel encoder");
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<WheelEncoderReading>, SensorError> {
        // TEMPLATE: Replace with actual hardware read
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }
        Ok(SensorReading {
            value: WheelEncoderReading {
                ticks: 0,
                rpm: 0.0,
                wheel_id: self.wheel_id,
            },
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::WheelSpeed,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::WheelSpeed
    }

    fn trust_level(&self) -> f64 {
        0.93
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

// ─── Data types ───────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LidarPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub intensity: f32,
    pub reflectivity: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RadarTarget {
    pub range: f64,
    pub velocity: f64,
    pub azimuth: f64,
    pub radar_cross_section: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CameraDetection {
    pub bbox: [f64; 4],
    pub class: String,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImuReading {
    pub acceleration: [f64; 3],
    pub gyroscope: [f64; 3],
    pub magnetic_field: [f64; 3],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsReading {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude: f64,
    pub heading: f64,
    pub speed: f64,
    pub satellites: u32,
    pub hdop: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WheelEncoderReading {
    pub ticks: u64,
    pub rpm: f64,
    pub wheel_id: u8,
}

/// Aggregate all vehicle sensors into a single VehicleState for the gate.
pub struct SensorFusionUnit {
    pub lidar: Vec<Box<dyn SensorDriver<Reading = Vec<LidarPoint>>>>,
    pub radar: Vec<Box<dyn SensorDriver<Reading = Vec<RadarTarget>>>>,
    pub camera: Vec<Box<dyn SensorDriver<Reading = Vec<CameraDetection>>>>,
    pub imu: Vec<Box<dyn SensorDriver<Reading = ImuReading>>>,
    pub gps: Vec<Box<dyn SensorDriver<Reading = GpsReading>>>,
    pub encoders: Vec<Box<dyn SensorDriver<Reading = WheelEncoderReading>>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lidar_driver_init_and_read() {
        let mut driver = LidarDriver::new("lidar_front", 360.0, 200.0);
        assert_eq!(driver.health(), HealthStatus::NotInitialized);
        driver.init().unwrap();
        assert_eq!(driver.health(), HealthStatus::Healthy);
        let reading = driver.read().unwrap();
        assert_eq!(reading.health, HealthStatus::Healthy);
        assert_eq!(reading.source.sensor, SensorType::Lidar);
    }

    #[test]
    fn radar_trust_level() {
        let driver = RadarDriver::new("radar_front", 250.0, 20.0);
        assert_eq!(driver.trust_level(), 0.90);
    }

    #[test]
    fn camera_self_test() {
        let mut driver = CameraDriver::new("cam_front", 1920, 1080, 120.0);
        let status = driver.self_test().unwrap();
        assert_eq!(status, HealthStatus::Healthy);
    }

    #[test]
    fn imu_init_and_read() {
        let mut driver = ImuDriver::new("imu_0");
        driver.init().unwrap();
        let reading = driver.read().unwrap();
        assert_eq!(reading.source.sensor, SensorType::Imu);
    }
}
