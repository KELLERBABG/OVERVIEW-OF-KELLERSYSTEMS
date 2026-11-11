# SMPC Dark Pool — Protocol Specification

**Version 1.0**

---

## 1. Network Protocol

### 1.1 Transport Layer

- **Transport**: TCP with length-delimited framing
- **Frame format**: 4-byte big-endian length prefix + payload bytes
- **Maximum message size**: 1 MiB (1,048,576 bytes)
- **Default ports**: 9000-9004 (one per node)

### 1.2 Connection Lifecycle

1. **Outbound connection**: Node A connects to Node B's `listen_addr`
2. **Handshake**: Immediately after TCP connect, each peer sends a 16-byte UUID identifying itself
3. **Registration**: On receiving a handshake, the recipient registers the sender as a peer
4. **Reconnection**: On disconnect, nodes retry with exponential backoff (default: 1s initial, 30s max, 1.5x multiplier, random jitter)

### 1.3 Network Configuration (`NetworkConfig`)

| Parameter | Default | Description |
|-----------|---------|-------------|
| `channel_size` | 1024 | mpsc channel buffer depth |
| `max_message_size` | 1 MiB | Maximum frame payload |
| `initial_retry_interval_ms` | 1000 | First reconnection delay |
| `max_retry_interval_ms` | 30000 | Cap on reconnection delay |
| `batch_timeout_ms` | 10 | Max wait to coalesce outbound messages |
| `enable_pipelining` | true | Fire-and-forget sends (no ack wait) |
| `enable_batching` | true | Coalesce messages before flush |
| `handshake_timeout_ms` | 5000 | Max wait for peer UUID handshake |
| `read_timeout_ms` | 30000 | Max wait for frame on idle connection |

### 1.4 Pipelined Broadcast

`broadcast_pipelined(data)` sends to all peers without collecting per-peer results, returns peer count. `pipelined_send(target, data)` is the single-peer fire-and-forget variant. Both log errors instead of returning them, eliminating backpressure stalls.

### 1.5 Message Batching

The `write_loop` accumulates outbound messages for up to `batch_timeout_ms` or until 10 messages are buffered, then flushes them as individual frames in rapid succession. This amortizes TCP framing overhead and syscall cost across multiple messages.

### 1.3 Message Format

```
[4-byte length (big-endian)] [payload bytes (JSON or bincode)]
```

All messages are serialized via `serde_json` for readability (production would use `bincode`).

### 1.4 Message Types

```rust
enum MessageType {
    PrePrepare,     // PBFT: leader proposes
    Prepare,        // PBFT: node agrees to prepare
    Commit,         // PBFT: node commits
    ViewChange,     // PBFT: view change request
    NewView,        // PBFT: new view established
    OrderCommit,    // Commit-reveal: order commitment
    OrderReveal,    // Commit-reveal: order data + nonce
    SettlementProof,// Match settlement proof
    Heartbeat,      // Liveness check
}
```

---

## 2. Cryptographic Primitives

### 2.1 Shamir's Secret Sharing

**Field**: BN254 scalar field $\mathbb{F}_r$

**Parameters**:
- $n = 5$ (total shares)
- $t = 3$ (threshold for reconstruction)

**Split** — `sss::sss::split_secret(secret: Fr, n: u64, t: u64) -> Result<Vec<Share>>`:

1. Generate random polynomial $f(x) = s + a_1x + ... + a_{t-1}x^{t-1}$
2. Evaluate $f(1), f(2), ..., f(n)$
3. Return vector of `Share { index: u64, value: Fr }`

**Reconstruct** — `sss::sss::reconstruct(shares: &[Share]) -> Result<Fr>`:

1. Compute Lagrange coefficients $\lambda_i = \prod_{j \neq i} \frac{x_j}{x_j - x_i}$
2. $s = \sum_{i} \lambda_i \cdot f(x_i)$

### 2.2 SMPC Operations

All operations work on `SecretShare`:

```rust
struct SecretShare {
    share: Share,        // Individual share (index + value)
}
```

**Secure Add** — `smpc_engine::add::secure_add(a, b)`:
- Returns `SecretShare { share: Share { index: a.share.index, value: a.share.value + b.share.value } }`

**Secure Multiply** — `smpc_engine::mul::secure_mul(a, b)`:
- Requires re-sharing via Beaver triples
- Returns product as `SecretShare`

**Secure Less Than** — `smpc_engine::compare::secure_less_than(a, b)`:
- Compares two secret-shared values
- Returns `SecretShare` containing 1 if `a < b`, 0 otherwise

