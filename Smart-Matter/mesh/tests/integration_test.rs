//! # Smart Matter Integration Tests
//!
//! End-to-end integration tests for the Smart Matter system.
//! Tests cover the complete pipeline: message creation → fragmentation →
//! mesh routing → storage → reassembly → decryption.

use heapless::Vec;
use smart_matter_core::crypto::CryptoProvider;
use smart_matter_core::energy::EnergyBudget;
use smart_matter_core::mesh::*;
use smart_matter_core::storage::*;
use smart_matter_core::types::*;

fn test_node_id(id: u8) -> NodeId {
    let mut bytes = [0u8; 8];
    bytes[0] = id;
    bytes[7] = 0x01;
    NodeId(bytes)
}

/// Test: Generate entropy, encrypt a message, fragment it, store on multiple
/// nodes, collect fragments, reassemble, and decrypt.
#[test]
fn test_full_dead_drop_pipeline() {
    let key = [0x42u8; 32];
    let crypto = CryptoProvider::new();

    // 1. Create a message
    let message = b"This is a secret message for dead-drop storage in the Smart Matter mesh!";

    // 2. Split into fragments and encrypt
    let fragment_count = message.len().div_ceil(MAX_PAYLOAD_SIZE);
    let mut fragments: Vec<DeadDropFragment, MAX_FRAGMENTS> = Vec::new();

    for i in 0..fragment_count.min(MAX_FRAGMENTS) {
        let start = i * MAX_PAYLOAD_SIZE;
        let end = (start + MAX_PAYLOAD_SIZE).min(message.len());
        let chunk = &message[start..end];

        let mut plain: Vec<u8, 128> = Vec::new();
        plain.extend_from_slice(chunk).ok();

        let nonce = [i as u8; 12];
        let ciphertext = crypto.encrypt(&key, &nonce, &plain).unwrap();

        let fragment = DeadDropFragment {
            header: FragmentHeader {
                msg_id: 42,
                total: fragment_count as u8,
                index: i as u8,
                ttl_sec: 3600,
                created: TimeMs::now(),
            },
            ciphertext,
            nonce,
        };
        fragments.push(fragment).ok();
    }

    // 3. Store fragments across simulated nodes
    let mut nodes: [DeadDropStore; 3] = [
        DeadDropStore::new(),
        DeadDropStore::new(),
        DeadDropStore::new(),
    ];

    for (i, fragment) in fragments.iter().enumerate() {
        nodes[i % 3].store(fragment.clone()).ok();
    }

    // 4. Collect fragments from all nodes
    let mut assembler = FragmentAssembler::new();
    let mut assembled = None;

    for node in nodes.iter() {
        let stored = node.retrieve(42);
        for fragment in stored {
            if let Ok(Some(msg)) = assembler.add_fragment(fragment, &crypto, &key) {
                assembled = Some(msg);
                break;
            }
        }
        if assembled.is_some() {
            break;
        }
    }

    // 5. Verify the assembled message
    let assembled = assembled.expect("Should have assembled the complete message");
    assert_eq!(&assembled[..message.len()], message);
}

/// Test: Mesh routing with 3 nodes — route a message from node 1 to node 3
/// via node 2.
#[test]
fn test_mesh_routing_chain() {
    let n1 = test_node_id(1);
    let n2 = test_node_id(2);
    let n3 = test_node_id(3);

    // Build routing tables
    let mut mesh1 = MeshNode::new(n1);
    let mut mesh2 = MeshNode::new(n2);
    let _mesh3 = MeshNode::new(n3);

    // n1 knows n2 (direct)
    mesh1.routing_table().update(RouteEntry {
        destination: n2,
        next_hop: n2,
        metric: 100,
        flags: RouteFlags::empty(),
        age_sec: 0,
    });

    // n2 knows n3 (direct)
    mesh2.routing_table().update(RouteEntry {
        destination: n3,
        next_hop: n3,
        metric: 100,
        flags: RouteFlags::empty(),
        age_sec: 0,
    });

    // n2 advertises route to n3
    let mut adv_entries: Vec<RouteEntry, 32> = Vec::new();
    adv_entries
        .push(RouteEntry {
            destination: n3,
            next_hop: n3,
            metric: 100,
            flags: RouteFlags::empty(),
            age_sec: 0,
        })
        .ok();

    let adv = RouteAdvertisement {
        origin: n2,
        seqno: 1,
        flags: RouteFlags::empty(),
        entries: adv_entries,
    };

    // n1 processes advertisement from n2
    mesh1.routing_table().process_advertisement(&adv, n2);

    // Now n1 should route to n3 via n2
    assert_eq!(mesh1.routing_table().next_hop(&n3), Some(n2));
    assert!(mesh1.routing_table().metric(&n3).unwrap() >= 100);
}

