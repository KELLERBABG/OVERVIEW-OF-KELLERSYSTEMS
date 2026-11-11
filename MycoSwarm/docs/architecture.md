# Architecture Overview

## Layers (from bottom to top)
1. **Mycelial Mesh (P2P WebRTC)** — Browser nodes form a peer-to-peer mesh using WebRTC, bootstrapped via a signaling server.
2. **Signal Transduction Protocol** — Lightweight binary messaging protocol for status propagation and gossip.
3. **Resource & Discovery Layer** — Nodes benchmark hardware, compute Bio-Scores, and broadcast capabilities.
4. **Task Distribution Layer** — Fractal task splitting, score-weighted assignment, and result reassembly.
5. **Integrity & Privacy Layer** — zk-SNARK proof generation/verification and homomorphic encryption for private computation.
6. **Incentive / Token Layer** — L2 blockchain micropayments rewarding compute contributions.
7. **Consensus Layer** — Leaderless voting for task priority, conflict resolution, and saturation-based admission control.
8. **Application Layer** — Task submission, result collection, and user-facing client UI.

## Data Flow
```
Node Discovery → Capability Broadcast → Task Submission → Fractal Split → Distribute Work Units → Execute (Wasm) → Generate Proof → Verify → Reassemble Results → Reward
```

## Key Design Decisions
- **Browser-first**: No install required, users join via URL
- **Ephemeral nodes**: Assume tabs close at any time; redundancy over reliability
- **Redundancy over reliability**: Probabilistic guarantees using Poisson distribution
- **Entropy-based scoring**: Bio-Score rewards balanced hardware, not just raw power
- **Probabilistic verification**: Only verify a random subset of zk-proofs to reduce overhead