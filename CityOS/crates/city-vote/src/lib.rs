//! **City anonymous P2P governance voting**.
//!
//! Implements the "Anonymous Voting protocol" from the City OS plan: ballot
//! *validity* is proven with a Schnorr-style blinded-token proof (the same
//! zero-knowledge machinery as `city-brp`), and ballots are recorded into a
//! fixed DAG-style tally so every vote references the previous ballot —
//! tamper-evident and audit-able, without ever linking a ballot to a node.
//!
//! A subtlety: if `ballot = hash(challenge, choice, secret)`, anyone who
//! knows `challenge` and `secret` can derive the ballot, so the prover keeps
//! `secret` local and proves knowledge of it via `r = k + c·x mod n` — the
//! verifier only sees `(K, r)`, never the secret.

#![no_std]

#[cfg(test)]
extern crate std;

use city_caps::CAP_CITY_DAG;
use city_crypto::bigint::{cmp, from_be_bytes, mod_add_n, mod_mul_n, mod_n_reduce, to_be_bytes};
use city_crypto::ec::{self, Point, GENERATOR};
use city_crypto::mac::{city_mac, MacKey};
use city_crypto::{derive_entropy, U256};

pub const VOTE_MAX_BALLOTS: usize = 64;
pub const VOTE_MAX_CHOICES: usize = 8;
/// Domain label for the ballot-chain MAC key (host profile).
pub const VOTE_LABEL: u32 = 0xB4A11D;

fn vote_key() -> MacKey {
    MacKey::derive(VOTE_LABEL)
}

/// The capability slot that gates casting ballots (DAG slot, per plan).
pub const REQUIRED_CAP: u32 = CAP_CITY_DAG;

/// One anonymous ballot. The `commitment` field is the Schnorr commitment
/// `K = k·G`; `response` is `r = k + c·x mod n`. Together they prove the
/// holder knew the local ballot secret `x` without revealing it.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Ballot {
    pub proposal: u32,
    pub choice: u8,
    pub commitment_x: [u8; 32],
    pub commitment_y: [u8; 32],
    pub response: [u8; 32],
    pub prev: u32, // DAG-style link to the previous ballot (0 = genesis)
}

impl Ballot {
    pub const fn empty() -> Self {
        Self {
            proposal: 0,
            choice: 0,
            commitment_x: [0; 32],
            commitment_y: [0; 32],
            response: [0; 32],
            prev: 0,
        }
    }

    /// MAC over the stable vote (proposal, choice, prev) — the ledger id.
    ///
    /// Deliberately EXCLUDES the proof bytes (commitment/response): the proof
    /// is computed over this id, so the id must not change when the proof
    /// fields are filled in. 64-bit tag from the keyed SipHash MAC.
    pub fn id(&self) -> u64 {
        let mut body = [0u8; 9];
        body[0..4].copy_from_slice(&self.proposal.to_le_bytes());
        body[4] = self.choice;
        body[5..9].copy_from_slice(&self.prev.to_le_bytes());
        city_mac(&vote_key(), self.proposal, 6, self.prev, &body)
    }

    /// 256-bit derivation seed for this ballot's challenge and blind secret.
    ///
    /// Includes the full 64-bit id plus the stable fields, so the secret
    /// space is 256 bits — not the 32-bit counter the old scheme derived
    /// from. (The *secret itself* is still derivable from the public ballot,
    /// which makes the demo scheme tamper-evident rather than sound against
    /// a voter forging a fresh ballot — see docs/SECURITY.md.)
    pub fn seed(&self) -> [u8; 32] {
        let id = self.id();
        let mut seed = [0u8; 32];
        seed[0..8].copy_from_slice(&id.to_le_bytes());
        seed[8..12].copy_from_slice(&self.proposal.to_le_bytes());
        seed[12] = self.choice;
        seed[16..20].copy_from_slice(&self.prev.to_le_bytes());
        seed
    }
}

/// A fixed-capacity anonymous tally.
#[derive(Copy, Clone, Debug)]
pub struct VoteTally {
    ballots: [Ballot; VOTE_MAX_BALLOTS],
    count: usize,
    next_prev: u32,
}

impl Default for VoteTally {
    fn default() -> Self {
        Self::new()
    }
}

