# KELLER-OS (Sovereign Bare-Metal Microkernel) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `KELLER-OS/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo check --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**KELLER-OS (Sovereign Bare-Metal Microkernel)** — A high-assurance, bare-metal sovereign microkernel and hypervisor written from scratch in #![no_std] Rust designed for zero-trust, post-quantum mesh networks, and strict memory safety.

### Current Engineering Baseline
A bare-metal `#![no_std]` sovereign x86_64/i686 microkernel (`src/main.rs`, `src/proc.rs`, `src/mm.rs`, `src/ipc.rs`, `src/crypto.rs`, `src/net.rs`, `src/storage.rs`, `src/vault.rs`, `src/zk.rs`, `src/panic.rs`) with Python QEMU verification harnesses (`dev-tools/qemu_check.py`, `wire_check.py`, `smp_check.py`, `disk_check.py`).

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- Microkernel Architecture (L4/seL4):
  Entwicklung eines minimalen Kernels, der mathematisch beweisbar (Formal Verification) sicher gegen Privileg-Eskalation ist.
    
- Formal Verification (TLA+ / Coq / Lean): Mathematische Beweisführung, dass der Code exakt das tut, was er soll, und keine undefinierten Zustände zulässt.
    
- Capability-based Security: Ein Zugriffsmodell, bei dem Prozesse explizite „Tokens“ benötigen, anstatt globale Admin-Rechte zu besitzen.
    
- Zero-Knowledge Authentication:
  Anmeldung am System, ohne dass Passwörter im Klartext oder als Hash auf der Festplatte liegen (via zk-SNARKs).
    
- Hardened Drivers in Userspace:
  Treiber laufen isoliert vom Kern, sodass ein abstürzender Grafiktreiber nicht das gesamte System kompromittieren kann.
    
- Sandboxing & Namespaces: 
  Absolute Isolation von Applikationen; eine App kann niemals Daten einer anderen App sehen, außer es wird explizit autorisiert.
    
- Deterministic Builds: Sicherstellen, dass der binäre Code zu 100% aus dem Open-Source-Quellcode reproduzierbar ist (Schutz vor Manipulation beim Kompilieren).

---

## 3. Theoretical Foundations & Protocol Mechanics (`Life Work.canvas`)

Extracted architectural derivations, mathematical formulations, and state-machine specifications from the master canvas:

### [KELLER OS] Specification Node
> Read Only Root
> 
> Immutability wird verwendet um Rootkits unmöglich zu machen

### [KELLER OS] Specification Node
> Temporal Isolation
> 
> Der Angreifer könnte messen, wie lange die CPU braucht für eine Operation, um auf geheime Schlüssel zu schließen.

### [KELLER OS] Specification Node
> Air-Gap Communication Interface
> 
> Eine physikalische oder logische Barriere, die sicherstelt das es kein elektrischer oder protokollseitiger Pfad zwischen den extranet und dem Kernel existiert.

### [KELLER OS] Specification Node
> Static Resource Allocation
> 
> In Windows z.B. fordert ein Programm speicher an, in diesem System ist es verboten

### [KELLER OS] Specification Node
> Deterministic Builds
> 
> Bit für Bit Identität für Quellcode kompilierung

### [KELLER OS] Specification Node
> Hash-Verifikation:
> Man definiert den Build-Prozess als reine Funktion $f$
> $$f(Source, Environment, Toolchain) = Binary$$
> Vertraue niemals der Binärdatei.
> Berechne $f$ selbst und vergleiche:
> $$H(Binary_{Server}) \stackrel{?}{=} H(Binary_{Lokal})$$
> Nur wenn $H$ identisch ist,ist die Chain of Trust vom Modell bis zur Hardware geschlossen

### [KELLER OS] Specification Node
> Capability-based Security
> 
> Anstelle von ACL's, setzen wir auf das ocap Model

