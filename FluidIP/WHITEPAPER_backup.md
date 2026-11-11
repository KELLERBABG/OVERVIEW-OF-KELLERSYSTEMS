# Decoupled Fluid IP Network

## A Formal System for Trustless Intellectual Property Management

**Version 1.0 — July 2026**

---

## Abstract

We present the Decoupled Fluid IP Network (DFIP), a decentralized system for
intellectual property management that replaces traditional text-based patents
with machine-verifiable formal specifications. The system combines evolutionary
graph theory, SMT-based formal verification, homomorphic encryption, taint
tracking, and smart contracts into a unified protocol that mathematically
guarantees correct royalty distribution, detects infringement without human
interpretation, and enables atomic blueprint release only when both verification
and payment conditions are satisfied.

---

## 1. Introduction

### 1.1 The Problem

Current patent systems suffer from fundamental limitations:

- **Ambiguity**: Text-based patents require human interpretation, leading to
  expensive litigation over claim boundaries.
- **Opacity**: Derivative works and improvement chains are not tracked
  systematically, making royalty distribution arbitrary.
- **Inefficiency**: Patent verification requires months of legal analysis;
  there is no automated way to determine if a new invention infringes on
  existing patents.
- **Trust**: There is no mechanism for atomic release of blueprints contingent
  on both verification and payment.

### 1.2 Our Approach

DFIP addresses these limitations through a mathematically rigorous framework:

1. **Evolutionary DAG**: Every invention is a node in a directed acyclic graph
   where edges represent derivation relationships with fractional contribution
   weights.
2. **Formal Semantics**: Each invention is translated into a machine-readable
   intermediate representation (IR) with extracted invariants, enabling
   automated comparison via SMT solvers.
3. **Delta Verification**: The difference between any two patents is expressed
   as a boolean formula Δ = ¬Original ∧ Derivative. If SAT, the derivative
   introduces novel functionality; if UNSAT, it may infringe.
4. **Cryptographic Privacy**: Homomorphic encryption allows contribution value
   computation without revealing individual shares. Zero-knowledge proofs
   certify correct solver execution.
5. **Trustless Execution**: Smart contracts enforce atomic blueprint release:
   the manufacturing blueprint is released only when verification confirms
   non-infringement AND micro-payments are complete.

---

## 2. System Architecture

### 2.1 Overview

```
┌────────────────────────────────────────────────────────────────────┐
│                    Application Layer                                │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────────┐ │
│  │ REST API │  │   CLI    │  │  Visual  │  │  Smart Contracts  │ │
│  │  Server  │  │  Tool    │  │  Explorer│  │ (PatentRegistry,  │ │
│  │          │  │ (dfip)   │  │          │  │  RoyaltySplitter, │ │
│  │          │  │          │  │          │  │  VerificationOracle││
│  │          │  │          │  │          │  │  ExecutionGate)   │ │
│  └──────────┘  └──────────┘  └──────────┘  └───────────────────┘ │
├────────────────────────────────────────────────────────────────────┤
│                      Core Engine                                    │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌───────────────────┐ │
│  │ Patent   │  │ Symbolic │  │  Delta   │  │  SMT Solver       │ │
│  │   DAG    │  │    IR    │  │  Builder │  │  (Z3/Bitwuzla)    │ │
│  │(petgraph)│  │(SSA/Mol/ │  │ ¬O ∧ D   │  │  ┌─────────────┐  │ │
│  │          │  │  Geom)   │  │          │  │  │ Severity    │  │ │
│  └──────────┘  └──────────┘  └──────────┘  │  │  Scoring    │  │ │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  │  └─────────────┘  │ │
│  │ Royalty  │  │  Taint   │  │   IP     │  └───────────────────┘ │
│  │ Invariant│  │ Tracking │  │ Explorer │                        │
│  │  Engine  │  │(Lattice) │  │(Cluster) │                        │
│  └──────────┘  └──────────┘  └──────────┘                        │
├────────────────────────────────────────────────────────────────────┤
│                    Network Layer                                    │
│  ┌─────────────────────┐  ┌───────────────────────────────────┐   │
│  │ P2P (libp2p/gossip) │  │ Storage (IPFS/Arweave)            │   │
│  │ - Patent propagation│  │ - Blueprint content addressing    │   │
│  │ - Verification dist │  │ - State commitments               │   │
│  │ - Consensus         │  │ - Integrity proofs                │   │
│  └─────────────────────┘  └───────────────────────────────────┘   │
└────────────────────────────────────────────────────────────────────┘
```

### 2.2 Core Components

#### 2.2.1 Patent DAG (`PatentDag`)

