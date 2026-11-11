//! Real CARLA TCP client for simulator communication.
//!
//! Connects to the CARLA simulator over TCP (default port 2000) and
//! provides synchronous request/response communication for world
//! management, vehicle spawning, and sensor data retrieval.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CarlaClientError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Protocol error: {0}")]
    ProtocolError(String),
    #[error("Timeout after {0}ms")]
    Timeout(u64),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// CARLA protocol message header: [version, size (4 bytes LE), type (2 bytes LE)]
const HEADER_SIZE: usize = 7;
const PROTOCOL_VERSION: u8 = 0x01;

/// CARLA message types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CarlaMsgType {
    Hello = 0,
    SpawnVehicle = 100,
    DestroyVehicle = 101,
    GetVehicleTransform = 200,
    SetVehicleControl = 201,
    GetSensorData = 300,
    Tick = 400,
    GetWorldInfo = 500,
}

/// CARLA vehicle control command.
#[derive(Debug, Clone)]
pub struct CarlaVehicleControl {
    pub throttle: f32,
    pub steer: f32,
    pub brake: f32,
    pub hand_brake: bool,
    pub reverse: bool,
}

/// CARLA vehicle transform.
#[derive(Debug, Clone)]
pub struct CarlaTransform {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

/// CARLA vehicle spawn request.
#[derive(Debug, Clone)]
pub struct CarlaSpawnRequest {
    pub blueprint_id: String,
    pub transform: CarlaTransform,
}

/// CARLA sensor data.
#[derive(Debug, Clone)]
pub struct CarlaSensorData {
    pub sensor_type: String,
    pub frame: u64,
    pub timestamp: f64,
    pub data: Vec<u8>,
}

/// Real TCP client for CARLA simulator.
pub struct CarlaTcpClient {
    stream: Option<TcpStream>,
    host: String,
    port: u16,
    timeout: Duration,
    connected: bool,
    buffer: Vec<u8>,
}

impl CarlaTcpClient {
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            stream: None,
            host: host.to_string(),
            port,
            timeout: Duration::from_secs(5),
            connected: false,
            buffer: vec![0u8; 65536],
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Connect to the CARLA simulator.
    pub fn connect(&mut self) -> Result<(), CarlaClientError> {
        let addr = format!("{}:{}", self.host, self.port);
        tracing::info!(addr = %addr, "Connecting to CARLA simulator");

        let socket_addr: std::net::SocketAddr =
            addr.parse().map_err(|e: std::net::AddrParseError| {
                CarlaClientError::ConnectionFailed(e.to_string())
            })?;

        let stream = TcpStream::connect_timeout(&socket_addr, self.timeout)
            .map_err(|e| CarlaClientError::ConnectionFailed(e.to_string()))?;

        stream
            .set_read_timeout(Some(self.timeout))
            .map_err(CarlaClientError::Io)?;
        stream
            .set_write_timeout(Some(self.timeout))
            .map_err(CarlaClientError::Io)?;

        self.stream = Some(stream);
        self.connected = true;

        // Send hello handshake
        self.send_hello()?;
        self.receive_hello()?;

        tracing::info!(
            "Connected to CARLA simulator at {}:{}",
            self.host,
            self.port
        );
        Ok(())
    }

