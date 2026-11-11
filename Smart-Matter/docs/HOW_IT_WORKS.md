# How Smart Matter Works, Explained for Humans

This document walks through the whole system in plain language. It is meant
to be read top to bottom, like a book. Every fact here matches the code, and
the file paths are there so you can go look at the real thing whenever you
want to dig deeper.

---

## 1. The idea

Concrete and asphalt are the largest man-made structures on the planet, and
they are basically dead weight. They hold up bridges, carry roads, and
absorb vibrations all day long, but they cannot tell you anything about
themselves. A bridge gets inspected by hand every year or two. A crack
grows invisibly until it becomes a headline.

Smart Matter tries to change that by putting small computers *inside* the
material itself. Each one is a tiny self-contained device that:

- harvests its own electricity from vibration, using a piezoelectric crystal
  (the same trick as a gas grill igniter, just continuous);
- talks to its neighbours over a wireless mesh, so there is no cell tower,
  no router, no cloud, and no central point of failure;
- stores encrypted messages for anyone who comes along later (like a
  dead-drop in a spy movie, but for sensor data);
- listens to the material it is embedded in, because cracks change how a
  structure vibrates;
- only talks to devices that are physically nearby, so data stays in the
  neighbourhood where it was created.

The hardware is deliberately boring and cheap: an STM32G031J6
microcontroller (a tiny Cortex-M0+ chip with 64 KB of flash and 8 KB of
RAM), a radio (nRF24L01+ at 2.4 GHz, or a CC1101 at Sub-1 GHz if it has to
talk through concrete), a 3-axis accelerometer, a 1 farad supercapacitor,
and a piezo disc. All of that costs about $14 in volume.

The software is written in Rust, because Rust gives you memory safety
without needing an operating system. That is the key trick that makes this
project possible: the exact same code runs on your desktop as a simulation,
and on the bare-metal chip with zero changes.

---

## 2. What lives inside one node

Open `firmware/node/src/node.rs`. A node is a struct that owns a copy of
every subsystem:

|---------------------|------------------------------------------|------------------------------------------------------------------------------|
|      Subsystem      |                    File                  |                                      Job                                     |
|---------------------|------------------------------------------|------------------------------------------------------------------------------|
| Energy budget       | `firmware/core/src/energy.rs`            | Knows how much charge is in the capacitor and what the node can afford to do |
| Mesh networking     | `firmware/core/src/mesh.rs`              | Finds neighbours, builds routing tables, forwards messages                   |
| Dead-drop storage   | `firmware/core/src/storage/dead_drop.rs` | Holds encrypted message fragments until their TTL runs out                   |
| Localized broadcast | `firmware/core/src/broadcast.rs`         | Sends messages that spread only within a physical radius                     |
| Crypto              | `firmware/core/src/crypto.rs`            | Encrypts fragments, derives per-node keys                                    |
| Entropy pool        | `firmware/core/src/sensor.rs`            | Collects environmental noise to make unpredictable nonces                    |
| Thermal scheduler   | `firmware/core/src/thermal.rs`           | Keeps the node from cooking itself inside the concrete                       |
| Integrity auditor   | `firmware/core/src/integrity.rs`         | Watches vibration for signs of structural damage                             |
| Wire format         | `firmware/core/src/wire.rs`              | Turns protocol messages into radio bytes with a checksum                     |
|---------------------|------------------------------------------|------------------------------------------------------------------------------|

The core crate is `#![no_std]`, which is Rust-speak for "I never touch the
operating system". That one property is what lets the same code run on a
desktop and on a chip with 8 KB of RAM.

### The heartbeat: `tick()`

The node's whole life is one loop, and one pass through the loop is called a
tick (`Node::tick`). Every tick does the same four things:

1. **Sample.** Read the accelerometer, the ADC noise, the temperature. Feed
   them to the entropy pool and the integrity auditor.
2. **Process.** Pull incoming radio messages out of the queue, run them
   through the mesh logic, store any dead-drop fragments that arrived.
3. **Transmit.** If it is time for a beacon, a route advertisement, or an
   integrity report, queue it. Then send what you can, but only spend energy
   you actually have (one slice costs about 50 µJ).
4. **Recharge.** In the host simulation, add back a little harvested energy.
   On the real chip, read the actual capacitor voltage from the ADC.

Between ticks the chip sleeps (`wfi`, which means "wait for interrupt").
That is how a node survives on harvested energy: it does almost nothing,
most of the time.

