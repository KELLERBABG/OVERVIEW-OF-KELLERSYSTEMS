# Smart Matter: Infrastructure-Embedded Intelligent Materials

## A Decentralized Mesh Computing Platform for Civil Infrastructure

**Version 1.1 — July 2026**

---

## Abstract

Smart Matter is an open-source platform for embedding intelligent, energy-autonomous compute nodes directly into civil infrastructure. Each node harvests kinetic energy from ambient vibration via piezoelectric elements, forms ad-hoc wireless mesh networks, provides decentralized dead-drop storage, self-monitors structural integrity, and enforces data locality through RSSI-based geo-fencing. This whitepaper describes the system architecture, protocol design, energy model, security considerations, and deployment strategy.

**Revision 1.1 updates:** Added Sub-1 GHz radio support for concrete embedding, piezoelectric tuning mitigations, and Flash wear-leveling storage analysis.

---

## 1. Introduction

### 1.1 Motivation

Modern cities contain millions of tons of concrete, asphalt, and steel — structural materials that are functionally inert. They bear loads, transmit vibrations, and experience environmental stress, yet they have no capacity to sense, communicate, or store information about their own condition.

Bridges are inspected manually every 1–2 years. Road degradation is detected only after potholes form. Structural damage from earthquakes or fatigue accumulates invisibly until catastrophic failure. Meanwhile, the physical energy of foot traffic, vehicular vibration, and wind remains entirely untapped.

Smart Matter addresses these gaps by embedding microscopic compute nodes directly into structural materials. These nodes are:

- **Energy-autonomous** — powered solely by ambient vibration harvesting
- **Self-networking** — forming ad-hoc mesh networks without infrastructure
- **Self-diagnosing** — monitoring structural integrity through vibration analysis
- **Data-sovereign** — enforcing geographic data locality
- **Decentralized** — no single point of failure, no cloud dependency

### 1.2 Design Philosophy

The design is guided by five principles:

1. **Energy is the constraint.** Every operation is measured against the energy budget. Communication is minimized, computation is optimized, and sleep is the default state.
2. **Physical locality is a security primitive.** Data remains in the geographic region where it was generated. RSSI-based geo-fencing replaces trust-based access control.
3. **Redundancy replaces reliability.** Individual nodes may fail. The mesh self-heals. Fragments are stored across multiple nodes.
4. **No single point of failure.** There is no central server, no cloud API, no trusted third party. The network operates as a pure peer-to-peer mesh.
5. **Open by default.** All hardware designs, firmware, and protocols are MIT-licensed.

---

## 2. System Architecture

### 2.1 Node Hardware Design

Each Smart Matter compute node consists of five integrated subsystems:

#### 2.1.1 Piezoelectric Energy Harvester

- **Element:** PZT-5A piezoelectric ceramic disk (20 mm × 1 mm)
- **Resonant frequency:** ~60 Hz (tuned to pedestrian footfall and bridge vibration)
- **Open-circuit voltage:** Up to 20 V peak at 1g acceleration
- **Rectification:** Full-wave bridge rectifier (DF10S, Schottky diodes, V_f ≈ 0.3 V)
- **Boost/MPPT:** BQ25504 ultra-low-power boost converter with maximum power point tracking
  - Cold-start voltage: 330 mV
  - Quiescent current: 330 nA
  - MPPT set to 80% of open-circuit voltage
- **Storage:** 1 F supercapacitor (5.5 V rating)
  - Energy capacity: ½ × 1 F × (3.3 V)² = 5.4 J ≈ 5,400,000 μJ
  - Usable range: 2.2 V (MCU UVLO) to 3.3 V (regulated output)
  - Usable energy: ~3,000,000 μJ

> **Piezoelectric tuning note:** PZT-5A elements at 60 Hz have a high Q-factor (Q ≈ 30–50), providing efficient harvesting only within a narrow ~2–4 Hz bandwidth. Real-world vibrations range from 1.5 Hz (pedestrian) to 120 Hz (railway). For initial prototypes the 60 Hz disk is adequate. For field deployment, a cantilever beam design (10–40 Hz bandwidth) or dynamic resonance tuning (Phase 3) is recommended. See Appendix C.

