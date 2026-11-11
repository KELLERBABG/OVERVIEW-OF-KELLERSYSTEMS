# Phase 0 — Foundation (Weeks 1–2)

**Goal:** Monorepo scaffold, shared types, build toolchain, CI pipeline, and documentation skeleton.

---

## 1. Prerequisites & Toolchain Setup

### 1.1 System Requirements
- Node.js >= 20 LTS
- pnpm >= 9.x (or npm workspaces — pnpm recommended for speed and strictness)
- Git
- VS Code (recommended extensions: ESLint, Prettier, Biome if used)

### 1.2 Initialize the Monorepo

```bash
mkdir fungal-compute-swarm && cd fungal-compute-swarm
pnpm init
```

Create root `pnpm-workspace.yaml`:
```yaml
packages:
  - "packages/*"
  - "apps/*"
```

Install root dev dependencies:
```bash
pnpm add -D -w typescript@5.5 @types/node tsx vitest turbo eslint prettier eslint-config-prettier
```

---

## 2. Root Configuration Files

### 2.1 `tsconfig.json` (root — base config)

```jsonc
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022", "DOM", "DOM.Iterable"],
    "strict": true,
    "noUncheckedIndexedAccess": true,
    "noImplicitReturns": true,
    "noFallthroughCasesInSwitch": true,
    "exactOptionalPropertyTypes": false,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "${configDir}/dist",
    "baseUrl": "${configDir}",
    "paths": {
      "@fungal/core": ["./packages/core/src"],
      "@fungal/signaling-server": ["./packages/signaling-server/src"],
      "@fungal/mycelial-mesh": ["./packages/mycelial-mesh/src"],
      "@fungal/wasm-executor": ["./packages/wasm-executor/src"],
      "@fungal/task-distribution": ["./packages/task-distribution/src"],
      "@fungal/redundancy": ["./packages/redundancy/src"],
      "@fungal/resource-scoring": ["./packages/resource-scoring/src"],
      "@fungal/zk-integrity": ["./packages/zk-integrity/src"],
      "@fungal/homomorphic": ["./packages/homomorphic/src"],
      "@fungal/signal-transduction": ["./packages/signal-transduction/src"],
      "@fungal/consensus": ["./packages/consensus/src"],
      "@fungal/incentive-token": ["./packages/incentive-token/src"],
      "@fungal/substrate-layer": ["./packages/substrate-layer/src"],
      "@fungal/client": ["./packages/client/src"]
    }
  },
  "exclude": ["node_modules", "dist", "**/*.test.ts"]
}
```

### 2.2 `turbo.json` (Turborepo pipeline)

```jsonc
{
  "$schema": "https://turbo.build/schema.json",
  "globalDependencies": ["**/.env.*local"],
  "pipeline": {
    "build": {
      "dependsOn": ["^build"],
      "outputs": ["dist/**"]
    },
    "test": {
      "dependsOn": ["build"],
      "outputs": []
    },
    "lint": {
      "outputs": []
    },
    "dev": {
      "cache": false,
      "persistent": true
    }
  }
}
```

### 2.3 `.eslintrc.cjs`
```js
module.exports = {
  root: true,
  parser: '@typescript-eslint/parser',
  plugins: ['@typescript-eslint'],
  extends: ['eslint:recommended', 'plugin:@typescript-eslint/recommended', 'prettier'],
  env: { node: true, browser: true, es2022: true },
  rules: {
    '@typescript-eslint/no-unused-vars': ['error', { argsIgnorePattern: '^_' }],
    '@typescript-eslint/explicit-function-return-type': 'warn',
  },
};
```

### 2.4 `.prettierrc`
```json
{
  "semi": true,
  "singleQuote": true,
  "trailingComma": "all",
  "printWidth": 100,
  "tabWidth": 2
}
```

### 2.5 `.gitignore`
```
node_modules/
dist/
coverage/
.env
*.local
.turbo
```

### 2.6 Root `package.json` scripts
```jsonc
{
  "scripts": {
    "build": "turbo run build",
    "dev": "turbo run dev",
    "test": "turbo run test",
    "lint": "turbo run lint",
    "format": "prettier --write .",
    "clean": "turbo run clean && rm -rf node_modules"
  }
}
```

---

## 3. The `@fungal/core` Package

This is the shared foundation every other package imports.

### 3.1 Package structure

