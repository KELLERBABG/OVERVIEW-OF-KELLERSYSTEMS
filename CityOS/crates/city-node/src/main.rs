//! **City OS Node** — host-side demo that boots every City service against
//! the KELLER OS-aligned foundation (capabilities, IPC, vault crypto).
//!
//! This binary is the *application profile* that KELLER OS Phase 3.6 ships to
//! city edge nodes. On real hardware, the same crates run as ring-3 services
//! on top of the KELLER microkernel; here, on the host, it runs the full City
//! service stack end-to-end and prints a PASS/FAIL report — mirroring the
//! `[TEST] name: PASS` assertion protocol the kernel uses at boot.

use city_alloc::DistrictAllocator;
use city_brp::{self, BrpToken};
use city_caps::{self, CAP_CITY_ALLOC, CAP_CITY_BRP, CAP_CITY_DAG, CAP_CITY_DEADDROP};
use city_crypto::{self, derive_entropy, entropy_seed};
use city_dag::Dag;
use city_deaddrop::DeadDropHeap;
use city_entropy::{derive_otp, EntropyBeacon, EnvSignature};
use city_heal::HealthTable;
use city_incentive::{IncentiveLedger, UptimeClaim, INCENT_CREDITS_PER_UPTIME};
use city_interop::{IntersectionScheduler, V2vMsg, KIND_V2V};
use city_p2p::{
    announce_payload,
    hw::{Nic, Sdr, Transport},
    P2pNet,
};
use city_shard::{District, GeoBox, ShardMap};
use city_syscall::{
    service_cap_slot, service_name, service_opcode, CAP_CITY_DAG as ABI_CAP_CITY_DAG,
    CITY_SERVICES, SYS_CITY_BASE, SYS_CITY_SDR_RX,
};
use city_types::Bytes32;
use city_vote::{make_ballot, VoteTally};

const PASS: &str = "[TEST] PASS";
const FAIL: &str = "[TEST] FAIL";

fn check(name: &str, ok: bool) -> bool {
    println!("  {name}: {}", if ok { PASS } else { FAIL });
    ok
}