#### 2.1.2 Compute Module

- **MCU:** STM32G031J6 (ARM Cortex-M0+, 64 KB Flash, 8 KB RAM)
- **Active current:** 4.6 mA at 48 MHz (1.8 V)
- **Deep sleep current:** 0.4 μA (retention RAM off)
- **Wake time:** < 5 μs from deep sleep
- **Flash endurance:** 10,000 write cycles per sector. See §2.3 for wear-leveling strategy.

#### 2.1.3 Wireless Transceiver

The platform supports two radio options via a frequency-agnostic `Radio` trait:

| Band | Radio Module | Free-Air Range | Concrete Penetration | Max Data Rate | Cost |
|------|-------------|----------------|---------------------|---------------|------|
| **2.4 GHz** (default) | nRF24L01+ | ~100 m | ~0.5 m | 2 Mbps | $1.80 |
| **Sub-1 GHz** (recommended for concrete) | CC1101 | ~500 m | ~5 m | 250 kbps | $2.50 |

**Critical risk:** 2.4 GHz signals are severely attenuated by concrete and potting compound due to moisture content and the high dielectric constant of structural materials (ε_r ≈ 6–12 for concrete). Laboratory testing has shown that 2.4 GHz range can collapse to under 1 meter when nodes are embedded in concrete, making Sub-1 GHz essential for structural embedding applications.

The Sub-1 GHz band (868 MHz EU / 915 MHz US) offers significantly better penetration through concrete based on the ITU-R P.2040 propagation model:

$$L_{concrete}(dB) = 5.31 \times f^{0.67} \times d \times M$$

Where f is frequency in GHz, d is thickness in meters, and M is a moisture multiplier (1× dry, up to 3× wet). At 915 MHz through 30 cm of damp concrete, path loss is approximately 15 dB — easily within the link budget at 10 dBm TX power. At 2.4 GHz, the same scenario yields >35 dB loss.

#### 2.1.4 Sensor Suite

- **Accelerometer:** LIS3DH (3-axis, ±2g/±4g/±8g/±16g)
  - Low-power mode: 2 μA at 10 Hz ODR
  - Noise floor: 220 μg/√Hz
- **Temperature sensor:** DS18B20 (optional, ±0.5°C accuracy)

#### 2.1.5 Power Budget

| Operation | Current | Voltage | Power | Duration | Energy |
|-----------|---------|---------|-------|----------|--------|
| Deep sleep | 0.4 μA | 1.8 V | 0.72 μW | indefinite | — |
| Sensor read (10 Hz) | 2 μA | 1.8 V | 3.6 μW | 1 ms | 3.6 nJ |
| Radio TX (1 slice, 2.4 GHz) | 11.3 mA | 1.8 V | 20.3 mW | 0.5 ms | 10.2 μJ |
| Radio TX (1 slice, Sub-1 GHz) | 15 mA | 1.8 V | 27 mW | 1 ms | 27 μJ |
| Radio RX (listen) | 13.5 mA | 1.8 V | 24.3 mW | 1 ms | 24.3 μJ |
| MCU active (48 MHz) | 4.6 mA | 1.8 V | 8.3 mW | 10 ms | 83 μJ |

A single footstep at 60 Hz resonance can harvest approximately 1 mJ — sufficient for approximately 50 transmission slices or 100 sensor reads.

### 2.2 Energy Model

The node's energy state is classified into four levels:

| Level | Condition | Behavior |
|-------|-----------|----------|
| Surplus | V_storage > 3.3 V | Full mesh routing, storage, broadcast, beacon every 5 s |
| Nominal | 2.7 V < V_storage ≤ 3.3 V | Normal ops, beacon every 5 s |
| Low | 2.2 V < V_storage ≤ 2.7 V | No mesh relay, beacon every 30 s, reduced sampling |
| Critical | V_storage ≤ 2.2 V | Wake-on-vibration only, no radio |

