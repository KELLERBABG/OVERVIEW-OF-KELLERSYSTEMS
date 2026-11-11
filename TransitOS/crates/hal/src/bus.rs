//! Communication bus abstraction for CAN and Ethernet.
//!
//! All in-memory bus adapters enforce a configurable `max_queue_depth` to
//! prevent unbounded memory growth under sustained high-frequency traffic.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BusError {
    #[error("Bus not initialized")]
    NotInitialized,
    #[error("Write failed: {0}")]
    WriteFailed(String),
    #[error("Bus timeout")]
    Timeout,
    #[error("Bus buffer full (max depth {max_depth})")]
    Full { max_depth: usize },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BusStatus {
    Up,
    Degraded,
    Down,
}

/// Trait for communication bus adapters.
pub trait BusAdapter {
    fn init(&mut self) -> Result<(), BusError>;
    fn write(&mut self, id: u32, data: &[u8]) -> Result<(), BusError>;
    fn read(&mut self) -> Result<Option<(u32, Vec<u8>)>, BusError>;
    fn status(&self) -> BusStatus;
}

/// In-memory CAN bus for testing and simulation.
///
/// Enforces `max_queue_depth` on both TX and RX buffers. When a buffer is
/// full, `write()` / `inject()` returns `BusError::Full`.
pub struct InMemoryCanBus {
    status: BusStatus,
    tx_buffer: Vec<(u32, Vec<u8>)>,
    rx_buffer: Vec<(u32, Vec<u8>)>,
    max_queue_depth: usize,
}

impl InMemoryCanBus {
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    pub fn with_capacity(max_queue_depth: usize) -> Self {
        Self {
            status: BusStatus::Up,
            tx_buffer: Vec::with_capacity(max_queue_depth.min(256)),
            rx_buffer: Vec::with_capacity(max_queue_depth.min(256)),
            max_queue_depth,
        }
    }

    /// Inject a frame into the RX buffer (for testing).
    pub fn inject(&mut self, id: u32, data: Vec<u8>) -> Result<(), BusError> {
        if self.rx_buffer.len() >= self.max_queue_depth {
            return Err(BusError::Full {
                max_depth: self.max_queue_depth,
            });
        }
        self.rx_buffer.push((id, data));
        Ok(())
    }

    /// Drain all transmitted frames (for testing).
    pub fn drain_tx(&mut self) -> Vec<(u32, Vec<u8>)> {
        std::mem::take(&mut self.tx_buffer)
    }

    pub fn tx_len(&self) -> usize {
        self.tx_buffer.len()
    }

    pub fn rx_len(&self) -> usize {
        self.rx_buffer.len()
    }
}

impl Default for InMemoryCanBus {
    fn default() -> Self {
        Self::new()
    }
}

impl BusAdapter for InMemoryCanBus {
    fn init(&mut self) -> Result<(), BusError> {
        self.status = BusStatus::Up;
        Ok(())
    }

    fn write(&mut self, id: u32, data: &[u8]) -> Result<(), BusError> {
        if self.tx_buffer.len() >= self.max_queue_depth {
            return Err(BusError::Full {
                max_depth: self.max_queue_depth,
            });
        }
        self.tx_buffer.push((id, data.to_vec()));
        Ok(())
    }

    fn read(&mut self) -> Result<Option<(u32, Vec<u8>)>, BusError> {
        Ok(self.rx_buffer.pop())
    }

    fn status(&self) -> BusStatus {
        self.status
    }
}

/// Ethernet bus adapter trait (for high-bandwidth sensor data).
pub trait EthernetAdapter: BusAdapter {
    fn connect(&mut self, addr: &str) -> Result<(), BusError>;
    fn disconnect(&mut self) -> Result<(), BusError>;
}

/// In-memory Ethernet bus for testing.
pub struct InMemoryEthernetBus {
    status: BusStatus,
    connected: bool,
    tx_buffer: Vec<(u32, Vec<u8>)>,
    rx_buffer: Vec<(u32, Vec<u8>)>,
    max_queue_depth: usize,
}

