*Build plan for this project, written from the repository listing, module names and line counts.*
*Not from running tests or reading every function body — where it says "add X", check X isn't already inside.*

## ASCA — Algorithmic Supply Chain Arbitrage

**8,846 lines, 11 crates.** The largest and most architecturally serious thing in the
archive: a workspace with types, graph engine, matching engine, contracts, payload
tracking, security, SMPC, SMT, spec generation and an API — plus four ADRs and a
551-line whitepaper.

### Where the weight is

| crate | lines | holds |
|---|---|---|
| `sca-types` | 908 | `lib.rs` 417 (the vocabulary), graph, routing, payload, error |
| `sca-graph` | 840 | `engine.rs` 450, `lib.rs` 253, `traversal.rs` 137 |
| `sca-security` | 1,484 | temporal 271, capabilities 238, build_verify 217, zk_auth 189, ipc 183, memory_encryption 156, panic_zero 153 |
| `sca-smt` | 1,231 | solver, space_explorer, smt_lib, invariants, path_verifier, constraint |
| `sca-matching` | 796 | engine 258, order 213, arbitrage 174, lib 151 |
| `sca-contracts` | 642 | contract 215, conditions 190, proof 145, escrow 64 |
| `sca-payload` | 709 | tracker 277, chain 266, integrity 141 |
| `sca-smpc` | 817 | protocol 360, shamir 222, field 196 |
| `sca-spec` | 378 | spec 191, generator 166 |
| `sca-api` | 233 | main 76, routes: nodes, edges, paths, topology |

### Build next, in order

1. **`sca-smt/solver.rs` — implement `Z3Solver` properly.** ADR-004 already scoped this
   behind a feature flag. Until it's real, every path-verification result in the graph
   engine rests on a mock. Highest-value single change in the crate.
2. **`sca-graph`** — paired-backhaul analysis (`detect_paired_backhauls`) is implemented
   and tested. The API endpoint `/api/arbitrage/backhaul` is wired up. The integration
   test `test_paired_backhaul_deadhead_reduction_reports_34_5_percent` has buggy
   assertions (the 34.5% doesn't match the 2880/900 numbers) and needs fixing.
3. **`sca-matching/engine.rs` (258 L)** — real order matching: price-time priority, partial
   fills, and a book that survives out-of-order arrival. `arbitrage.rs` should then run
   against it rather than alongside it.
4. **`sca-p2p`** — `protocol.rs` 232 and `network.rs` 208 exist; make two real processes
   exchange shipments end to end and prove it in a test. That's the difference between a
   library and a system.
5. **`sca-api`** — 233 lines of routes is a thin surface. Add auth (the `sca-security`
   capability model is right there), pagination, and OpenAPI generation from the types.
6. **Then the shared `smt-core`** from the section above — after step 1, so you're
   extracting a working implementation rather than a mock.

**What it can achieve:** a graph-native engine for multi-modal freight where arbitrage
opportunities, contract conditions and route proofs are all first-class — with the
verification story (SMT + SMPC + contracts) being the part nobody else bothers with.

---
