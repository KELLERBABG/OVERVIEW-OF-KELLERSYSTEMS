# Phase 5 — Client, Demo & Polish (Weeks 18–20)

**Goal:** A polished browser client with network visualization, a demo app showcasing the full task lifecycle, hardening against attacks and edge cases, and comprehensive documentation.

---

## 1. Packages Involved

| Package | Responsibility |
|---------|---------------|
| `@fungal/client` | Browser client entry point — UI, background worker, wallet integration |
| `apps/demo` | Demo application with visual walkthrough |
| `apps/dev-tools` | Network visualizer, debug console, performance dashboard |
| (root) | Documentation, hardening, fuzz testing |

---

## 2. Browser Client (`packages/client`)

### 2.1 Purpose
The main browser entry point that users load. It initializes the mesh, connects to the signaling server, starts background workers, and provides a UI for monitoring and control.

### 2.2 File Structure

```
packages/client/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── public/
│   └── favicon.ico
├── src/
│   ├── main.ts              # Entry point — boots the client
│   ├── App.svelte            # Root Svelte component (or React/Vanilla)
│   ├── components/
│   │   ├── NetworkGraph.svelte   # Force-directed graph of peers
│   │   ├── NodeStatus.svelte     # Local node status panel
│   │   ├── TaskQueue.svelte      # Pending/completed tasks
│   │   ├── WalletPanel.svelte    # Token balance & stake controls
│   │   ├── TaskSubmitter.svelte  # Form to submit new tasks
│   │   └── LogViewer.svelte      # Real-time event log
│   ├── lib/
│   │   ├── boot.ts           # Bootstrap all subsystems
│   │   ├── worker-manager.ts # Spawn & manage background worker
│   │   ├── ui-state.ts       # Reactive UI state store
│   │   └── router.ts         # Simple hash-based router
│   ├── styles/
│   │   ├── global.css
│   │   └── theme.css
│   └── workers/
│       └── compute.worker.ts # Background Web Worker for execution
├── tests/
│   ├── boot.test.ts
│   └── ui-state.test.ts
```

### 2.3 Boot Sequence

```typescript
// src/lib/boot.ts
export async function bootClient(config: ClientConfig): Promise<ClientInstance> {
  // 1. Detect capabilities
  const adapter = new BrowserAdapter();
  const capabilities = adapter.runtime;

  // 2. Initialize storage
  const storage = await adapter.initStorage('fungal-swarm');

  // 3. Generate or load node identity
  let identity = await storage.get('identity') as NodeIdentity | null;
  if (!identity) {
    identity = {
      id: generateNodeId(),
      keyPair: await generateKeyPair(),
      createdAt: Date.now(),
    };
    await storage.set('identity', identity);
  }

  // 4. Connect to signaling server → form mesh
  const mesh = await adapter.connect({
    signalingUrl: config.signalingUrl,
    roomId: config.roomId,
    topology: config.nodeCount < 10 ? 'full' : 'sparse',
    ...config.mesh,
  });

  // 5. Start resource scoring
  const scorer = new ResourceScorer();
  const scoreBroadcaster = new ScoreBroadcaster(scorer, mesh);
  scoreBroadcaster.start();

  // 6. Initialize task distribution
  const nodeRegistry = new NodeRegistry(mesh);
  const distributor = new TaskDistributor(mesh, nodeRegistry, defaultDistributionConfig);

  // 7. Initialize consensus engine
  const consensus = new ConsensusEngine(mesh, nodeRegistry, mesh.eventBus);

  // 8. Start compute worker
  const workerManager = new WorkerManager();
  await workerManager.start();

  // 9. Connect wallet (if available)
  let walletClient: TokenClient | null = null;
  if (typeof window.ethereum !== 'undefined') {
    const provider = new ethers.BrowserProvider(window.ethereum);
    walletClient = new TokenClient(provider, config.tokenAddress, config.marketAddress);
  }

  // 10. Report ready
  return {
    identity,
    mesh,
    scorer,
    distributor,
    consensus,
    workerManager,
    walletClient,
    adapter,
  };
}
```

### 2.4 Compute Worker