The energy budget model tracks stored energy as:

$$E_{stored} = \frac{1}{2} C V^2$$

Where C = 1 F and V is the supercapacitor voltage. At 3.3 V, this yields approximately 5.4 J. After reserve for boot and sensing operations, approximately 3,000,000 μJ is available for communication.

Each transmission slice costs approximately 50 μJ (radio TX + MCU active). A fully charged node can therefore sustain approximately 60,000 slices — though in practice, the 255-slice cap per session prevents energy exhaustion from a single burst.

### 2.3 Flash Memory & Wear-Leveling

The STM32G031J6 has 64 KB of internal Flash organized into 4 KB sectors with ~10,000 write cycles per sector. Without wear-leveling, storing dead-drop fragments or frequent routing table updates would rapidly destroy the Flash.

The firmware implements a two-tier storage strategy:

| Store | Location | Capacity | Volatile? | Write Rate Limit |
|-------|----------|----------|-----------|-----------------|
| Dead-drop store | SRAM | 32 fragments (~4.6 KB) | Yes (power loss = data loss) | Unlimited |
| Wear-level store | Flash (log) | 12 KB (3 sectors) | No | ≤2 writes/second |

**Dead-drop store (SRAM):** Fragments are ephemeral by design (TTL-based expiration) and replicated across multiple mesh nodes. Keeping them in SRAM avoids Flash wear entirely. Data loss on power failure is acceptable because fragments are redundant across the mesh. The 32-fragment capacity (~4.6 KB of ciphertext, ~6 KB including headers) leaves room in the STM32G031J6's 8 KB SRAM for mesh state, the entropy pool, and the stack.

**Wear-level store (Flash):** Critical persistent state (routing table checkpoints, configuration, integrity snapshots) uses a log-structured circular buffer:

1. Entries are written sequentially to a sector in SRAM first (up to 16 buffered)
2. On buffer full or explicit flush, entries are written to Flash
3. When a sector fills, the write pointer advances to the next sector (erasing it)
4. All 3 sectors are rotated evenly

**Estimated lifetimes:**
- At 10 writes/hour: 12 KB × 3 sectors × 10,000 cycles ÷ (128 bytes/write × 10/hr × 24 × 365) ≈ **10.5 years**
- At 1 write/hour: >**50 years**
- At 100 writes/hour (burst): still >1 year before any sector reaches end-of-life

The wear-level store also implements rate limiting (≤2 writes/second) to prevent rapid cycling during error storms.

### 2.4 Memory Map (STM32G031J6)

```
0x0800_0000 – 0x0800_3FFF: Bootloader         (16 KB, 4 sectors)
0x0800_4000 – 0x0800_BFFF: Application firmware (32 KB, 8 sectors)
0x0800_C000 – 0x0800_EEFF: Wear-level log store (12 KB, 3 sectors)
0x0800_F000 – 0x0800_F7FF: Configuration data   (2 KB, 1 sector)
0x0800_F800 – 0x0800_FFFF: Free                 (2 KB, 1 sector)
```

---

## 3. Protocol Specification

### 3.1 Physical Layer

- **Frequency band:** 2.4 GHz ISM (default) or 868/915 MHz Sub-1 GHz (recommended for concrete embedding)
- **Modulation:** GFSK (Gaussian Frequency Shift Keying)
- **Data rate:** 250 kbps (Sub-1 GHz primary), 250 kbps–2 Mbps (2.4 GHz)
- **TX power:** Adjustable −18 to +10 dBm
- **Channel spacing:** 1 MHz (2.4 GHz), 200 kHz (Sub-1 GHz)
- **Channel selection:** Adaptive frequency hopping (avoiding WiFi/congestion)
- **Concrete penetration model:** ITU-R P.2040 coefficients implemented in firmware for range prediction