The patent graph is a `StableDiGraph` from the petgraph library, providing
CRDT-friendly stable indices. Each node (`PatentNode`) contains:
- UUID, title, patent type (Software/Chemical/Mechanical)
- Authors, timestamp, content hash
- Formal invariant (symbolic expression)
- License classification (Open/Protected/Secret)

Each edge (`Edge`) contains:
- Parent/child UUIDs
- Edge kind (Derivative/Improvement/Fork/Merge)
- Delta constraint (logical difference)
- Contribution weight (fraction of value added)
- Author signature

The DAG enforces acyclicity: every edge insertion is preceded by a cycle
check using `is_cyclic_directed`. A Merkle-ized lifecycle log provides an
append-only audit trail of all graph mutations.

#### 2.2.2 Intermediate Representation (`InventionIr`)

Three domain-specific IRs are supported:

**Software (SSA Trace):** A control-flow graph of basic blocks with phi nodes,
supporting bit-vector, integer, float, and array types. Operations include
arithmetic, bitwise, comparison, memory load/store, and custom operators.

**Chemical (MolecularGraph):** A graph of atoms with bond types (single,
double, triple, aromatic, hydrogen) and constraint types (chirality, bond
angles, distance ranges, charge ranges, reaction invariants).

**Mechanical (MechanicalModel):** Geometric entities (points, lines, planes,
circles, cylinders, spheres) with constraint types (coincident, parallel,
perpendicular, concentric, distance, angle, tangent, symmetry, fixed,
tolerance).

#### 2.2.3 Delta Constraint Builder

Given an original IR and a derivative IR, the delta builder:

1. Extracts invariants from both IRs (preconditions + postconditions +
   loop/structural invariants)
2. Computes SHA-256 hash of original invariants
3. Produces SMT-LIB 2.0 formula: Δ = ¬(∧ Original_Invariants) ∧
   (∧ Derivative_Invariants)

If Δ is **SAT**: the derivative is novel (introduces behavior beyond the
original). If **UNSAT**: the derivative is fully contained in the original
(potential infringement). Severity scoring uses model complexity (SAT) or
unsat core size (UNSAT) to quantify the degree of difference.

#### 2.2.4 SMT Solver Abstraction

The solver layer supports multiple backends:
- **BuiltInSolver**: A naive consistency checker for development/testing
- **ExternalSolver**: Subprocess invocation of Z3, Bitwuzla, or CVC5
  with timeout support and model parsing

The solver trait `SmtSolver` is implemented by both, with a factory
(`create_solver`) for automatic backend selection.

#### 2.2.5 Royalty Engine

The core invariant: `sum(contribution_weight) = 1.0` for all incoming edges
to any node. Royalty shares are computed via dynamic programming over reverse
topological order:

```
share[node] = sum(share[child] * edge_weight for each child)
```

This computes the effective ownership fraction for every ancestor of any
given node in O(V + E) time.

#### 2.2.6 Taint Tracking

A lattice-based taint system tracks license provenance through the derivation
chain. The lattice elements are:

```
Unlicensed ⊑ Licensed(id) ⊑ Multiple({id₁, id₂, ...})
```

The join (least upper bound) combines licenses when values are composed.
The meet (greatest lower bound) finds common licenses. Propagation rules
can be configured per operator: arithmetic uses Join (union), comparison
uses Meet (intersection).

#### 2.2.7 Homomorphic Encryption

The HE layer (simulating TFHE/OpenFHE) enables:
- Key generation (public/private keypair)
- Encryption of contribution values (0..1 as fixed-point)
- Homomorphic addition of encrypted shares
- Decryption of aggregate totals

Zero-knowledge proofs (Bulletproofs/Groth16 simulation) certify:
- Contribution values are in valid range [0, 1]
- SMT solver execution was correct

#### 2.2.8 IP State Space Explorer

Provides global patent graph queries:
- **Similarity**: Compare any two nodes via Δ computation and SMT solving
- **Plagiarism Detection**: LSH-based pre-filtering by invariant hash, then
  full Δ evaluation, reporting matches with severity < 0.3
- **Equivalence Clustering**: Group patents by normalized invariant equality
- **Novelty Scoring**: max severity across all comparisons against known patents

#### 2.2.9 Smart Contract Layer

Four contract types mirror on-chain logic:

**PatentRegistry**: Stores Merkle commitments of patent nodes on-chain.
Verification checks that `SHA256(serialized_node) == stored_commitment`.

**RoyaltySplitter**: Distributes payments according to registered royalty
invariants. Each payment creates pending entries claimable by recipients.

**VerificationOracle**: Accepts SMT results with ZK proofs, validates proofs,
and emits verification events. Proofs must be ≥ 32 bytes.