```
packages/core/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts              # barrel export
│   ├── types/
│   │   ├── node.ts           # NodeId, NodeInfo, NodeStatus
│   │   ├── task.ts           # Task, WorkUnit, TaskResult
│   │   ├── message.ts        # Message, MessageType, Envelope
│   │   ├── crypto.ts         # Proof, KeyPair, Ciphertext
│   │   ├── scoring.ts        # BioScore, BenchmarkResult
│   │   └── consensus.ts      # Vote, Proposal, ConsensusState
│   ├── constants/
│   │   ├── defaults.ts       # Default ports, timeouts, limits
│   │   └── errors.ts         # Error codes and messages
│   ├── utils/
│   │   ├── id.ts             # Nanoid-based unique ID generator
│   │   ├── time.ts           # Timestamp helpers, TTL utilities
│   │   └── serialization.ts  # Structured clone, binary codec stubs
│   └── events/
│       └── event-bus.ts      # Typed event emitter for internal pub/sub
├── tests/
│   ├── types.test.ts
│   └── utils.test.ts
```

### 3.2 Core Type Definitions

#### `src/types/node.ts`
```typescript
/** Unique node identifier (nanoid, 21 chars) */
export type NodeId = string;

export enum NodeRole {
  LEAF = 'leaf',           // Standard browser node
  RELAY = 'relay',         // Node that forwards messages
  BOOTSTRAP = 'bootstrap', // Signaling server / initial contact
}

export enum NodeStatus {
  OFFLINE = 'offline',
  ONLINE = 'online',
  BUSY = 'busy',          // Currently executing a task
  DEGRADED = 'degraded',  // High latency or low resources
}

export interface NodeInfo {
  id: NodeId;
  role: NodeRole;
  status: NodeStatus;
  version: string;
  /** Public key for crypto operations (hex string) */
  publicKey: string;
  /** List of capabilities advertised by this node */
  capabilities: string[];
  /** Human-readable label (optional) */
  label?: string;
}

export interface PeerDescriptor {
  id: NodeId;
  /** WebRTC connection state */
  connectionState: 'new' | 'connecting' | 'connected' | 'disconnected' | 'failed';
  /** Round-trip time in ms (latest measurement) */
  latency: number;
  /** Number of messages exchanged */
  messagesExchanged: number;
  /** When the connection was established (epoch ms) */
  connectedAt: number;
}
```

#### `src/types/task.ts`
```typescript
export type TaskId = string;
export type WorkUnitId = string;

export enum TaskPriority {
  LOW = 0,
  MEDIUM = 1,
  HIGH = 2,
  CRITICAL = 3,
}

export enum TaskStatus {
  PENDING = 'pending',
  SPLITTING = 'splitting',
  DISTRIBUTING = 'distributing',
  EXECUTING = 'executing',
  VERIFYING = 'verifying',
  COMPLETED = 'completed',
  FAILED = 'failed',
  TIMEOUT = 'timeout',
}

export interface Task {
  id: TaskId;
  /** Wasm bytecode or JS function reference */
  code: string | Uint8Array;
  /** Input data (structured clone compatible) */
  input: unknown;
  /** Maximum wall-clock time for the entire task (ms) */
  deadline: number;
  priority: TaskPriority;
  /** Expected output schema (for verification) */
  outputSchema?: string;
  /** Whether this task requires zk-SNARK proof */
  requireProof: boolean;
  /** Whether input must be homomorphically encrypted */
  requireEncryption: boolean;
  createdAt: number;
  ownerId: NodeId;
}

export interface WorkUnit {
  id: WorkUnitId;
  taskId: TaskId;
  /** Subset of the original input for this unit */
  input: unknown;
  /** Indices/ranges this unit covers (for reassembly) */
  range: [number, number];
  /** Size estimate in bytes */
  estimatedSize: number;
  /** Individual deadline (derived from task deadline) */
  deadline: number;
  priority: TaskPriority;
  /** Number of redundant copies dispatched */
  redundancyFactor: number;
}

export interface TaskResult {
  workUnitId: WorkUnitId;
  taskId: TaskId;
  nodeId: NodeId;
  output: unknown;
  /** Execution time in ms */
  executionTime: number;
  /** Wasm memory used (bytes) */
  memoryUsed: number;
  /** zk-SNARK proof if requireProof was true */
  proof?: Uint8Array;
  /** Error message if failed */
  error?: string;
  completedAt: number;
}
```