```typescript
// src/workers/compute.worker.ts
/**
 * Background Web Worker that:
 * - Listens for work unit assignments from the main thread
 * - Executes them in the Wasm sandbox
 * - Generates zk-proofs if required
 * - Sends results back to the main thread
 */

import { WasmExecutor } from '@fungal/wasm-executor';
import { SnarkProver } from '@fungal/zk-integrity';

const executor = new WasmExecutor({ sandboxType: 'worker' });

self.onmessage = async (e: MessageEvent) => {
  const { type, payload } = e.data;

  switch (type) {
    case 'execute': {
      const { workUnit, provingKey } = payload;

      try {
        // Execute
        const result = await executor.execute(workUnit.code, workUnit.input);

        // Generate proof if required
        let proof = undefined;
        if (workUnit.requireProof && provingKey) {
          const prover = new SnarkProver(provingKey);
          const { proof: p, publicSignals } = await prover.prove(
            { inputHash: hashInput(workUnit.input), outputHash: hashOutput(result.output) },
            { input: workUnit.input, output: result.output },
          );
          proof = { proof: p, publicSignals };
        }

        // Send result back
        self.postMessage({
          type: 'result',
          payload: {
            workUnitId: workUnit.id,
            taskId: workUnit.taskId,
            output: result.output,
            executionTime: result.executionTime,
            memoryUsed: result.memoryUsed,
            proof,
          },
        });
      } catch (error) {
        self.postMessage({
          type: 'error',
          payload: {
            workUnitId: workUnit.id,
            taskId: workUnit.taskId,
            error: (error as Error).message,
          },
        });
      }
      break;
    }

    case 'terminate':
      executor.destroy();
      self.close();
      break;
  }
};
```

### 2.5 UI Components

#### Network Graph (force-directed visualization)

```svelte
<!-- src/components/NetworkGraph.svelte -->
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import * as d3 from 'd3-force';

  export let nodes: PeerNode[] = [];
  export let links: PeerLink[] = [];

  let canvas: HTMLCanvasElement;
  let simulation: d3.Simulation<PeerNode, PeerLink>;

  onMount(() => {
    const ctx = canvas.getContext('2d')!;

    simulation = d3.forceSimulation(nodes)
      .force('link', d3.forceLink(links).id(d => d.id))
      .force('charge', d3.forceManyBody().strength(-100))
      .force('center', d3.forceCenter(canvas.width / 2, canvas.height / 2))
      .on('tick', () => {
        ctx.clearRect(0, 0, canvas.width, canvas.height);

        // Draw links
        ctx.strokeStyle = '#666';
        ctx.lineWidth = 1;
        for (const link of links) {
          ctx.beginPath();
          ctx.moveTo(link.source.x, link.source.y);
          ctx.lineTo(link.target.x, link.target.y);
          ctx.stroke();
        }

        // Draw nodes
        for (const node of nodes) {
          ctx.beginPath();
          ctx.arc(node.x, node.y, 6, 0, Math.PI * 2);
          ctx.fillStyle = node.status === 'online' ? '#4ade80' : '#f87171';
          ctx.fill();
          ctx.fillStyle = '#fff';
          ctx.font = '10px monospace';
          ctx.fillText(node.label ?? node.id.slice(0, 6), node.x + 10, node.y + 4);
        }
      });
  });

  onDestroy(() => {
    simulation?.stop();
  });
</script>

<canvas bind:this={canvas} width={800} height={600} class="network-graph" />
```

#### Task Submitter

```typescript
// Task submitter logic (framework-agnostic)
export class TaskSubmitter {
  private distributor: TaskDistributor;

  async submitTaskFromUI(formData: TaskFormData): Promise<string> {
    const task: Task = {
      id: generateTaskId(),
      code: formData.code,
      input: JSON.parse(formData.input),
      deadline: Date.now() + formData.timeout * 1000,
      priority: formData.priority as TaskPriority,
      requireProof: formData.requireProof,
      requireEncryption: formData.requireEncryption,
      createdAt: Date.now(),
      ownerId: this.mesh.peerId,
    };

    // Quick validation
    if (!task.code) throw new Error('Code is required');
    if (!task.input) throw new Error('Input is required');

    // Submit
    const result = await this.distributor.submitTask(task);
    return result;
  }
}
```

