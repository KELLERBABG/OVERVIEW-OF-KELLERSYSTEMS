# MycoSwarm — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `TypeScript`
> **Summary:** A TypeScript/Turborepo monorepo (`packages/core`, `packages/client`, `packages/coordinator`, `packages/wasm-runtime`, `packages/crypto`, `packages/p2p`, `packages/verification`, `packages/tokenomics`) implementing browser-based WebAssembly P2P distributed compute.

## Task Queue

- [ ] **TASK-01: Implement honey-pot canary task injection and Byzantine worker slashing in verification**
  - **Target Files:** `packages/verification/src/index.ts`
  - **Context Files:** `packages/core/src/types/task.ts`, `packages/tokenomics/src/index.ts`
  - **Verify:** `npx vitest run`
  - **Spec:** Enhance `packages/verification` so the coordinator can inject deterministic pre-computed canary work units into a worker's queue.

1. Verify that a worker returning a mismatched digest on a canary task or diverging from a 2-of-3 redundant quorum is immediately flagged as Byzantine, its pending reward is forfeited, and its node reputation score drops below scheduling threshold.
2. Add Vitest unit tests covering honest quorum consensus, 1-of-3 Byzantine minority, and canary trap detection.

- [ ] **TASK-02: Enforce deterministic execution timeout and memory cap in wasm-runtime**
  - **Target Files:** `packages/wasm-runtime/src/index.ts`
  - **Context Files:** `packages/core/src/types/task.ts`
  - **Verify:** `npx vitest run`
  - **Spec:** Ensure `packages/wasm-runtime` enforces maximum WASM linear memory pages (`maxMemoryPages <= 256` / 16 MB default) and hard wall-clock execution timeout, aborting runaway tasks cleanly with a structured `WasmExecutionLimitError`.

1. Add unit tests verifying resource limit enforcement.