---

## 3. Energy: the currency everything is priced in

The first rule of this system is that energy is scarce. All the clever
networking is pointless if the node cannot afford to switch on its radio.
So the energy model comes first.

### Where the power comes from

```
vibration → piezo disc → bridge rectifier → boost converter → 1 F capacitor → regulator → chip
```

A piezo disc under vibration produces an AC voltage that can reach 20 V
peak. The bridge rectifier turns that into DC. The boost converter (a
BQ25504 chip) is special: it can start working from as little as 330 mV,
which matters because a half-vibrated piezo puts out almost nothing. It
charges up the supercapacitor. The capacitor is the battery. Finally, a
regulator drops the voltage to what the radio (3.3 V) and the chip (1.8 V)
actually need.

### The numbers

The energy stored in a capacitor is **E = ½ · C · V²**. With C = 1 F and
V = 3.3 V that is **5.445 J**, or about 5.4 million µJ.

Transmitting one slice costs about **50 µJ**. So a full capacitor could in
theory pay for roughly a hundred thousand transmissions. The protocol caps
a single burst at **255 slices** (`available_slices()`), so no single
activity burst can ever drain the capacitor dry.

The node sorts itself into four states based on voltage:

|----------|--------------|-----------------------------------------------------------|
|   State  | Capacitor v. |                         Behaviour                         |
|----------|--------------|-----------------------------------------------------------|
| Surplus  | above 3.3 V  | Full routing, storage, beacons every 5 s                  |
| Nominal  | 2.7 to 3.3 V | Normal operation                                          |
| Low      | 2.2 to 2.7 V | Stops relaying other people's traffic, beacons every 30 s |
| Critical | below 2.2 V  | Wake on vibration only, radio off                         |
|----------|--------------|-----------------------------------------------------------|

The low and critical states are how the mesh protects itself: a node that
is running dry stops volunteering as a relay before it dies completely.

### The honest physics caveat

A piezo disc only harvests efficiently at its resonant frequency. This
design targets about 60 Hz, which matches footsteps and bridge vibration.
Real-world vibrations span 1.5 to 120 Hz, so the harvest is often
sub-optimal. The whitepaper's appendix C has the mitigation ladder:
cantilever-mounted crystals (wider bandwidth), dynamic resonance tuning
with the accelerometer (a later phase), and arrays of crystals staggered in
frequency. That is hardware work; the software just models it
(`PiezoModel`).

---

## 4. What goes over the radio: slices, frames, checksums

Everything a node sends is called a protocol slice: a fixed header plus up
to 128 bytes of payload.

### Slice types

|------|-----------------|-------------------------------------------|
| Code |       Name      |               What it's for               |
|------|-----------------|-------------------------------------------|
| 0x01 | Beacon          | "I am here" presence announcement         |
| 0x02 | RouteAdv        | "Here are the routes I know about"        |
| 0x03 | DataFragment    | A piece of an encrypted dead-drop message |
| 0x04 | DataAck         | "Send me message number N"                |
| 0x05 | IntegrityReport | Structural health report                  |
| 0x06 | Broadcast       | Geo-fenced localized message              |
| 0x07 | Command         | Configuration from the desktop tool       |
| 0x08 | FwUpdate        | A chunk of new firmware                   |
|------|-----------------|-------------------------------------------|

### The frame on the wire

A slice is wrapped in a frame with a checksum before it is transmitted:

| type | src id (8) | dst id (8) | hops | seqno (2) | energy | payload (up to 128) | CRC16 (2) |


- `seqno` is a per-sender counter, used to spot duplicates.
- `energy` tells the receiver how healthy the sender is, so receivers know
  whether to trust its relay.
- `CRC16` is a checksum over everything before it. If it does not match on
  receive, the frame is thrown away. This is how the system shrugs off
  radio noise.

### The small-radio problem

The nRF24L01+ radio can only carry 32 bytes per packet, and the CC1101 can
carry 64. A full frame can be 151 bytes. So `wire.rs` includes a link-layer
fragmenter: `LinkLayer::fragment()` cuts a frame into chunks that fit the
radio, each with a tiny header (`seqno | total | index | len`), and
`Reassembler::push()` stitches them back together at the other end. The mesh
protocol itself never has to care which radio it is running over.

---

## 5. The mesh: how nodes find each other with no help