#### `src/types/message.ts`
```typescript
export enum MessageType {
  // Discovery
  HELLO = 0x01,
  HELLO_ACK = 0x02,
  GOODBYE = 0x03,
  PEER_LIST = 0x04,

  // Signaling (WebRTC)
  OFFER = 0x10,
  ANSWER = 0x11,
  ICE_CANDIDATE = 0x12,

  // Status
  STATUS_UPDATE = 0x20,
  SCORE_BROADCAST = 0x21,

  // Task
  TASK_ASSIGNMENT = 0x30,
  TASK_ACCEPT = 0x31,
  TASK_REJECT = 0x32,
  TASK_RESULT = 0x33,
  TASK_PROGRESS = 0x34,

  // Consensus
  VOTE = 0x40,
  PROPOSAL = 0x41,
  COMMIT = 0x42,

  // Governance
  TOKEN_TRANSFER = 0x50,
  STAKE_UPDATE = 0x51,
}

export interface MessageEnvelope {
  type: MessageType;
  senderId: NodeId;
  targetId: NodeId | '*';  // '*' for broadcast/gossip
  messageId: string;
  /** Unix timestamp (ms) */
  timestamp: number;
  /** Time-to-live in hops (for gossip) */
  ttl: number;
  /** Optional signature (hex string) */
  signature?: string;
  payload: Uint8Array;
}
```

#### `src/types/crypto.ts`
```typescript
export interface KeyPair {
  publicKey: string;  // hex-encoded
  privateKey: string; // hex-encoded
}

export interface ZKProof {
  proof: Uint8Array;
  publicSignals: Uint8Array[];
  verificationKey: Uint8Array;
}

export interface Ciphertext {
  data: Uint8Array;
  /** Which scheme was used (e.g. "BFV", "CKKS") */
  scheme: string;
  /** Public key used for encryption */
  publicKey: string;
}
```

### 3.3 Utility Functions

#### `src/utils/id.ts`
- Uses `nanoid` (21-char, URL-safe) for all IDs
- `generateNodeId(): NodeId`
- `generateTaskId(): TaskId`
- `generateMessageId(): string`

#### `src/utils/time.ts`
- `now(): number` — wraps `Date.now()`
- `remaining(deadline: number): number` — time until deadline
- `isExpired(timestamp: number, ttl: number): boolean`
- `sleep(ms: number): Promise<void>`

#### `src/utils/serialization.ts`
- `serialize(obj: unknown): Uint8Array` — uses structured clone + encode
- `deserialize(bytes: Uint8Array): unknown`
- `estimateSize(obj: unknown): number` — approximate byte size

#### `src/events/event-bus.ts`
```typescript
type EventHandler<T> = (payload: T) => void;

export class EventBus {
  private listeners = new Map<string, Set<EventHandler<unknown>>>();

  on<T>(event: string, handler: EventHandler<T>): void { ... }
  off<T>(event: string, handler: EventHandler<T>): void { ... }
  emit<T>(event: string, payload: T): void { ... }
  once<T>(event: string, handler: EventHandler<T>): void { ... }
}
```

### 3.4 `package.json` for core
```jsonc
{
  "name": "@fungal/core",
  "version": "0.1.0",
  "type": "module",
  "main": "./dist/index.js",
  "types": "./dist/index.d.ts",
  "scripts": {
    "build": "tsup src/index.ts --format esm --dts",
    "test": "vitest run",
    "clean": "rm -rf dist"
  },
  "dependencies": {
    "nanoid": "^5.0.0"
  },
  "devDependencies": {
    "tsup": "^8.0.0",
    "vitest": "^1.6.0"
  }
}
```

### 3.5 `tsconfig.json` for core
```jsonc
{
  "extends": "../../tsconfig.json",
  "compilerOptions": {
    "outDir": "./dist",
    "rootDir": "./src"
  },
  "include": ["src"]
}
```

---

## 4. Temporary Scaffold for All Remaining Packages

To make the monorepo buildable immediately, create a minimal scaffold in every package directory:

```
packages/<name>/
├── package.json    # { "name": "@fungal/<name>", "version": "0.1.0", "type": "module", "main": "./dist/index.js", "scripts": { "build": "tsup src/index.ts --format esm --dts", "test": "vitest run", "clean": "rm -rf dist" } }
├── tsconfig.json   # extends root, outDir ./dist, rootDir ./src
└── src/
    └── index.ts    # export const placeholder = true;
```