impl InMemoryEthernetBus {
    pub fn new() -> Self {
        Self::with_capacity(4096)
    }

    pub fn with_capacity(max_queue_depth: usize) -> Self {
        Self {
            status: BusStatus::Down,
            connected: false,
            tx_buffer: Vec::with_capacity(max_queue_depth.min(256)),
            rx_buffer: Vec::with_capacity(max_queue_depth.min(256)),
            max_queue_depth,
        }
    }

    pub fn inject(&mut self, id: u32, data: Vec<u8>) -> Result<(), BusError> {
        if self.rx_buffer.len() >= self.max_queue_depth {
            return Err(BusError::Full {
                max_depth: self.max_queue_depth,
            });
        }
        self.rx_buffer.push((id, data));
        Ok(())
    }
}

impl Default for InMemoryEthernetBus {
    fn default() -> Self {
        Self::new()
    }
}

impl BusAdapter for InMemoryEthernetBus {
    fn init(&mut self) -> Result<(), BusError> {
        Ok(())
    }

    fn write(&mut self, id: u32, data: &[u8]) -> Result<(), BusError> {
        if !self.connected {
            return Err(BusError::NotInitialized);
        }
        if self.tx_buffer.len() >= self.max_queue_depth {
            return Err(BusError::Full {
                max_depth: self.max_queue_depth,
            });
        }
        self.tx_buffer.push((id, data.to_vec()));
        Ok(())
    }

    fn read(&mut self) -> Result<Option<(u32, Vec<u8>)>, BusError> {
        Ok(self.rx_buffer.pop())
    }

    fn status(&self) -> BusStatus {
        self.status
    }
}

impl EthernetAdapter for InMemoryEthernetBus {
    fn connect(&mut self, _addr: &str) -> Result<(), BusError> {
        self.connected = true;
        self.status = BusStatus::Up;
        Ok(())
    }

    fn disconnect(&mut self) -> Result<(), BusError> {
        self.connected = false;
        self.status = BusStatus::Down;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_can_bus_roundtrip() {
        let mut bus = InMemoryCanBus::new();
        bus.init().unwrap();
        bus.inject(0x100, vec![1, 2, 3]).unwrap();
        let frame = bus.read().unwrap().unwrap();
        assert_eq!(frame.0, 0x100);
        assert_eq!(frame.1, vec![1, 2, 3]);

        bus.write(0x200, &[4, 5]).unwrap();
        let tx = bus.drain_tx();
        assert_eq!(tx.len(), 1);
        assert_eq!(tx[0].0, 0x200);
    }

    #[test]
    fn can_bus_rejects_when_full() {
        let mut bus = InMemoryCanBus::with_capacity(2);
        bus.init().unwrap();
        bus.write(0x01, &[1]).unwrap();
        bus.write(0x02, &[2]).unwrap();
        let err = bus.write(0x03, &[3]).unwrap_err();
        assert!(matches!(err, BusError::Full { max_depth: 2 }));
    }

    #[test]
    fn can_bus_inject_rejects_when_full() {
        let mut bus = InMemoryCanBus::with_capacity(1);
        bus.init().unwrap();
        bus.inject(0x01, vec![1]).unwrap();
        let err = bus.inject(0x02, vec![2]).unwrap_err();
        assert!(matches!(err, BusError::Full { .. }));
    }

    #[test]
    fn ethernet_connect_disconnect() {
        let mut bus = InMemoryEthernetBus::new();
        assert_eq!(bus.status(), BusStatus::Down);
        bus.connect("192.168.1.1:8080").unwrap();
        assert_eq!(bus.status(), BusStatus::Up);
        bus.disconnect().unwrap();
        assert_eq!(bus.status(), BusStatus::Down);
    }

    #[test]
    fn ethernet_bus_rejects_when_full() {
        let mut bus = InMemoryEthernetBus::with_capacity(2);
        bus.connect("test").unwrap();
        bus.write(0x01, &[1]).unwrap();
        bus.write(0x02, &[2]).unwrap();
        let err = bus.write(0x03, &[3]).unwrap_err();
        assert!(matches!(err, BusError::Full { max_depth: 2 }));
    }
}