There is no base station and no coordinator. Nodes wake up, hear each
other, and slowly build a map of who is reachable and how to get there.
This is called gradient-based routing, and the idea is simple: every route
has a cost, and lower is better.

### Discovery: beacons

Every 5 seconds (30 in low energy), a node broadcasts a beacon. Any node
that hears it now knows a direct route to the sender, with a cost of 100
(`HOP_PENALTY`). In effect: "you are one hop away from me."

### Learning: route advertisements

Every 10 seconds, a node broadcasts a RouteAdv containing up to 5 of its
best routes. When node B hears A's advertisement, two things happen:

1. B records a direct route to A, cost 100.
2. For every route A advertised (say "A can reach C"), B records "C via A"
   with a cost of the advertised cost plus 100 for the extra hop, plus a
   penalty if A is low on energy.

Because every hop adds cost, the metric naturally grows with distance. A
node always forwards toward the cheapest option, which is what makes it a
gradient field.

The advertisement is serialized into the slice payload with postcard, a
compact binary format. This used to be broken: the entries were never
actually put on the wire, so no node could learn routes beyond its direct
neighbours. It works now. One constraint remains: a serialized route entry
is 23 bytes, so a single slice fits at most 5 entries. Nodes rotate which
entries they advertise, so the full table eventually spreads across the
mesh.

### De-duplication

Broadcasts are heard by many nodes, and forwarded messages could loop
forever. So every node keeps a small cache of `(source, seqno)` pairs it has
seen before, and anything already seen is dropped.

### Forwarding

When a node receives a slice that is not addressed to it, it asks its
routing table for the next hop and passes the slice along. Two important
details:

- The slice keeps its original source (needed for de-duplication) and its
  final destination. Only the hop counter increases. (There used to be a
  bug where the code rewrote the destination to the next hop, which made the
  message think it had arrived at the second-to-last node and silently die.
  There is a test for this now: `test_multi_hop_forwarding_keeps_destination`.)
- If the node has no route to the destination, it drops the slice. If the
  hop counter reaches 64, it stops. Routes expire after 300 seconds without
  a refresh.

---

## 6. Dead-drop storage: leaving encrypted messages in the concrete

A dead drop, in spy terms, is a place where one agent leaves something and
another agent picks it up later, with no meeting in between. Smart Matter
does this with data. The full pipeline is exercised in
`mesh/tests/integration_test.rs`, so you can watch it happen in a test.

1. **Encrypt.** The sender encrypts the message with ChaCha20-Poly1305, a
   modern authenticated cipher. Authenticated means it both hides the
   content and proves nobody tampered with it. The output is ciphertext
   plus a 16-byte tag.
2. **Fragment.** Messages longer than 128 bytes are split into fragments.
   Each fragment carries a header saying which message it belongs to, its
   position in the message, how long it may live, and when it was created.
3. **Store.** Fragments go into the node's SRAM store, up to 32 fragments
   (about 6 KB). Storage is volatile, so a power cut wipes it. That is
   deliberate: fragments are short-lived anyway, and they are replicated
   across several nodes, so losing one node's copy is fine. Keeping them in
   RAM also means the flash memory never wears out from constant writing.
4. **Retrieve.** Anyone who knows the message ID sends a DataAck request.
   The node gathers all fragments of that message, decrypts each one, and
   puts the original message back together. Once complete, the message is
   removed from the store, because it has been read.

### Why fragments at all?

A message may be bigger than one radio slice, and spreading fragments over
several nodes means no single node ever holds a whole message. An attacker
who captures one node learns nothing, and the message survives even if a
few nodes die.

### Garbage collection

Every 60 seconds the store sweeps out fragments older than their TTL. A
fragment with a TTL of zero expires immediately.

---

## 7. Sovereign broadcast: how data stays local

Some data should never travel far. A traffic alert for one intersection, a
sensor reading for one bridge span. The mechanism is called RSSI
geo-fencing, and it is beautifully physical.

A broadcast message carries an RSSI threshold, for example -80 dBm. When a
node hears the broadcast, it looks at the actual signal strength of the
packet that just arrived:

- signal stronger than the threshold means the sender is physically close,
  so relay;
- signal weaker means the sender is far away, so drop it.

Combined with a hop limit (16) and de-duplication, this creates a bubble.
The message spreads outward until the signal is too weak to qualify, and it
can never cross a city. No software anywhere decides "this data may travel";
the physics of radio waves decide. Physical locality becomes a security
property, which is exactly what the "sovereign" in the project name means.

