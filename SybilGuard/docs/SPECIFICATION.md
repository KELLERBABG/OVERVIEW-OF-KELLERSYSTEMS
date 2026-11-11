# Sybil-Resistentes Protocol Specification

## 1. Identity Model

### 1.1 Node Identity
Each participant (node) is identified by a `NodeId` (UUID v4). The node holds an ed25519 keypair locally. The public key is encoded as a W3C DID.

### 1.2 DID Method: `did:sybil`
```
did:sybil:<base64(ed25519_public_key)>
```

### 1.3 Identity Nullifier
```
nullifier = BLAKE3("nullifier" || seed)
```
The nullifier is used in zk proofs to prevent double-credential creation without revealing the identity.

## 2. Graph Model

### 2.1 Undirected Social Graph
Edges represent mutual trust relationships. Edge weights are 1.0 by default.

### 2.2 Small-World Properties
Human social graphs are expected to exhibit:
- Clustering coefficient: 0.3–0.8
- Average path length: ~ln(N) / ln(avg_degree)
- Average degree: ~6

## 3. Web of Trust

### 3.1 Vouching
- A node A can vouch for node B (A → B).
- Vouching power is limited by reputation (max 10 vouches per node).
- Maximum trust chain depth: 5 hops.
- Vouches expire after 90 days.

### 3.2 Trust Chain Verification
```
verify(node, anchors):
    BFS from node to nearest anchor
    verify all vouch signatures along path
    check total depth ≤ 5
    return TrustChain{cumulative_weight, path}
```

## 4. Reputation Scoring

### 4.1 Base Score
```
score = base_score
       + depth * 0.1          // embedding bonus
       + cc * 0.05            // cluster bonus
       - days_inactive * 0.001 // decay
       - sybil_penalty        // -10 if flagged
```

### 4.2 Vouching Power
```
vouching_power = floor(score * 2), capped at max(10)
```

## 5. Sybil Detection

### 5.1 Graph Entropy
```
entropy = clustering_deviation × path_length_ratio
```
Higher entropy = more anomalous = more likely Sybil.

### 5.2 Combined Score
```
combined = entropy × 0.4 + anomaly × 0.4 + |internal_density - external_density| × 0.2
```

### 5.3 Classification
- `combined > 2.5` → Suspected Sybil
- `combined > 1.75` → Watchlisted
- `combined < 1.75` → Likely legitimate

## 6. Attack Mitigation

### 6.1 Economic Barriers
- Minimum stake: 100 units to initiate vouching
- Time lock: 30 days before new node can vouch
- Cooldown: 7 days between vouch changes

### 6.2 Verification Events
- Periodic liveness checks (every 30 days)
- Minimum 10 participants per event
- Nodes inactive >90 days require re-vouching

## 7. API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | /health | Health check |
| GET | /api/v1/nodes | List all nodes |
| POST | /api/v1/nodes | Create node |
| POST | /api/v1/graph/analyze | Analyze topology |
| POST | /api/v1/wot/vouch | Issue vouch |
| POST | /api/v1/sybil/detect | Run detection |
| GET | /api/v1/reputation/{id} | Get score |
| POST | /api/v1/did | Create DID |
| GET | /api/v1/did?did=... | Resolve DID |
| POST | /api/v1/nullifier | Register nullifier |

## 8. Configuration Constants

See `crates/core/src/constants.rs` for the authoritative list of all protocol constants.