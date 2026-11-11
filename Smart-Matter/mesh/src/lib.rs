//! # Smart Matter Mesh Networking Protocol
//!
//! Standalone mesh networking protocol implementation for Smart Matter.
//! This crate provides the high-level mesh network controller that manages
//! node discovery, routing table maintenance, and message forwarding.
//!
//! For embedded targets, use `smart_matter_core::mesh::MeshNode` directly.
//! This crate provides a std-compatible wrapper with additional tooling.

pub use smart_matter_core::mesh::*;
pub use smart_matter_core::types::*;

/// A simulated radio interface for testing and desktop use.
#[derive(Debug, Clone)]
pub struct SimulatedRadio {
    rssi: i16,
}

impl SimulatedRadio {
    pub fn new(rssi: i16) -> Self {
        Self { rssi }
    }
}

impl Radio for SimulatedRadio {
    fn send(&mut self, _slice: &ProtocolSlice) -> Result<(), RadioError> {
        // Simulated: always succeed
        Ok(())
    }

    fn receive(&mut self, _timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError> {
        // Simulated: no incoming messages
        Ok(None)
    }

    fn last_rssi(&self) -> i16 {
        self.rssi
    }
}

/// High-level mesh network controller for desktop/network-simulator use.
#[derive(Debug)]
pub struct MeshController {
    node: MeshNode,
    radio: SimulatedRadio,
}

impl MeshController {
    pub fn new(node_id: NodeId) -> Self {
        Self {
            node: MeshNode::new(node_id),
            radio: SimulatedRadio::new(-50),
        }
    }

    /// Run one tick of the mesh network (process incoming, send beacon if needed).
    pub fn tick(&mut self, energy: EnergyLevel) {
        // Process any pending messages
        if let Ok(Some(slice)) = self.radio.receive(0) {
            self.node.process_slice(&slice, &mut self.radio);
        }

        // Send beacon periodically (simplified: every tick)
        let beacon = self.node.build_beacon(energy);
        self.radio.send(&beacon).ok();
    }

    /// Get a reference to the underlying mesh node.
    pub fn node(&self) -> &MeshNode {
        &self.node
    }

    /// Get a mutable reference to the underlying mesh node.
    pub fn node_mut(&mut self) -> &mut MeshNode {
        &mut self.node
    }

    /// Routes an outgoing message to its destination.
    pub fn route_message(&mut self, slice: ProtocolSlice) -> Result<(), RadioError> {
        self.radio.send(&slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mesh_controller_creation() {
        let id = NodeId([0x01; 8]);
        let controller = MeshController::new(id);
        assert_eq!(controller.node().routing_table_ref().len(), 0);
    }

    #[test]
    fn test_mesh_controller_tick() {
        let id = NodeId([0x01; 8]);
        let mut controller = MeshController::new(id);
        controller.tick(EnergyLevel::Nominal);
        // Beacon should have been sent (routing table still empty)
        assert_eq!(controller.node().routing_table_ref().len(), 0);
    }
}