### [KELLER OS] Specification Node
> Coq & Lean 
> (Interactive Theorem Provers)
> 
> Sie funktionieren anders als ein Model Checker. Sie basieren auf dem Curry-Howard-Isomorphismus: Ein Programm ist ein Beweis, und sein Typ ist die Aussage die bewiesen wird

### [KELLER OS] Specification Node
> Hoare Tripel:
> 
> $$\{P\} \, C \, \{Q\}$$
> $P$ (Precondition) Was muss wahr sein?
> $C$ (Command) Code Befehl
> $Q$ (Postcondition) Was ist nachher Wahr?
> 
> Die Verifikation nutzt Weakest Precondition Calculus, es rechnet rückwärts "was ist P, damit bei C, Q eintritt"

### [KELLER OS] Specification Node
> TLA+ (Temporal Logic of Actions)
> 
> Kern: TLA+ beschreibt Systeme als State Machines. Ein Systemzustand ist eine Zuweisung von Werten zu Variablen. Eine "Action" ist ein Übergang von einem Zustand zum nächsten.

### [KELLER OS] Specification Node
> 1. Außenwelt: spricht TCP IP
> 2. Proxy: Terminiert die Verbindung und extrahiert die Rohdaten
> 3. Transport: Daten werden über einen SMB an den isolierten Kern gegeben
> 4. Kern hat keinen Netzwerk-Stack, es sieht nur Daten Objekte

### [KELLER OS] Specification Node
> Ein Prozess $P_n$ kann auf Ressource $R$ zugreifen, wenn es eine Kante im Capability-Graphen gibt di durch legaler Grant-Operationen entstanden ist

### [KELLER OS] Specification Node
> Das OS-Image liegt auf einem Medium das keine Schreibvorgänge erlaubt
> 
> Da der Kernel und die System-Dateien NICHT verändert werden können, kann keine Malware angreifen

### [KELLER OS] Specification Node
> Das Problem: Nicht-Determinismus im Compiler.
> 
> Der Compiler baut Rauschen in die Dateien hinein, Zeitstempel, Pfadnamen und Ungeordnete Datenstrukturen

### [KELLER OS] Specification Node
> In einer ocap ist die Berechtigung nicht trennbar mit dem Schlüssel verbunden.
> 
> **Was ist eine Capability?** Ein Paar aus einem Objekt-Zeiger und einem Rechte-Bitmaske (Read Write Grant)
> 
> **Mathematische Basis**
> Wir haben hier eine Access Control Matrix $M$. Aber speichern diese nicht zentral, sondern es wird in teile zerlegt die der Prozess besitzt

### [KELLER OS] Specification Node
> Determinismus statt Dynamik
> Jede Komponente bekommt ein fixes Budget an Ressourcenm die sich nie ändern
> - Fixed Partitioning: Der Ram wird in Blöcke unterteilt, Prozess A bekommt 10 MB, wenn das voll ist, stirbt es
> - Kein Page-Swapping: Keine Auslagerungsdatei auf der Festplatte.

### [KELLER OS] Specification Node
> Wenn ein Algorithmus bei "1" länger als bei einer "0" braucht, verrät die Ausführungszeit den Key
> Caches, die von zwei Prozessen geteilt werden, verraten Information

### [KELLER OS] Specification Node
> Data Diode (Einweg-Komm.)
> z.B. eine Glasfaserstrecke, auf einer Seite die LED, auf der anderen eine Fotodiode, der Empfänger.
> $\text{Fluss}(A \to B) > 0$, aber $\text{Fluss}(B \to A) \equiv 0$

### [KELLER OS] Specification Node
> Take-Grant Schutz:
> Take: Ein Subjekt nimm eine Capability von einem anderen
> Grant: Ein Subjekt gibt eine Capability weiter.
> Create: Ein Subjekt erschafft ein neues Objekt
> Remove: Ein Subjekt löscht eine Capability.
> 
> Das beweist, dass trotz beliebiger Schritte kein Informationsfluss zwischen isolierten Bereichen entsteht