---

## 8. Crypto: the cipher and the keys

### The cipher

`crypto.rs` uses ChaCha20-Poly1305 from the RustCrypto project, the same
algorithm family that protects TLS connections. It runs in constant time,
allocates no memory, and works in `no_std`. Encryption produces
`ciphertext || tag`; decryption verifies the tag before decrypting. A
flipped bit, a wrong key, or a reused nonce all fail the check.

### The keys: nothing hardcoded

The original version of this project shipped a hardcoded key. Keys are now derived:

```
node_key = HMAC-SHA256(deployment_secret, "smart-matter-v1" + node_id)
```

A deployment picks one random secret, for example with `openssl rand
-hex 32`. Every node's key is derived from that secret plus its own node ID.
What this buys you:

- no two nodes share the same key material;
- if one node is captured, only that node's traffic is exposed, because the
  attacker still cannot derive anyone else's key;
- changing the deployment secret re-keys the whole deployment.

The secret reaches the firmware at compile time through the
`SMART_MATTER_DEPLOYMENT_SECRET` environment variable (embedded), or at
runtime through the same variable in the desktop tool and host simulation.

### Nonces: why the entropy pool matters

A cipher is only secure if the same key never encrypts two things with the
same nonce. Nonces (12 bytes) come from the entropy pool in `sensor.rs`,
which gathers real physical noise:

- the low bits of ADC readings, which are essentially thermal noise;
- vibration-floor samples from the accelerometer;
- temperature jitter.

These get mixed into a 64-bit state (`extract_entropy`) and become nonce
material. On hardware, the nonce would also be combined with the chip's
tick counter to guarantee uniqueness even if two samples happen to be
identical.

---

## 9. Thermal awareness: not melting inside the bridge

`thermal.rs`. Concrete is a decent heat sink, but a node embedded inside it
cannot dump heat quickly, and an overheated chip stops working. The thermal
model treats the node plus its surroundings as an RC circuit: thermal
resistance R (in K/W) and thermal capacity C (in J/K) combine into a time
constant τ = R·C.

- `predict_temperature_rise(power, duration)` estimates how much a compute
  burst will heat things up, using ΔT = P·R·(1 − e^(−t/τ)).
- `max_duty_cycle(power)` works out, given the headroom before the 85 °C
  junction limit, what fraction of the time the node may be active.
- `ThermalScheduler` is a tiny priority queue of operations. Before running
  the highest-priority one, it checks the thermal budget; if the node is
  too hot, the operation waits.

One implementation detail worth knowing: the scheduler finds the
highest-priority operation with a linear scan rather than sorting. The queue
holds at most 16 items, and on current Rust the standard slice sort moved
into the `alloc` crate, which a `no_std` binary cannot use. A scan is the
simplest thing that works.

---

## 10. Integrity monitoring: listening for cracks

`integrity.rs`. Cracks, delamination, and fatigue change how a structure
vibrates, the way a cracked bell sounds different from a whole one. The
auditor learns the baseline vibration signature after 100 samples, then
compares every new sample against it:

- a deviation of 20% or more counts as an anomaly;
- after 5 anomalies, the node broadcasts a localized alert (geo-fenced, so
  it only spreads around the damaged area) and records the event;
- every 60 seconds it can emit a full report: baseline amplitude, current
  amplitude, deviation percentage, anomaly count, and a confidence score.

The algorithm is deliberately simple, thresholds rather than machine
learning. A single node cannot run TinyML on 8 KB of RAM and still harvest
its own power. Phase 3 of the roadmap adds proper anomaly detection.

---

## 11. Flash memory: surviving power loss without wearing out

The chip has 64 KB of flash that tolerates roughly 10,000 writes per
sector. Writing routing checkpoints and configuration naively would kill
the chip in months.

### The memory map

One file, `firmware/core/src/memmap.rs`, is the single source of truth:

```
0x0800_0000  Bootloader     16 KB   (where the CPU starts)
0x0800_4000  Application    32 KB   (the node firmware)
0x0800_C000  Wear-level log 12 KB   (3 sectors of 4 KB)
0x0800_F000  Configuration   2 KB
0x0800_F800  OTA metadata    2 KB   (the pending-update record)
```

The node firmware links at `0x0800_4000` (`firmware/node/memory.x`), the
bootloader at `0x0800_0000`.

