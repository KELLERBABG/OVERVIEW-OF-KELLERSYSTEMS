# SymEx-EVM (Symbolic Execution Engine for Smart Contracts) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `SymEx-EVM/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**SymEx-EVM (Symbolic Execution Engine for Smart Contracts)** — A formal verification and symbolic execution engine for Ethereum Virtual Machine (EVM) bytecode that uses Z3 to generate execution path constraints and detect smart contract vulnerabilities.

### Current Engineering Baseline
A 2-crate Rust symbolic execution engine for EVM bytecode (`crates/core` and `crates/cli`) integrating Z3 SMT solving, abstract interpretation, concolic fuzzing, opcode parsing, gas accounting, and reentrancy/overflow vulnerability pattern matching.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Abstract Interpretation Layer:** Der Quellcode des Smart Contracts wird in eine abstrakte mathematische Darstellung übersetzt, wobei Variablen durch Symbole anstatt durch konkrete Werte ersetzt werden.
    
- **Path Constraint Generation:** Während die Engine den Code durchläuft, sammelt sie für jede Verzweigung (if/else) die logischen Bedingungen und erstellt einen booleschen Ausdruck, der den gesamten Pfad beschreibt.
    
- **State Space Exploration:** Systematische Untersuchung aller theoretisch möglichen Zustände des Contracts, um auch extrem seltene "Edge Cases" zu finden, die bei manuellem Testing übersehen würden.
    
- **SMT-Solver Integration (Z3):** Die gesammelten Pfadbedingungen werden an einen High-Performance-Solver übergeben, der prüft, ob die logische Formel für einen fehlerhaften Zustand (z. B. Integer Overflow) erfüllbar (SAT) ist.
    
- **Formal Verification:** Mathematischer Beweis der Korrektheit des Codes im Hinblick auf vordefinierte Sicherheits-Invarianten, wie etwa die Unveränderlichkeit von Besitzverhältnissen ohne autorisierte Signatur.
    
- **Counter-Example Generation:** Wenn eine Schwachstelle gefunden wird, generiert der Solver automatisch die exakten Eingabewerte, die notwendig sind, um den Exploit in der realen Blockchain-Umgebung auszulösen.
    
- **Vulnerability Pattern Matching:** Automatisierte Suche nach bekannten Angriffsvektoren wie Reentrancy, Front-running oder fehlerhafter Zugriffskontrolle durch vordefinierte logische Schablonen.
    
- **Gas Consumption Analysis:** Symbolische Berechnung der maximal möglichen Gas-Kosten für jede Funktion, um "Out-of-Gas"-Angriffe oder Endlosschleifen bereits vor dem Deployment auszuschließen.
    
- **Taint Analysis:** Verfolgung des Datenflusses von unsicheren Benutzereingaben bis hin zu kritischen Zustandsänderungen, um sicherzustellen, dass keine manipulierten Daten logische Schutzwälle durchbrechen.
    
- **Automated Formal Specification:** Generierung von formalen Spezifikationen aus dem Code-Verhalten, die als mathematische Referenz für zukünftige Upgrades oder Audits dienen.

---

## 3. Theoretical Foundations & Protocol Mechanics (`Life Work.canvas`)

Extracted architectural derivations, mathematical formulations, and state-machine specifications from the master canvas:

### [Execution Engine für Smart Contracts] Specification Node
> Execution Tree
> 
> Hier wird der Kontrollfluss des Contracts in eine Baumstruktur übersetzt
> 
> Jede Verzweigung erzeugt neue Zustände im Baum, wessen Pfade durch die gesammelten Constraint definiert sind
> 
> Der Zustand (Symbolic State)
> Der Zustand besteht aus einem Tuple $\langle \sigma, PC \rangle$
> $\sigma:$ Der symbolische Speicher
> $PC:$ Die aktuelle Pfadbedingung

### [Execution Engine für Smart Contracts] Specification Node
> Abstract Interpretation Layer
> 
> Hier wird Programmcode in ein mathematisches Modell übersetzt.
> 5 wird zu $\alpha$.

### [Execution Engine für Smart Contracts] Specification Node
> Verbandstheorie
> 
> - Halbordnungen:
>   Mengen, in denen Elemente vergleichbar sind
> - Complete Lattices
>   Ich brauche das Konzept von Least Upper Bound und Greates Lower Bound. Wenn ein Codepfad zwei Möglichkeiten hat, nutze ich den Join-Operator um eine Annäherung zu finden, die beide Pfade abdeckt
> - Fixpunkt-Iteration
>   Ich suche den kleinsten Fixpunkt einer Funktion um zu garantieren das meine Analyse terminiert, selbst wenn der Code loops hat

### [Execution Engine für Smart Contracts] Specification Node
> Path Constraint Generation
> 
> dynamische symbolische Auführung des Codes. Das Ziel? für jeden Weg durch den Smart Contract eine mathematische Formel aufbauen

### [Execution Engine für Smart Contracts] Specification Node
> Formale Semantik
> Ich muss definieren, was jeder Befehl bewirkt
> 
> Zustandsübergangsfunktionen:
> Ich muss hier die Mathematik von Funktionen nutzen die einen Zustand $S$ in einen Nachfolgezustand $S'$ verwandelt $f : S \to S'$.
> Abstraktionsfunktionen: 
> Ich muss eine Abstraktionsfunktion definieren, die Werte auf abstrakte Domänen abbildet und eine Konkretisierungsfunktion $\gamma$, die den umgekehrten Weg geht