### 3.2 Protocol Slice

All communication occurs in **protocol slices** — fixed-overhead packets designed to fit within a single sub-millisecond energy window:

```
┌──────────────────────────────────────────────────────────────┐
│ Byte 0:     Slice Type (0x01–0x08)                           │
│ Bytes 1–8:  Source Node ID (8 bytes)                         │
│ Bytes 9–16: Destination Node ID (8 bytes)                    │
│ Byte 17:    Hop Count (0–64)                                 │
│ Bytes 18–19: Sequence Number (for deduplication)             │
│ Byte 20:    Energy Level (0–3)                               │
│ Bytes 21–148: Payload (0–128 bytes)                          │
│ Bytes 149–150: CRC16 (CCITT)                                 │
└──────────────────────────────────────────────────────────────┘
```

**Minimum packet size:** 23 bytes (empty payload)  
**Maximum packet size:** 151 bytes (128-byte payload)  
**Overhead:** 21 bytes per packet (14%)

### 3.3 Slice Types

| Type | Code | Purpose | Payload | Handling |
|------|------|---------|---------|----------|
| Beacon | 0x01 | Node presence announcement | Empty | Received → update routing table, not forwarded |
| RouteAdv | 0x02 | Route table advertisement | Serialized entries | Received → merge into routing table |
| DataFragment | 0x03 | Encrypted dead-drop fragment | FragmentHeader + ciphertext | Stored in SRAM, forwarded toward destination |
| DataAck | 0x04 | Fragment receipt confirmation | msg_id (4 bytes) | Routed back to sender |
| IntegrityReport | 0x05 | Structural health report | Serialized report fields | Routed to sink/gateway |
| Broadcast | 0x06 | Localized data broadcast | Broadcast header + data | RSSI-checked, relayed if in range |
| Command | 0x07 | Node configuration | Serialized command | Executed locally, acknowledged |
| FwUpdate | 0x08 | Firmware update data | Index (2 bytes) + binary data | Written to staging area |

### 3.4 Mesh Routing (Gradient-Based)

The mesh uses a gradient-based routing protocol optimized for energy-constrained nodes:

#### Routing Table

Each node maintains a table of up to 32 entries:

| Field | Size | Description |
|-------|------|-------------|
| Destination | 8 bytes | Target node ID |
| Next Hop | 8 bytes | Next node toward destination |
| Metric | 2 bytes | Path cost (lower is better) |
| Flags | 1 byte | RouteFlags bitfield |
| Age | 4 bytes | Seconds since last update |

**Total per entry:** 23 bytes  
**Maximum table size:** 736 bytes (32 entries)

#### Metric Calculation

The metric for a route is:

$$M_{dest} = M_{advertised} + H_{penalty} + L_{penalty}$$

Where:
- $M_{advertised}$ is the metric advertised by the neighbor
- $H_{penalty} = 100$ (per hop)
- $L_{penalty} = 50$ if the neighbor is in LOW_POWER mode

This creates a natural gradient toward nodes with better energy status and fewer hops.

#### Route Discovery

1. Nodes broadcast **Beacon** slices on wake/startup
2. Nodes periodically broadcast **RouteAdv** slices containing their routing table
3. Receiving nodes update their tables, preferring lower metrics
4. Stale entries (age > 300 s) are evicted

#### Route Maintenance

- Routes are soft-state: they expire after 300 seconds without refresh
- Nodes in CRITICAL energy level stop forwarding (do not update their sequence number)
- When a node fails, neighbors detect the absence of beacons and remove stale routes

### 3.5 Dead-Drop Storage Protocol

The dead-drop protocol enables asynchronous, encrypted message passing across the mesh:

#### Fragmentation

Messages are split into encrypted fragments (max 32 fragments, each ≤ 128 bytes plaintext):

