//! # Shared Node Logic
//!
//! The energy-aware node main loop, independent of the platform. The host
//! simulator and the embedded firmware both drive this `Node` type; only the
//! sensors/radio/sleep differ.
//!
//! Loop phases (per tick):
//! 1. **Wake**  — tick invoked by the platform (vibration interrupt / RTC /
//!    timer on hardware, wall clock in the host simulator).
//! 2. **Sample** — read accelerometer + ADC noise + temperature; feed the
//!    integrity auditor and the entropy pool.
//! 3. **Process** — drain the radio; run mesh routing/dedup; build periodic
//!    beacons, route advertisements and integrity reports.
//! 4. **Transmit** — send queued slices while the energy budget allows.
//! 5. **Sleep** — the caller sleeps (WFI / `thread::sleep`).

use heapless::Vec;
use smart_matter_core::broadcast::{LocalizedBroadcaster, MAX_BROADCAST_TTL};
use smart_matter_core::crypto::{derive_key, CryptoProvider};
use smart_matter_core::energy::{EnergyBudget, PIEZO_NOMINAL_MV, SUPERCAP_TARGET_MV};
use smart_matter_core::integrity::{AnomalyAlert, IntegrityAuditor};
use smart_matter_core::mesh::{MeshNode, Radio, BEACON_INTERVAL_LOW_MS, BEACON_INTERVAL_MS};
use smart_matter_core::sensor::EntropyPool;
use smart_matter_core::sensor::{Accelerometer, AdcNoiseSource, TemperatureSensor};
use smart_matter_core::storage::dead_drop::{payload_to_fragment, FragmentAssembler};
use smart_matter_core::storage::DeadDropStore;
use smart_matter_core::thermal::{ThermalModel, ThermalScheduler};
#[cfg(feature = "embedded")]
use smart_matter_core::types::VoltageReading;
use smart_matter_core::types::{
    EnergyLevel, IntegrityReport, NodeId, ProtocolSlice, SliceType, TimeMs, VibrationSample,
};

/// Storage supercapacitor capacitance (1 F, matches the BOM/whitepaper).
pub const STORAGE_CAPACITANCE_UF: u32 = 1_000_000;

/// Simulated harvest per tick while the node is awake (µJ) — host mode.
const HARVEST_PER_TICK_UJ: u64 = 500;

/// Maximum number of queued outgoing slices per tick window.
const MAX_TX_QUEUE: usize = 16;

/// A single tick's outcome, used for status reporting and tests.
///
/// The host simulator reads most fields; the embedded firmware only needs
/// `tick` and `energy_level` (LED behavior), so the rest is host-only.
#[cfg_attr(feature = "embedded", allow(dead_code))]
#[derive(Debug, Clone)]
pub struct TickReport {
    pub tick: u64,
    pub energy_level: EnergyLevel,
    pub stored_uj: u64,
    pub available_slices: u32,
    pub routing_entries: usize,
    pub storage_util_pct: u8,
    pub anomaly_count: u32,
    pub integrity: Option<IntegrityReport>,
    pub tx_count: u32,
    pub retrieved_count: u32,
}

/// The fully-wired compute node.
#[cfg_attr(feature = "embedded", allow(dead_code))]
pub struct Node<R: Radio, A: Accelerometer, N: AdcNoiseSource, T: TemperatureSensor> {
    id: NodeId,
    key: [u8; 32],
    energy: EnergyBudget,
    mesh: MeshNode,
    storage: DeadDropStore,
    broadcaster: LocalizedBroadcaster,
    crypto: CryptoProvider,
    entropy: EntropyPool,
    thermal: ThermalScheduler,
    auditor: IntegrityAuditor,
    radio: R,
    accel: A,
    adc: N,
    temp: T,
    pending_tx: Vec<ProtocolSlice, MAX_TX_QUEUE>,
    tick_count: u64,
}

impl<R: Radio, A: Accelerometer, N: AdcNoiseSource, T: TemperatureSensor> Node<R, A, N, T> {
    /// Create a node with its deployment-derived key.
    pub fn new(id: NodeId, deployment_secret: &[u8], radio: R, accel: A, adc: N, temp: T) -> Self {
        let key = derive_key(deployment_secret, &id);
        let thermal_model = ThermalModel::new(50.0, 10.0); // R=50 K/W, C=10 J/K
        Self {
            id,
            key,
            energy: EnergyBudget::new(STORAGE_CAPACITANCE_UF),
            mesh: MeshNode::new(id),
            storage: DeadDropStore::new(),
            broadcaster: LocalizedBroadcaster::new(id),
            crypto: CryptoProvider::new(),
            entropy: EntropyPool::new(),
            thermal: ThermalScheduler::new(thermal_model),
            auditor: IntegrityAuditor::new(id),
            radio,
            accel,
            adc,
            temp,
            pending_tx: Vec::new(),
            tick_count: 0,
        }
    }