### 2.6 UI State Management

```typescript
// src/lib/ui-state.ts
import { writable, derived } from 'svelte/store';

export const nodeState = writable({
  id: '',
  status: 'offline' as NodeStatus,
  score: 0,
  peers: [] as string[],
  tasksCompleted: 0,
  tokensEarned: 0n,
});

export const meshState = writable({
  peerCount: 0,
  saturation: 0,
  avgScore: 0,
  tasksQueued: 0,
  tasksInProgress: 0,
});

export const walletState = writable({
  connected: false,
  address: '',
  balance: 0n,
  staked: 0n,
});

export const taskLog = writable<TaskLogEntry[]>([]);

// Derived
export const isHealthy = derived(meshState, $m => $m.saturation < 0.8);
```

---

## 3. Demo App (`apps/demo`)

### 3.1 Purpose
A polished demo that showcases the full task lifecycle. Users can open multiple browser tabs to form a swarm, submit demo computations, and watch the results in real-time.

### 3.2 File Structure

```
apps/demo/
├── package.json
├── tsconfig.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.ts
│   ├── App.svelte
│   ├── pages/
│   │   ├── Home.svelte         # Landing page — "Join the Swarm"
│   │   ├── Dashboard.svelte    # Network overview & stats
│   │   ├── TaskLab.svelte      # Submit custom computations
│   │   ├── Demos.svelte        # Pre-built demos
│   │   └── Wallet.svelte       # Token management
│   ├── demos/
│   │   ├── prime-search.ts     # Distributed prime number search
│   │   ├── mandelbrot.ts       # Mandelbrot set rendering
│   │   ├── word-count.ts       # Distributed word count (map-reduce)
│   │   └── matrix-mul.ts       # Matrix multiplication benchmark
│   └── lib/
│       ├── demo-runner.ts      # Orchestrate demo execution
│       └── results-chart.ts    # Chart rendering
```

### 3.3 Demo: Prime Number Search

```typescript
// src/demos/prime-search.ts
export async function runPrimeSearchDemo(distributor: TaskDistributor, limit: number) {
  const task: Task = {
    id: generateTaskId(),
    code: `
      // Wasm or JS: Sieve of Eratosthenes for input range
      function sieve(limit) {
        const sieve = new Uint8Array(limit + 1);
        const primes = [];
        for (let i = 2; i <= limit; i++) {
          if (!sieve[i]) {
            primes.push(i);
            for (let j = i * i; j <= limit; j += i) sieve[j] = 1;
          }
        }
        return primes;
      }
      return sieve(input.limit);
    `,
    input: { limit },
    deadline: Date.now() + 300_000,
    priority: TaskPriority.MEDIUM,
    requireProof: false,
    requireEncryption: false,
    createdAt: Date.now(),
    ownerId: distributor.mesh.peerId,
  };

  const startTime = performance.now();
  const result = await distributor.submitTask(task);
  const elapsed = performance.now() - startTime;

  return {
    taskId: task.id,
    primes: result as number[],
    primeCount: (result as number[]).length,
    executionTime: elapsed,
    speedup: estimateSpeedup(elapsed, limit),
  };
}
```

### 3.4 Demo Runner

```typescript
// src/lib/demo-runner.ts
export class DemoRunner {
  private distributor: TaskDistributor;
  private results: DemoResult[] = [];

  async runDemo(name: string, params: Record<string, unknown>): Promise<DemoResult> {
    const demo = this.getDemo(name);
    if (!demo) throw new Error(`Unknown demo: ${name}`);

    const result = await demo(this.distributor, params);
    this.results.push(result);
    return result;
  }

  async runBenchmark(): Promise<BenchmarkReport> {
    // Run the same computation with varying node counts
    // and report speedup vs single-node
    const singleNodeTime = await this.measureSingleNode();
    const multiNodeTime = await this.measureMultiNode();

    return {
      singleNodeTime,
      multiNodeTime,
      speedup: singleNodeTime / multiNodeTime,
      nodeCount: this.mesh.getConnectedPeers().length + 1,
    };
  }
}
```

---

## 4. Dev Tools (`apps/dev-tools`)

### 4.1 Purpose
Network visualizer, debug console, and performance dashboard for developers.