### Wear leveling

The 12 KB log region is written log-structured. Entries pile up in RAM
first (a 16-entry buffer), then flush to flash in order. When a sector
fills, the write pointer moves to the next sector, erasing it, so all
sectors wear evenly. Writes are rate-limited to two per second so an error
storm cannot burn the flash. The arithmetic: 3 sectors × 4 KB × 10k cycles
is 120 MB of write capacity, which at one write per hour is more than 50
years of life.

---

## 12. The bootloader and OTA: updating firmware in the field

You cannot open up the bridge to reflash a node, so updates travel over the
mesh. The receiver side lives in `firmware/bootloader/`.

### The space problem

64 KB of flash, and the application already uses 32 KB. There is no room
for a second copy of the firmware to boot from while the first is being
rewritten. So the plan is: the new image gets staged in an external SPI
flash chip (a cheap 4 Mbit part listed as optional in the BOM), and the
bootloader copies it into the application region only after checking it.

### The boot sequence

On power-up, in order:

1. Read the OTA metadata record at `0x0800_F800`
   (`magic | image_len | sha256 | flags`).
2. If there is no valid magic or no pending flag, jump straight to the
   application.
3. If an update is pending, validate it:
   - the staged length matches what was recorded and is at most 32 KB;
   - the SHA-256 of the staged image matches the one recorded when the
     update was prepared.
4. On any failure, clear the pending flag and boot the old application.
   This is fail-closed: a corrupt or truncated image is never booted, and
   the previous firmware keeps running.
5. On success, erase the application region, copy the image in, clear the
   pending flag, and jump.

The jump itself reads the application's vector table at `0x0800_4000` (its
initial stack pointer and reset vector), disables interrupts, sets the
stack pointer, and branches to the reset handler.

The whole logic is written against a `Flash` trait (read, write, erase) and
a `StagedImage` trait, which is why it runs on the host against simulated
memory for tests (six of them: no update, valid, corrupt, length mismatch,
oversized, and the second boot after an apply) and on the chip against the
real flash controller.

### Preparing an update

`smart-matter-flasher ota --firmware app.bin` validates that the file fits
the 32 KB region, computes its real SHA-256, works out the FwUpdate slice
plan (128-byte payloads, 50 µJ of transmit energy per slice), and writes a
`.ota` staging file whose 44-byte header matches exactly what the
bootloader expects.

---

## 13. The tools you run on your desktop

### `smart-matter-node` (host mode)

`cargo run -p smart-matter-node -- --ticks 200` runs the real firmware loop
on your desktop. The sensors are simulated (traffic vibration with
periodic "crack" bursts), the radio is a loopback, and you watch the tick
report: energy level, stored microjoules, available slices, routing table
size, storage utilization, anomaly count, deviation percentage, and
transmissions. It is the fastest way to see the whole system breathe. The
deployment secret comes from the `SMART_MATTER_DEPLOYMENT_SECRET`
environment variable.

### `smart-matter-network-simulator`

`cargo run --bin smart-matter-network-simulator -- --nodes 10 --duration 40`
places nodes on a line with 1 m spacing and a 1.5 m radio range, so
only neighbours hear each other and every hop must be routed. It then runs
the actual mesh protocol: beacons and advertisements propagate until
routing converges, and dead-drop messages get fragmented and routed hop by
hop. The reported metrics (fragments sent and delivered, delivery ratio,
average hop count, energy consumed, routing coverage) are computed from the
run. Nothing is fabricated.

### `smart-matter-desktop`

The operator's tool:

- `monitor` builds a local 3-node mesh and shows the real routing tables
  and derived keys;
- `send` encrypts and fragments a message with the deployment-derived key
  and stores it;
- `retrieve` reassembles and decrypts a message by its ID;
- `integrity` feeds simulated vibration samples to the real auditor and
  shows the computed report;
- `configure` applies settings (RSSI threshold, dead-drop TTL) to real
  protocol objects.

### `smart-matter-flasher`

`swd` validates a firmware binary and prints the exact probe-rs commands to
flash it. `ota` computes the checksum and slice plan and writes the `.ota`
staging file described above.

---

## 14. Stories from start to finish

### A message crosses the mesh

1. You run `smart-matter-desktop send --node-id 0x07 --message "..."`.
2. The tool derives node 7's key, encrypts the message, fragments it, and
   stores the fragments.