```
FragmentHeader (16 bytes):
  msg_id:  u32    — Unique message identifier
  total:   u8     — Total fragments in message
  index:   u8     — This fragment index (0-based)
  ttl_sec: u32   — Time-to-live in seconds
  created: u64   — Creation timestamp (ms since epoch)

FragmentBody (variable):
  nonce:       [u8; 12]  — ChaCha20 nonce
  ciphertext:  variable  — ChaCha20-Poly1305 encrypted + 16-byte tag
```

#### Encryption

- **Algorithm:** ChaCha20-Poly1305 AEAD
- **Key:** 256-bit symmetric key (pre-shared or derived from environmental entropy)
- **Nonce:** 96-bit, unique per fragment
- **Authentication:** 128-bit Poly1305 tag, verified on decryption

#### Storage

Each node stores up to 64 fragments in SRAM (volatile). Fragments expire after TTL seconds and are garbage collected. Storage is opportunistic: nodes store whatever fragments they receive, without knowledge of the complete message.

> **Design note:** Fragments are intentionally stored in SRAM rather than Flash to avoid wear from high-frequency write operations. Fragments are replicated across multiple mesh nodes, so individual node power loss does not cause data loss.

#### Retrieval

To retrieve a message, a node broadcasts a **DataAck** with the desired msg_id. Nodes holding fragments respond by forwarding them toward the requester. Once all fragments arrive, the message is reassembled and decrypted.

### 3.6 Localized Broadcast (RSSI Geo-Fencing)

The Sovereign Localized Broadcaster ensures data locality through RSSI-based geo-fencing:

```
BroadcastMessage:
  origin:         NodeId — Originating node
  rssi_threshold: i16   — Minimum RSSI (dBm) for relay
  seqno:          u16   — Sequence number
  payload:        bytes — Message content (max 128 bytes)
  ttl:            u8    — Maximum hop count
```

**Relay decision:**

A node relays a broadcast if AND only if:

1. `current_rssi >= rssi_threshold` (signal is strong enough → within geographic range)
2. Message has not been seen before (dedup cache check)
3. `ttl > 0` (hop limit not exceeded)

This creates a physical "fence" around the broadcast origin. A message with `rssi_threshold = -75 dBm` will propagate approximately 10–30 meters in an urban environment depending on obstacles, then stop. This is a **physical**, not cryptographic, access control — data cannot leave the intended area because the radio signals cannot carry it there.

---

## 4. Thermal Management

### 4.1 Thermal Model

Nodes embedded in concrete experience significant thermal inertia. The thermal model captures this:

$$T_{core}(t) = T_{ambient} + (T_{initial} - T_{ambient}) \cdot e^{-t/\tau} + P \cdot R_{JA} \cdot D$$

Where:
- $T_{core}$ = Junction temperature (millidegrees Celsius)
- $T_{ambient}$ = Ambient temperature (default: 25°C)
- $\tau = R_{JA} \cdot C_{TH}$ = Thermal time constant (seconds)
- $P$ = Power dissipation (W)
- $R_{JA}$ = Junction-to-ambient thermal resistance (K/W)
- $D$ = Duty cycle (0–1)
- $C_{TH}$ = Thermal capacitance of node + surrounding material (J/K)

### 4.2 Thermal-Aware Scheduling

The thermal scheduler prevents compute operations from exceeding the maximum junction temperature (85°C):

```
For each pending operation:
  1. Calculate expected temperature rise: ΔT = P × R_JA × (1 − e^(-t/τ))
  2. If core_temp + ΔT > 85°C: defer operation, return thermal limit error
  3. Execute operation if budget allows
  4. Update estimated core temperature
```

Operations are prioritized: radio transmission > mesh routing > sensor sampling > housekeeping.

---

## 5. Structural Integrity Monitoring

### 5.1 Vibration Baseline

Each node establishes a vibration baseline using an exponentially weighted moving average:

$$B_{t} = \alpha \cdot S_t + (1 - \alpha) \cdot B_{t-1}$$

Where $\alpha = 0.1$ and 100 samples are required before the baseline is considered valid.

### 5.2 Anomaly Detection