### 4.2 Key Features

```
apps/dev-tools/
├── src/
│   ├── network-inspector.ts   # Real-time packet capture
│   ├── topology-viewer.ts     # Visualize mesh topology
│   ├── benchmark-suite.ts     # Run & compare benchmarks
│   ├── log-analyzer.ts        # Parse and analyze event logs
│   └── debug-console.ts       # Interactive REPL against the mesh
```

- **Packet Inspector:** Shows every message flowing through the node (type, size, latency)
- **Topology Viewer:** Real-time graph of the mesh with connection quality indicators
- **Benchmark Suite:** Run standardized benchmarks and compare results across nodes
- **Performance Dashboard:** Charts for task completion times, node scores, saturation history

---

## 5. Hardening & Edge Cases (Weeks 19–20)

### 5.1 Fuzz Testing for Wasm Sandbox

```typescript
// scripts/hardening/fuzz-wasm.ts
/**
 * Fuzz test the Wasm sandbox with malicious inputs:
 * - Infinite loops
 * - Memory exhaustion attacks
 * - Stack overflow
 * - Invalid bytecode
 * - Attempts to access DOM/Node APIs from Wasm
 */
async function fuzzWasmSandbox(): Promise<FuzzReport> {
  const executor = new WasmExecutor({ maxExecutionTime: 1000, maxMemory: 1024 * 1024 });
  const failures: FuzzFailure[] = [];

  const testCases = [
    // Infinite loop
    generateInfiniteLoop(),
    // Memory bomb
    generateMemoryBomb(100 * 1024 * 1024),
    // Stack overflow (deep recursion)
    generateDeepRecursion(100_000),
    // Invalid magic bytes
    generateInvalidWasm(),
    // DOM access attempt
    generateDomAccess(),
    // Network fetch attempt
    generateFetchAttempt(),
  ];

  for (const testCase of testCases) {
    try {
      await executor.execute(testCase.code, testCase.input);
      // If it didn't throw when it should have, that's a failure
      if (testCase.shouldFail) {
        failures.push({ testCase: testCase.name, issue: 'Should have been blocked' });
      }
    } catch (e) {
      if (!testCase.shouldFail) {
        failures.push({ testCase: testCase.name, issue: `Unexpected error: ${e}` });
      }
    }
  }

  executor.destroy();
  return { total: testCases.length, passed: testCases.length - failures.length, failures };
}
```

### 5.2 Network Partition / Re-merge Testing

```typescript
// scripts/hardening/network-partition.ts
/**
 * Simulate network partitions and verify the swarm recovers.
 *
 * Test scenarios:
 * 1. Split mesh into two groups, verify each group continues working
 * 2. Re-merge groups, verify state synchronization
 * 3. Drop 50% of nodes mid-computation, verify remaining nodes complete
 * 4. Drop leader (if any), verify no single point of failure
 */
async function testNetworkPartition(): Promise<void> {
  const mesh = await createTestMesh(10);

  // Simulate partition by closing connections between groups
  const groupA = mesh.nodes.slice(0, 5);
  const groupB = mesh.nodes.slice(5);
  await partition(mesh, groupA, groupB);

  // Verify each group functions independently
  const resultA = await submitTaskToGroup(groupA);
  expect(resultA).toBeDefined();

  // Re-merge
  await healPartition(mesh);

  // Verify full mesh communication restored
  await mesh.nodes[0].send(mesh.nodes[9].id, testMessage);
  const received = await waitForMessage(mesh.nodes[9]);
  expect(received).toBe(testMessage);
}
```

### 5.3 Sybil Attack Resistance