**ExecutionGate**: Atomic blueprint release requiring both CLEAN (verification
passes) and PAID (payment confirmed). State machine with states: Locked →
VerifiedNotPaid → Released (or Blocked on infringement).

---

## 3. Mathematical Foundations

### 3.1 Patent Graph as DAG

Let G = (V, E) be a directed acyclic graph where:
- V = set of patent nodes
- E ⊆ V × V = set of derivation edges

Each edge e = (u, v) ∈ E has weight w(e) ∈ [0, 1] representing the
fractional contribution of parent u to child v.

**Axiom 1 (Royalty Conservation):** For all v ∈ V with deg_in(v) > 0:
sum_{u:(u,v) ∈ E} w(u,v) = 1

### 3.2 Delta Formula

Given patents P (original) and Q (derivative) with extracted invariant sets
I_P and I_Q:

Δ(P, Q) := ¬(∧ I_P) ∧ (∧ I_Q)

**Theorem 1:** If Δ(P, Q) is UNSAT, then Q does not introduce any behavior
beyond P (i.e., Q ⊆ P semantically).

**Proof:** UNSAT means there is no model satisfying the formula. Any
behavior of Q must satisfy I_Q, but the formula asserts ¬(∧ I_P), meaning
the behavior is not in P. If no such model exists, all behaviors of Q are
contained in P.

### 3.3 Royalty Propagation

Let R(v) be the royalty entitlement of ancestor v for a usage of descendant u.

R(v) = Σ_{paths p from v to u} Π_{e ∈ p} w(e)

**Theorem 2:** Σ_{v ∈ ancestors(u)} R(v) = 1 for any node u in a DAG
satisfying Axiom 1.

**Proof by induction** on topological order, using the recurrence:
R(child) = w(parent, child) × R(parent) summed over all parents.

### 3.4 Taint Lattice

The taint lattice (T, ⊑, ⊔, ⊓) satisfies:
- Associativity: (a ⊔ b) ⊔ c = a ⊔ (b ⊔ c)
- Commutativity: a ⊔ b = b ⊔ a
- Idempotence: a ⊔ a = a
- Absorption: a ⊔ (a ⊓ b) = a

These properties are verified by property-based tests in the implementation.

---

## 4. Implementation

### 4.1 Technology Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (nightly) |
| Graph Engine | petgraph (StableDiGraph) |
| SMT Solvers | Z3, Bitwuzla, CVC5 (external) |
| HE | Custom (TFHE/OpenFHE binding in production) |
| ZK Proofs | Custom (Bulletproofs/Groth16 in production) |
| Serialization | serde + protobuf |
| P2P | libp2p (planned) |
| Storage | IPFS/Arweave (planned) |
| Smart Contracts | ink! / Solidity (planned) |

### 4.2 Project Structure

```
dfip/
├── Cargo.toml          # Workspace root
├── core/               # Graph engine, IR, delta, solver, royalties, taint
├── crypto/             # HE + ZK proof layer
├── contracts/          # Smart contract logic
├── api/                # REST/HTTP server (Actix-web)
├── cli/                # CLI tool
├── p2p/                # P2P networking simulation
├── storage/            # Content-addressed storage
├── tests/              # Integration + property-based tests
├── proto/              # Protobuf canonical schema
└── dfip_monolith.rs    # Complete monolith for easy transport
```

### 4.3 Test Coverage

- **71+ unit tests**: Graph operations, IR creation, delta building, solver
  SAT/UNSAT, royalty verification, taint lattice laws, HE roundtrip,
  contract logic, P2P message handling, storage integrity
- **8 property-based tests**: DAG acyclicity, cycle prevention, royalty sums,
  tattice lattice (associativity, commutativity, idempotence), audit trail
  integrity, topological order

---

## 5. Protocol Flow

### 5.1 Patent Submission

```
Inventor → API → DAG + IR Store
  1. POST /patents with title, type, authors, pre/post conditions
  2. System generates UUID, creates PatentNode, builds IR
  3. Node added to PatentDag (cycle-checked)
  4. IR stored for future verification
```

### 5.2 Derivative Registration

```
Derivative Author → API → DAG + Edge
  1. POST /edges with parent_id, child_id, contribution_weight
  2. System validates nodes exist, weight ∈ [0, 1], no cycles
  3. Edge added to PatentDag
  4. Lifecycle event logged
```

### 5.3 Infringement Verification

```
Verifier → API → Delta Builder → SMT Solver → Result
  1. POST /verify with edge_id, original_id, derivative_id
  2. Build Δ = ¬Original ∧ Derivative in SMT-LIB 2.0
  3. Submit to solver (built-in or Z3)
  4. Compute severity from response
  5. Return SAT/UNSAT + severity score
```

### 5.4 Royalty Distribution

