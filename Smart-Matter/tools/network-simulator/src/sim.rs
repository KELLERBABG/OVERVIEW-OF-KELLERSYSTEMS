//! # Mesh Network Simulation
//!
//! Runs the real mesh protocol (`MeshNode` + `wire` codec) over a simulated
//! radio channel: nodes are placed on a line, beacons/route advertisements
//! propagate hop-by-hop until routing converges, then dead-drop messages are
//! fragmented, routed and measured (delivery ratio, hop count, energy).
//!
//! All numbers in [`SimStats`] come from the simulation itself — nothing is
//! hardcoded or randomly fabricated.

use smart_matter_core::energy::EnergyBudget;
use smart_matter_core::mesh::MeshNode;
use smart_matter_core::storage::dead_drop::fragment_to_payload;
use smart_matter_core::types::{
    DeadDropFragment, EnergyLevel, FragmentHeader, NodeId, ProtocolSlice, SliceType, TimeMs,
};

/// Node spacing on the line (meters). Radio range is 1.5 m, so only
/// neighbours hear each other — every hop must be routed, not overheard.
const SPACING_M: f32 = 1.0;
const RADIO_RANGE_M: f32 = 1.5;
const RX_SENSITIVITY_DBM: i16 = -90;

/// Energy cost of one slice transmission (matches `energy::ENERGY_PER_SLICE_UJ`).
const ENERGY_PER_SLICE_UJ: u64 = 50;
const SUPERCAP_UF: u32 = 1_000_000; // 1 F

/// Beacon / route-adv periods (ticks).
const BEACON_PERIOD: u64 = 2;
const ROUTE_ADV_PERIOD: u64 = 2;
/// Ticks of routing warm-up before messages are injected.
const WARMUP_TICKS: u64 = 10;

/// Simulation configuration.
#[derive(Debug, Clone)]
pub struct SimConfig {
    pub nodes: u32,
    pub duration_ticks: u64,
    /// Number of dead-drop messages to route after warm-up.
    pub messages: u32,
}

impl Default for SimConfig {
    fn default() -> Self {
        Self {
            nodes: 10,
            duration_ticks: 60,
            messages: 20,
        }
    }
}

/// Simulation results — all computed from the actual protocol run.
#[derive(Debug, Clone, Default)]
pub struct SimStats {
    pub fragments_sent: u64,
    pub fragments_delivered: u64,
    pub avg_hops: f32,
    pub energy_consumed_uj: u64,
    /// Nodes whose routing table has an entry for the far end of the line.
    pub routing_coverage: u32,
    pub routed_messages: u32,
}

/// One simulated node.
struct Node {
    mesh: MeshNode,
    energy: EnergyBudget,
    tx_energy_uj: u64,
}

impl Node {
    fn new(id: NodeId) -> Self {
        Self {
            mesh: MeshNode::new(id),
            energy: EnergyBudget::new(SUPERCAP_UF),
            tx_energy_uj: 0,
        }
    }

    /// Charge the node fully (fresh deployment).
    fn charge(&mut self) {
        self.energy
            .update(&smart_matter_core::types::VoltageReading {
                harvester_mv: 5000,
                storage_mv: 3300,
                vdd_mv: 1800,
            });
    }

    fn energy_level(&self) -> EnergyLevel {
        self.energy.energy_level()
    }
}

/// A slice travelling between nodes this tick.
struct Flight {
    slice: ProtocolSlice,
    /// Unicast next hop (routing) or `None` for a broadcast.
    next_hop: Option<NodeId>,
}