3. Each fragment becomes a DataFragment slice with a fresh sequence number
   from `MeshNode::next_seqno`.
4. Node 7 looks up the next hop toward the destination and transmits. At
   every hop: the CRC is verified, the slice is de-duplicated, the routing
   table is consulted, and the slice moves on with the hop counter
   increased and source and destination untouched.
5. The destination node receives the fragment, sees the destination is
   itself, and stores it in its SRAM dead-drop store.
6. When the recipient sends a DataAck with the message ID, the node gathers
   all fragments, decrypts each, reassembles the message, and deletes it.

### The bridge starts cracking

1. The accelerometer samples vibration about a hundred times a second.
2. The vibration analyzer maintains an exponentially weighted baseline. A
   crack changes the signature, and the deviation climbs past 20%.
3. Five anomalous samples in a row trigger an alert.
4. The node builds a geo-fenced broadcast with the anomaly data and
   transmits it. Neighbours relay it only while the received signal stays
   above the threshold, so the alert bubbles through the damaged area but
   never leaks across the city.
5. Periodically the node also emits a full integrity report that a passing
   technician, or the desktop tool, can collect.

### Pushing new firmware

1. You build the new firmware and run `smart-matter-flasher ota --node-id
   0x01 --firmware app.bin`.
2. The flasher validates the size, computes the SHA-256, splits the image
   into FwUpdate slices, and writes the `.ota` staging file.
3. The slices travel through the mesh, budgeted against each node's
   energy, and the target stages them into the external SPI flash and
   records the metadata (`magic | len | sha256 | pending`).
4. On the node's next reset, the bootloader hashes the staged image,
   compares it, and on a match erases, copies, clears the flag, and jumps
   into the new firmware. On a mismatch it boots the old firmware and
   clears the flag.

---

## 15. Where to look in the code

|--------------------------------------------|--------------------------------------------------------|
|          You want to understand...         |                          Read                          |
|--------------------------------------------|--------------------------------------------------------|
| The node's daily loop                      | `firmware/node/src/node.rs` (`Node::tick`)             |
| The host simulation                        | `firmware/node/src/host.rs`                            |
| The embedded platform (LED, ADC, bring-up) | `firmware/node/src/platform.rs`                        |
| Everything the nodes exchange              | `firmware/core/src/types.rs`, `wire.rs`                |
| Routing, beacons, forwarding               | `firmware/core/src/mesh.rs`                            |
| Encryption and keys                        | `firmware/core/src/crypto.rs`                          |
| Storage and reassembly                     | `firmware/core/src/storage/dead_drop.rs`               |
| Flash layout and wear leveling             | `firmware/core/src/memmap.rs`, `storage/wear_level.rs` |
| The boot sequence                          | `firmware/bootloader/src/boot.rs`                      |
| OTA preparation                            | `tools/flasher/src/main.rs`                            |
| Protocol-level tests                       | `mesh/tests/integration_test.rs`                       |
|--------------------------------------------|--------------------------------------------------------|
### How to run everything

```bash
cargo test --workspace                # all unit and integration tests
cargo run -p smart-matter-node -- --ticks 200          # watch one node live
cargo run --bin smart-matter-network-simulator -- --nodes 10 --duration 40
cargo run -p smart-matter-bootloader                     # OTA scenario demo
cargo build --release -p smart-matter-node --target thumbv6m-none-eabi \
    --no-default-features --features embedded            # the real firmware
export SMART_MATTER_DEPLOYMENT_SECRET=$(openssl rand -hex 32)   # keys
```

---

## 16. Honest limitations

A few things are real but not finished, and it is worth knowing exactly
which ones:

- **The peripheral drivers are not written yet.** The firmware builds,
  boots, blinks an LED, and reads the real supercapacitor voltage, but the
  accelerometer (I2C), the radio (SPI), the temperature sensor (1-Wire),
  and the external staging flash are documented "plug in here" points. They
  return `NotReady` or `Busy` until a driver is attached. All the protocol
  logic is exercised by the host simulation and the test suite instead.
- **No PCB has been fabricated.** The bill of materials, the schematic
  description, and the build guide are ready; the board itself is not.
- **The power budget is an estimate.** The model assumes roughly one
  millijoule per vibration event. Real deployments will likely need the
  cantilever and tuning mitigations from the whitepaper before the nodes
  are truly energy-autonomous.
