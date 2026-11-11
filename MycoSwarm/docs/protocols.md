# Protocol Specification

## Binary Message Format

Messages use a compact binary format with a 28-byte minimum header:

```
┌───────────────────────────────────────────────────────┐
│ Byte 0:    Message Type (1 byte)                      │
│ Byte 1-8:  Sender ID length (1 byte) + ID (var str)  │
│ Byte 9-16: Target ID length (1 byte) + ID (var str)  │
│ Byte 17-24: Timestamp (8 bytes, big-endian)           │
│ Byte 25:   TTL (1 byte)                               │
│ Byte 26-27: Payload length (2 bytes, big-endian)      │
│ Byte 28+:  Payload (raw bytes)                        │
└───────────────────────────────────────────────────────┘
```

## Message Types

| Code | Type | Direction | Description |
|------|------|-----------|-------------|
| 0x01 | HELLO | Client→Server | Join request with room ID |
| 0x02 | HELLO_ACK | Server→Client | Assigned ID + peer list |
| 0x03 | GOODBYE | Bidirectional | Disconnect notification |
| 0x04 | PEER_LIST | Broadcast | Updated peer roster |
| 0x10 | OFFER | Bidirectional | WebRTC SDP offer |
| 0x11 | ANSWER | Bidirectional | WebRTC SDP answer |
| 0x12 | ICE_CANDIDATE | Bidirectional | ICE candidate relay |
| 0x20 | STATUS_UPDATE | Gossip | Node status change |
| 0x21 | SCORE_BROADCAST | Gossip | Bio-Score update |
| 0x30 | TASK_ASSIGNMENT | P2P | Work unit assignment |
| 0x31 | TASK_ACCEPT | P2P | Accept assignment |
| 0x32 | TASK_REJECT | P2P | Reject assignment |
| 0x33 | TASK_RESULT | P2P | Computation result |
| 0x34 | TASK_PROGRESS | P2P | Execution progress |
| 0x40 | VOTE | Gossip | Consensus vote |
| 0x41 | PROPOSAL | Gossip | New proposal |
| 0x42 | COMMIT | Gossip | Commit to state |
| 0x50 | TOKEN_TRANSFER | P2P | Token payment |
| 0x51 | STAKE_UPDATE | Broadcast | Staking change |

## Gossip Propagation Rules

1. Each node maintains a dedup set of seen message IDs
2. On receipt, if TTL > 1: decrement TTL, forward to `fanout` random peers (excluding sender)
3. Default fanout: 3, max TTL: 5 hops
4. Periodic heartbeat broadcasts node status every 30s

## Task Lifecycle State Machine

```
PENDING → SPLITTING → DISTRIBUTING → EXECUTING → VERIFYING → COMPLETED
                              ↓            ↓           ↓
                           TIMEOUT      FAILED      FAILED
```

## Consensus Voting Protocol

1. Proposer creates a Proposal with type, subject, payload, 30s deadline
2. Proposal is broadcast via gossip
3. Each node casts weighted vote (weight = Bio-Score)
4. Quorum: >50% of total score must vote
5. Acceptance: >66% of non-abstain weighted votes approve
6. Result is broadcast and applied network-wide