### [Execution Engine für Smart Contracts] Specification Node
> Ich muss die logischen Bedingungen entlang des Kontrollflussgraphen akkumulieren.
> 
> Die Pfadbedingung ist ein dynamisch erweiternder boolescher Ausdruck. Der start macht: $PC = true$ 
> 
> Die Mechanik: 
> Jedes mal wenn ein if-Statement oder ein require() constraint kommt, kann es nicht Entscheidung ob die Bedingung true oder false ist, stattdessen spaltet die Engine die Ausführung in zwei Zustände auf
> 
> True-Pfad: $PC_{neu} = PC_{alt} \land \text{Bedingung}$ 
> False-Pfad:
> $PC_{neu} = PC_{alt} \land \neg (\text{Bedingung})$

### [Execution Engine für Smart Contracts] Specification Node
> Symbolische Logik & Termsetzung
> 
> - Algebraische Strukturen
>   Ich behandle Programmausdrücke als Terme einer formalen Algebra. Ich muss Regeln setzen wie Terme verieinfacht werde
>   $\alpha + 0$ $\to$ $\alpha$

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `SymEx-EVM/`: **28**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 29 | Build / Package Manifest |
| `crates/cli/Cargo.toml` | 16 | Build / Package Manifest |
| `crates/cli/src/main.rs` | 227 | Core Implementation Module |
| `crates/core/Cargo.toml` | 19 | Build / Package Manifest |
| `crates/core/examples/basic_analysis.rs` | 49 | Core Implementation Module |
| `crates/core/src/lib.rs` | 83 | Core Implementation Module |
| `crates/core/src/abstract_interpretation/mod.rs` | 118 | Core Implementation Module |
| `crates/core/src/analysis/mod.rs` | 160 | Core Implementation Module |
| `crates/core/src/concolic/mod.rs` | 225 | Core Implementation Module |
| `crates/core/src/counter_examples/mod.rs` | 117 | Core Implementation Module |
| `crates/core/src/evm/executor.rs` | 396 | Core Implementation Module |
| `crates/core/src/evm/gas.rs` | 262 | Core Implementation Module |
| `crates/core/src/evm/mod.rs` | 14 | Core Implementation Module |
| `crates/core/src/evm/opcodes.rs` | 317 | Core Implementation Module |
| `crates/core/src/evm/parser.rs` | 118 | Core Implementation Module |
| `crates/core/src/gas/mod.rs` | 156 | Core Implementation Module |
| `crates/core/src/hash/mod.rs` | 216 | Core Implementation Module |
| `crates/core/src/ir/mod.rs` | 186 | Core Implementation Module |
| `crates/core/src/path_constraints/mod.rs` | 160 | Core Implementation Module |
| `crates/core/src/smt/mod.rs` | 196 | Core Implementation Module |
| `crates/core/src/spec_gen/mod.rs` | 145 | Verification & Test Suite |
| `crates/core/src/state_explorer/mod.rs` | 253 | Core Implementation Module |
| `crates/core/src/taint/mod.rs` | 231 | Core Implementation Module |
| `crates/core/src/types/mod.rs` | 315 | Core Implementation Module |
| `crates/core/src/verification/mod.rs` | 158 | Core Implementation Module |
| `crates/core/src/vulnerabilities/mod.rs` | 277 | Core Implementation Module |
| `crates/core/src/z3_backend/mod.rs` | 386 | Core Implementation Module |
| `tests/integration_test.rs` | 82 | Verification & Test Suite |

---

## 5. Gap Analysis & Production-Readiness Audit

- **SMT Constraint Propagation & Reentrancy State Verification (`crates/core/src/vulnerabilities/mod.rs` & `crates/core/src/evm/executor.rs`):** Must verify that state changes (storage writes `SSTORE`) occurring after external calls (`CALL`, `DELEGATECALL`) generate explicit path constraints and counterexample execution traces showing balance drainage.
- **Integer Overflow/Underflow & Unchecked Math Detection (`crates/core/src/path_constraints/mod.rs`):** Must prove whether arithmetic opcodes (`ADD`, `SUB`, `MUL`, `EXP`) can exceed $2^{256}-1$ under feasible calldata inputs.
- **Gas Exhaustion & Out-of-Gas Path Feasibility (`crates/core/src/evm/gas.rs`):** Model dynamic gas costs (memory expansion quadratic factor $\frac{N^2}{512} + 3N$) to detect unbounded loop DoS vulnerabilities.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement cross-function reentrancy detection with state-write after call constraint matching | `crates/core/src/vulnerabilities/mod.rs`, `crates/core/src/evm/executor.rs` | `cargo test -p symbolic_execution_core -p symbolic-execution-cli --quiet` |
| **TASK-02** | Add quadratic memory expansion gas accounting and unbounded loop DoS solver | `crates/core/src/evm/gas.rs`, `crates/core/src/analysis/mod.rs` | `cargo test -p symbolic_execution_core --quiet` |