```typescript
// scripts/hardening/sybil-resistance.ts
/**
 * Sybil attack: one entity runs many nodes to gain disproportionate influence.
 *
 * Mitigations:
 * 1. Stake requirement — each node must stake tokens to participate
 * 2. Bio-Score bootstrapping — new nodes start with score=0, must earn trust
 * 3. Proof-of-Work handshake — nodes must solve a small PoW to join
 * 4. IP diversity scoring — nodes from same /24 range are clustered
 */
export class SybilResistance {
  private ipClusterMap = new Map<string, number>();

  /**
   * Propose an anti-Sybil score (0–1) for a node.
   * 1 = definitely unique, 0 = likely Sybil.
   */
  async evaluateNode(nodeId: NodeId, ip: string, stake: bigint): Promise<number> {
    const ipPrefix = ip.split('.').slice(0, 3).join('.');
    const clusterCount = this.ipClusterMap.get(ipPrefix) ?? 0;

    // Penalize nodes from same /24 subnet
    const ipDiversity = Math.max(0, 1 - clusterCount / 5);

    // Reward staked nodes
    const stakeScore = stake > 0n ? Math.min(Number(stake) / 1000, 1) : 0;

    return (ipDiversity * 0.6 + stakeScore * 0.4);
  }
}
```

### 5.4 Rate Limiting & DoS Protection

```typescript
// src/hardening/rate-limiter.ts
export class RateLimiter {
  private counters = new Map<string, { count: number; windowStart: number }>();

  constructor(
    private windowMs: number = 60_000,
    private maxRequests: number = 100,
  ) {}

  /**
   * Check if a request from this peer should be allowed.
   * Uses sliding window rate limiting.
   */
  allow(peerId: string): boolean {
    const now = Date.now();
    const entry = this.counters.get(peerId);

    if (!entry || now - entry.windowStart > this.windowMs) {
      this.counters.set(peerId, { count: 1, windowStart: now });
      return true;
    }

    if (entry.count >= this.maxRequests) {
      return false;
    }

    entry.count++;
    return true;
  }

  /** Get current rate stats for a peer */
  getStats(peerId: string): { count: number; remaining: number; resetAt: number } {
    const entry = this.counters.get(peerId);
    if (!entry) return { count: 0, remaining: this.maxRequests, resetAt: Date.now() + this.windowMs };
    return {
      count: entry.count,
      remaining: Math.max(0, this.maxRequests - entry.count),
      resetAt: entry.windowStart + this.windowMs,
    };
  }
}
```

---

## 6. Documentation

### 6.1 Architecture Decision Records (ADRs)

Create `docs/adr/` with records for key decisions:

```
docs/adr/
├── 001-use-typescript.md
├── 002-webrtc-over-websocket.md
├── 003-zk-snark-over-zk-stark.md
├── 004-poisson-redundancy.md
├── 005-entropy-scoring.md
├── 006-layer-2-blockchain.md
└── template.md
```

### 6.2 Protocol Specification

`docs/protocols.md` should specify:
- Binary message format (from Phase 1)
- Message types and their payloads
- Gossip propagation rules
- Task lifecycle state machine
- Consensus voting protocol
- Token transfer message format

### 6.3 API Reference

Auto-generated from TypeScript types using `typedoc`:

```bash
pnpm add -D typedoc
npx typedoc --out docs/api packages/*/src/index.ts
```

### 6.4 Deployment Guide

`docs/deployment.md` covers:
- Self-hosting the signaling server (Docker)
- Building the client for production
- Configuring STUN/TURN servers
- Deploying smart contracts to L2
- Environment variables reference

---

## 7. Testing Strategy for Phase 5

| Test type | What it covers | Tool |
|-----------|---------------|------|
| Unit | UI state, boot logic, rate limiter, Sybil resistance | Vitest |
| Component | Svelte components rendering with mock data | Vitest + jsdom |
| E2E | Full browser flow: join mesh → submit task → see result | Playwright |
| E2E | Multi-tab: 5 browser tabs form mesh, complete computation | Playwright |
| Fuzz | Wasm sandbox with malicious inputs | Custom script |
| Chaos | Network partition, node dropout, re-merge | Custom script |
| Performance | Task throughput, mesh latency, client startup time | Benchmark.js |

### E2E Test with Playwright