    /// The node's deployment-scoped key.
    #[cfg(feature = "host")]
    pub fn key(&self) -> &[u8; 32] {
        &self.key
    }

    /// Run one wake→sample→process→transmit cycle.
    pub fn tick(&mut self) -> TickReport {
        self.tick_count += 1;
        let energy_level = self.energy.energy_level();
        let mut retrieved_count = 0u32;

        // ---- Phase 2: sample sensors -------------------------------------
        let sample = self.read_vibration();
        self.entropy.add_vibration_sample(sample.x.unsigned_abs());
        if let Ok(adc_raw) = self.adc.read_raw() {
            self.entropy.add_adc_sample(adc_raw);
        }
        if let Ok(temp_mc) = self.temp.read_millicelsius() {
            self.entropy.add_temp_sample(temp_mc);
        }

        // Integrity auditing (anomaly alert → localized broadcast)
        if let Some(alert) = self.auditor.process_sample(&sample) {
            let slice = self.build_alert_broadcast(&alert, energy_level);
            let _ = self.pending_tx.push(slice);
        }

        // ---- Phase 3: process incoming radio -----------------------------
        if let Ok(Some(slice)) = self.radio.receive(0) {
            retrieved_count += self.process_incoming(&slice);
            if let Some(fwd) = self.mesh.process_slice(&slice, &mut self.radio) {
                let _ = self.pending_tx.push(fwd);
            }
        }

        // ---- Periodic transmissions --------------------------------------
        let beacon_interval_ms = if energy_level == EnergyLevel::Low {
            BEACON_INTERVAL_LOW_MS
        } else {
            BEACON_INTERVAL_MS
        };

        // Beacon (presence announcement)
        if self.tick_count.is_multiple_of(beacon_interval_ms as u64) {
            let beacon = self.mesh.build_beacon(energy_level);
            let _ = self.pending_tx.push(beacon);
        }
        // Route advertisement (multi-hop learning)
        if self
            .tick_count
            .is_multiple_of((beacon_interval_ms * 2) as u64)
        {
            let adv = self.mesh.build_route_adv(energy_level);
            let _ = self.pending_tx.push(adv);
        }
        // Integrity report
        if self.auditor.should_check() && energy_level != EnergyLevel::Low {
            let report = self.auditor.build_report_slice(energy_level);
            let _ = self.pending_tx.push(report);
            self.auditor.mark_checked();
        }

        // ---- Phase 4: transmit -------------------------------------------
        let mut tx_count = 0u32;
        while !self.pending_tx.is_empty() && self.energy.available_slices() > 0 {
            let slice = self.pending_tx.remove(0);
            match self.radio.send(&slice) {
                Ok(()) => {
                    let _ = self.energy.reserve_slices(1);
                    tx_count += 1;
                }
                Err(_) => {
                    // Radio busy / energy low — drop this slice, keep going.
                }
            }
        }

        // Recharge: ambient vibration keeps harvesting while awake (host
        // sim). On hardware this comes from the real piezo → BQ25504 chain.
        self.recharge();

        // ---- Thermal scheduling ------------------------------------------
        let _ = self.thermal.execute_next(|_op| {
            // Ops are executed inline; the model keeps junction temp bounded.
        });

        TickReport {
            tick: self.tick_count,
            energy_level: self.energy.energy_level(),
            stored_uj: self.energy.stored_uj(),
            available_slices: self.energy.available_slices(),
            routing_entries: self.mesh.routing_table_ref().len(),
            storage_util_pct: self.storage.utilization_pct(),
            anomaly_count: self.auditor.anomaly_count(),
            integrity: Some(self.auditor.build_report()),
            tx_count,
            retrieved_count,
        }
    }

    /// Build a localized broadcast slice for a structural-anomaly alert.
    fn build_alert_broadcast(
        &mut self,
        alert: &AnomalyAlert,
        energy: EnergyLevel,
    ) -> ProtocolSlice {
        // Alert payload: node_id (8) | deviation (1) | count (4) | confidence (1)
        let mut payload: Vec<u8, 128> = Vec::new();
        let _ = payload.extend_from_slice(alert.node_id.as_bytes());
        let _ = payload.push(alert.deviation_pct);
        let _ = payload.extend_from_slice(&alert.anomaly_count.to_le_bytes());
        let _ = payload.push(alert.confidence);

        let msg = self
            .broadcaster
            .create_broadcast(payload, MAX_BROADCAST_TTL);
        self.broadcaster.to_slice(&msg, energy)
    }

