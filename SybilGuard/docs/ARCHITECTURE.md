# Sybil-Resistentes Architecture

## Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Sybil-Resistentes System                   │
│                                                             │
│  ┌─────────┐  ┌──────────┐  ┌────────┐  ┌──────────────┐  │
│  │  Graph   │  │  Sybil   │  │  Rep   │  │  Web of      │  │
│  │ Analysis │◄─┤ Detection│◄─┤utation │◄─┤Trust (Wo T)  │  │
│  └────┬────┘  └──────────┘  └────────┘  └──────┬───────┘  │
│       │                                         │          │
│  ┌────▼──────────────────────────────────────────▼───────┐  │
│  │              Core (types, traits, errors)              │  │
│  └────────────────────────┬──────────────────────────────┘  │
│                           │                                  │
│  ┌────────────────────────▼──────────────────────────────┐  │
│  │     DID  │  Nullifier  │  zk Proofs  │  Storage       │  │
│  └──────────────────────────────────────────────────────┘  │
│                           │                                  │
│  ┌────────────────────────▼──────────────────────────────┐  │
│  │              API  │  SDK  │  CLI                      │  │
│  └──────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
```

## Crate Dependency Graph

```
sybil-core ───┬── sybil-graph ───┬── sybil-sybil
              │                  └── sybil-reputation
              ├── sybil-wot
              ├── sybil-did
              ├── sybil-nullifier
              ├── sybil-zk (Phase 2)
              ├── sybil-api ───┬── sybil-sdk
              └── sybil-cli ───┘
```

## Key Design Decisions

1. **Undirected graph model**: Trust relationships are inherently bidirectional; a vouch from A→B implies mutual recognition.

2. **Entropy-based Sybil detection**: Rather than relying on fixed thresholds, we measure deviation from expected small-world properties. Human social networks have characteristic clustering coefficients (0.3-0.8) and short average path lengths.

3. **Reputation is embedded in the graph**: A node's score is derived from its position and behavior in the trust network, not from external biometrics.

4. **zk-SNARKs deferred to v2**: The privacy layer is architecturally planned but deferred until the core graph and WoT protocols are validated at scale.

5. **Deterministic nullifiers**: Using `BLAKE3("nullifier" || seed)` ensures unlinkable yet deterministic identity commitments.

## Data Flow

```
User Creates Identity
  → Keypair generated (ed25519)
  → DID document created (did:sybil:...)
  → Node added to social graph
  → Initial reputation score (1.0)

User Gets Vouched For
  → Vouch record signed by anchor
  → Trust chain verified (BFS from anchor)
  → Embedding depth updated
  → Reputation recalculated

Sybil Detection
  → Community detection (connected components)
  → Per-region entropy analysis
  → Anomaly scoring (clustering coefficient deviation)
  → Confidence-weighted classification

Periodic Maintenance
  → Reputation decay applied (0.1%/day)
  → Inactive nodes flagged for reactivation
  → Verification events validate liveness