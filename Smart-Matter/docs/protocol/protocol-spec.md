# Smart Matter Protocol Specification

## 1. Physical Layer

- Frequency: 2.4 GHz ISM band
- Modulation: GFSK (nRF24L01+)
- Data rate: 250 kbps (for maximum range)
- TX power: 0 dBm (1 mW)
- Channel spacing: 1 MHz
- Channels: 2400–2483 MHz (channels 1–83)

## 2. Protocol Slice Format

Every transmission is a single **protocol slice** (max 128 bytes payload):

```
┌─────────────────────────────────────────────────────────┐
│ Slice Type (1 byte)    │  Source Node ID (8 bytes)       │
├─────────────────────────┼───────────────────────────────┤
│ Dest Node ID (8 bytes)  │  Hops (1 byte)                 │
├─────────────────────────┼───────────────┬───────────────┤
│ Sequence No (2 bytes)   │ Energy Lvl    │ Payload       │
│                         │ (1 byte)      │ (0–128 bytes) │
├─────────────────────────┴───────────────┴───────────────┤
│ CRC16 (2 bytes) - CRC-16/CCITT-FALSE over all bytes      │
│                       before this field                  │
└─────────────────────────────────────────────────────────┘
```

**Total overhead:** 21 bytes (minimum frame: 23 bytes with empty payload).

> **Link-layer fragmentation.** Radios have small payload limits. The
> nRF24L01+ carries 32 bytes and the CC1101 64 bytes, while a full frame can
> reach 151 bytes. The link layer (`smart_matter_core::wire`) fragments a
> frame into MTU-sized chunks with a 5-byte header
> (`seqno | total | index | len`) and reassembles them on the receiver. The
> mesh protocol above is independent of the radio's payload limit.

## 3. Slice Types

| Type Code | Name | Purpose | Payload Format |
|-----------|------|---------|----------------|
| 0x01 | Beacon | Presence announcement | Empty |
| 0x02 | RouteAdv | Route advertisement | RouteAdvertisement |
| 0x03 | DataFragment | Dead-drop fragment | FragmentHeader + ciphertext |
| 0x04 | DataAck | Fragment acknowledgment | msg_id (4 bytes) |
| 0x05 | IntegrityReport | Health report | IntegrityReport fields |
| 0x06 | Broadcast | Localized broadcast | BroadcastMessage header |
| 0x07 | Command | Node configuration | NodeCommand |
| 0x08 | FwUpdate | Firmware update | Fragment index + data |

## 4. Energy Level Encoding

| Value | Level | Behavior |
|-------|-------|----------|
| 0 | Surplus | Full operation, relay traffic |
| 1 | Nominal | Normal operation |
| 2 | Low | Reduced beacon, no relay |
| 3 | Critical | Wake-on-vibration only |

## 5. Mesh Routing (Gradient-Based)

Each node maintains a routing table with up to 32 entries:

| Field | Size | Description |
|-------|------|-------------|
| Destination | 8 bytes | Target node ID |
| Next Hop | 8 bytes | Next node on path |
| Metric | 2 bytes | Gradient value (lower = better) |
| Flags | 1 byte | RouteFlags bitfield |
| Age | 4 bytes | Seconds since last update |

**Metric calculation:** `metric = base_metric + HOP_PENALTY(100) + LOW_POWER_PENALTY(50)` if applicable.

**Route timeout:** 300 seconds without update.

**Route advertisements on the wire.** A RouteAdv slice carries the
advertisement serialized with **postcard** (a compact binary format) in its
payload. A serialized route entry is 23 bytes, so each slice carries at most
**5 entries** (`MAX_ADV_ENTRIES_PER_SLICE`); senders rotate which entries
they advertise so the full table propagates over several advertisement
rounds. Forwarding never rewrites the destination. The slice keeps its
original source (for de-duplication) and final destination; only the hop
count increments.

## 6. Dead-Drop Storage

Messages are split into fragments (max 32 fragments, each ≤128 bytes):

```
Fragment Header:
┌─────────────────────────────────────────────────────────┐
│ Message ID (4 bytes)   │ Total (1 byte)   │ Index      │
│                         │                  │ (1 byte)   │
├─────────────────────────┴─────────────────┴───────────┤
│ TTL (4 bytes)          │ Created (8 bytes timestamp)    │
└─────────────────────────────────────────────────────────┘

Fragment Body:
┌─────────────────────────────────────────────────────────┐
│ Nonce (12 bytes)                                         │
├─────────────────────────────────────────────────────────┤
│ Ciphertext (ChaCha20-Poly1305)                           │
│ (plaintext length + 16-byte tag)                         │
└─────────────────────────────────────────────────────────┘
```

## 7. Localized Broadcast (RSSI Geo-Fencing)

Broadcast messages include an RSSI threshold. Nodes only relay if:
- `current_rssi >= rssi_threshold` (signal is strong enough)
- Message hasn't been seen before (dedup cache)
- TTL > 0 (hops remaining)

This ensures data stays within a physical radius.

## 8. Security

- **Encryption:** ChaCha20-Poly1305 AEAD (**RFC 8439**, RustCrypto
  `chacha20poly1305`, constant-time, no heap allocation, `no_std`).
- **Key size:** 256-bit symmetric key.
- **Nonce:** 96-bit, unique per encryption (environmental entropy pool +
  MCU tick counter).
- **Key management:** keys are **derived**, never shared verbatim:
  `node_key = HMAC-SHA256(deployment_secret, "smart-matter-v1" ‖ node_id)`.
  A deployment picks one random `deployment_secret` (e.g.
  `openssl rand -hex 32`); per-node keys are derived from it and the node
  ID. A captured node exposes only its own key. Provisioning: compile-time
  env var on embedded (`SMART_MATTER_DEPLOYMENT_SECRET`), runtime env var in
  the desktop tool / host simulation.
- **Authenticated encryption:** the 16-byte Poly1305 tag is verified before
  decryption; any tampering, wrong key, or nonce reuse is rejected.

## 9. Commands (Desktop Tool → Node)

| Command | Payload | Description |
|---------|---------|-------------|
| Reboot | Empty | Reset the node |
| Sleep | duration_ms (4 bytes) | Enter low-power sleep |
| SetRssiThreshold | threshold (2 bytes, i16) | Set geo-fencing threshold |
| SetDropTtl | ttl_sec (4 bytes) | Set dead-drop TTL |
| RequestIntegrityReport | Empty | Request health report |
| SetDutyCycle | duty_pct (1 byte) | Set maximum duty cycle |