/// Run a line-topology simulation; returns aggregate statistics.
pub fn run_simulation(cfg: &SimConfig) -> SimStats {
    let n = cfg.nodes.max(2) as usize;
    let mut nodes: Vec<Node> = (0..n).map(|i| Node::new(node_id(i as u8))).collect();
    for node in nodes.iter_mut() {
        node.charge();
    }

    let mut stats = SimStats::default();
    let mut inboxes: Vec<std::vec::Vec<ProtocolSlice>> = (0..n).map(|_| Vec::new()).collect();
    let mut msg_queue: Vec<(usize, usize, u32)> = Vec::new(); // (src, dst, msg_id)

    for tick in 1..=cfg.duration_ticks {
        let mut air: Vec<Flight> = Vec::new();

        // ---- Receive: process the previous tick's inbox ------------------
        for i in 0..n {
            let inbox = std::mem::take(&mut inboxes[i]);
            for slice in inbox {
                if slice.dst == nodes[i].mesh.node_id() {
                    // Arrived at the final destination.
                    if slice.slice_type == SliceType::DataFragment {
                        stats.fragments_delivered += 1;
                        stats.avg_hops += slice.hops as f32;
                    }
                    continue;
                }
                let mut dummy = DummyRadio;
                if let Some(fwd) = nodes[i].mesh.process_slice(&slice, &mut dummy) {
                    let next = nodes[i].mesh.routing_table_ref().next_hop(&fwd.dst);
                    air.push(Flight {
                        slice: fwd,
                        next_hop: next,
                    });
                }
            }
        }

        // ---- Periodic beacons / route advertisements ----------------------
        for node in nodes.iter_mut() {
            let energy = node.energy_level();
            if energy == EnergyLevel::Critical {
                continue; // no radio in critical
            }
            if tick.is_multiple_of(BEACON_PERIOD) {
                let beacon = node.mesh.build_beacon(energy);
                air.push(Flight {
                    slice: beacon,
                    next_hop: None,
                });
            }
            if tick.is_multiple_of(ROUTE_ADV_PERIOD) {
                let adv = node.mesh.build_route_adv(energy);
                air.push(Flight {
                    slice: adv,
                    next_hop: None,
                });
            }
        }

        // ---- Inject dead-drop messages after warm-up ----------------------
        if tick == WARMUP_TICKS {
            for m in 0..cfg.messages {
                let src = m as usize % n;
                let dst = (m as usize + 1 + n / 2) % n;
                if src != dst {
                    msg_queue.push((src, dst, m));
                }
            }
        }

        // ---- Route queued messages: split into fragments and send ---------
        if tick > WARMUP_TICKS {
            let mut still_queued = Vec::new();
            for (src, dst, msg_id) in msg_queue.drain(..) {
                if nodes[src]
                    .mesh
                    .routing_table_ref()
                    .next_hop(&node_id(dst as u8))
                    .is_some()
                {
                    send_message(&mut nodes[src], src, dst, msg_id, &mut air, &mut stats);
                    stats.routed_messages += 1;
                } else {
                    still_queued.push((src, dst, msg_id)); // wait for a route
                }
            }
            msg_queue = still_queued;
        }

        // ---- Deliver flights (broadcast to in-range, unicast to hop) ------
        for flight in air {
            match flight.next_hop {
                Some(hop) => {
                    let hop_idx = hop_id(&hop);
                    if let Some(inbox) = inboxes.get_mut(hop_idx) {
                        inbox.push(flight.slice);
                    }
                }
                None => {
                    // Broadcast: every node within radio range receives it.
                    // (The origin drops its own broadcast via dedup.)
                    for (j, inbox) in inboxes.iter_mut().enumerate() {
                        if in_range(j, n, &flight) {
                            inbox.push(flight.slice.clone());
                        }
                    }
                }
            }
        }
    }

    // ---- Aggregate --------------------------------------------------------
    if stats.fragments_delivered > 0 {
        stats.avg_hops /= stats.fragments_delivered as f32;
    }
    stats.energy_consumed_uj = nodes.iter().map(|nd| nd.tx_energy_uj).sum();
    let last_id = node_id(n as u8 - 1);
    stats.routing_coverage = nodes
        .iter()
        .filter(|nd| nd.mesh.routing_table_ref().next_hop(&last_id).is_some())
        .count() as u32;
    stats
}