    /// Handle an incoming slice that is addressed to this node: store dead-
    /// drop fragments and reassemble complete messages on `DataAck`.
    /// Returns the number of messages retrieved.
    fn process_incoming(&mut self, slice: &ProtocolSlice) -> u32 {
        let mut retrieved = 0u32;

        match slice.slice_type {
            SliceType::DataFragment => {
                if let Ok(fragment) = payload_to_fragment(&slice.payload) {
                    let _ = self.storage.store(fragment);
                }
            }
            SliceType::DataAck if slice.payload.len() >= 4 => {
                // Payload: msg_id (4 bytes LE) — request to reassemble.
                let msg_id = u32::from_le_bytes([
                    slice.payload[0],
                    slice.payload[1],
                    slice.payload[2],
                    slice.payload[3],
                ]);
                // Clone the fragments out of the store so the store can
                // be mutated during assembly.
                let fragments: Vec<smart_matter_core::types::DeadDropFragment, 32> =
                    self.storage.retrieve(msg_id).into_iter().cloned().collect();
                let mut assembler = FragmentAssembler::new();
                for fragment in &fragments {
                    if let Ok(Some(_message)) =
                        assembler.add_fragment(fragment, &self.crypto, &self.key)
                    {
                        // Complete message reassembled and decrypted.
                        self.storage.remove(msg_id);
                        retrieved += 1;
                        break;
                    }
                }
            }
            _ => {}
        }

        retrieved
    }

    /// Read the accelerometer, tolerating a not-ready sensor (fallback 0s).
    fn read_vibration(&mut self) -> VibrationSample {
        match self.accel.read_sample() {
            Ok(s) => s,
            Err(_) => VibrationSample {
                x: 0,
                y: 0,
                z: 0,
                time: TimeMs::now(),
            },
        }
    }

    /// Simulate ambient harvesting (host mode). On hardware the energy model
    /// is updated from the ADC voltage sense instead.
    fn recharge(&mut self) {
        let mut reading = self.energy.voltage_reading();
        reading.harvester_mv = PIEZO_NOMINAL_MV;
        // If we have spare budget, top the storage voltage back up slowly.
        if self.energy.stored_uj() < 5_000_000 {
            let v = libm_sqrt_voltage(self.energy.stored_uj() + HARVEST_PER_TICK_UJ);
            reading.storage_mv = v.min(SUPERCAP_TARGET_MV);
        } else {
            reading.storage_mv = SUPERCAP_TARGET_MV;
        }
        self.energy.update(&reading);
    }

    /// Current energy level.
    #[cfg(feature = "host")]
    pub fn energy_level(&self) -> EnergyLevel {
        self.energy.energy_level()
    }

    /// Node identifier.
    #[cfg(feature = "host")]
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Total ticks executed.
    #[cfg(feature = "host")]
    pub fn tick_count(&self) -> u64 {
        self.tick_count
    }

    /// Access to the dead-drop store (for tests / retrieval).
    #[cfg(feature = "host")]
    pub fn storage(&self) -> &DeadDropStore {
        &self.storage
    }

    /// Mutable access to the ADC noise source (the embedded platform reads
    /// storage voltage through it each tick).
    #[cfg(feature = "embedded")]
    pub fn adc_mut(&mut self) -> &mut N {
        &mut self.adc
    }

    /// Update the energy model from a platform voltage reading (embedded:
    /// ADC supercap sense; host mode uses `recharge` instead).
    #[cfg(feature = "embedded")]
    pub fn update_voltage(&mut self, reading: VoltageReading) {
        self.energy.update(&reading);
    }
}

/// V = sqrt(2E/C) with E in µJ, C in µF → V in mV.
fn libm_sqrt_voltage(stored_uj: u64) -> u32 {
    // E_μJ = ½ C_μF V_mV² / 10⁶  ⇒  V_mV = sqrt(2·E·10⁶ / C)
    let e = stored_uj as f64;
    let c = STORAGE_CAPACITANCE_UF as f64;
    (libm::sqrt((2.0 * e * 1_000_000.0) / c) * 1000.0) as u32
}

#[cfg(test)]
mod tests {
    use super::*;
    use smart_matter_core::crypto::derive_key;
    use smart_matter_core::mesh::RadioError;
    use smart_matter_core::sensor::SensorError;
    use smart_matter_core::storage::dead_drop::fragment_to_payload;
    use smart_matter_core::types::{DeadDropFragment, FragmentHeader};

