# TradeShield — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `TypeScript`
> **Summary:** A TypeScript IoT microfiber capture and ledger monorepo (`packages/api`, `packages/shared`, `packages/web`) with 63 passing integration tests, ESP32 telemetry ingestion, tamper-evident hash-chained SQLite credit ledger, and a React dashboard.

## Task Queue

- [ ] **TASK-01: Harden cryptographic hash chain linkage in wash-cycle event ingestion**
  - **Target Files:** `packages/api/src/routes/events.ts`, `packages/api/tests/events.test.ts`
  - **Context Files:** `packages/api/src/db/connection.ts`, `packages/shared/src/types.ts`
  - **Verify:** `npm test --prefix packages/api`
  - **Spec:** Strengthen `packages/api/src/routes/events.ts` so that every ingested wash event links directly to the device's previous block hash.

1. Verify hash continuity: reject with HTTP 400 any event whose `prev_hash` does not match the current tip of that device's chain.
2. Add unit tests in `packages/api/tests/events.test.ts` verifying that out-of-order, duplicate, or modified telemetry events break the chain and are rejected.

- [ ] **TASK-02: Implement filter bypass valve manipulation and sensor fraud detection**
  - **Target Files:** `packages/api/src/routes/events.ts`, `packages/api/tests/events.test.ts`
  - **Context Files:** `packages/shared/src/validation.ts`
  - **Verify:** `npm test --prefix packages/api`
  - **Spec:** Add physical plausibility validation in `packages/api/src/routes/events.ts`.

1. If flow volume $V_{\text{water}} > 50\text{ L}$ is recorded but differential pressure $\Delta P < 0.05\text{ bar}$ after 50 wash cycles, flag `DeviceAnomaly::BypassValveOpenOrFilterRemoved` and flag all resulting credits as `SUSPENDED_PENDING_AUDIT`.
2. Add tests covering nominal filter loading curves vs. sudden pressure drops indicative of filter bypass.

- [ ] **TASK-03: Implement EU Ecodesign ESPR 2027 CSRD compliance certificate generator**
  - **Target Files:** `packages/api/src/routes/credits.ts`, `packages/api/tests/credits.test.ts`
  - **Context Files:** `packages/shared/src/types.ts`
  - **Verify:** `npm test --prefix packages/api`
  - **Spec:** Enhance `packages/api/src/routes/credits.ts` to generate verifiable PDF/JSON CSRD audit certificates when textile brands retire microplastic offset credits.

1. Certificate must include list of participating laundry machine IDs, total water volume treated, certified captured mass ($1\text{ EUR / g}$), and cryptographic hash-chain root.
2. Add integration tests verifying certificate generation and credit retirement state transitions.