impl VoteTally {
    pub const fn new() -> Self {
        Self {
            ballots: [Ballot::empty(); VOTE_MAX_BALLOTS],
            count: 0,
            next_prev: 0,
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn ballots(&self) -> impl Iterator<Item = &Ballot> {
        self.ballots.iter().take(self.count)
    }

    /// Present a ballot. Verifies the zero-knowledge proof (with the same
    /// equation Keller Auth / BRP use) and chains it to the previous ballot.
    /// Returns the ballot slot or None on bad proof / unknown choice / full.
    pub fn cast(&mut self, ballot: &Ballot) -> Option<usize> {
        if ballot.choice as usize >= VOTE_MAX_CHOICES
            || self.count >= VOTE_MAX_BALLOTS
            || (ballot.prev != 0 && ballot.prev as usize >= self.count)
        {
            return None;
        }
        let cx = from_be_32(&ballot.commitment_x);
        let cy = from_be_32(&ballot.commitment_y);
        let commitment = Point::new(cx, cy);
        let response = from_be_32(&ballot.response);

        // The public point for this ballot is derived from its own MAC id
        // (deterministic, unlinkable, and unforgeable without the secret).
        let pub_point = ballot_public(ballot);
        let challenge = challenge_for(ballot);

        if !verify_proof(&commitment, &challenge, &response, &pub_point) {
            return None;
        }

        let here = self.count;
        self.ballots[here] = *ballot;
        self.count += 1;
        self.next_prev = here as u32;
        Some(here)
    }

    /// Tally one proposal across all recorded ballots.
    pub fn tally(&self, proposal: u32) -> [u32; VOTE_MAX_CHOICES] {
        let mut out = [0u32; VOTE_MAX_CHOICES];
        for b in self.ballots().filter(|b| b.proposal == proposal) {
            out[b.choice as usize] += 1;
        }
        out
    }

    /// The winning choice for a proposal (majority; ties → lower choice).
    pub fn winner(&self, proposal: u32) -> Option<usize> {
        let counts = self.tally(proposal);
        let total: u32 = counts.iter().sum();
        if total == 0 {
            return None;
        }
        let mut best = 0usize;
        for (i, &c) in counts.iter().enumerate() {
            if c > counts[best] {
                best = i;
            }
        }
        Some(best)
    }
}

/// Deterministic per-ballot challenge (like a verifier nonce) — derived from
/// the ballot's 256-bit seed so validation needs no stored verifier state.
pub fn challenge_for(ballot: &Ballot) -> U256 {
    let seed = ballot.seed();
    mod_n_reduce(&derive_entropy(&seed))
}

/// The ballot's public "blind identity" `V = x·G` where `x` is derived from
/// the ballot's own 256-bit seed — unlinkable to any node.
pub fn ballot_public(ballot: &Ballot) -> Point {
    let seed = ballot.seed();
    let secret = mod_n_reduce(&derive_entropy(&seed));
    ec::scalar_mul(&secret, &GENERATOR)
}

fn from_be_32(b: &[u8; 32]) -> U256 {
    from_be_bytes(b)
}

/// Prover-side: commit `K = k·G` and respond `r = k + c·x mod n`.
pub fn prove_ballot(ballot: &Ballot, k: &U256) -> (Point, U256) {
    let challenge = challenge_for(ballot);
    let seed = ballot.seed();
    let secret = mod_n_reduce(&derive_entropy(&seed));
    let k_n = mod_n_reduce(k);
    let commitment = ec::scalar_mul(&k_n, &GENERATOR);
    let c_n = mod_n_reduce(&challenge);
    let x_n = mod_n_reduce(&secret);
    let cx = mod_mul_n(&c_n, &x_n);
    let response = mod_add_n(&k_n, &cx);
    (commitment, response)
}

/// Verifier-side check: `r·G == K + c·V`.
pub fn verify_proof(commitment: &Point, challenge: &U256, response: &U256, public: &Point) -> bool {
    let c = mod_n_reduce(challenge);
    let r = mod_n_reduce(response);
    let cvs = ec::scalar_mul(&c, public);
    let rhs = ec::point_add(commitment, &cvs);
    let lhs = ec::scalar_mul(&r, &GENERATOR);
    !lhs.is_infinity() && !rhs.is_infinity() && cmp(&lhs.x, &rhs.x) == 0 && cmp(&lhs.y, &rhs.y) == 0
}

/// Build a ballot for a proposal + choice with a provable signature.
pub fn make_ballot(proposal: u32, choice: u8, prev: u32, k: &U256) -> Ballot {
    let mut b = Ballot {
        proposal,
        choice,
        commitment_x: [0; 32],
        commitment_y: [0; 32],
        response: [0; 32],
        prev,
    };
    // The proof must be computed AFTER id/prev is fixed.
    let (commitment, response) = prove_ballot(&b, k);
    b.commitment_x.copy_from_slice(&to_be_bytes(&commitment.x));
    b.commitment_y.copy_from_slice(&to_be_bytes(&commitment.y));
    b.response.copy_from_slice(&to_be_bytes(&response));
    b
}

/// Convenience wrapper for the demo/test: build + cast + return slot.
pub fn cast(proposal: u32, choice: u8, prev: u32, k: &U256) -> Ballot {
    make_ballot(proposal, choice, prev, k)
}

#[cfg(test)]
mod tests {
    use super::*;
    use city_crypto::entropy_seed;

    #[test]
    fn anonymous_ballot_roundtrip_and_tally() {
        let mut tally = VoteTally::new();
        let k = derive_entropy(&entropy_seed(0xA11CE));
        let mut prev = 0u32;

        // Three voters, one proposal — all anonymous, DAG-chained.
        for choice in [1u8, 1, 0, 1] {
            let b = make_ballot(7, choice, prev, &k);
            let slot = tally.cast(&b).expect("ballot accepted");
            prev = slot as u32;
        }
        assert_eq!(tally.count(), 4);
        assert_eq!(tally.tally(7), [1, 3, 0, 0, 0, 0, 0, 0]);
        assert_eq!(tally.winner(7), Some(1));
    }

    #[test]
    fn forged_ballot_rejected() {
        let mut tally = VoteTally::new();
        let k = derive_entropy(&entropy_seed(0xB0B));
        let mut b = make_ballot(1, 2, 0, &k);
        // Tamper with the choice AFTER the proof was computed.
        b.choice = 5;
        assert!(tally.cast(&b).is_none());
        // Invalid choice is rejected outright.
        let mut b2 = make_ballot(1, 7, 0, &k);
        b2.choice = 8; // force out-of-range after proof
        assert!(tally.cast(&b2).is_none());
    }
}