**Secure Min** — `smpc_engine::compare::secure_min(a, b)`:
- Returns the smaller of two secret-shared values

### 2.3 BLS Threshold Signatures

**Algorithm**: BLS over BN254

**Key generation**: `threshold_crypto::bls::generate_keypair()`
- Returns `(SecretKey, PublicKey)`

**Sign**: `threshold_crypto::bls::sign(sk, msg) -> Signature`

**Verify**: `threshold_crypto::bls::verify(pk, msg, sig) -> bool`

**Threshold aggregate**: `threshold_crypto::bls::aggregate_signatures(sigs, indices) -> Signature`
- Combines $t$ partial signatures using Lagrange coefficients
- The aggregated signature is valid against the aggregated public key

### 2.4 Groth16 zk-SNARKs

**Circuit**: `BalanceCircuit` over BN254

**Public inputs**:
- `balance_commitment: Fr` — hash commitment to user's balance
- `order_value: Fr` — value of the order

**Private witnesses**:
- `balance: Fr` — actual balance
- `nonce: Fr` — blinding factor

**Constraints**:
1. Multiplicative constraint: `balance_commitment = balance * nonce + balance`
2. Bit decomposition (64 bits): each variable `< 2^64`
3. Range check: `balance - order_value < 2^64` (ensures `balance >= order_value`)

**Proving**: `zk_validation::prover::generate_proof(witness, order_value) -> Proof`
- Uses `ark_groth16::create_random_proof`
- Output: `{ proof_bytes: Vec<u8>, public_inputs: Vec<Fr> }`

**Strategies** (`ProvingStrategy`):
- `Cpu` — Default, on-node CPU proving (~500ms)
- `Gpu` — CUDA/Metal accelerated (roadmap, Q3 2026)
- `Fpga` — Custom NTT accelerator (roadmap, Q4 2026)
- `Remote` — Dedicated proving cluster via gRPC (roadmap, Q1 2027)

**Async proving**: `ProvingService` queues proof requests via mpsc channel and processes them on a background `spawn_blocking` task. Submit via `service.submit(circuit) -> oneshot::Receiver<Proof>`.

**Hardware accelerator trait**: `HardwareAccelerator` defines `msm()` and `ntt()` methods. `CpuAccelerator` is the no-op fallback. Future GPU/FPGA backends implement this trait.

**Verification**: `zk_validation::verifier::verify(proof, balance_commitment) -> bool`
- Uses `ark_groth16::verify_proof`

### 2.5 Commitments

**Pedersen commitment**: `commitments::pedersen::commit(value, blinding) -> [u8; 32]`
- $C = v \cdot G + r \cdot H$

**Hash commitment**: `commitments::hash_commitment::commit(value, nonce) -> [u8; 32]`
- $C = \text{SHA256}(\text{nonce} || \text{value})$

---

## 3. Order Matching Protocol

### 3.1 Data Structures

```rust
enum OrderSide { Bid, Ask }

struct Order {
    id: u64,
    party_id: u64,
    side: OrderSide,
    price: Share,      // Secret-shared
    quantity: Share,   // Secret-shared
    timestamp: Share,  // Secret-shared
}

struct Match {
    buy_order_id: u64,
    sell_order_id: u64,
    price: Share,       // Bid's price (dark pool convention)
    quantity: Share,    // Min of bid and ask quantities
}
```

### 3.2 Matching Algorithm

```
function process_matches(book):
    while book.is_crossed():
        best_bid = book.bids[0]
        best_ask = book.asks[0]
        match_price = best_bid.price
        match_qty = secure_min(best_bid.quantity, best_ask.quantity)
        record_match(best_bid, best_ask, match_price, match_qty)
        book.remove(best_bid.id)
        book.remove(best_ask.id)
```

Crossing detection uses `homomorphic_compare::priority::compare_price_time`, which compares shares without revealing values.

### 3.3 Book Ordering

**Bids**: Sorted by price descending, then time ascending (highest bid first)
**Asks**: Sorted by price ascending, then time ascending (lowest ask first)

Sorting is performed entirely on secret-shared values using `compare_price_time`.

---

## 4. Settlement Protocol

### 4.1 Settlement Proof

```rust
struct SettlementProof {
    order_id: u64,
    match_proof_hex: String,          // SHA256 of match data
    signature_aggregate_hex: String,  // Aggregated BLS signature
    taker_order_id: u64,
    maker_order_id: u64,
    price_hex: String,
    quantity_hex: String,
    timestamp: DateTime<Utc>,
}
```