fn main() {
    println!("[CITY OS] node boot (host demo, KELLER OS foundation)");

    // ------------------------------------------------------------------
    // L0 — Kernel foundation (KELLER-aligned)
    // ------------------------------------------------------------------
    println!("[L0] kernel foundation");

    // Capability table: KELLER slots 0..=6 + City slots 7..=11.
    city_caps::init();
    let handles: Vec<usize> = vec![
        city_caps::issue_for_process(CAP_CITY_DAG, 1).unwrap(),
        city_caps::issue_for_process(CAP_CITY_BRP, 1).unwrap(),
        city_caps::issue_for_process(CAP_CITY_DEADDROP, 1).unwrap(),
        city_caps::issue_for_process(CAP_CITY_ALLOC, 1).unwrap(),
    ];
    let ok_caps = city_caps::check_handle(handles[0], CAP_CITY_DAG)
        && !city_caps::check_handle(handles[0], CAP_CITY_BRP)
        && city_caps::revoke_handle(handles[3])
        && !city_caps::check_handle(handles[3], CAP_CITY_ALLOC);
    if !check("caps_issue_revoke", ok_caps) {
        std::process::exit(1);
    }

    // IPC: salted-MAC channel roundtrip + anti-replay.
    city_ipc::init();
    let ok_ipc = city_ipc::send(1, 1, 1, b"ledger-txn")
        && city_ipc::recv(1).map(|m| m.sender == 1).unwrap_or(false);
    if !check("ipc_salted_mac", ok_ipc) {
        std::process::exit(1);
    }

    // Vault: AES-XTS store/retrieve.
    city_crypto::vault::dispose_all();
    let vault_ok = city_crypto::vault::store(b"deaddrop-secret").is_some() && {
        let mut out = [0u8; 15];
        city_crypto::vault::retrieve(0, &mut out) && out[..] == b"deaddrop-secret"[..]
    };
    if !check("vault_aes_xts", vault_ok) {
        std::process::exit(1);
    }

    // ------------------------------------------------------------------
    // L1 — City services
    // ------------------------------------------------------------------
    println!("[L1] district services");

    // DAG ledger: author + tip + finality.
    let mut dag = Dag::new();
    let b0 = Bytes32::from_slice(b"energy-tx/001").unwrap();
    let b1 = Bytes32::from_slice(b"energy-tx/002").unwrap();
    let b2 = Bytes32::from_slice(b"energy-tx/003").unwrap();
    let v0 = dag.author(1, &b0).unwrap();
    let _v1 = dag.author(1, &b1).unwrap();
    let tips01 = dag.tips();
    let v2 = dag.insert(2, tips01, &b2, false).unwrap();
    let mut parents = [v2 as u8, v0 as u8];
    for _ in 0..10 {
        let slot = dag.insert(1, parents, &b1, false).unwrap();
        parents = [slot as u8, parents[0]];
    }
    let finalized = dag.advance_finality();
    let ok_dag = finalized > 0 && dag.get(v0).unwrap().finalized_under > 0 && dag.count() >= 13;
    if !check("dag_ledger", ok_dag) {
        std::process::exit(1);
    }

    // BRP identity-less access: token + Schnorr proof (256-bit nonce).
    let nonce32 = entropy_seed(0xDEAD_BEEF);
    let tok = BrpToken(city_brp::issue(0xC1, 7, &nonce32).0);
    let challenge = derive_entropy(&entropy_seed(0x1234_5678));
    let blind = derive_entropy(&entropy_seed(0xCAFE_F00D));
    let ok_brp = city_brp::verify(&tok, 0xC1, 7, &nonce32) && {
        // commit + response + verify
        let x = city_brp::token_secret(&tok);
        let comm = city_brp::commit(&blind);
        let r = city_brp::compute_response(&blind, &challenge, &x);
        let pubpt = city_brp::token_public(&tok);
        city_brp::verify_proof(&comm, &challenge, &r, &pubpt)
    };
    if !check("brp_identityless", ok_brp) {
        std::process::exit(1);
    }

    // P2P gossip: verified frames (MAC + replay).
    let mut net = P2pNet::new();
    net.add_peer(2, 2);
    net.add_peer(3, 3);
    let announce = announce_payload(1, 7);
    let mut f = net.build_frame(1, city_p2p::kind::GOSSIP, 1, &announce);
    let ok_gossip = net.ingest(&f)
        && !net.ingest(&f) // replay rejected
        && net.broadcast(&mut f) == 2;
    if !check("p2p_gossip", ok_gossip) {
        std::process::exit(1);
    }

    // District simulation: 4 nodes gossip 10 rounds and converge on the
    // same verified ledger contents.
    let mut district = city_sim::Simulation::new(4, 10);
    district.run();
    let ok_sim = district.converged(4 * 10);
    if !check("sim_convergence", ok_sim) {
        std::process::exit(1);
    }

    // Shards: geofenced district ownership + vault roundtrip.
    let mut map = ShardMap::new();
    map.add(District::new(
        1,
        GeoBox {
            lat0: 0,
            lon0: 0,
            lat1: 100,
            lon1: 100,
        },
    ));
    let tag = ((50u32) << 16) | 50;
    let shard: &[u8] = b"district-local-shard-24!";
    let ok_shard = map.put(tag, shard).map(|(d, _)| d == 1).unwrap_or(false) && {
        let mut out = [0u8; 24];
        map.get(tag, &mut out) && out[..] == shard[..]
    };
    if !check("shard_geofence", ok_shard) {
        std::process::exit(1);
    }

    // Dead-drops: TTL-encrypted pickup.
    let mut drops = DeadDropHeap::new();
    let dtag = Bytes32::from_slice(b"lamp-42").unwrap();
    let mut drop_out = [0u8; 18];
    let ok_drop = drops.drop(7, &dtag, b"pickup-coordinates", 5).is_some()
        && drops
            .pickup(&dtag, &mut drop_out)
            .map(|o| o == 7)
            .unwrap_or(false)
        && drop_out[..] == *b"pickup-coordinates";
    if !check("deaddrop_ttl", ok_drop) {
        std::process::exit(1);
    }

    // Resource allocation: prediction + scarcity enforcement.
    let mut alloc = DistrictAllocator::new();
    alloc.record_demand(0, 100, 200);
    alloc.record_demand(1, 110, 200);
    alloc.record_demand(2, 120, 200);
    alloc.record_demand(3, 130, 200);
    let pred = alloc.predict(4);
    let rec = alloc.grant(7, 4, 50);
    let mut scarce = DistrictAllocator::new();
    scarce.record_demand(3, 200, 50);
    let ok_alloc = pred == 115
        && rec
            .map(|r| r.granted == 100 && r.is_valid())
            .unwrap_or(false)
        && scarce.grant(1, 5, 50).is_none();
    if !check("alloc_predict_grant", ok_alloc) {
        std::process::exit(1);
    }

    // ------------------------------------------------------------------
    // L2 — Governance, interop, incentives & entropy
    // ------------------------------------------------------------------
    println!("[L2] governance & edge services");

    // Anonymous governance: DAG-chained ballots with ZK validity proofs.
    let mut tally = VoteTally::new();
    let k = derive_entropy(&entropy_seed(0xA11CE));
    let mut prev = 0u32;
    let mut ok_vote = true;
    for choice in [1u8, 1, 0, 1] {
        let b = make_ballot(42, choice, prev, &k);
        match tally.cast(&b) {
            Some(slot) => prev = slot as u32,
            None => ok_vote = false,
        }
    }
    ok_vote &= tally.count() == 4 && tally.winner(42) == Some(1);
    if !check("vote_anonymous", ok_vote) {
        std::process::exit(1);
    }

    // Self-healing: standby promoted when the primary drifts offline.
    let mut health = HealthTable::with_threshold(5);
    health.register(1, 10);
    health.register(2, 0);
    health.set_standby(2);
    for _ in 0..6 {
        health.tick();
    }
    let ok_heal =
        !health.get(1).map(|n| n.healthy).unwrap_or(true) && health.failover(10) == Some(2);
    if !check("heal_failover", ok_heal) {
        std::process::exit(1);
    }

    // Interop: verified V2V frame + FIFO right-of-way at an intersection.
    let vbody = Bytes32::from_slice(b"right-of-way request").unwrap();
    let mut msg = V2vMsg {
        kind: KIND_V2V,
        from: 1,
        seq: 7,
        body: vbody,
        mac: 0,
    };
    let mut inter = IntersectionScheduler::with_capacity(2);
    inter.arrive(11, 0);
    inter.arrive(22, 1);
    msg.mac = msg.compute_mac();
    let ok_interop = msg.signature_ok() && inter.grant_next() == 2 && inter.clear_passed() == 2;
    if !check("interop_v2v_traffic", ok_interop) {
        std::process::exit(1);
    }

    // Incentive: MAC-signed uptime claim → energy credits; forgery rejected.
    let mut ledger = IncentiveLedger::new();
    ledger.register(1);
    let mut claim = UptimeClaim {
        node: 1,
        window: 1,
        uptime_ticks: 100,
        reward: 0,
        mac: 0,
    };
    claim.reward = claim.uptime_ticks * INCENT_CREDITS_PER_UPTIME;
    claim.mac = claim.compute_mac();
    let ok_inc = ledger.submit(&claim) == Some(1000) && ledger.credits(1) == Some(1000) && {
        let mut forged = claim;
        forged.mac ^= 0x0000_0001;
        ledger.submit(&forged).is_none()
    };
    if !check("incentive_uptime", ok_inc) {
        std::process::exit(1);
    }

    // Environmental entropy: noisy neighbors sync a shared OTP key.
    let mut beacon = EntropyBeacon::new();
    let sig_a = EnvSignature::from_readings(3, 0x15, 0x2B, 0x1C);
    let sig_b = EnvSignature::from_readings(3, 0x15, 0x2B + 3, 0x1C);
    beacon.record(&sig_a);
    let shared = sig_a.otp_key();
    let otp1 = derive_otp(&shared, 1);
    let ok_entropy = sig_a.is_in_sync(&sig_b, 8)
        && beacon.confirm_sync(&sig_b, 8)
        && sig_a.otp_key() == sig_b.otp_key()
        && otp1 != derive_otp(&shared, 2);
    if !check("entropy_sync_otp", ok_entropy) {
        std::process::exit(1);
    }

    // ------------------------------------------------------------------
    // L3 — Kernel syscall ABI + hardware transport bridge (Phase 3.6)
    // ------------------------------------------------------------------
    println!("[L3] kernel ABI & hardware bridge");

    // The host-side ABI mirrors the KELLER kernel's SYS_CITY_* range.
    let ok_abi = SYS_CITY_BASE == 64
        && SYS_CITY_SDR_RX == 77
        && service_cap_slot(city_syscall::CITY_DAG) == Some(ABI_CAP_CITY_DAG)
        && (0..CITY_SERVICES as u32).all(|s| service_opcode(s).is_some())
        && service_name(0) == "dag";
    if !check("syscall_abi", ok_abi) {
        std::process::exit(1);
    }

    // e1000 NIC bridge: host build has no PCI config space, so absent;
    // the virtual MMIO ring is still safely programmable.
    let nic = Nic::probe();
    let mut buf = [0u8; 32];
    let ok_nic = !nic.present
        && !Transport::Nic(nic).tx(b"gossip-frame", 0) // no hw
        && Transport::Nic(Nic::from_base(0xDEAD_0000)).rx(&mut buf, 0) == 32;
    if !check("hw_e1000_bridge", ok_nic) {
        std::process::exit(1);
    }

    // SDR Shadow Grid seam: armed is required for TX; RX buffers until
    // real RF hardware binds in Phase 4.
    let mut sdr = Sdr::new();
    sdr.arm(true);
    let ok_sdr = !Sdr::new().tx(b"disarmed-reject") && sdr.tx(b"sdr-shadow-frame");
    if !check("hw_sdr_seam", ok_sdr) {
        std::process::exit(1);
    }

    // ------------------------------------------------------------------
    // L4 — Summary
    // ------------------------------------------------------------------
    println!(
        "[CITY OS] node boot complete: caps={} dag={} peers={} votes={} credits={}",
        city_caps::TABLE.lock().live_count(),
        dag.count(),
        net.peer_count(),
        tally.count(),
        ledger.credits(1).unwrap_or(0)
    );
    println!("[HARNESS] CITY OS BOOT: PASS");
}
