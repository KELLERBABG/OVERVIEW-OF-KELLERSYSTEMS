# What Was Deleted & Why

## The Problem

Windows Explorer was reporting ~6,500 files / 1,300 folders. The real count was **~50,600 files / ~6,800 folders** — almost entirely from `node_modules/`.

## What Was Deleted (~50,378 files removed)

| Directory | Files | Description |
|---|---|---|
| `node_modules/` (root) | ~36,000 | pnpm's global store (`.pnpm/`) — a content-addressable cache of every npm package used anywhere in the project |
| `packages/*/node_modules/` | ~10,000+ | Per-package `node_modules` with symlinks to the `.pnpm` store |
| `apps/demo/node_modules/` | ~3,500 | Demo app's dependency tree (vite, vitest, and copies of every internal `@fungal/*` package with their own nested deps) |
| `apps/dev-tools/node_modules/` | ~500 | Dev-tools dependencies |
| `apps/demo/dist/` | ~10 | Built output from Vite (HTML, JS bundles, assets) |
| `.turbo/` | ~150 | Turborepo build cache (serialized task outputs) |

**Total deleted:** ~50,000+ files / ~6,600+ folders

## What Remains (219 files)

Only your **actual project source code**:

### Root config files (9)
- `pnpm-workspace.yaml` — workspace definition
- `package.json` — root package manifest
- `tsconfig.json` — root TypeScript config
- `turbo.json` — Turborepo pipeline
- `vitest.config.ts` — test runner config
- `.eslintrc.cjs` — linting rules
- `.prettierrc` — formatting rules
- `.gitignore` — git ignore rules
- `README.md` — project documentation

### Scripts (3)
- `scripts/build.sh`, `scripts/dev.sh`, `scripts/test.sh`

### Docs (5)
- `docs/architecture.md`, `docs/protocols.md`, `docs/api-reference.md`, `docs/deployment.md`, `docs/whitepaper.md`
- `docs/adr/` (template)

### Planning (8)
- `planning/DEVELOPMENT_PLAN.md`, planning canvas, `phase-0` through `phase-5` docs

### CI (1)
- `.github/workflows/ci.yml`

### Packages — Source Code & Tests (~180 files)

Each of the 14 packages has:
- `package.json` — package manifest
- `tsconfig.json` — TypeScript config
- `src/` — TypeScript source files
- `tests/` — Vitest test files

Packages (on disk without `node_modules`/`dist`/`.turbo`):

| Package | Source Files | Test Files |
|---|---|---|
| `core` | 10 | 4 |
| `signaling-server` | 4 + 1 handler | 1 |
| `mycelial-mesh` | 6 | 1 |
| `signal-transduction` | 3 | 2 |
| `wasm-executor` | 9 | 3 |
| `task-distribution` | 9 | 3 |
| `redundancy` | 4 | 2 |
| `resource-scoring` | 8 | 1 |
| `zk-integrity` | 6 | 1 |
| `homomorphic` | 4 | 0 |
| `consensus` | 0 | 0 |
| `incentive-token` | 0 | 0 |
| `substrate-layer` | 0 | 0 |
| `client` | 0 | 0 |

### Apps — Source Code & Build Config (~12 files)

| App | Files |
|---|---|
| `apps/demo/` | `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/index.ts` |
| `apps/dev-tools/` | `package.json`, `tsconfig.json`, `vite.config.ts`, `index.html`, `src/index.ts` |

## How to Restore

```bash
# Install all dependencies again
pnpm install

# Build all packages
pnpm build

# Run tests
pnpm test
```

After `pnpm install`, the file count will go back up to ~50,000. This is **normal** for a TypeScript monorepo — the `node_modules/` directory is not "your code," it's the toolchain's cache.

## The Real Takeaway

**219 files** is your actual project. The other ~50,000 files are npm dependencies that every Node.js/TypeScript project needs. Windows Explorer counts them because they're on disk, but they're not source code — they're the `node_modules/` equivalent of a compiler cache.