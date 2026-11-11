//! **City OS district simulation** — a multi-node gossip/ledger harness.
//!
//! Runs several `city-dag` + `city-p2p` instances as a small district and
//! proves that gossip **converges**: every transaction authored by any node
//! is verified (MAC), recorded exactly once, and present in every node's
//! ledger after enough rounds.
//!
//! The transport here is a direct in-memory frame delivery (each node keeps
//! an inbox); the real node transports are the KELLER IPC channels and the
//! e1000 NIC. The harness exercises the *node-level* protocol exactly as
//! deployed: frames are built by `city-p2p` (keyed SipHash MAC + seq
//! anti-replay), verified on ingest, and only verified frames enter the DAG.
//!
//! **Honest scope.** Each node keeps its own DAG; gossip replicates the
//! *transaction bodies* across nodes, and each node chains them into its
//! local ledger using its own tips. There is no shared global DAG identity
//! or Byzantine consensus here — that is the kernel's shared-ledger work.
//! What this proves: verified broadcast + replay rejection + identical
//! ledger *contents* on every node.

#![no_std]

#[cfg(test)]
extern crate std;

use city_dag::Dag;
use city_p2p::{Frame, P2pNet};
use city_types::{Bytes32, NodeId};

/// Maximum district size (nodes).
pub const SIM_MAX_NODES: usize = 8;
/// Per-node pending-frame inbox capacity.
pub const SIM_MAX_PENDING: usize = 96;

/// One district node: its own ledger + gossip neighborhood + inbox.
#[derive(Copy, Clone, Debug)]
pub struct SimNode {
    pub id: NodeId,
    pub dag: Dag,
    pub net: P2pNet,
    seq: u32,
    inbox: [Frame; SIM_MAX_PENDING],
    inbox_count: usize,
}

impl SimNode {
    pub const fn new(id: NodeId) -> Self {
        Self {
            id,
            dag: Dag::new(),
            net: P2pNet::new(),
            seq: 0,
            inbox: [Frame {
                seq: 0,
                kind: 0,
                from: 0,
                body: Bytes32::EMPTY,
                mac: 0,
            }; SIM_MAX_PENDING],
            inbox_count: 0,
        }
    }

    fn deliver(&mut self, frame: Frame) {
        if self.inbox_count < SIM_MAX_PENDING {
            self.inbox[self.inbox_count] = frame;
            self.inbox_count += 1;
        }
    }

    /// Process every pending frame: verify the MAC (drop invalid/replayed),
    /// then record the transaction body in the local ledger exactly once.
    /// Returns (accepted, rejected).
    fn drain_and_apply(&mut self) -> (usize, usize) {
        let mut accepted = 0usize;
        let mut rejected = 0usize;
        for i in 0..self.inbox_count {
            let frame = self.inbox[i];
            if self.net.ingest(&frame) {
                // Only MAC-verified, non-replayed frames reach the ledger.
                if self
                    .dag
                    .insert(frame.from, self.dag.tips(), &frame.body, true)
                    .is_some()
                {
                    accepted += 1;
                } else {
                    rejected += 1;
                }
            } else {
                rejected += 1;
            }
        }
        self.inbox_count = 0;
        (accepted, rejected)
    }
}

/// A full-mesh district simulation.
#[derive(Copy, Clone, Debug)]
pub struct Simulation {
    nodes: [SimNode; SIM_MAX_NODES],
    node_count: usize,
    rounds: usize,
}

/// Deterministic transaction body for `(node, round)` — unique per pair, so
/// convergence is checkable by body.
pub fn body_for(node: usize, round: usize) -> Bytes32 {
    Bytes32::from_slice(&[node as u8, round as u8]).unwrap()
}

impl Simulation {
    /// Create a full-mesh district of `node_count` nodes (max 8) that will
    /// gossip for `rounds` rounds.
    pub fn new(node_count: usize, rounds: usize) -> Self {
        let mut nodes = [SimNode::new(0); SIM_MAX_NODES];
        let node_count = node_count.min(SIM_MAX_NODES);
        for (i, node_slot) in nodes.iter_mut().enumerate().take(node_count) {
            let mut n = SimNode::new(i as NodeId);
            // Full mesh: every node peers with every other node.
            for j in 0..node_count {
                if i != j {
                    n.net.add_peer(j as NodeId, j % 8);
                }
            }
            *node_slot = n;
        }
        Self {
            nodes,
            node_count,
            rounds,
        }
    }

