# SMPC Dark Pool — Architecture Overview

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Client Layer                                  │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌─────────────────────┐ │
│  │ smpc-cli │  │  Python  │  │   JS    │  │  curl / Postman     │ │
│  │  (CLI)   │  │  Script  │  │ Web App │  │  (testing)          │ │
│  └────┬─────┘  └────┬─────┘  └────┬─────┘  └──────────┬──────────┘ │
│       │             │             │                    │            │
└───────┼─────────────┼─────────────┼────────────────────┼────────────┘
        │             │             │                    │
        │       REST/WebSocket      │                    │
        ▼             ▼             ▼                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                       API Layer                                      │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                    api-server (Axum 0.7)                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │  Routes  │  │   WS    │  │  Auth   │  │ Rate Limiter │  │  │
│  │  └────┬─────┘  └────┬─────┘  └──────────┘  └──────────────┘  │  │
│  └───────┼─────────────┼─────────────────────────────────────────┘  │
│          │             │                                            │
└──────────┼─────────────┼────────────────────────────────────────────┘
           │             │
           ▼             ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Application Layer                                 │
│  ┌────────────┐  ┌────────────┐  ┌────────────┐  ┌───────────────┐ │
│  │  Matching  │  │ Settlement │  │    BFT    │  │    Node       │ │
│  │  Engine    │  │  Engine    │  │ Consensus │  │  Discovery    │ │
│  └─────┬──────┘  └─────┬──────┘  └─────┬──────┘  └───────┬───────┘ │
└────────┼───────────────┼───────────────┼──────────────────┼─────────┘
         │               │               │                  │
         ▼               ▼               ▼                  ▼
┌─────────────────────────────────────────────────────────────────────┐
│                    Cryptographic Layer                                │
│  ┌──────┐  ┌──────────┐  ┌─────────────┐  ┌──────────┐  ┌────────┐ │
│  │ SSS  │  │  SMPC   │  │   BLS       │  │    ZK   │  │  Comm  │ │
│  │      │  │ Engine   │  │  Threshold  │  │ Groth16 │  │ itments│ │
│  └──────┘  └──────────┘  └─────────────┘  └──────────┘  └────────┘ │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Network Layer                                   │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                    TCP Transport                               │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────────┐  │  │
│  │  │  Node 1  │  │  Node 2  │  │  Node 3  │  │   ... Node n │  │  │
│  │  │  :9000   │  │  :9001   │  │  :9002   │  │    :9004     │  │  │
│  │  └──────────┘  └──────────┘  └──────────┘  └──────────────┘  │  │
│  └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
                                    │
                                    ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Settlement Layer                                │
│  ┌────────────────────────────────────────────────────────────────┐  │
│  │                    Ethereum / Ganache                          │  │
│  │                   Settlement.sol Contract                      │  │
│  └────────────────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Descriptions

### 1. Cryptographic Layer (Phase 1)

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `sss` | Shamir's Secret Sharing | `Share { index: u64, value: Fr }` |
| `smpc-engine` | SMPC arithmetic | `SecretShare { share: Share }` |
| `homomorphic-compare` | Secret-shared comparison | `Order { price, quantity, timestamp: Share }` |
| `commitments` | Pedersen + hash commitments | `[u8; 32]` commitments |

### 2. Application Layer (Phase 2)

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `matching-engine` | Blind order matching | `MatchingEngine`, `OrderBook` |
| `price-discovery` | MPC price discovery | `ClearingPrice`, `AggregatedOrder` |
| `threshold-crypto` | BLS threshold signatures | `SecretKey`, `PublicKey`, `Signature` |
| `zk-validation` | Groth16 zk-SNARKs | `BalanceCircuit`, `Proof` |
| `front-running` | Commit-reveal protection | `Commitment`, `Mempool` |

### 3. Infrastructure Layer (Phase 3)

| Crate | Purpose | Key Types |
|-------|---------|-----------|
| `settlement` | Settlement proof engine | `SettlementEngine`, `SettlementProof` |
| `bft` | PBFT consensus with configurable timeouts, jitter, partition detection, exponential backoff | `Consensus`, `ConsensusConfig`, `ViewChangeProtocol` |
| `node` | Node networking + reputation; TCP with pipelined broadcast, message batching, configurable retry | `NetworkManager`, `NetworkConfig`, `ReputationSystem` |
| `api-server` | REST + WebSocket API | `AppState`, routes, middleware |
| `zk-validation` | Groth16 proofs with async proving service, hardware accelerator trait, multi-strategy support | `ProvingService`, `ProvingStrategy`, `HardwareAccelerator` |

---

## Data Flow Diagrams

### Order Submission Flow

```
Client                      API Server            Matching Engine          Settlement
  │                            │                       │                     │
  │  POST /api/orders          │                       │                     │
  │───────────────────────────►│                       │                     │
  │                            │                       │                     │
  │                            │ Commit order hash     │                     │
  │                            │ to mempool            │                     │
  │                            │──────┐                │                     │
  │                            │      │ (internal)     │                     │
  │                            │◄─────┘                │                     │
  │                            │                       │                     │
  │                            │ Split order into      │                     │
  │                            │ secret shares         │                     │
  │                            │──────┐                │                     │
  │                            │      │                │                     │
  │                            │◄─────┘                │                     │
  │                            │                       │                     │
  │                            │ Submit order  │
  │                            │──────────────────────►│                     │
  │                            │                       │                     │
  │                            │                       │ Process matches     │
  │                            │                       │ using SMPC compare  │
  │                            │                       │──────┐              │
  │                            │                       │      │              │
  │                            │                       │◄─────┘              │
  │                            │                       │                     │
  │                            │ Return match(es)      │                     │
  │                            │◄───────────────────────│                     │
  │                            │                       │                     │
  │                            │ Create settlement    │
  │                            │ proof                │
  │                            │──────────────────────────────────────────► │
  │                            │                       │                     │
  │  Response +                │                       │                     │
  │  order status              │                       │                     │
  │◄───────────────────────────│                       │                     │
  │                            │                       │                     │
```

