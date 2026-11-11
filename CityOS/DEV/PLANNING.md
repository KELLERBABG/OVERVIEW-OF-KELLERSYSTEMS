# City OS — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `CityOS/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**City OS** — A decentralized urban operating system built on top of KELLER-OS that handles municipal infrastructure, local sharded data ledgers, physical dead-drop networking, and identity-less citizen service delivery.

### Current Engineering Baseline
An 18-crate `#![no_std]` Rust urban operating system workspace (`city-dag`, `city-crypto`, `city-caps`, `city-brp`, `city-shard`, `city-deaddrop`, `city-alloc`, `city-heal`, `city-zkp`, `city-ata`, `city-ehar`, `city-smartmatter`, `city-node`, etc.) built to run as sovereign municipal infrastructure atop KELLER-OS.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Distributed Ledger Technology (DAGs / Hashgraph):** Nutzung von gerichteten azyklischen Graphen für extrem schnelle, gebührenfreie Transaktionen und Status-Updates der Stadt.
    
- **Edge Computing & Fog Networking:** Datenverarbeitung direkt am Sensor (z.B. Ampel, Stromzähler), um Latenz zu minimieren und das Ghost-Net zu entlasten.
    
- **Autonomous Resource Allocation:** KI-gesteuerte Verteilung von Energie und Wasser basierend auf Echtzeit-Bedarf, festgeschrieben in Smart Contracts.
    
- **P2P Infrastructure Governance:** Protokolle, die es Bürgern ermöglichen, über lokale Belange abzustimmen, wobei die Stimmen kryptographisch gesichert und anonym sind.
    
- **Interoperability Protocols:** Standards, die dafür sorgen, dass autonome Fahrzeuge, Drohnen und Logistiksysteme über das Black-Hole-Protokoll sicher miteinander kommunizieren.
    
- **Resilience & Self-Healing:** Algorithmen, die bei Ausfall eines Stadtteils die kritischen Dienste automatisch auf andere lokale Knoten im Ghost-Net umleiten.

---

## 3. Theoretical Foundations & Protocol Mechanics (`Life Work.canvas`)

Extracted architectural derivations, mathematical formulations, and state-machine specifications from the master canvas:

### [City OS] Specification Node
> Graphentheorie
> 
> In der Mathematik wird ein Graph $G$ als ein geordnetes Paar aus Knoten und Kanten definiert

### [City OS] Specification Node
> Der Directed Acyclic Graph
> 
> Hier, der DAG ist ein Graph der gerichtet ist (Pfeile haben eine Richtung) und Azyklisch ist (man kann den Weg nicht verfolgen zum Startknoten)
> - Die Anwendung: Wenn das Dorf eine Transaktion fälschungsweise speichert, kommt DAG
> - Die Logik: Ein Knoten ist ein Ereignis, weil der Graph Azyklisch ist, kann man keine Schleife bauen.

### [City OS] Specification Node
> Algorithmen auf dem Graphen (Das Routing)
> 
> Wenn der Graph aufgebaut ist, muss man daten durchschicken. Dafür nutzt man 2 Algorithmen.
> 
> Dijkstra / A*: Findet den Kürzesten Weg von A nach B in einem gewichteten Graphen
> Max Flow / Min Cut: Berechnet wie viel maximal durch einen Graphen fließen kann, wenn eine Kante eine maximale Kapazität hat.

### [City OS] Specification Node
> Asynchronous Byzantine Fault Tolerance
> 
> Das beweist, das eine Netzwerk aus unabhängigen Knoten einen identischen, fälschunssicheren Zustand erreicht, welbst wenn ein ein knoten lügt und wenn das Netzwerk unzuverlässig ist

### [City OS] Specification Node
> Lange rede gar kein Sinn:
> 
> aBFT ist wie ein ABS im Auto, wenn es ein Break gibt, rutsche ich nicht außer Kontrolle

### [City OS] Specification Node
> Gossip-about-Gossip Protokolle
> 
> Die Kommunikation zwischen der Häuser

### [City OS] Specification Node
> Zero Knowledge Proofs.
> 
> Wie beweise ich, das ich ein Geheimnis kenne, ohne das Geheimnis zu verraten

### [City OS] Specification Node
> Software Ebene:
> 1. Arithmetische Circuit
>    - Die Logik wird ein einen logischen Schaltkreis aus Additions- und Multiplikations-Gattern gebaut
> 1. Polynome
>    - Dieser Schaltkreis wird in Polynome umgewandelt.
> 1. Fiat-Shamir-Heuristik
>    - SNARKs nutzen eine Hash-Funktion, diese generiert dann die Zufallsfragen des Prüfers aus dem Beweis heraus. Der Prover füttert seine Lösung in die Hash-Funktion, die Polynome werden blind an einem Punkt ausgewertet

### [City OS] Specification Node
> Signalverbreitung
> 
> Zwei Geräte nutzen ihre Sensoren, um denselben Schlüssel zu generieren ohne den Schlüssel übers netz zu senden

### [City OS] Specification Node
> Hier müssen wir dann noch einmal einhaken, wie wissen wir, das das Signal chaotisch genug ist?
> 
> Shannon-Entropie. 
> Es misst den Informationsgehalt.