### [KELLER OS] Specification Node
> Safety vs. Liveness:
> 1. Safety
>    - Mathematisch eine Invariante $Inv$, die in jedem erreichbaren Zustand $s$ wahr sein muss: 
>     - $\forall s \in States: s \models Inv$
> 1. Liveness
>    - Nutzt Temporale Operatoren wie $\square$ (Immer) und $\diamond$ (Irgendwann).
>    - $\square(Request \implies \diamond Response)$

### [KELLER OS] Specification Node
> Dependent Types:
> In Sprachen, hast du den Typ "List", bei Coq/Lean ist es Typ "List n" wobei n die Länge der Liste ist. Ein Fehler wie "Index out of bounds" ist kein Laufzeitfehler, es lässt sich einfach nicht kompilieren, weil der Beweis für den Zugriff fehlt

### [KELLER OS] Specification Node
> Konfigurationen und temp Data liegen nur im RAM
> 
> - Nach einem Neustart ist das System in einem mathematisch verifizierten Zustand
> - Ein Angreifer kann den "Zustand" niemals dauerhaft korrumpieren

### [KELLER OS] Specification Node
> Der Algorithmus: Normalisierung des Build-Prozesses
> 1. Source Normalization: Zeitstempel werden auf einen fixen Wert gesetzt.
> 2. Environment Stripping: Alle Pfadinformationen werden durch relative Pfade ersetzt
> 3. Fixed Toolchain: Der Compiler und die Bibliotheken werden gezwungen einen exakten Hash zu erstellen

### [KELLER OS] Specification Node
> Die Lösung:
> Constant-Time Execution
> Kritische Funktionen müssen so gebaut werden, das die alle die gleiche Anzahl an CPU-Zyklen benötigen, unabhängig vom Input
> $\forall x, y: T(x) = T(y)$
> - Time-Partitioning Scheduler: Der Scheduler im seL4 verteilt CPU-Zeit in fixen Slots. Damit B nicht A messen kann zum Beispiel, bekommt A 10ms, auch wenn es in 2ms fertig wäre.
> - Cache Flushing: Beim Kontextwechsel zwischen Sicherheitsdomänen wird der L1 Cashe gelöscht