Structural anomalies are detected as deviations from the baseline:

$$\delta = \frac{\|S_t - B_t\|}{\|B_t\|} \times 100$$

If $\delta \geq 20\%$, the sample is flagged as anomalous. After 5 anomalous samples, an alert is broadcast through the mesh.

The confidence of an anomaly is calculated as:

$$C = \frac{\delta - 20}{80} \times 255$$

Where C ranges from 0 (uncertain) to 255 (high confidence).

### 5.3 Limitations

This method detects **changes** in vibration transmission characteristics. It cannot distinguish between:
- A crack in the structural material
- A change in the node's mechanical coupling (e.g., enclosure degradation)
- Environmental changes (e.g., seasonal temperature effects)

For production deployment, this should be complemented with TinyML-based classification (Phase 3).

---

## 6. Environmental Entropy Generation

Each node generates cryptographic-quality entropy from physical noise sources:

1. **ADC noise:** A floating/differential ADC input captures Johnson–Nyquist noise from a high-impedance input
2. **Vibration noise floor:** The accelerometer's noise floor varies with environmental conditions
3. **Temperature variations:** Micro-fluctuations in the temperature sensor reading

These are combined using a simple hash:

```
state = 0xDEADBEEF_CAFEBABE
for each sample s:
    state = state × 6364136223846793005 + s
output = state[0..12]
```

This provides 96 bits of entropy for ChaCha20 nonces. In deployment, this should be validated with a statistical test suite (e.g., NIST SP 800-22).

---

## 7. Security Considerations

### 7.1 Threat Model

| Threat | Impact | Mitigation |
|--------|--------|------------|
| Physical tampering | Node extraction/replacement | Encrypted storage, tamper-evident potting |
| Eavesdropping | Message interception | ChaCha20-Poly1305 encryption |
| Message injection | False data injection | Authenticated encryption (AEAD) |
| Replay attacks | Stale message replay | Per-message nonces, sequence numbers |
| Traffic analysis | Metadata leakage | Fixed-size slices, constant-rate beacons |
| Node capture | Key compromise | Key rotation, threshold cryptography |
| Denial of service | Energy exhaustion | Energy budgeting, anti-spam metrics |
| Concrete attenuation | Range reduction | Sub-1 GHz radio for embedded nodes |

### 7.2 Key Management

The default deployment uses a pre-shared symmetric key (256-bit). For production:

- Keys should be injected during manufacturing (not stored in firmware source)
- Periodic key rotation over the mesh (Phase 3)
- Threshold secret sharing for split-key operation

### 7.3 Data Locality as Security

RSSI geo-fencing provides a novel security property: **physical data locality**. Data cannot be exfiltrated beyond a physical radius because the radio signals that carry it attenuate below the relay threshold. This is particularly valuable for:

- Traffic pattern data (should not leave the road segment)
- Structural integrity data (should not leave the bridge)
- Localized civic alerts (should not leave the neighborhood)

---

## 8. Deployment Strategy

### 8.1 Phase 1: Laboratory Prototype (Months 1–14)

- Hand-solder 5–10 prototype nodes (2.4 GHz nRF24L01+ for bench testing)
- Verify energy harvesting from laboratory shaker table
- Test mesh connectivity at 10 m range
- Measure power budget and refine energy model
- **Validate piezoelectric tuning:** characterize harvester output at 10–120 Hz

### 8.2 Phase 2: Controlled Environment (Months 15–28)

- Deploy 20 nodes in a test structure (concrete slab or asphalt patch)
- **Switch to Sub-1 GHz (CC1101)** for concrete-embedded nodes
- Test mesh routing with node failures
- Validate dead-drop storage and retrieval
- Measure packet delivery ratio under various conditions
- Test Flash wear-leveling with sustained write loads

### 8.3 Phase 3: Field Deployment (Months 29–42)

- Deploy 50 nodes in a bridge or roadway
- Add TinyML-based anomaly classification
- Implement dynamic piezoelectric resonance tuning
- Integrate with City OS DAG ledger for civic data infrastructure
- Develop desktop monitoring application