### [City OS] Specification Node
> es ist in 3 Phasen unterteilt:
> 1. Normales Gossip (Der sendet da, aha, ja er hat da hingesendet)
> 2. Hier kommt Hashgraph-Protokoll ins Spiel, bei jeder Kommunikation erstellt ein Knoten einen self-Hash und Sync-Hash, was aufzeigt was der Knoten selbst erstellt hat und was die Nachricht war, der der andere Knoten aufgegeben hat
> 3. Durch diese Hashes, kann jeder Knoten ein Netz aufbauen, das ist der DAG. jeder Raspberry Pi in diesem Fall hat dieselbe Landkarte

### [City OS] Specification Node
> Analogie: Überwachung ist überflüssig, da wenn du in die U-Bahn gehst, musst du ein gültiges ticket gekauft haben. Jetzt läuft das nur durch Ticket und Ausweis vorzeigen. Mit einem ZKP ist das anders: 
> du übergibst einer Schranke eine Mathematische Gleichung, und wenn sie TRUE ausgibt nach dem ausrechnen, kannst du passieren, somit weiß die Schranke nicht wer du bist

### [City OS] Specification Node
> bei aBFT-DAG sind ALLE entscheidungen Final.
> 
> Es ist Offline Resistent

### [City OS] Specification Node
> Die Regel:
> Wenn alle gleichzeitig angreifen oder gleichzeitig den Rückzug antreten, gewinnen sie.
> Das Problem: Einige Knoten sind Verräter, sie senden falsche Nachrichten für Angriff/Rückzug
> 
> Ein system ist BFT wenn die ehrlichen Knoten immer eine gemeinsame Übereinstimmung finden, solange es nicht zu viele verräter sind.
> Um genauer zu sein, 1/3 darf bösartig sein bevor es kippt
> $$n \ge 3f + 1$$
> bei 3 Verrätern, braucht man 10 gute Knoten.

### [City OS] Specification Node
> Das Problem: PRNG vs. TRNG
> 
> Ein Computer nutzt einen Pseudo-Random Number Generator (PRNG), was durch einen Seed errechnet werden kann.
> Wir brauchen den True Random Number Generator (TRNG). Wir nehmen ein thermisches Rauschen eines Widerstandes oder sonstiges.

### [City OS] Specification Node
> $V$ (Vertices): Das sind die Physischen oder logischen Akteure im Netz PC oder Satellit
> $E$ (Edges): Verbindungen zwischen den Knoten.

### [City OS] Specification Node
> Formel: 
> $$H(X) = - \sum_{i=1}^{n} P(x_i) \log_2 P(x_i)$$

### [City OS] Specification Node
> Wobei $P(x_i)$ die Wahrscheinlichkeit eines Byte-Musters darstellt

### [City OS] Specification Node
> Wenn das Signal also perfekt zufällig ist, ist jedes der 256 bytes gleich warscheinlich. 
> 
> Entropie beträgt damit:
> 8.0 Bits pro Byte

### [City OS] Specification Node
> Metriken und Kademlia
> (BRP hat die Formel)
> 
> Routing sucht den kürzesten weg physisch, hier ist das egal.
> 
> Jeder Knoten bekommt eine 256-Bit ID
> Die Entfernung wird durch bitweise XOR-Operation berechnet
> $$d(x,y) = x \oplus y$$
> Warum?
> Weil es die Perfekte Metrik eigenschaften erfüllt:
> 1. Die Distanz von einem Knoten und sich selbst ist 0
> 2. Die Distanz ist symmetrisch
> 3. Es erfüllt die Dreiecksgleichung. Der direkte weg ist kürzer oder gleich lang wie ein Umweg über einen dritten.

### [City OS] Specification Node
> Die Kanten haben spezifische Beschaffenheiten: 
> Ungerichtet: Wenn das Wi-Fi zwischen 2 Häusern gut funktioniert ist die kante {$A, B$}
> Gerichtet: Wenn Satellit $A$ per Laser an Satelit $B$ schießen kann, aber $B$ keinen Sichtkontakt zu $A$ für den Rückweg hat, ist die kante eine Richtung: ($A, B$)
> Gewichtet: Jede Kante hat eine Zahl, die Latenz im Internet sozusagen. Hier könnte es der widerstand oder Übertragungspreis sein

### [City OS] Specification Node
> Dann, damit es funktioniert, kommt Fuzzy Extractors ins Spiel:
> Es ist sozusagen die Fehlerkorrektur in dem System.

### [City OS] Specification Node
> Regeln eines ZKP's
> 1. Vollständigkeit
>    - Wenn die Aussage TRUE ist, wird es immer geglaubt
> 1. Zuverlässigkeit
>    - Wenn die Aussage falsch ist, ist es mathematisch nahezu unmöglich zu betrügen
> 1. Null-Wissen
>    - Der Prüfer weiß nur: Das ist TRUE, diese Person darf passieren