### [KELLER OS] Specification Node
> Semantische Filterung & Marshalling
> 
> Wenn bidirektionaler Austausch gebraucht ist, nutze ich ein Gatekeeper-Modell:
> - Protocol Scrubbing: Externe Datenströme werden zerlegt, keine komplexen pakete in den Kern lassen
> - Format-Erzwingung: nur fest definierte, einfache Strukturen werden akzeptiert
> - Mathematische Validierung: Jeder Datensatz wird gegen ein Schema geprüft, wenn es nicht in der Definition ist, wird es verworfen

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `KELLER-OS/`: **51**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 25 | Build / Package Manifest |
| `build.rs` | 11 | Core Implementation Module |
| `dev-tools/build_iso.py` | 175 | Core Implementation Module |
| `dev-tools/disk_check.py` | 379 | Core Implementation Module |
| `dev-tools/make_font.py` | 136 | Core Implementation Module |
| `dev-tools/qemu_check.py` | 816 | Core Implementation Module |
| `dev-tools/smp_check.py` | 347 | Core Implementation Module |
| `dev-tools/verify_gui.py` | 346 | Core Implementation Module |
| `dev-tools/wire_check.py` | 955 | Core Implementation Module |
| `src/block.rs` | 326 | Core Implementation Module |
| `src/boot.rs` | 190 | Core Implementation Module |
| `src/bootinfo.rs` | 426 | Core Implementation Module |
| `src/clock.rs` | 58 | Core Implementation Module |
| `src/crypto.rs` | 1297 | Core Implementation Module |
| `src/eth.rs` | 683 | Core Implementation Module |
| `src/fb.rs` | 540 | Core Implementation Module |
| `src/integrity.rs` | 91 | Core Implementation Module |
| `src/ipc.rs` | 260 | Core Implementation Module |
| `src/main.rs` | 703 | Core Implementation Module |
| `src/mm.rs` | 239 | Core Implementation Module |
| `src/net.rs` | 2026 | Core Implementation Module |
| `src/nic.rs` | 1201 | Core Implementation Module |
| `src/panic.rs` | 235 | Core Implementation Module |
| `src/pci.rs` | 682 | Core Implementation Module |
| `src/port.rs` | 43 | Core Implementation Module |
| `src/proc.rs` | 2409 | Core Implementation Module |
| `src/sched.rs` | 164 | Core Implementation Module |
| `src/serial.rs` | 182 | Core Implementation Module |
| `src/session.rs` | 212 | Core Implementation Module |
| `src/shell.rs` | 1012 | Core Implementation Module |
| `src/storage.rs` | 1458 | Core Implementation Module |
| `src/vault.rs` | 278 | Core Implementation Module |
| `src/zk.rs` | 406 | Core Implementation Module |
| `src/arch/acpi.rs` | 576 | Core Implementation Module |
| `src/arch/ahci.rs` | 1133 | Core Implementation Module |
| `src/arch/cpu.rs` | 162 | Core Implementation Module |
| `src/arch/gdt.rs` | 253 | Core Implementation Module |
| `src/arch/idt.rs` | 807 | Core Implementation Module |
| `src/arch/ioapic.rs` | 623 | Core Implementation Module |
| `src/arch/lapic.rs` | 1207 | Core Implementation Module |
| `src/arch/mod.rs` | 36 | Core Implementation Module |
| `src/arch/msi.rs` | 489 | Core Implementation Module |
| `src/arch/paging.rs` | 648 | Core Implementation Module |
| `src/arch/pic.rs` | 163 | Core Implementation Module |
| `src/arch/ps2.rs` | 1686 | Core Implementation Module |
| `src/arch/smp.rs` | 929 | Core Implementation Module |
| `src/gui/canvas.rs` | 323 | Core Implementation Module |
| `src/gui/font.rs` | 279 | Core Implementation Module |
| `src/gui/mod.rs` | 2099 | Core Implementation Module |
| `verification/kani_proofs.rs` | 122 | Core Implementation Module |
| `verification/verify_all.py` | 110 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Capability-Graph Reachability & Take-Grant Attenuation (`src/proc.rs` & `src/ipc.rs`):** Per `KELLER OS.canvas`, process access to any resource $R$ must be governed by an unforgeable Object-Capability (OCap) token coupled with a monotonic generation counter so revoked capabilities fail in $O(1)$ time.
- **Cold-Boot Panic-Zeroing (`src/panic.rs` & `src/vault.rs`):** Upon tamper NMI or fatal invariant violation, `panic.rs` must deterministically overwrite all active vault key slots and session pages with `write_volatile` barriers before halting the CPU.
- **Constant-Time Crypto & Replay Window (`src/crypto.rs` & `src/session.rs`):** Ensure sliding-window anti-replay bitmask (Base Offset Shifting from `Life Work.canvas`) rejects duplicate or stale packet sequence numbers in $O(1)$ without branches on secret bits.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Enforce OCap capability attenuation and monotonic revocation counters in ipc.rs and proc.rs | `src/ipc.rs`, `src/proc.rs` | `python dev-tools/wire_check.py` |
| **TASK-02** | Verify Base-Offset-Shifting anti-replay sliding window in session.rs | `src/session.rs`, `src/net.rs` | `python dev-tools/wire_check.py` |
| **TASK-03** | Enforce volatile RAM key scrubbing in panic.rs and vault.rs on tamper NMI | `src/panic.rs`, `src/vault.rs` | `python dev-tools/wire_check.py` |