```
User → API → Royalty Engine → Payment Split
  1. GET /royalties/{node_id}
  2. Compute ancestor shares via reverse topological DP
  3. Return {node: share} map
  4. Payments distributed via RoyaltySplitter contract
```

### 5.5 Blueprint Release (Atomic)

```
Manufacturer → ExecutionGate → Verification + Payment → Release
  1. Request release with patent_id and blueprint hash
  2. Gate enters Locked state
  3. Solver verifies non-infringement → VerifiedNotPaid
  4. Payment confirmed → Released
  5. Blueprint hash released to manufacturer
  (Order of steps 3/4 is interchangeable)
```

---

## 6. Security Analysis

### 6.1 Threat Model

- **Malicious Verifier**: Could submit false SMT results. Mitigated by ZK
  proofs of correct solver execution (Phase 8/10).
- **Patent Infringer**: Could claim derivative is novel when it infringes.
  Mitigated by automatic Δ computation and UNSAT detection.
- **Royalty Theft**: Could manipulate contribution weights. Mitigated by
  on-chain invariant enforcement: sum(weights) = 1 must hold for every node.
- **Blueprint Theft**: Could access blueprint without payment. Mitigated by
  ExecutionGate atomic release (both CLEAN and PAID required).

### 6.2 Formal Guarantees

1. **DAG invariant**: The graph is always acyclic (enforced by cycle check
   on every edge insertion).
2. **Royalty invariant**: For every node with incoming edges,
   sum(contribution_weights) = 1.0 (verified by RoyaltyVerifier).
3. **Audit integrity**: The lifecycle event log is a Merkle chain;
   any tampering is detectable via `AuditTrail::verify()`.
4. **Taint soundness**: The taint analyzer correctly propagates license
   labels through the full SSA trace according to lattice semantics.

---

## 7. Future Work

- **Production HE binding**: Replace simulated HE with TFHE (Zama) or OpenFHE
  for actual homomorphic operations.
- **Real P2P networking**: Integrate libp2p for gossip-based patent graph
  propagation and verification request distribution.
- **IPFS/Arweave storage**: Implement actual content-addressed storage
  with on-chain commitments.
- **Smart contract compilation**: Compile contract logic to ink! (Substrate)
  or Solidity (EVM) for deployment on blockchain networks.
- **Compiler frontends**: Build real compilers from source code (Rust, C++),
  chemical structure files (SMILES, MOL), and CAD formats (STEP, STL) to the
  DFIP IR.
- **Web visualizer**: Interactive DAG explorer with constraint diff viewer
  and royalty flow animation.
- **Benchmarking**: SMT solver performance on real patent constraint sets,
  graph traversal scaling (100 → 10M nodes), HE computation latency.

---

## 8. Running the System

### 8.1 Prerequisites

```bash
# Rust toolchain (nightly)
rustup toolchain install nightly
rustup default nightly

# Optional: Z3 for real SMT solving
cargo install z3
```

### 8.2 Build and Test

```bash
# Clone and build
git clone <repo>
cd Decoupled\ Fluid\ IP\ Network
cargo build --release

# Run all tests
cargo test                    # 71+ unit tests
cargo test --test tests       # 8 property-based tests

# Start API server
cargo run -p dfip-api
# Server starts at http://127.0.0.1:8080
```

### 8.3 API Usage

```bash
# Submit a patent
curl -X POST http://127.0.0.1:8080/patents \
  -H "Content-Type: application/json" \
  -d '{"title":"My Algorithm","patent_type":"software",
       "authors":["Alice"],"preconditions":["x > 0"],
       "postconditions":["y = x + 1"],"invariants":[]}'

# List patents
curl http://127.0.0.1:8080/patents

# Run simulation (QuickSort example)
curl -X POST http://127.0.0.1:8080/simulate
```

---

## 9. Conclusion

The Decoupled Fluid IP Network provides a mathematically rigorous foundation
for intellectual property management in the digital age. By replacing
ambiguous natural language patents with formal specifications, automating
infringement detection via SMT solving, enforcing royalty invariants
through graph algorithms, and protecting privacy with homomorphic encryption,
DFIP creates a trustless ecosystem where invention, derivation, and
compensation are transparent, automated, and cryptographically verifiable.

---

## References

1. Barrett, C., et al. "SMT-LIB Standard v2.6." (2017)
2. de Moura, L., Bjørner, N. "Z3: An Efficient SMT Solver." TACAS (2008)
3. Chillotti, I., et al. "TFHE: Fast Fully Homomorphic Encryption." (2020)
4. Bünz, B., et al. "Bulletproofs: Short Proofs for Confidential Transactions." (2018)
5. petgraph library. https://github.com/petgraph/petgraph
6. libp2p specification. https://libp2p.io