/// Test: Energy budget and transmission scheduling
#[test]
fn test_energy_aware_scheduling() {
    let mut budget = EnergyBudget::new(1_000_000); // 1 F

    // Fully charged
    budget.update(&VoltageReading {
        harvester_mv: 5000,
        storage_mv: 3300,
        vdd_mv: 1800,
    });

    assert_eq!(budget.energy_level(), EnergyLevel::Surplus);
    let slices = budget.available_slices();
    assert_eq!(slices, 255); // Session cap (protocol field)

    // Reserving the cap consumes 255 × 50 µJ from the stored budget.
    assert!(budget.reserve_slices(slices));
    assert_eq!(budget.stored_uj(), 5_445_000 - 255 * 50);
}

/// Test: Localized broadcast with RSSI geo-fencing
#[test]
fn test_broadcast_geo_fencing() {
    use smart_matter_core::broadcast::LocalizedBroadcaster;

    let n1 = test_node_id(1);
    let n2 = test_node_id(2);
    let mut bc = LocalizedBroadcaster::new(n2);

    let slice = ProtocolSlice {
        slice_type: SliceType::Broadcast,
        src: n1,
        dst: NodeId([0xff; 8]),
        hops: 1,
        seqno: 1,
        payload: {
            let mut p: Vec<u8, 128> = Vec::new();
            p.extend_from_slice(&n1.as_bytes()).ok();
            p.extend_from_slice(&(-80i16).to_le_bytes()).ok();
            p.extend_from_slice(&1u16.to_le_bytes()).ok();
            p.extend_from_slice(&3u8.to_le_bytes()).ok();
            p
        },
        energy: EnergyLevel::Nominal,
    };

    // RSSI -75 is stronger than threshold -80 → should relay
    assert!(bc.process_incoming(&slice, -75).is_some());

    // RSSI -85 is weaker than threshold -80 → should drop
    assert!(bc.process_incoming(&slice, -85).is_none());
}

/// Test: Thermal-aware scheduling defers when hot
#[test]
fn test_thermal_throttling() {
    use smart_matter_core::thermal::*;

    let model = ThermalModel::new(50.0, 10.0);
    let mut scheduler = ThermalScheduler::new(model);

    // Add a high-power operation
    let op = ScheduledOp {
        name: "radio_tx",
        power_mw: 200.0,
        duration_ms: 10,
        priority: 5,
    };
    scheduler.schedule(op).ok();

    // Should execute at nominal temp
    assert!(scheduler.execute_next(|_| {}).is_ok());

    // Add another op
    let op2 = ScheduledOp {
        name: "radio_tx",
        power_mw: 200.0,
        duration_ms: 10,
        priority: 5,
    };
    scheduler.schedule(op2).ok();

    // Force temperature high
    scheduler.model_mut().set_core_temperature(100_000); // Above shutdown
    let result = scheduler.execute_next(|_| {});
    assert!(result.is_err()); // Should fail — thermal limit
}

/// Test: Integrity auditor baseline and anomaly detection
#[test]
fn test_integrity_monitoring() {
    use smart_matter_core::integrity::IntegrityAuditor;

    let n1 = test_node_id(1);
    let mut auditor = IntegrityAuditor::new(n1);

    // Establish baseline
    for i in 0..100 {
        auditor.process_sample(&VibrationSample {
            x: 10,
            y: 20,
            z: 15,
            time: TimeMs(i as u64),
        });
    }

    // Should have baseline now
    let report = auditor.build_report();
    assert_eq!(report.node_id, n1);

    // Anomalous vibration
    for i in 0..5 {
        if let Some(alert) = auditor.process_sample(&VibrationSample {
            x: 100,
            y: 200,
            z: 150,
            time: TimeMs(200 + i as u64),
        }) {
            assert_eq!(alert.node_id, n1);
            assert!(alert.confidence > 0);
        }
    }
}

/// Test: Entropy pool produces different nonces for different inputs
#[test]
fn test_entropy_for_crypto_nonce() {
    use smart_matter_core::sensor::EntropyPool;

    let mut pool1 = EntropyPool::new();
    let mut pool2 = EntropyPool::new();

    // Simulate different environmental noise
    pool1.add_adc_sample(0xDEAD);
    pool1.add_vibration_sample(0xBEEF);
    pool2.add_adc_sample(0xCAFE);
    pool2.add_vibration_sample(0xBABE);

    let entropy1 = pool1.extract_entropy();
    let entropy2 = pool2.extract_entropy();

    assert_ne!(entropy1, entropy2);

    // Both should be valid 12-byte nonces
    assert_eq!(entropy1.len(), 12);
    assert_eq!(entropy2.len(), 12);
}
