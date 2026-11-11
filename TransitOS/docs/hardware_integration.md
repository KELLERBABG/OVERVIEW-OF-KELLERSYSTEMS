# Hardware Integration Guide

**Version 1.0.0** | **Implementing custom sensor drivers for Transit OS**

## Overview

Transit OS provides a Hardware Abstraction Layer (HAL) with sensor driver
templates. This guide explains how to implement real hardware drivers for
your specific sensors.

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Transit OS HAL                            │
├─────────────────────────────────────────────────────────────┤
│  SensorDriver Trait                                         │
│  ├── init() → Result<(), SensorError>                      │
│  ├── read() → Result<SensorReading<T>, SensorError>        │
│  ├── self_test() → Result<HealthStatus, SensorError>       │
│  ├── sensor_type() → SensorType                             │
│  ├── trust_level() → f64                                    │
│  └── health() → HealthStatus                                │
├─────────────────────────────────────────────────────────────┤
│  Your Implementation                                        │
│  ├── Hardware communication (SPI/I2C/USB/Network)          │
│  ├── Data parsing and validation                            │
│  ├── Error handling and recovery                            │
│  └── Health monitoring                                      │
└─────────────────────────────────────────────────────────────┘
```

## Implementing a LiDAR Driver

### Step 1: Define Your Data Structure

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyLidarPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub intensity: f32,
    pub reflectivity: f32,
    pub ring: u8,  // LiDAR ring/layer
}
```

### Step 2: Implement the SensorDriver Trait

```rust
use transit_hal::sensor::{
    SensorDriver, SensorReading, HealthStatus, SensorError, LidarDriver
};
use transit_taint_analysis::sensors::{SensorSource, SensorType};

pub struct MyLidarDriver {
    health: HealthStatus,
    id: String,
    port: u16,
    socket: Option<TcpStream>,
    fov_deg: f64,
    max_range: f64,
}

impl MyLidarDriver {
    pub fn new(id: &str, port: u16, fov_deg: f64, max_range: f64) -> Self {
        Self {
            health: HealthStatus::NotInitialized,
            id: id.to_string(),
            port,
            socket: None,
            fov_deg,
            max_range,
        }
    }
}

impl SensorDriver for MyLidarDriver {
    type Reading = Vec<MyLidarPoint>;

    fn init(&mut self) -> Result<(), SensorError> {
        // Connect to your LiDAR hardware
        let addr = format!("192.168.1.100:{}", self.port);
        self.socket = Some(
            TcpStream::connect(&addr)
                .map_err(|e| SensorError::NotResponding(format!("LiDAR connect failed: {}", e)))?
        );
        self.health = HealthStatus::Healthy;
        tracing::info!(sensor = %self.id, "LiDAR connected on port {}", self.port);
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<Vec<MyLidarPoint>>, SensorError> {
        if self.health != HealthStatus::Healthy {
            return Err(SensorError::NotResponding(self.id.clone()));
        }

        let socket = self.socket.as_mut()
            .ok_or_else(|| SensorError::NotResponding("LiDAR not connected".into()))?;

        // Read data from your LiDAR
        let mut buffer = vec![0u8; 4096];
        let bytes_read = socket.read(&mut buffer)
            .map_err(|e| SensorError::NotResponding(format!("Read failed: {}", e)))?;

        // Parse your LiDAR protocol
        let points = self.parse_lidar_data(&buffer[..bytes_read])?;

        Ok(SensorReading {
            value: points,
            timestamp_us: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_micros() as u64,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Lidar,
                id: self.id.clone(),
            },
        })
    }

    fn self_test(&mut self) -> Result<HealthStatus, SensorError> {
        // Send self-test command to LiDAR
        // Check response
        self.health = HealthStatus::Healthy;
        Ok(self.health)
    }

    fn sensor_type(&self) -> SensorType {
        SensorType::Lidar
    }

    fn trust_level(&self) -> f64 {
        0.95  // LiDAR has highest trust
    }

    fn health(&self) -> HealthStatus {
        self.health
    }
}

impl MyLidarDriver {
    fn parse_lidar_data(&self, data: &[u8]) -> Result<Vec<MyLidarPoint>, SensorError> {
        // Implement your LiDAR protocol parsing here
        // This is vendor-specific
        Ok(Vec::new())
    }
}
```