    struct TestAccel;
    impl Accelerometer for TestAccel {
        fn read_sample(&mut self) -> Result<VibrationSample, SensorError> {
            Ok(VibrationSample {
                x: 10,
                y: 5,
                z: 2,
                time: TimeMs::now(),
            })
        }
        fn set_rate(&mut self, _hz: u32) -> Result<(), SensorError> {
            Ok(())
        }
        fn noise_floor(&self) -> f32 {
            220.0
        }
    }

    struct TestAdc;
    impl AdcNoiseSource for TestAdc {
        fn read_raw(&mut self) -> Result<u32, SensorError> {
            Ok(2048)
        }
    }

    struct TestTemp;
    impl TemperatureSensor for TestTemp {
        fn read_millicelsius(&mut self) -> Result<i32, SensorError> {
            Ok(25_000)
        }
    }

    /// Radio that plays back pre-queued incoming slices, one per tick.
    struct QueueRadio {
        rx: Vec<ProtocolSlice, 8>,
        tx: u32,
    }

    impl Radio for QueueRadio {
        fn send(&mut self, _slice: &ProtocolSlice) -> Result<(), RadioError> {
            self.tx += 1;
            Ok(())
        }
        fn receive(&mut self, _timeout_ms: u32) -> Result<Option<ProtocolSlice>, RadioError> {
            if self.rx.is_empty() {
                Ok(None)
            } else {
                Ok(Some(self.rx.remove(0)))
            }
        }
        fn last_rssi(&self) -> i16 {
            -50
        }
    }

    const SECRET: &[u8] = b"test-deployment-secret";

    fn node_id(id: u8) -> NodeId {
        let mut b = [0u8; 8];
        b[0] = id;
        NodeId(b)
    }

    fn make_node(radio: QueueRadio) -> Node<QueueRadio, TestAccel, TestAdc, TestTemp> {
        Node::new(node_id(1), SECRET, radio, TestAccel, TestAdc, TestTemp)
    }

    #[test]
    fn node_runs_full_loop() {
        let mut node = make_node(QueueRadio {
            rx: Vec::new(),
            tx: 0,
        });

        let mut total_tx = 0;
        for _ in 0..60 {
            let report = node.tick();
            total_tx += report.tx_count;
        }

        // Node survives 60 ticks, stays on the energy budget and transmits.
        assert_eq!(node.tick_count(), 60);
        assert!(node.energy_level() != EnergyLevel::Critical);
        assert!(total_tx > 0);
        // Storage capacity constant is consistent with SRAM (see docs).
        assert!(node.storage().len() <= 32);
    }

    #[test]
    fn node_stores_and_retrieves_dead_drop() {
        let key = derive_key(SECRET, &node_id(1));
        let crypto = CryptoProvider::new();

        // One-fragment message, encrypted with the node's derived key.
        let mut plain: Vec<u8, 128> = Vec::new();
        plain.extend_from_slice(b"bridge health payload").ok();
        let nonce = [0x11u8; 12];
        let ciphertext = crypto.encrypt(&key, &nonce, &plain).unwrap();
        let fragment = DeadDropFragment {
            header: FragmentHeader {
                msg_id: 7,
                total: 1,
                index: 0,
                ttl_sec: 3600,
                created: TimeMs::now(),
            },
            ciphertext,
            nonce,
        };

        // DataFragment slice addressed to the node.
        let data_slice = ProtocolSlice {
            slice_type: SliceType::DataFragment,
            src: node_id(2),
            dst: node_id(1),
            hops: 1,
            seqno: 1,
            payload: fragment_to_payload(&fragment).unwrap(),
            energy: EnergyLevel::Nominal,
        };
        // DataAck slice requesting msg_id 7.
        let ack_slice = ProtocolSlice {
            slice_type: SliceType::DataAck,
            src: node_id(2),
            dst: node_id(1),
            hops: 1,
            seqno: 2,
            payload: Vec::from_slice(&7u32.to_le_bytes()).unwrap(),
            energy: EnergyLevel::Nominal,
        };

        let radio = QueueRadio {
            rx: Vec::from_slice(&[data_slice, ack_slice]).unwrap(),
            tx: 0,
        };
        let mut node = Node::new(node_id(1), SECRET, radio, TestAccel, TestAdc, TestTemp);

        // Tick 1: store the fragment.
        let report1 = node.tick();
        assert_eq!(node.storage().len(), 1);
        assert_eq!(report1.retrieved_count, 0);

        // Tick 2: DataAck → reassemble + decrypt + remove.
        let report2 = node.tick();
        assert_eq!(report2.retrieved_count, 1);
        assert_eq!(node.storage().len(), 0);
    }
}
