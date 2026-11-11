# Credit Mesh Protocol v0.1

## Overview

The Decentralized Sovereign Credit Mesh is a trust-based, privacy-preserving
credit system that uses social graph topology, web-of-trust vouching,
secure multi-party computation (SMPC) for solvency verification,
zero-knowledge proofs for cluster membership, and autonomous credit
pool issuance.

## Core Protocol Layers

### Layer 1: Social Graph

- **Graph model:** Directed weighted graph with typed edges
- **Relationship types:** Friend, Family, Colleague, Guarantor, Custom
- **Trust weights:** f64 in (0.0, 1.0]
- **Identity:** ed25519 keypair per node
- **Algorithms:**
  - PageRank for global trust ranking
  - Local clustering coefficient for community analysis
  - BFS shortest path for trust path discovery
  - Graph entropy for Sybil detection

### Layer 2: Web of Trust

- Vouching statements are SHA256-canonicalized messages signed with ed25519
- Each vouch contains: voucher_id, target_id, relationship, trust_weight, timestamp, signature
- Chain verification: cumulative trust = product of weights along path
- Trust propagation limited by configurable depth (default 5) and breadth (default 50)
- Exponential decay: `effective = base_weight * decay_factor^depth`

### Layer 3: SMPC Solvency

- Solvency is verified without revealing raw asset/liability values
- Uses Shamir's Secret Sharing over GF(2^31 - 1)
- Threshold scheme: k-of-n parties must cooperate
- Each party receives `(asset_share, liability_share)`
- Net position = `sum(asset_shares) - sum(liability_shares)` reconstructed via Lagrange interpolation
- If net position >= 0, entity is solvent

### Layer 4: Zero-Knowledge Proofs (simplified)

- Hash-based commitments (SHA256)
- Merkle tree for cluster membership
- Cluster membership proof demonstrates:
  - "I belong to a cluster of size ≥ N"
  - "The cluster's average reputation ≥ R"
- Without revealing exact position or individual scores

### Layer 5: Credit Engine

- Boolean constraint system with AND/OR/NOT combinators
- Constraint types:
  - TrustPathExists: is there a trust path within depth?
  - MinReputation: does user exceed reputation threshold?
  - MaxExposure: is total credit within limit?
  - VouchingLimit: is propagation depth within bounds?
  - SybilScoreBelow: is Sybil risk acceptable?
- Interest rate model: `rate = base + risk_spread + sybil_penalty`
- Risk tiers: low (2% spread), medium (6%), high (12%)

### Layer 6: Settlement

- Credit pools aggregate liquidity from providers
- Providers deposit assets, receive proportional shares
- Utilization rate: allocated / total_liquidity
- Effective APR: target_apr * (1 + utilization_rate)
- Automated loan issuance evaluates:
  1. Credit constraints
  2. SMPC solvency proof (optional)
  3. ZKP cluster membership proof (optional)
- Repayments return to pool, reducing allocated amount

## Message Formats

### Vouch Statement (JSON)
```json
{
  "voucher_id": "user_alice",
  "target_id": "user_bob",
  "relationship": "Friend",
  "trust_weight": 0.9,
  "timestamp": 1700000000,
  "signature_bytes": [0x...]
}
```

### SMPC Share (JSON)
```json
{
  "party_index": 0,
  "asset_share": 1946882304,
  "liability_share": 1192909491
}
```

### Cluster Membership Proof (JSON)
```json
{
  "identity_id": "bob",
  "identity_commitment": { "value": "0x..." },
  "merkle_proof": { "leaf": "0x...", "path": [...], "root": "0x..." },
  "reputation_proof": { "commitment": {...}, "min_reputation": 0.05, "range_proof": "0x..." },
  "cluster_size": 10,
  "avg_reputation": 0.325,
  "timestamp": 1700000000
}
```