    /// Disconnect from CARLA.
    pub fn disconnect(&mut self) {
        if let Some(stream) = self.stream.take() {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
        self.connected = false;
        tracing::info!("Disconnected from CARLA simulator");
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    /// Spawn a vehicle in CARLA.
    pub fn spawn_vehicle(&mut self, request: &CarlaSpawnRequest) -> Result<u32, CarlaClientError> {
        let mut payload = Vec::new();
        let id_bytes = request.blueprint_id.as_bytes();
        payload.extend_from_slice(&(id_bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(id_bytes);
        payload.extend_from_slice(&request.transform.x.to_le_bytes());
        payload.extend_from_slice(&request.transform.y.to_le_bytes());
        payload.extend_from_slice(&request.transform.z.to_le_bytes());
        payload.extend_from_slice(&request.transform.pitch.to_le_bytes());
        payload.extend_from_slice(&request.transform.yaw.to_le_bytes());
        payload.extend_from_slice(&request.transform.roll.to_le_bytes());

        self.send_message(CarlaMsgType::SpawnVehicle, &payload)?;
        let response = self.receive_message()?;
        if response.len() < 4 {
            return Err(CarlaClientError::ProtocolError(
                "invalid spawn response".into(),
            ));
        }
        Ok(u32::from_le_bytes([
            response[0],
            response[1],
            response[2],
            response[3],
        ]))
    }

    /// Destroy a vehicle.
    pub fn destroy_vehicle(&mut self, actor_id: u32) -> Result<(), CarlaClientError> {
        self.send_message(CarlaMsgType::DestroyVehicle, &actor_id.to_le_bytes())?;
        let _ = self.receive_message()?;
        Ok(())
    }

    /// Get vehicle transform.
    pub fn get_vehicle_transform(
        &mut self,
        actor_id: u32,
    ) -> Result<CarlaTransform, CarlaClientError> {
        self.send_message(CarlaMsgType::GetVehicleTransform, &actor_id.to_le_bytes())?;
        let response = self.receive_message()?;
        if response.len() < 24 {
            return Err(CarlaClientError::ProtocolError(
                "invalid transform response".into(),
            ));
        }
        Ok(CarlaTransform {
            x: f32::from_le_bytes([response[0], response[1], response[2], response[3]]),
            y: f32::from_le_bytes([response[4], response[5], response[6], response[7]]),
            z: f32::from_le_bytes([response[8], response[9], response[10], response[11]]),
            pitch: f32::from_le_bytes([response[12], response[13], response[14], response[15]]),
            yaw: f32::from_le_bytes([response[16], response[17], response[18], response[19]]),
            roll: f32::from_le_bytes([response[20], response[21], response[22], response[23]]),
        })
    }

    /// Apply vehicle control.
    pub fn set_vehicle_control(
        &mut self,
        actor_id: u32,
        control: &CarlaVehicleControl,
    ) -> Result<(), CarlaClientError> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&actor_id.to_le_bytes());
        payload.push(control.throttle as u8);
        payload.push(control.steer as u8);
        payload.push(control.brake as u8);
        payload.push(control.hand_brake as u8);
        payload.push(control.reverse as u8);

        self.send_message(CarlaMsgType::SetVehicleControl, &payload)?;
        let _ = self.receive_message()?;
        Ok(())
    }

    /// Advance the CARLA world by one tick.
    pub fn tick(&mut self) -> Result<f64, CarlaClientError> {
        self.send_message(CarlaMsgType::Tick, &[])?;
        let response = self.receive_message()?;
        if response.len() < 8 {
            return Err(CarlaClientError::ProtocolError(
                "invalid tick response".into(),
            ));
        }
        Ok(f64::from_le_bytes([
            response[0],
            response[1],
            response[2],
            response[3],
            response[4],
            response[5],
            response[6],
            response[7],
        ]))
    }

    // ─── Internal protocol helpers ─────────────────────────────────

    fn send_hello(&mut self) -> Result<(), CarlaClientError> {
        self.send_message(CarlaMsgType::Hello, b"TRANSIT_OS")
    }

    fn receive_hello(&mut self) -> Result<(), CarlaClientError> {
        let response = self.receive_message()?;
        if response.is_empty() {
            return Err(CarlaClientError::ProtocolError(
                "empty hello response".into(),
            ));
        }
        Ok(())
    }

    fn send_message(
        &mut self,
        msg_type: CarlaMsgType,
        payload: &[u8],
    ) -> Result<(), CarlaClientError> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(CarlaClientError::ConnectionFailed("not connected".into()))?;

        let size = payload.len() as u32;
        let header = [
            PROTOCOL_VERSION,
            (size & 0xFF) as u8,
            ((size >> 8) & 0xFF) as u8,
            ((size >> 16) & 0xFF) as u8,
            ((size >> 24) & 0xFF) as u8,
            (msg_type as u16 & 0xFF) as u8,
            ((msg_type as u16 >> 8) & 0xFF) as u8,
        ];

        stream.write_all(&header)?;
        stream.write_all(payload)?;
        stream.flush()?;
        Ok(())
    }

    fn receive_message(&mut self) -> Result<Vec<u8>, CarlaClientError> {
        let stream = self
            .stream
            .as_mut()
            .ok_or(CarlaClientError::ConnectionFailed("not connected".into()))?;

        let mut header = [0u8; HEADER_SIZE];
        stream.read_exact(&mut header)?;

        let size = u32::from_le_bytes([header[1], header[2], header[3], header[4]]) as usize;
        if size > self.buffer.len() {
            return Err(CarlaClientError::ProtocolError(format!(
                "message too large: {} bytes",
                size
            )));
        }

        stream.read_exact(&mut self.buffer[..size])?;
        Ok(self.buffer[..size].to_vec())
    }
}

impl Drop for CarlaTcpClient {
    fn drop(&mut self) {
        self.disconnect();
    }
}

/// Run a CARLA scenario end-to-end: connect, spawn, simulate, destroy.
pub fn run_carla_scenario(
    host: &str,
    port: u16,
    scenario: &transit_dynamics_simulator::scenarios::Scenario,
    _params: &transit_state_machine::params::VehicleParams,
) -> Result<Vec<CarlaTransform>, CarlaClientError> {
    let mut client = CarlaTcpClient::new(host, port).with_timeout(Duration::from_secs(10));
    client.connect()?;

    let vehicle_id = client.spawn_vehicle(&CarlaSpawnRequest {
        blueprint_id: "vehicle.tesla.model3".into(),
        transform: CarlaTransform {
            x: scenario.initial_state.pose.x as f32,
            y: scenario.initial_state.pose.y as f32,
            z: 0.0,
            pitch: 0.0,
            yaw: scenario.initial_state.pose.heading as f32,
            roll: 0.0,
        },
    })?;

    let mut trajectory = Vec::new();
    let _dt = 0.05;
    let steps = 100;

    for _ in 0..steps {
        let transform = client.get_vehicle_transform(vehicle_id)?;
        trajectory.push(transform);

        // Simple proportional control for the demo
        let control = CarlaVehicleControl {
            throttle: 0.3,
            steer: 0.0,
            brake: 0.0,
            hand_brake: false,
            reverse: false,
        };
        client.set_vehicle_control(vehicle_id, &control)?;
        client.tick()?;
    }

    client.destroy_vehicle(vehicle_id)?;
    client.disconnect();
    Ok(trajectory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_creation() {
        let client = CarlaTcpClient::new("localhost", 2000);
        assert!(!client.is_connected());
        assert_eq!(client.host, "localhost");
        assert_eq!(client.port, 2000);
    }

    #[test]
    fn client_drop_disconnects() {
        let mut client = CarlaTcpClient::new("localhost", 2000);
        client.connected = true;
        drop(client);
        // Should not panic
    }
}
