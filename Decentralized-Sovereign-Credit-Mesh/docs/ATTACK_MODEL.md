# Attack Model & Security Analysis

## Threat Classification

### 1. Sybil Attack
**Goal:** Create many fake identities to gain disproportionate trust/credit.

**Mitigations:**
- Graph entropy analysis detects low-entropy clusters (bots have uniform degree distributions)
- Sybil scoring combines: entropy ratio (40%), reciprocity (30%), cluster density (30%)
- `detect_sybil_clusters()` performs BFS from high-scoring nodes to find connected Sybil groups
- Vouching limits constrain propagation from Sybil nodes

**Residual risk:** Small, carefully constructed Sybils can evade detection
until the graph reaches meaningful size.

### 2. Double-Borrow Attack
**Goal:** Obtain multiple credit lines using the same real identity.

**Mitigations:**
- Deterministic nullifier: SHA256(identity_id || public_key || nonce)
- NullifierRegistry prevents reuse of the same (identity, nonce) pair
- Collision resistance from SHA256

**Residual risk:** If an attacker compromises the nullifier registry
state file, they could replay older nullifiers. Mitigated by file integrity.

### 3. Collusion Attack
**Goal:** A group of nodes vouches for each other to inflate reputation.

**Mitigations:**
- Vouching limits cap breadth (max 50 outgoing vouches)
- Trust decay reduces influence per hop (0.8x per hop)
- PageRank naturally penalizes sink nodes with no external edges
- Sybil detection identifies dense intra-group connections

**Residual risk:** A determined group with external connections
can still accumulate reputation through legitimate behavior.

### 4. Reputation Gaming
**Goal:** Manipulate reputation scores through strategic repayments.

**Mitigations:**
- Taint tracking records all repayment flows
- Reputation boost from taint is capped (composite += flow * 0.2)
- PageRank is the dominant factor (40% of composite score)
- Historical flow data makes gaming expensive

**Residual risk:** Well-funded attackers can temporarily boost
reputation through circular repayments (net flow ~ zero).

### 5. SMPC Share Collusion
**Goal:** t-of-n parties collude to reconstruct raw asset/liability values.

**Mitigations:**
- Threshold must be > n/2 for honest majority
- Shares reveal nothing individually (uniform random in field)
- Field arithmetic in GF(2^31 - 1) prevents overflow analysis

**Residual risk:** If t parties collude, they can reconstruct the
secret. This is a fundamental limitation of threshold cryptography.

### 6. ZKP Forgery
**Goal:** Create a valid cluster membership proof without being in the cluster.

**Mitigations:**
- Merkle proof requires knowledge of a specific leaf hash
- Collision-resistant SHA256 prevents leaf forgery
- Merkle path verification ensures leaf is in tree

**Residual risk:** Simplified proof system (not Groth16/PLONK).
For production, replace with proper zk-SNARK circuits.

### 7. Front-Running / Replay
**Goal:** Intercept and replay signed vouching statements.

**Mitigations:**
- Timestamps in all signed messages
- Vouch uniqueness enforced by graph (duplicate edges are idempotent)
- Nullifier nonce prevents replay of loan registrations

**Residual risk:** Network-level replay requires sequence numbers
or challenge-response in a full P2P implementation.

## Security Assumptions

1. **Honest majority** of SMPC parties (threshold ≤ n/2 is insecure)
2. **Secure hashing** (SHA256 is collision-resistant)
3. **Ed25519 signature security** (no quantum resistance)
4. **Local state file integrity** (no tamper-proof storage)
5. **No timing side channels** in current implementation

## Recommended Hardening for Production

1. Replace ZKP with Groth16/PLONK over BLS12-381
2. Add TLS for all network communication
3. Use hardware security module (HSM) for signing keys
4. Add Rust-specific hardening (constant-time comparisons)
5. Implement secure state persistence (encrypted at rest)
6. Add rate limiting on API endpoints
7. Regular key rotation for identities
8. Fuzz testing with cargo-fuzz on all cryptographic operations