```typescript
// tests/e2e/swarm.e2e.ts
import { test, expect } from '@playwright/test';

test.describe('Fungal Compute Swarm E2E', () => {
  test('multiple browser tabs form a mesh and complete a task', async ({ browser }) => {
    // Open 3 browser tabs
    const page1 = await browser.newPage();
    const page2 = await browser.newPage();
    const page3 = await browser.newPage();

    // Navigate all to the demo app
    await page1.goto('http://localhost:3000');
    await page2.goto('http://localhost:3000');
    await page3.goto('http://localhost:3000');

    // Each tab joins the same room
    const roomId = 'e2e-test-room';
    await page1.fill('#room-input', roomId);
    await page1.click('#join-btn');
    await page2.fill('#room-input', roomId);
    await page2.click('#join-btn');
    await page3.fill('#room-input', roomId);
    await page3.click('#join-btn');

    // Wait for mesh to form
    await page1.waitForSelector('.peer-count:has-text("2")');

    // Submit a task from page 1
    await page1.click('#demo-prime-search');
    await page1.fill('#prime-limit', '100000');
    await page1.click('#submit-task');

    // Wait for result
    await page1.waitForSelector('.task-completed', { timeout: 60_000 });
    const resultText = await page1.textContent('.result-count');
    expect(resultText).toContain('9592'); // π(100000) = 9592

    // Verify other pages show network activity
    await page2.waitForSelector('.task-activity');
    const completedCount = await page2.textContent('.tasks-completed');
    expect(Number(completedCount)).toBeGreaterThan(0);
  });
});
```

---

## 8. Build & Deploy Pipeline

### 8.1 Docker Compose (Signaling Server)

```dockerfile
# Dockerfile (Root)
FROM node:20-alpine AS builder
WORKDIR /app
COPY . .
RUN pnpm install && pnpm build

FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app/packages/signaling-server/dist ./dist
COPY --from=builder /app/packages/signaling-server/package.json .
EXPOSE 8765
CMD ["node", "dist/index.js"]
```

```yaml
# docker-compose.yml
version: '3.8'
services:
  signaling-server:
    build: .
    ports:
      - "8765:8765"
    environment:
      - PORT=8765
      - MAX_PEERS_PER_ROOM=50
      - LOG_LEVEL=info
    restart: unless-stopped
```

### 8.2 Production Build Commands

```bash
# Build all packages
pnpm build

# Build client for production
cd packages/client
pnpm build --outDir dist

# Deploy smart contracts
cd packages/incentive-token
npx hardhat run scripts/deploy.ts --network arbitrum-sepolia

# Start signaling server
cd packages/signaling-server
node dist/index.js
```

---

## 9. Deliverables Checklist (End of Phase 5)

- [ ] Browser client with boot sequence, identity management, and subsystem initialization
- [ ] Background compute Web Worker for Wasm execution
- [ ] UI components: NetworkGraph, NodeStatus, TaskQueue, WalletPanel, TaskSubmitter
- [ ] Reactive UI state store for real-time updates
- [ ] Demo app with landing, dashboard, task lab, demos, and wallet pages
- [ ] Prime search demo (distributed prime sieve)
- [ ] Mandelbrot demo (distributed fractal rendering)
- [ ] Demo runner with benchmark/speedup reporting
- [ ] Dev tools: network inspector, topology viewer, benchmark suite
- [ ] Fuzz testing for Wasm sandbox (infinite loops, memory bombs, DOM access)
- [ ] Network partition/re-merge chaos tests
- [ ] Sybil attack resistance (stake requirement, IP clustering, PoW handshake)
- [ ] Rate limiting and DoS protection (sliding window)
- [ ] ADRs for key architectural decisions
- [ ] Protocol specification document
- [ ] API reference (auto-generated with Typedoc)
- [ ] Deployment guide (Docker, STUN/TURN, contracts)
- [ ] Playwright E2E test: multi-tab mesh → task → result
- [ ] Docker Compose setup for signaling server
- [ ] Production build pipeline
- [ ] `pnpm build` + `pnpm test` all passing

---

## 10. v1.0 Release Checklist

- [ ] 10+ browser nodes form a working mesh and exchange messages
- [ ] A non-trivial computation (1M primes) distributes and completes 2× faster than single-node
- [ ] zk-SNARK verification completes in under 1s per task
- [ ] Token rewards correctly reflect contributed compute
- [ ] Network recovers from >50% node dropout mid-computation
- [ ] E2E tests passing in CI
- [ ] Documentation complete and published
- [ ] Docker image published to registry
- [ ] Smart contracts deployed to testnet
- [ ] Demo accessible at a public URL