### 8.4 Phase 4: Open Source (Months 43–54)

- Release all hardware designs under MIT license
- Publish firmware and protocol specifications
- Publish field test results
- Build community around the platform

---

## 9. Related Work

| Project | Focus | Differentiation |
|---------|-------|-----------------|
| **Smart Dust** (Berkeley, 1990s) | MEMS sensor nodes | Energy harvesting from vibration, not battery |
| **WISP** (Intel/University of Washington) | RFID-scale sensing | Ad-hoc mesh, not reader-dependent |
| **OpenTag** (University of Washington) | Backscatter sensors | Active radio, longer range |
| **LoRaWAN** (Semtech) | Long-range IoT | Decentralized mesh, no gateway |
| **Helium Network** | Decentralized IoT | Infrastructure-embedded, structural monitoring |
| **City OS** | Urban data platform | Native DAG ledger integration in Phase 3 |

---

## 10. Future Work

### 10.1 TinyML Anomaly Classification (Phase 3)

Train a TensorFlow Lite Micro model on vibration signatures to classify:
- Normal traffic vibration
- Structural crack propagation
- Material fatigue precursor signals
- Node degradation

### 10.2 Crystalline Lattice Resonance Tuning

Implement adaptive frequency matching between the piezo element's resonant frequency and the dominant ambient vibration frequency. This can be achieved by:

1. Measuring the vibration spectrum using the accelerometer
2. Adjusting a digital trim capacitor bank to shift the piezo resonance
3. Locking to the peak frequency using a simple gradient ascent algorithm

### 10.3 Integration with City OS

The Smart Matter mesh can serve as a physical-layer data source for City OS, a DAG-based urban data ledger. Each node would periodically commit integrity reports and entropy samples to the ledger, creating an immutable record of structural health.

### 10.4 Self-Healing Mesh

Implement predictive routing that anticipates node failures based on energy trends and reroutes traffic proactively before nodes go critical.

---

## 11. Conclusion

Smart Matter demonstrates that it is possible to embed intelligent, energy-autonomous computing into the very materials that constitute our urban infrastructure. By harvesting ambient vibration, forming decentralized mesh networks, and enforcing data locality through physics rather than policy, Smart Matter enables a new class of civic applications: self-diagnosing bridges, traffic-responsive roads, and data-sovereign neighborhoods.

The platform is designed to be **built by one person, deployed by many**. The total BOM cost of ~$14 per node, the use of widely available COTS components, and the MIT-licensed open-source release ensure that the barrier to entry is low enough for citizen scientists, civic hackers, and research groups to participate.

The 4.5-year development plan is aggressive but achievable. The recommended MVP — energy harvesting + mesh networking + dead-drop storage — can be demonstrated within 12 months and provides immediate value for structural monitoring applications.

---

## References

1. Roundy, S., Wright, P. K., & Rabaey, J. (2003). *Energy Scavenging for Wireless Sensor Networks*. Springer.
2. Priya, S., & Inman, D. J. (2009). *Energy Harvesting Technologies*. Springer.
3. Polastre, J., Hill, J., & Culler, D. (2004). "Versatile low power media access for wireless sensor networks." *SenSys '04*.
4. Woo, A., Tong, T., & Culler, D. (2003). "Taming the underlying challenges of reliable multihop routing in sensor networks." *SenSys '03*.
5. Bernstein, D. J. (2008). "ChaCha, a variant of Salsa20." *Workshop Record of SASC 2008*.
6. Nir, Y., & Langley, A. (2018). "ChaCha20-Poly1305 for Transport Layer Security." *RFC 8439*.
7. Texas Instruments. (2015). "BQ25504 Ultra Low-Power Boost Converter with MPPT." Datasheet.
8. STMicroelectronics. (2020). "STM32G031x6/x8: Ultra-low-power MCU." Datasheet.
9. Nordic Semiconductor. (2020). "nRF24L01+ Single Chip 2.4 GHz Transceiver." Product Specification.
10. NIST. (2010). "A Statistical Test Suite for Random and Pseudorandom Number Generators." *NIST SP 800-22 Rev. 1a*.
11. ITU-R. (2015). "Effects of building materials on radio wave propagation." *Recommendation ITU-R P.2040-1*.

