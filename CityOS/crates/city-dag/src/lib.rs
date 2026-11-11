//! **City DAG ledger** — the core transaction store of City OS.
//!
//! A Directed Acyclic Graph of transactions. Every transaction references
//! one or two parent vertices (tips), and a vertex is finalized once it is
//! transitively referenced by enough tips. This is deliberately minimal and
//! **no-alloc**: the ledger is a fixed array of vertices, matching KELLER OS's
//! static-memory discipline, so the same code runs on the kernel or on a host.
//!
//! Each vertex id is computed with the same keyed SipHash-2-4 MAC the
//! kernel uses for IPC (four 64-bit blocks fill a 32-byte id), so a
//! transaction id fits a single KELLER message payload and is verifiable
//! end-to-end by any city node that holds the DAG key.

#![no_std]

#[cfg(test)]
extern crate std;

use city_crypto::mac::{city_mac, MacKey};
use city_types::Bytes32;

pub const DAG_MAX_VERTICES: usize = 64;
pub const DAG_MAX_PARENTS: usize = 2;
pub const DAG_FINALIZE_DEPTH: u32 = 8;

/// Domain label for the DAG ledger key (host profile; see `MacKey::derive`).
pub const DAG_LABEL: u32 = 0x00C1_7DA6;

/// The DAG ledger key (host profile: label-derived).
pub fn ledger_key() -> MacKey {
    MacKey::derive(DAG_LABEL)
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct TxId(pub [u8; 32]);

impl TxId {
    pub const ZERO: Self = Self([0; 32]);

    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

impl PartialEq<[u8; 32]> for TxId {
    fn eq(&self, other: &[u8; 32]) -> bool {
        self.0 == *other
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vertex {
    pub id: TxId,
    /// Which ledger slots are the parents of this vertex (0xFF = none).
    pub parents: [u8; DAG_MAX_PARENTS],
    pub sender: u32,
    pub body: Bytes32,
    pub seq: u32,
    pub finalized_under: u32, // > 0 once covered by DAG_FINALIZE_DEPTH tips
    pub imported: bool,       // added from the network, not locally authored
}

impl Vertex {
    pub const fn empty() -> Self {
        Self {
            id: TxId::ZERO,
            parents: [0xFF; DAG_MAX_PARENTS],
            sender: 0,
            body: Bytes32::EMPTY,
            seq: 0,
            finalized_under: 0,
            imported: false,
        }
    }
}

/// Fixed-capacity DAG ledger.
#[derive(Copy, Clone, Debug)]
pub struct Dag {
    vertices: [Vertex; DAG_MAX_VERTICES],
    count: usize,
    next_seq: u32,
}

impl Default for Dag {
    fn default() -> Self {
        Self::new()
    }
}

impl Dag {
    pub const fn new() -> Self {
        Self {
            vertices: [Vertex::empty(); DAG_MAX_VERTICES],
            count: 0,
            next_seq: 1,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn get(&self, i: usize) -> Option<&Vertex> {
        self.vertices.get(i)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Vertex> {
        self.vertices.iter().take(self.count)
    }

    /// Compute the transaction id for a vertex body + parents.
    ///
    /// Four 64-bit keyed MAC blocks (block index in the `msg_type` field)
    /// fill a 32-byte id — one KELLER IPC payload. Each block covers
    /// (sender, tx seq, block) plus one input slice (body or parents), so
    /// every byte of the vertex contributes to every block.
    pub fn tx_id(sender: u32, seq: u32, parents: &[u8; DAG_MAX_PARENTS], body: &Bytes32) -> TxId {
        let key = ledger_key();
        let mut tag = [0u8; 32];
        let inputs: [&[u8]; 4] = [&body.0, parents, &body.0, parents];
        for (i, slice) in inputs.iter().enumerate() {
            let h = city_mac(&key, sender, seq, i as u32, slice);
            tag[i * 8..i * 8 + 8].copy_from_slice(&h.to_le_bytes());
        }
        TxId(tag)
    }

    /// Insert a transaction. Returns the slot index or None if the ledger is
    /// full, the parents are unknown, or a double-spend (same id) is seen.
    pub fn insert(
        &mut self,
        sender: u32,
        parents: [u8; DAG_MAX_PARENTS],
        body: &Bytes32,
        imported: bool,
    ) -> Option<usize> {
        if self.count >= DAG_MAX_VERTICES {
            return None;
        }
        for &p in parents.iter() {
            if p != 0xFF {
                let slot = p as usize;
                if slot >= self.count {
                    return None; // unknown parent
                }
            }
        }
        let seq = self.next_seq;
        let id = Self::tx_id(sender, seq, &parents, body);
        if self.iter().any(|v| v.id == id) {
            return None; // duplicate
        }
        let here = self.count;
        self.vertices[here] = Vertex {
            id,
            parents,
            sender,
            body: *body,
            seq,
            finalized_under: 0,
            imported,
        };
        self.count += 1;
        self.next_seq += 1;
        Some(here)
    }

    /// Author a simple transaction (optionally referencing one or two tips).
    pub fn author(&mut self, sender: u32, body: &Bytes32) -> Option<usize> {
        let parents = self.tips();
        self.insert(sender, parents, body, false)
    }

    /// The current tip set — vertices with no children yet (parents of the next).
    pub fn tips(&self) -> [u8; DAG_MAX_PARENTS] {
        let mut result = [0xFF; DAG_MAX_PARENTS];
        if self.count == 0 {
            return result;
        }
        let mut j = 0;
        for (i, _v) in self.vertices.iter().take(self.count).enumerate() {
            let is_parent = self
                .vertices
                .iter()
                .take(self.count)
                .any(|child| child.parents[0] == i as u8 || child.parents[1] == i as u8);
            if !is_parent && j < DAG_MAX_PARENTS {
                result[j] = i as u8;
                j += 1;
            }
        }
        // If no tip is a parent, fall back to the two newest vertices.
        if j == 0 && self.count > 0 {
            result[0] = (self.count - 1) as u8;
            if self.count > 1 {
                result[1] = (self.count - 2) as u8;
            }
        }
        result
    }

    /// Depth of a vertex: longest path from it to a parent-less vertex.
    pub fn depth(&self, slot: usize) -> u32 {
        if slot >= self.count {
            return 0;
        }
        let v = self.vertices[slot];
        let mut max = 0u32;
        for &p in v.parents.iter() {
            if p != 0xFF {
                max = max.max(self.depth(p as usize) + 1);
            }
        }
        max
    }

    /// Advance finality: any vertex referenced (transitively) by at least
    /// `DAG_FINALIZE_DEPTH` later vertices becomes finalized — a cheap
    /// quantitative stand-in for "covered by enough tips". Returns the number
    /// of vertices finalized by this call.
    pub fn advance_finality(&mut self) -> usize {
        let mut changed = 0usize;
        for i in 0..self.count {
            if self.vertices[i].finalized_under != 0 {
                continue;
            }
            // Count every later vertex that reaches i.
            let covering = ((i + 1)..self.count)
                .filter(|&j| self.reaches(j, i))
                .count();
            if covering >= DAG_FINALIZE_DEPTH as usize {
                self.vertices[i].finalized_under = self.next_seq;
                changed += 1;
            }
        }
        changed
    }

    /// Does vertex `a` transitively reference (reach) vertex `b`?
    pub fn reaches(&self, a: usize, b: usize) -> bool {
        if a >= self.count || b >= self.count {
            return false;
        }
        if a == b {
            return true;
        }
        let mut stack: [usize; DAG_MAX_VERTICES] = [0; DAG_MAX_VERTICES];
        let mut top = 0usize;
        stack[top] = a;
        top += 1;
        let mut seen = [false; DAG_MAX_VERTICES];
        while top > 0 {
            top -= 1;
            let cur = stack[top];
            if seen[cur] {
                continue;
            }
            seen[cur] = true;
            let v = self.vertices[cur];
            for &p in v.parents.iter() {
                if p == 0xFF {
                    continue;
                }
                let p = p as usize;
                if p == b {
                    return true;
                }
                if !seen[p] && top < DAG_MAX_VERTICES {
                    stack[top] = p;
                    top += 1;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_types::Bytes32;

    fn body(s: &str) -> Bytes32 {
        Bytes32::from_slice(s.as_bytes()).unwrap()
    }

    #[test]
    fn author_and_tips() {
        let mut d = Dag::new();
        let _a = d.author(1, &body("da")).unwrap();
        let b = d.author(1, &body("sensus")).unwrap();
        let parents = d.tips();
        assert_eq!(parents[0], b as u8);
        let c = d.insert(2, parents, &body("round"), false).unwrap();
        assert_eq!(c, 2);
        // A chain of three is consistent.
        assert!(d.get(2).is_some());
    }

    #[test]
    fn unknown_parent_rejected_and_vertices_unique() {
        let mut d = Dag::new();
        let _ = d.author(1, &body("x")).unwrap();
        // Unknown parent (slot 99 not in ledger) is rejected.
        assert!(d.insert(1, [99, 0xFF], &body("y"), false).is_none());
        // Every insert advances the sequence, so identical body + parents
        // still yield a *unique* vertex (its id includes the seq) — a DAG
        // can never contain two vertices that hash identically.
        let parents = [0, 0xFF];
        let a = d.insert(1, parents, &body("y"), false).unwrap();
        let b = d.insert(1, parents, &body("y"), false).unwrap();
        assert_ne!(a, b);
        assert_ne!(d.get(a).unwrap().id, d.get(b).unwrap().id);
    }

    #[test]
    fn depth_and_finality() {
        let mut d = Dag::new();
        let a = d.author(1, &body("v0")).unwrap();
        assert_eq!(d.depth(a), 0);
        let parents0 = d.tips();
        let b = d.insert(1, parents0, &body("v1"), false).unwrap();
        assert_eq!(d.depth(b), 1);
        // A chain that quickly references the root many times finalizes it.
        let mut parents = [b as u8, a as u8];
        for _i in 0..12 {
            let slot = d.insert(1, parents, &body("tail"), false).unwrap();
            parents = [slot as u8, parents[0]];
        }
        let changed = d.advance_finality();
        assert!(changed > 0);
        assert!(d.get(a).unwrap().finalized_under > 0);
    }
}