### 4.2 Verification

```
function verify_settlement_proof(proof):
    if len(proof.signature) < 64: return THRESHOLD_SIG_INVALID
    if len(proof.match_proof) < 32: return PROOF_TOO_SHORT
    if not verify_match_proof(proof): return PROOF_MISMATCH
    return OK
```

### 4.3 On-Chain Settlement

The Solidity contract `Settlement.sol`:

```solidity
function settleMatch(
    uint256 orderId,
    uint256 takerOrderId,
    uint256 makerOrderId,
    bytes calldata matchProof,
    bytes calldata thresholdSignature,
    uint256 price,
    uint256 quantity
) external returns (bool);
```

Events:
- `MatchSettled(orderId, taker, maker)` — on successful settlement
- `SettlementFailed(orderId, reason)` — on verification failure

---

## 5. PBFT Consensus Protocol

### 5.1 Protocol State Machine

```
NewView ──► PrePrepared ──► Prepared ──► Committed
                │               │
                ▼               ▼
           ViewChange ────► NewView
```

### 5.2 Quorum Sizes

- Total nodes: $n$
- Fault tolerance: $f = \lfloor (n-1)/3 \rfloor$
- Quorum: $2f + 1$
- For $n = 4$: $f = 1$, quorum $= 3$

### 5.3 Message Flow

```
Leader          Node 1          Node 2          Node 3
  │               │               │               │
  ├──PrePrepare──►│               │               │
  │               ├──Prepare─────►│               │
  │               ├──Prepare─────►│               │
  │◄──────────────┤               ├──Prepare─────►│
  │               │◄──────────────┤               │
  │               │               │◄──────────────┤
  │               ├──Commit──────►│               │
  │               ├──Commit──────►│               │
  │◄──────────────┤               ├──Commit──────►│
  │◄──────────────┤               │               │
  │               │               │               │
  ▼               ▼               ▼               ▼
Committed       Committed       Committed       Committed
```

### 5.4 View Change

When a node detects the leader is faulty (no progress after timeout):

1. Broadcast `ViewChange` message with new view number
2. On receiving $2f+1$ view changes, the new leader broadcasts `NewView`
3. All nodes reset to `NewView` phase with incremented view number

### 5.5 View-Change Resilience (`ConsensusConfig`)

| Parameter | Default | Description |
|-----------|---------|-------------|
| `view_change_timeout_ms` | 3000 | Base timeout before suspecting leader |
| `jitter_range_ms` | 1000 | Random jitter added to timeout (`rand(0..1000)`) |
| `backoff_multiplier` | 1.5 | Exponential backoff per consecutive view change |
| `max_timeout_ms` | 60000 | Cap on cumulative backoff timeout |
| `min_peers_for_quorum` | 3 | Minimum reachable peers to allow view change |
| `heartbeat_miss_threshold` | 3 | Missed heartbeats before marking leader faulty |

**Timeout computation**:

$$ \text{timeout} = \min(\text{base} + \text{jitter} + \text{base} \times 1.5^{\text{consecutive}}, \text{max}) $$

**Partition detection**: `should_trigger_view_change(active_peers)` returns `false` if `active_peers < min_peers_for_quorum`, preventing unnecessary view changes during network splits. It also returns `false` if the node has received zero messages (node may be isolated rather than the leader being faulty).

---

## 6. Front-Running Prevention

### 6.1 Commit-Reveal

1. **Commit phase**: Client submits $C = H(\text{order} || \text{nonce})$ to mempool
2. **Reveal phase**: After deadline, client submits $(\text{order}, \text{nonce})$
3. **Verification**: Mempool checks $H(\text{order} || \text{nonce}) == C$

### 6.2 Encrypted Mempool

- Orders are encrypted with the aggregated public key of the validator set
- Only validators can decrypt (threshold decryption)
- Prevents validators from seeing orders before commit deadline

---

## 7. API Protocol

See [API Reference](api-reference.md) for full details.

**Endpoints**:

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/health` | Health check |
| GET | `/api/metrics` | Prometheus metrics |
| POST | `/api/orders` | Submit order |
| GET | `/api/orders/{id}` | Order status |
| GET | `/api/book` | Order book snapshot (blinded) |
| GET | `/api/trades` | Trade history |
| WS | `/api/ws` | Real-time trade feed |