    pub fn node(&self, i: usize) -> &SimNode {
        &self.nodes[i]
    }

    /// Run the district for the configured number of rounds.
    ///
    /// Each round: every node authors one transaction, fans the verified
    /// frame out to all peers, then all nodes ingest + record their inbox.
    /// Returns the total number of ledger inserts across the district.
    pub fn run(&mut self) -> usize {
        let mut total = 0usize;
        for round in 0..self.rounds {
            // 1. Author + fan out.
            for i in 0..self.node_count {
                let body = body_for(i, round);
                // The author records its own transaction locally too.
                let _ = self.nodes[i].dag.author(self.nodes[i].id, &body);
                let (seq, from) = {
                    let n = &mut self.nodes[i];
                    n.seq += 1;
                    (n.seq, n.id)
                };
                let frame = self.nodes[i]
                    .net
                    .build_frame(seq, city_p2p::kind::GOSSIP, from, &body);
                for j in 0..self.node_count {
                    if i != j {
                        self.nodes[j].deliver(frame);
                    }
                }
            }
            // 2. Verify + record on every node.
            for i in 0..self.node_count {
                let (accepted, _) = self.nodes[i].drain_and_apply();
                total += accepted;
            }
        }
        total
    }

    /// Every node has `expected_per_node` transactions, all identical in
    /// content (converged ledger contents).
    pub fn converged(&self, expected_per_node: usize) -> bool {
        for i in 0..self.node_count {
            let n = &self.nodes[i];
            if n.dag.count() != expected_per_node {
                return false;
            }
            // Every authored body (any node, any round) is present here.
            for node in 0..self.node_count {
                for round in 0..self.rounds {
                    let b = body_for(node, round);
                    if !n.dag.iter().any(|v| v.body == b) {
                        return false;
                    }
                }
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gossip_converges_across_nodes() {
        let nodes = 4usize;
        let rounds = 10usize;
        let expected = nodes * rounds;

        let mut sim = Simulation::new(nodes, rounds);
        // Every frame is MAC-verified + recorded exactly once: accepted ==
        // expected per node and no double inserts.
        sim.run();
        assert!(sim.converged(expected), "district did not converge");

        // Each node's ledger holds every body exactly once and finalizes.
        for i in 0..nodes {
            let n = sim.node(i);
            assert_eq!(n.dag.count(), expected, "node {i} vertex count");
            // Uniqueness: no two vertices share a body.
            for a in 0..expected {
                for b in (a + 1)..expected {
                    assert_ne!(n.dag.get(a).unwrap().body, n.dag.get(b).unwrap().body);
                }
            }
            // Finality advances once the ledger is deep enough.
            let mut dag = n.dag;
            assert!(dag.advance_finality() > 0, "node {i} did not finalize");
            assert!(
                dag.get(0).unwrap().finalized_under > 0,
                "node {i} root not finalized"
            );
        }
    }

    #[test]
    fn replayed_and_tampered_frames_never_enter_the_ledger() {
        let nodes = 3usize;
        let rounds = 5usize;
        let expected = nodes * rounds;

        let mut sim = Simulation::new(nodes, rounds);
        sim.run();
        assert!(sim.converged(expected));

        // Redeliver an old frame to node 1: replay must be rejected and the
        // ledger must not grow.
        let old = sim
            .node(0)
            .net
            .build_frame(1, city_p2p::kind::GOSSIP, 0, &body_for(0, 0));
        sim.nodes[1].deliver(old);
        let before = sim.node(1).dag.count();
        sim.nodes[1].drain_and_apply();
        assert_eq!(sim.node(1).dag.count(), before, "replay double-inserted");

        // A tampered frame (bad MAC) is dropped before the ledger.
        let mut forged = sim
            .node(0)
            .net
            .build_frame(2, city_p2p::kind::GOSSIP, 0, &body_for(0, 1));
        forged.body.0[0] ^= 0xFF;
        sim.nodes[1].deliver(forged);
        sim.nodes[1].drain_and_apply();
        assert_eq!(
            sim.node(1).dag.count(),
            before,
            "forged frame entered ledger"
        );
    }

    #[test]
    fn district_capacity_is_bounded() {
        // The fixed inbox + peer tables bound the district: oversized
        // districts are clamped, and the ledger caps at DAG_MAX_VERTICES.
        let mut sim = Simulation::new(9, 1); // 9 requested, clamped to 8
        assert!(sim.node_count <= SIM_MAX_NODES);
        sim.run();
        assert!(sim.converged(sim.node_count));
    }
}