### Settlement Flow

```
Node A (Leader)         Node B                 Node C                 Ethereum
     │                     │                     │                      │
     │  Propose match      │                     │                      │
     │────────────────────►│                     │                      │
     │  Propose match      │                     │                      │
     │──────────────────────────────────────────►│                      │
     │                     │                     │                      │
     │  Prepare (BLS sig)  │                     │                      │
     │◄────────────────────│                     │                      │
     │                     │  Prepare (BLS sig)  │                      │
     │◄──────────────────────────────────────────│                      │
     │                     │                     │                      │
     │  Commit (BLS sig)   │                     │                      │
     │────────────────────►│                     │                      │
     │  Commit (BLS sig)   │                     │                      │
     │──────────────────────────────────────────►│                      │
     │                     │                     │                      │
     │  Aggregate sigs     │                     │                      │
     │──────┐              │                     │                      │
     │      │ (threshold   │                     │                      │
     │◄─────┘  3-of-5)     │                     │                      │
     │                     │                     │                      │
     │  settleMatch(order) │                     │                      │
     │─────────────────────────────────────────────────────────────────►│
     │                     │                     │                      │
     │                     │                     │   MatchSettled event │
     │◄─────────────────────────────────────────────────────────────────│
```

---

## Security Architecture

### Trust Model

```
Node Types:
├── Validator Nodes (n=5, t=3)
│   ├── Hold secret shares
│   ├── Run PBFT consensus
│   ├── Sign settlement proofs
│   └── Subject to slashing
├── API Nodes
│   ├── Accept client connections
│   ├── Route orders to validators
│   └── Stateless (no private keys)
└── Clients
    └── Submit orders (encrypted + committed)
```

### Key Hierarchy

```
Network Key (threshold BLS, t=3, n=5)
├── Node 1 Secret Key ──► Node 1 Public Key
├── Node 2 Secret Key ──► Node 2 Public Key
├── Node 3 Secret Key ──► Node 3 Public Key
├── Node 4 Secret Key ──► Node 4 Public Key
└── Node 5 Secret Key ──► Node 5 Public Key

User Key (ECDSA for signing orders)
├── User Secret Key
└── User Public Key (registered on-chain)
```

---

## Deployment Architecture

### Development (Docker Compose)

```
┌─────────────────────────────────────────┐
│  smpc-net (bridge network)              │
│                                         │
│  ┌──────┐  ┌──────┐  ┌──────┐         │
│  │node0 │  │node1 │  │node2 │  ...     │
│  │:8080 │  │:9001 │  │:9002 │  node4   │
│  │:9000 │  │      │  │      │  :9004   │
│  └──────┘  └──────┘  └──────┘         │
│                                         │
│  ┌──────────────────────────────────┐   │
│  │  ganache (Ethereum testnet)      │   │
│  │  :8545                           │   │
│  └──────────────────────────────────┘   │
└─────────────────────────────────────────┘
```

### Production (Kubernetes — Future)

```
┌──────────────────────────────────────┐
│  SMPC Validator StatefulSet          │
│  ├── 5 pods (one per node)           │
│  ├── TCP headless service            │
│  └── Persistent volumes for keys     │
├──────────────────────────────────────┤
│  API Deployment                      │
│  ├── 3+ replicas                     │
│  ├── Load balancer (TLS termination) │
│  └── Horizontal Pod Autoscaler       │
├──────────────────────────────────────┤
│  Monitoring Stack                    │
│  ├── Prometheus (metrics scrape)     │
│  ├── Grafana (dashboards)            │
│  └── AlertManager (PagerDuty/Slack)  │
└──────────────────────────────────────┘
```

---

## Alerting Stubs

The `AlertManager` in `node::monitoring` provides placeholder infrastructure for PagerDuty/Slack:

```rust
AlertManager::new(AlertConfig { pagerduty_key: Some("..."), slack_webhook: None });
manager.send_alert("critical", "Node 3 unreachable for 30s");
```

Currently logs via `tracing::warn!`. In production, dispatches to external alerting channels.

## zk-SNARK Proving Service

The `ProvingService` in `zk-validation` provides async proof generation with configurable strategy:

```rust
let service = ProvingService::new(ProvingStrategy::Cpu, 100);
let circuit = BalanceCircuit { balance, nonce, balance_commitment, order_value };
let proof = service.submit(circuit).await?;
```

Proofs are queued on an mpsc channel and processed on a `spawn_blocking` task, preventing proof generation from blocking the async runtime. `ProvingStrategy` supports `Cpu` (default), `Gpu` (roadmap Q3 2026), `Fpga` (roadmap Q4 2026), and `Remote` (roadmap Q1 2027) backends via the `HardwareAccelerator` trait.

## Monitoring & Observability

### Metrics (Prometheus)

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `orders_total` | Counter | — | Total orders submitted |
| `matches_total` | Counter | — | Total matches found |
| `active_connections` | Gauge | — | Active WebSocket connections |
| `http_requests_total` | Counter | `method`, `path` | HTTP request count |

### Alerting

| Alert | Condition | Severity |
|-------|-----------|----------|
| NodeDown | No heartbeat for 30s | Critical |
| ConsensusStalled | No new blocks for 60s | Critical |
| HighRejectionRate | >10% settlement rejections | Warning |
| LowReputationScore | Any node score < 0.3 | Warning |