## Implementing a Radar Driver

```rust
pub struct MyRadarDriver {
    health: HealthStatus,
    id: String,
    can_id: u32,
    bus: InMemoryCanBus,
}

impl SensorDriver for MyRadarDriver {
    type Reading = Vec<RadarTarget>;

    fn init(&mut self) -> Result<(), SensorError> {
        self.bus.init()?;
        self.health = HealthStatus::Healthy;
        Ok(())
    }

    fn read(&mut self) -> Result<SensorReading<Vec<RadarTarget>>, SensorError> {
        let mut targets = Vec::new();

        // Read from CAN bus
        while let Some((id, data)) = self.bus.read()
            .map_err(|e| SensorError::NotResponding(e.to_string()))? 
        {
            if id == self.can_id && data.len() >= 8 {
                let target = self.parse_can_frame(&data)?;
                targets.push(target);
            }
        }

        Ok(SensorReading {
            value: targets,
            timestamp_us: 0,
            health: self.health,
            source: SensorSource {
                sensor: SensorType::Radar,
                id: self.id.clone(),
            },
        })
    }

    fn trust_level(&self) -> f64 {
        0.90
    }
    // ... other methods
}
```

## Trust Levels

Each sensor type has an inherent trust level based on its spoofing resistance:

| Sensor | Trust Level | Rationale |
|---|---|---|
| LiDAR | 0.95 | Difficult to spoof (physical laser) |
| Wheel Encoder | 0.93 | Direct mechanical measurement |
| IMU | 0.92 | Physical inertial measurement |
| Radar | 0.90 | Radio frequency, moderate spoofing risk |
| GPS | 0.85 | Known spoofing vulnerabilities |
| Camera | 0.75 | Highest spoofing risk (adversarial attacks) |
| V2X | 0.50 | Communication-based, easily spoofed |

## Error Handling

### Sensor Errors

```rust
pub enum SensorError {
    NotResponding(String),      // Hardware not responding
    CalibrationFailed(String),  // Calibration error
    OutOfRange { detail: String },  // Data out of expected range
    SelfTestFailed(String),     // BIST failed
}
```

### Recovery Strategies

1. **Transient Errors:** Retry with exponential backoff
2. **Persistent Errors:** Mark sensor as degraded, reduce trust level
3. **Critical Errors:** Trigger MRM, bring vehicle to safe state

## Testing Your Driver

### Unit Tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lidar_init() {
        let mut driver = MyLidarDriver::new("test_lidar", 2369, 360.0, 200.0);
        assert_eq!(driver.health(), HealthStatus::NotInitialized);
        driver.init().unwrap();
        assert_eq!(driver.health(), HealthStatus::Healthy);
    }

    #[test]
    fn test_lidar_read() {
        let mut driver = MyLidarDriver::new("test_lidar", 2369, 360.0, 200.0);
        driver.init().unwrap();
        let reading = driver.read().unwrap();
        assert_eq!(reading.source.sensor, SensorType::Lidar);
    }
}
```

### Integration Tests

```rust
#[test]
fn test_lidar_in_pipeline() {
    // Create your driver
    let mut lidar = MyLidarDriver::new("lidar_0", 2369, 360.0, 200.0);
    lidar.init().unwrap();

    // Create other sensors
    let mut radar = MyRadarDriver::new("radar_0");
    radar.init().unwrap();

    // Create fusion unit
    let fusion = SensorFusionUnit {
        lidar: vec![Box::new(lidar)],
        radar: vec![Box::new(radar)],
        // ... other sensors
    };

    // Use in verification pipeline
    // ...
}
```

## Performance Considerations

- **Latency:** Sensor reads should complete within 1ms for real-time operation
- **Throughput:** LiDAR can produce 300K+ points/second
- **Memory:** Pre-allocate buffers for sensor data
- **Concurrency:** Use separate threads for each sensor

## References

- [Sensor Driver Examples](../crates/hal/src/sensor.rs)
- [CAN Bus Integration](can_bus_integration.md)
- [ROS2 Integration](ros2_integration.md)