---

## Appendix A: Bill of Materials

| Component | Part Number | Qty | Cost | Source |
|-----------|-------------|-----|------|--------|
| MCU | STM32G031J6 | 1 | $1.50 | DigiKey, Mouser |
| Radio (2.4 GHz) | nRF24L01+ | 1 | $1.80 | DigiKey, AliExpress |
| Radio (Sub-1 GHz) | CC1101 | 1 | $2.50 | DigiKey, Mouser |
| Accelerometer | LIS3DH | 1 | $1.20 | DigiKey, Mouser |
| LDO Regulator | MCP1700-3302E | 1 | $0.35 | DigiKey |
| Boost Converter | BQ25504 | 1 | $3.50 | DigiKey, Mouser |
| Bridge Rectifier | DF10S | 1 | $0.25 | DigiKey |
| Supercapacitor | SCMR18C105MRBA0 | 1 | $2.50 | DigiKey, Mouser |
| Piezo Element | PZT-5A 20 mm | 1 | $2.00 | APC International, STEMiNC |
| Passive components | — | ~12 | $1.05 | Any |
| **Total (2.4 GHz)** | | | **~$14.15** | |
| **Total (Sub-1 GHz)** | | | **~$14.85** | |

## Appendix B: Quick Start

```bash
# Build all workspace crates
cargo build

# Run all tests (49+)
cargo test

# Run the network simulator
cargo run --bin smart-matter-network-simulator -- --nodes 50 --duration 60

# Use the desktop tool
cargo run --bin smart-matter-desktop -- monitor
```

For hardware assembly instructions, see `docs/hardware/build-guide.md`.
For firmware flashing, see `docs/firmware/firmware-guide.md`.
For the full protocol specification, see `docs/protocol/protocol-spec.md`.

## Appendix C: Piezoelectric Tuning & Risk Mitigation

### Narrow Bandwidth Risk

PZT-5A elements at 60 Hz have a Q-factor of 30–50, limiting efficient energy harvesting to a ~2–4 Hz bandwidth. Real-world vibration sources span 1.5–120 Hz:

| Source | Frequency | Amplitude | Energy Potential |
|--------|-----------|-----------|-----------------|
| Pedestrian footfall | 1.5–3 Hz | 0.1–0.5 g | Low |
| Bridge vibration (traffic) | 10–40 Hz | 0.05–0.3 g | Medium |
| Road traffic | 5–30 Hz | 0.1–0.8 g | Medium-High |
| Railway | 40–120 Hz | 0.2–1.0 g | High |

### Mitigation Strategies (ordered by practicality)

1. **Cantilever beam (recommended for Phase 2+):** Replace the rigid PZT disk with a cantilever-mounted bimorph. Adjustable proof mass provides 10–40 Hz bandwidth. Demonstrated in literature to increase harvestable energy 3–5× in typical traffic scenarios.

2. **Dynamic resonance tuning (Phase 3):** Use the accelerometer to measure dominant frequency, then adjust a digital trim capacitor bank to shift the piezo's electrical resonance. Implemented in firmware at `firmware/core/src/energy.rs`.

3. **Array of elements:** Deploy nodes with staggered resonant frequencies (20, 40, 60 Hz) at each location. Increases cost but ensures some harvesting across the full spectrum.

4. **Nonlinear/bistable harvesting:** Advanced technique using snap-through buckling for wideband response. Active research area, potentially deployable in Phase 4.

---

**Smart Matter — Intelligent Materials for a Self-Aware Infrastructure**  
MIT License — Build. Embed. Monitor. Decentralize.