This ensures `turbo run build` won't error on missing packages. Each phase will flesh out its packages.

---

## 5. Testing Infrastructure

### 5.1 `vitest.config.ts` (root)
```typescript
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    globals: true,
    environment: 'node',
    include: ['packages/*/tests/**/*.test.ts', 'apps/*/tests/**/*.test.ts'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'lcov'],
      include: ['packages/*/src/**/*.ts'],
    },
  },
});
```

### 5.2 First Tests (for core)
Create `packages/core/tests/types.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { NodeStatus, NodeRole, TaskPriority } from '../src/types/node';

describe('NodeStatus enum', () => {
  it('should have expected values', () => {
    expect(NodeStatus.ONLINE).toBe('online');
    expect(NodeStatus.OFFLINE).toBe('offline');
  });
});
```

Create `packages/core/tests/utils.test.ts`:
```typescript
import { describe, it, expect } from 'vitest';
import { generateNodeId } from '../src/utils/id';

describe('generateNodeId', () => {
  it('should return a 21-character string', () => {
    const id = generateNodeId();
    expect(id).toHaveLength(21);
  });

  it('should produce unique IDs', () => {
    const ids = new Set(Array.from({ length: 100 }, () => generateNodeId()));
    expect(ids.size).toBe(100);
  });
});
```

---

## 6. CI Pipeline (GitHub Actions)

Create `.github/workflows/ci.yml`:
```yaml
name: CI
on: [push, pull_request]

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: pnpm/action-setup@v2
      - uses: actions/setup-node@v4
        with:
          node-version: 20
          cache: 'pnpm'
      - run: pnpm install
      - run: pnpm lint
      - run: pnpm build
      - run: pnpm test
```

---

## 7. Documentation Skeleton

Create initial docs structure:

```
docs/
├── architecture.md       # High-level architecture overview
├── protocols.md          # Protocol specification placeholders
├── api-reference.md      # Auto-generated API docs placeholder
├── adr/
│   └── template.md       # Architecture Decision Record template
└── images/               # Diagrams
```

### `docs/architecture.md` outline
```markdown
# Architecture Overview

## Layers (from bottom to top)
1. Mycelial Mesh (P2P WebRTC)
2. Signal Transduction Protocol
3. Resource & Discovery Layer
4. Task Distribution Layer
5. Integrity & Privacy Layer
6. Incentive / Token Layer
7. Consensus Layer
8. Application Layer

## Data Flow
[Diagram placeholder: node discovery → capability broadcast → task submission → split → distribute → execute → prove → verify → reassemble → reward]

## Key Design Decisions
- Browser-first: no install required, users join via URL
- Ephemeral nodes: assume tabs close at any time
- Redundancy over reliability: probabilistic guarantees
```

---

## 8. Deliverables Checklist (End of Phase 0)

- [ ] Root `package.json` with workspace configuration
- [ ] `pnpm-workspace.yaml`
- [ ] `turbo.json` with build/test/lint pipeline
- [ ] Root `tsconfig.json` with path aliases for all packages
- [ ] ESLint + Prettier configuration
- [ ] `.gitignore`
- [ ] `@fungal/core` package with all type definitions
- [ ] `@fungal/core` utilities (ID generation, time, serialization, EventBus)
- [ ] Temporary scaffold for all 13 remaining packages
- [ ] Vitest configured and first tests passing
- [ ] `.github/workflows/ci.yml`
- [ ] `docs/` skeleton with architecture overview
- [ ] `pnpm build` completes without errors
- [ ] `pnpm test` passes with core tests
- [ ] Initial commit to GitHub

---

## 9. Build Command Verification

```bash
# From repository root, these should all succeed:
pnpm install
pnpm build    # Turborepo builds all packages in dependency order
pnpm test     # Runs all tests
pnpm lint     # Lints all packages
```

---

## 10. Risk Mitigation in Phase 0

| Risk | Mitigation |
|------|-----------|
| Path aliases don't resolve at runtime | Use tsup bundler — it resolves TS paths at build time; also test with `tsx` for dev |
| Monorepo tooling churn (Turborepo updates) | Pin turbo version in `package.json` |
| Package scaffolds too minimal to build | Each scaffold has a valid `package.json` with `tsup` config; build will produce valid ESM |
| Windows compatibility | Use `cross-platform` idioms (no `rm -rf` in scripts — use `rimraf` or `del-cli`) |