### [City OS] Specification Node
> Muster sind nicht erlaubt, da es deterministisch ist.
> 
> Daher: FFT-Algorithmus. 
> Der FFT Algorithmus nimmt das rohe Zeitsignal und zerlegt es in Frequenzen. 
> Bei den Frequenzen sieht man bei 50Hz (das Stromnetz) einen Spike. Dafür benötigen wir einen Filter der das heraus-cuttet. dann bleibt nur noch das RANDOM NOISE

### [City OS] Specification Node
> Das Ergebnis ist eine winziger String der beweist dass die Polynome korrekt berechnet wurden

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `CityOS/`: **45**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 65 | Build / Package Manifest |
| `crates/city-alloc/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/city-alloc/src/lib.rs` | 200 | Core Implementation Module |
| `crates/city-brp/Cargo.toml` | 13 | Build / Package Manifest |
| `crates/city-brp/src/lib.rs` | 183 | Core Implementation Module |
| `crates/city-caps/Cargo.toml` | 10 | Build / Package Manifest |
| `crates/city-caps/src/lib.rs` | 290 | Core Implementation Module |
| `crates/city-crypto/Cargo.toml` | 15 | Build / Package Manifest |
| `crates/city-crypto/src/aes.rs` | 289 | Core Implementation Module |
| `crates/city-crypto/src/bigint.rs` | 531 | Core Implementation Module |
| `crates/city-crypto/src/ec.rs` | 172 | Core Implementation Module |
| `crates/city-crypto/src/lib.rs` | 124 | Core Implementation Module |
| `crates/city-crypto/src/mac.rs` | 244 | Core Implementation Module |
| `crates/city-crypto/src/osrng.rs` | 163 | Core Implementation Module |
| `crates/city-crypto/src/spinlock.rs` | 62 | Core Implementation Module |
| `crates/city-crypto/src/vault.rs` | 189 | Core Implementation Module |
| `crates/city-dag/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/city-dag/src/lib.rs` | 335 | Core Implementation Module |
| `crates/city-deaddrop/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/city-deaddrop/src/lib.rs` | 200 | Core Implementation Module |
| `crates/city-entropy/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/city-entropy/src/lib.rs` | 186 | Core Implementation Module |
| `crates/city-heal/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/city-heal/src/lib.rs` | 220 | Core Implementation Module |
| `crates/city-incentive/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/city-incentive/src/lib.rs` | 189 | Core Implementation Module |
| `crates/city-interop/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/city-interop/src/lib.rs` | 224 | Core Implementation Module |
| `crates/city-ipc/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/city-ipc/src/lib.rs` | 288 | Core Implementation Module |
| `crates/city-node/Cargo.toml` | 26 | Build / Package Manifest |
| `crates/city-node/src/main.rs` | 338 | Core Implementation Module |
| `crates/city-p2p/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/city-p2p/src/hw.rs` | 216 | Core Implementation Module |
| `crates/city-p2p/src/lib.rs` | 260 | Core Implementation Module |
| `crates/city-shard/Cargo.toml` | 11 | Build / Package Manifest |
| `crates/city-shard/src/lib.rs` | 217 | Core Implementation Module |
| `crates/city-sim/Cargo.toml` | 13 | Build / Package Manifest |
| `crates/city-sim/src/lib.rs` | 279 | Core Implementation Module |
| `crates/city-syscall/Cargo.toml` | 10 | Build / Package Manifest |
| `crates/city-syscall/src/lib.rs` | 141 | Core Implementation Module |
| `crates/city-types/Cargo.toml` | 9 | Build / Package Manifest |
| `crates/city-types/src/lib.rs` | 142 | Core Implementation Module |
| `crates/city-vote/Cargo.toml` | 12 | Build / Package Manifest |
| `crates/city-vote/src/lib.rs` | 282 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Cryptographic Primitives (`crates/city-crypto` & `crates/city-zkp`):** Needs constant-time MAC/signature verification and strict nullifier uniqueness in `city-zkp` so anonymous municipal service vouchers cannot be double-spent.
- **DAG Consensus & Equivocation Detection (`crates/city-dag`):** `city-dag/src/lib.rs` (335 lines) needs formal fork/equivocation slashing when a municipal edge node signs two conflicting state transitions at the same sequence number.
- **Spatial Geohash Sharding & Dead-Drop TTL (`crates/city-shard`, `crates/city-deaddrop`):** Needs automatic shard rebalancing and strict dead-drop epoch expiration to prevent RAM/flash exhaustion on constrained municipal street nodes.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement equivocation detection and causal topological ordering in city-dag | `crates/city-dag/src/lib.rs` | `cargo test -p city-dag --quiet` |
| **TASK-02** | Enforce anonymous credential nullifier uniqueness and epoch bounds in city-zkp and city-caps | `crates/city-zkp/src/lib.rs`, `crates/city-caps/src/lib.rs` | `cargo test -p city-zkp -p city-caps --quiet` |
| **TASK-03** | Implement automated utility load-shedding and self-healing failover across city-alloc and city-heal | `crates/city-alloc/src/lib.rs`, `crates/city-heal/src/lib.rs` | `cargo test -p city-alloc -p city-heal --quiet` |