/// Whether node `j` hears a broadcast flight.
fn in_range(j: usize, n: usize, flight: &Flight) -> bool {
    // Flight carries no position; use hop distance via the slice source id.
    let src = node_id(flight.slice.src.0[0]);
    let src_idx = src.0[0] as usize;
    if j >= n {
        return false;
    }
    let dist = (src_idx as i32 - j as i32).unsigned_abs() as f32 * SPACING_M;
    let rssi = -40.0 - 40.0 * (dist / RADIO_RANGE_M);
    rssi as i16 >= RX_SENSITIVITY_DBM
}

/// Split `msg` into one or more encrypted dead-drop fragments and launch them
/// toward `dst` via the source node's route.
fn send_message(
    src_node: &mut Node,
    src: usize,
    dst: usize,
    msg_id: u32,
    air: &mut Vec<Flight>,
    stats: &mut SimStats,
) {
    let crypto = smart_matter_core::crypto::CryptoProvider::new();
    let key = [0x42u8; 32];
    let payload = format!("dead-drop message #{msg_id} from node {src} to node {dst}").into_bytes();
    let now = TimeMs::now();

    for (idx, chunk) in payload.chunks(64).enumerate() {
        let mut plain = heapless::Vec::<u8, 128>::new();
        let _ = plain.extend_from_slice(chunk);
        let nonce = [msg_id as u8 ^ idx as u8; 12];
        let ciphertext = crypto.encrypt(&key, &nonce, &plain).unwrap();

        let fragment = DeadDropFragment {
            header: FragmentHeader {
                msg_id,
                total: payload.chunks(64).count() as u8,
                index: idx as u8,
                ttl_sec: 3600,
                created: now,
            },
            ciphertext,
            nonce,
        };

        let slice = ProtocolSlice {
            slice_type: SliceType::DataFragment,
            src: node_id(src as u8),
            dst: node_id(dst as u8),
            hops: 0,
            seqno: src_node.mesh.next_seqno(),
            payload: fragment_to_payload(&fragment).unwrap(),
            energy: src_node.energy_level(),
        };

        // Launch: deliver to the first hop (if the source can afford it).
        if src_node.energy.available_slices() > 0 {
            let _ = src_node.energy.reserve_slices(1);
            src_node.tx_energy_uj += ENERGY_PER_SLICE_UJ;
            stats.fragments_sent += 1;
            if let Some(hop) = src_node
                .mesh
                .routing_table_ref()
                .next_hop(&node_id(dst as u8))
            {
                air.push(Flight {
                    slice,
                    next_hop: Some(hop),
                });
            }
        }
    }
}

fn node_id(id: u8) -> NodeId {
    let mut b = [0u8; 8];
    b[0] = id;
    NodeId(b)
}

fn hop_id(id: &NodeId) -> usize {
    id.0[0] as usize
}

/// No-op radio for `process_slice` (sim delivers flights explicitly).
struct DummyRadio;
impl smart_matter_core::mesh::Radio for DummyRadio {
    fn send(&mut self, _s: &ProtocolSlice) -> Result<(), smart_matter_core::mesh::RadioError> {
        Ok(())
    }
    fn receive(
        &mut self,
        _t: u32,
    ) -> Result<Option<ProtocolSlice>, smart_matter_core::mesh::RadioError> {
        Ok(None)
    }
    fn last_rssi(&self) -> i16 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_topology_routing_converges() {
        let cfg = SimConfig {
            nodes: 6,
            duration_ticks: 40,
            messages: 10,
        };
        let stats = run_simulation(&cfg);
        // With a 6-node line, every node should learn a route to the far end.
        assert_eq!(stats.routing_coverage, 6);
        assert!(stats.fragments_sent > 0);
        assert!(stats.energy_consumed_uj > 0);
    }

    #[test]
    fn messages_get_delivered() {
        let cfg = SimConfig {
            nodes: 4,
            duration_ticks: 30,
            messages: 6,
        };
        let stats = run_simulation(&cfg);
        // Messages routed after warm-up; a fraction must arrive.
        assert_eq!(stats.routed_messages, 6);
        assert!(stats.fragments_delivered >= 1);
        assert!(stats.avg_hops > 0.0);
    }
}
