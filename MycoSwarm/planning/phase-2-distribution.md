# Phase 2 — Resource Scoring & Task Distribution (Weeks 6–9)

**Goal:** Nodes benchmark their hardware to produce a Bio-Score, which is broadcast through the mesh. Task submitters split computations into work units using fractal algorithms and distribute them across the swarm with probabilistic redundancy.

---

## 1. Packages Involved

| Package | Responsibility |
|---------|---------------|
| `@fungal/wasm-executor` | Secure WebAssembly sandbox for executing work units in the browser |
| `@fungal/resource-scoring` | Benchmark suite and entropy-based Bio-Score calculation |
| `@fungal/task-distribution` | Fractal task splitting, assignment, and result reassembly |
| `@fungal/redundancy` | Poisson-distribution-based redundancy factor and fault tolerance |

---

## 2. Wasm Executor (`packages/wasm-executor`)

### 2.1 Purpose
Provides a secure sandbox to load WebAssembly modules and execute work units. Must be isolated from the DOM, have limited memory and CPU budgets, and support both synchronous and asynchronous execution.

### 2.2 File Structure

```
packages/wasm-executor/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── executor.ts          # Main executor — load, run, teardown
│   ├── sandbox.ts           # Sandbox creation (iframe/worker)
│   ├── memory-manager.ts    # Memory budget tracking
│   ├── timeout.ts           # Execution timeout with cancellation
│   ├── fallback.ts          # JS fallback when Wasm is unavailable
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── executor.test.ts
│   ├── sandbox.test.ts
│   ├── memory-manager.test.ts
│   └── fallback.test.ts
├── fixtures/
│   ├── add.wasm             # Simple test Wasm module
│   └── matrix-mul.wasm      # Compute-heavy test module
```

### 2.3 Core Implementation

```typescript
// src/types.ts
export interface ExecutorConfig {
  /** Max memory (bytes) the sandbox can use */
  maxMemory: number;            // default: 64MB
  /** Max execution time per work unit (ms) */
  maxExecutionTime: number;     // default: 30_000 (30s)
  /** Max stack depth for JS fallback */
  maxStackDepth: number;
  /** Sandbox type: 'worker' | 'iframe' | 'same-thread' */
  sandboxType: 'worker' | 'iframe' | 'same-thread';
}

export interface ExecutionResult {
  output: unknown;
  executionTime: number;
  memoryUsed: number;
  instructionsExecuted?: number;
}

export interface ExecutionError {
  code: 'TIMEOUT' | 'OOM' | 'PANIC' | 'INVALID_CODE' | 'SANDBOX_CRASH';
  message: string;
}
```

#### `src/executor.ts`
```typescript
export class WasmExecutor {
  private config: ExecutorConfig;
  private sandbox: Sandbox;

  constructor(config: Partial<ExecutorConfig> = {}) {
    this.config = { ...defaultConfig, ...config };
    this.sandbox = new Sandbox(this.config);
  }

  /**
   * Execute a work unit.
   * code: Wasm bytecode (Uint8Array) or JS function string.
   * input: Structured-clone-compatible input.
   */
  async execute(code: Uint8Array | string, input: unknown): Promise<ExecutionResult> {
    const startTime = performance.now();

    // Determine if Wasm or JS
    if (code instanceof Uint8Array) {
      return this.executeWasm(code, input, startTime);
    } else {
      return this.executeJsFallback(code, input, startTime);
    }
  }

  private async executeWasm(bytecode: Uint8Array, input: unknown, startTime: number): Promise<ExecutionResult> {
    const memoryBudget = this.config.maxMemory;

    // Load Wasm in sandbox
    const instance = await this.sandbox.compile(bytecode, memoryBudget);

    // Set up timeout
    const timeoutSignal = createTimeoutSignal(this.config.maxExecutionTime);

    // Execute main export with input
    const output = await this.sandbox.run(instance, 'main', [serialize(input)], timeoutSignal);

    const executionTime = performance.now() - startTime;
    const memoryUsed = instance.exports.memory?.buffer.byteLength ?? 0;

    return { output: deserialize(output), executionTime, memoryUsed };
  }

  private async executeJsFallback(code: string, input: unknown, startTime: number): Promise<ExecutionResult> {
    // Wrap in a sandboxed function with limited scope
    const fn = new Function('input', code);
    const output = await fn(input);
    return {
      output,
      executionTime: performance.now() - startTime,
      memoryUsed: 0,
    };
  }

  destroy(): void {
    this.sandbox.terminate();
  }
}
```

#### `src/sandbox.ts`
```typescript
export class Sandbox {
  private worker: Worker | null = null;
  private iframe: HTMLIFrameElement | null = null;

  constructor(private config: ExecutorConfig) {
    if (config.sandboxType === 'worker') {
      this.worker = this.createSandboxedWorker();
    } else if (config.sandboxType === 'iframe') {
      this.iframe = this.createSandboxedIframe();
    }
  }

  private createSandboxedWorker(): Worker {
    // Inline worker code that has no access to DOM
    const blob = new Blob([`
      self.onmessage = async (e) => {
        const { bytecode, input, maxMemory } = e.data;
        try {
          const importObj = {
            env: {
              memory: new WebAssembly.Memory({ initial: maxMemory >> 16, maximum: maxMemory >> 16 }),
              // Only expose safe host functions
              abort: () => { throw new Error('abort'); },
            }
          };
          const module = await WebAssembly.instantiate(bytecode, importObj);
          const result = module.instance.exports.main(input);
          self.postMessage({ type: 'result', data: result });
        } catch (err) {
          self.postMessage({ type: 'error', data: err.message });
        }
      };
    `], { type: 'application/javascript' });
    return new Worker(URL.createObjectURL(blob));
  }

  // For environments without Worker support, use an iframe with sandbox attr
  private createSandboxedIframe(): HTMLIFrameElement {
    const iframe = document.createElement('iframe');
    iframe.sandbox = 'allow-scripts'; // No allow-same-origin
    iframe.style.display = 'none';
    document.body.appendChild(iframe);
    return iframe;
  }

  async compile(bytecode: Uint8Array, memoryBudget: number): Promise<WebAssembly.Instance> {
    if (this.worker) {
      return new Promise((resolve, reject) => {
        this.worker!.onmessage = (e) => {
          if (e.data.type === 'result') resolve(e.data.instance);
          else reject(new Error(e.data.data));
        };
        this.worker!.postMessage({ bytecode, memoryBudget });
      });
    }
    // Fallback: direct compilation
    const module = await WebAssembly.compile(bytecode);
    return WebAssembly.instantiate(module, {
      env: { memory: new WebAssembly.Memory({ initial: memoryBudget >> 16 }) },
    });
  }

  terminate(): void {
    this.worker?.terminate();
    this.iframe?.remove();
  }
}
```

#### `src/timeout.ts`
```typescript
export function createTimeoutSignal(ms: number): { aborted: boolean; abort: () => void } {
  const signal = { aborted: false, abort: () => {} };
  const timer = setTimeout(() => { signal.aborted = true; }, ms);
  signal.abort = () => { clearTimeout(timer); signal.aborted = true; };
  return signal;
}
```

### 2.4 Testing

```typescript
describe('WasmExecutor', () => {
  it('should execute a simple add.wasm module', async () => {
    const executor = new WasmExecutor({ sandboxType: 'same-thread' });
    const addWasm = await readFixture('add.wasm');
    const result = await executor.execute(addWasm, { a: 5, b: 3 });
    expect(result.output).toBe(8);
    expect(result.executionTime).toBeGreaterThan(0);
    executor.destroy();
  });

  it('should timeout long-running code', async () => {
    const executor = new WasmExecutor({ maxExecutionTime: 100 });
    const infiniteLoop = createInfiniteLoopWasm();
    await expect(executor.execute(infiniteLoop, {})).rejects.toMatch('TIMEOUT');
    executor.destroy();
  });

  it('should enforce memory budget', async () => {
    const executor = new WasmExecutor({ maxMemory: 1024 * 1024 }); // 1MB
    const memoryHog = createMemoryHogWasm(10 * 1024 * 1024); // tries to use 10MB
    await expect(executor.execute(memoryHog, {})).rejects.toMatch('OOM');
    executor.destroy();
  });

  it('should fall back to JS execution', async () => {
    const executor = new WasmExecutor();
    const jsCode = 'return input.x * 2;';
    const result = await executor.execute(jsCode, { x: 21 });
    expect(result.output).toBe(42);
    executor.destroy();
  });
});
```

---

## 3. Resource Scoring (`packages/resource-scoring`)

### 3.1 Purpose
Each node benchmarks its own hardware (CPU, RAM, network) and computes a Bio-Score (0–100) representing its compute capacity. Scores are broadcast periodically so the task distributor can make informed work assignments.

### 3.2 File Structure

```
packages/resource-scoring/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── scorer.ts            # Main Scorer — orchestrates benchmarks
│   ├── benchmarks/
│   │   ├── cpu.ts           # CPU benchmark (ALU + FP ops)
│   │   ├── memory.ts        # RAM latency/bandwidth benchmark
│   │   ├── network.ts       # Network throughput benchmark
│   │   └── storage.ts       # Cache/storage benchmark (IndexedDB)
│   ├── entropy-score.ts     # Entropy-based score formula
│   ├── broadcast.ts         # Score broadcast scheduler
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── scorer.test.ts
│   ├── cpu.test.ts
│   ├── memory.test.ts
│   └── entropy-score.test.ts
```

### 3.3 Benchmark Implementations

```typescript
// src/benchmarks/cpu.ts
export interface CpuBenchResult {
  /** Million integer operations per second */
  aluOpsPerSecond: number;
  /** Million float operations per second */
  fpOpsPerSecond: number;
  /** Time to calculate 1M primes (ms) */
  primeBenchMs: number;
}

export async function benchmarkCpu(): Promise<CpuBenchResult> {
  // ALU benchmark: integer arithmetic
  const aluStart = performance.now();
  let aluCount = 0;
  while (performance.now() - aluStart < 500) {
    for (let i = 0; i < 10000; i++) {
      let x = i * 3 + 7 - 2 / 1;
      x = (x << 2) | (x >> 1);
      void (x ^ 0xFF);
    }
    aluCount += 10000;
  }
  const aluElapsed = (performance.now() - aluStart) / 1000;
  const aluOpsPerSecond = (aluCount / aluElapsed) / 1_000_000;

  // FP benchmark: float operations
  const fpStart = performance.now();
  let fpCount = 0;
  while (performance.now() - fpStart < 500) {
    for (let i = 0; i < 10000; i++) {
      let x = Math.sqrt(i * 1.0);
      x = Math.sin(x) * Math.cos(x);
      x = Math.exp(Math.log(x + 1));
      void (x * 0.5);
    }
    fpCount += 10000;
  }
  const fpElapsed = (performance.now() - fpStart) / 1000;
  const fpOpsPerSecond = (fpCount / fpElapsed) / 1_000_000;

  // Prime sieve benchmark (1M primes)
  const primeStart = performance.now();
  const limit = 15_000_000; // ~1M primes
  const sieve = new Uint8Array(limit);
  let primeCount = 0;
  for (let i = 2; i < limit; i++) {
    if (!sieve[i]) {
      primeCount++;
      for (let j = i * i; j < limit; j += i) sieve[j] = 1;
    }
    if (primeCount >= 1_000_000) break;
  }
  const primeBenchMs = performance.now() - primeStart;

  return { aluOpsPerSecond, fpOpsPerSecond, primeBenchMs };
}
```

```typescript
// src/benchmarks/memory.ts
export interface MemoryBenchResult {
  /** Read latency (ns) */
  readLatencyNs: number;
  /** Write bandwidth (MB/s) */
  writeBandwidthMBs: number;
}

export async function benchmarkMemory(): Promise<MemoryBenchResult> {
  const size = 100_000_000; // 100MB buffer
  const buf = new Uint8Array(size);

  // Write bandwidth: fill buffer
  const writeStart = performance.now();
  for (let i = 0; i < size; i += 64) {
    buf[i] = 0xFF;
  }
  const writeElapsed = performance.now() - writeStart;
  const writeBandwidthMBs = (size / writeElapsed) / 1000;

  // Read latency: random access (pointer chase)
  const indices = new Uint32Array(10000);
  for (let i = 0; i < indices.length; i++) {
    indices[i] = Math.floor(Math.random() * (size - 8));
  }
  const readStart = performance.now();
  let sum = 0;
  for (const idx of indices) {
    sum += buf[idx];
  }
  const readElapsed = performance.now() - readStart;
  const readLatencyNs = (readElapsed * 1_000_000) / indices.length;

  return { readLatencyNs, writeBandwidthMBs };
}
```

### 3.4 Entropy-Based Score Formula

```typescript
// src/entropy-score.ts
export interface BioScoreInput {
  cpu: CpuBenchResult;
  memory: MemoryBenchResult;
  network?: NetworkBenchResult;
  /** Upvote count from other nodes (reputation) */
  upvotes: number;
  /** Session duration so far (ms) */
  sessionAge: number;
}

/**
 * Bio-Score (0–100) using an entropy-weighted formula.
 *
 * Higher entropy = more uncertainty = more "growth potential".
 * A balanced machine (good CPU + good RAM + long session) scores higher
 * than one that's imbalanced (great CPU but terrible RAM).
 *
 * Formula:
 *   normalized = [cpuScore, memScore, netScore, stabilityScore]
 *   entropy = -Σ(p_i * log₂(p_i))  // Shannon entropy
 *   bioScore = entropy * 25         // Scale to 0-100
 */
export function calculateBioScore(input: BioScoreInput): number {
  // Normalize each metric to 0–1
  const cpuScore = normalizeCpu(input.cpu);
  const memScore = normalizeMemory(input.memory);
  const netScore = input.network ? normalizeNetwork(input.network) : 0.5;
  const stabilityScore = Math.min(input.sessionAge / 300_000, 1); // 5 min to full

  const components = [cpuScore, memScore, netScore, stabilityScore];
  const total = components.reduce((a, b) => a + b, 0);

  // Shannon entropy
  let entropy = 0;
  for (const c of components) {
    if (c === 0) continue;
    const p = c / total;
    entropy -= p * Math.log2(p);
  }

  // Normalize entropy to 0–100 (max entropy for 4 components is log₂(4) = 2)
  const maxEntropy = Math.log2(components.length);
  const normalizedEntropy = entropy / maxEntropy;

  // Bio-Score
  return Math.round(normalizedEntropy * 100);
}

function normalizeCpu(cpu: CpuBenchResult): number {
  // Reference: ~1000 MOPS = 0.5 score
  const aluScore = Math.min(cpu.aluOpsPerSecond / 2000, 1);
  const fpScore = Math.min(cpu.fpOpsPerSecond / 500, 1);
  return (aluScore * 0.6 + fpScore * 0.4);
}

function normalizeMemory(mem: MemoryBenchResult): number {
  // <50ns = 1.0, >500ns = 0.0
  const latencyScore = Math.max(0, 1 - (mem.readLatencyNs - 10) / 490);
  return latencyScore;
}
```

### 3.5 Broadcast Scheduler

```typescript
// src/broadcast.ts
export class ScoreBroadcaster {
  private scorer: ResourceScorer;
  private mesh: MeshManager;
  private intervalId: ReturnType<typeof setInterval> | null = null;

  constructor(scorer: ResourceScorer, mesh: MeshManager) {
    this.scorer = scorer;
    this.mesh = mesh;
  }

  start(intervalMs: number = 30_000): void {
    // Broadcast immediately, then every interval
    this.broadcast();
    this.intervalId = setInterval(() => this.broadcast(), intervalMs);
  }

  stop(): void {
    if (this.intervalId) clearInterval(this.intervalId);
  }

  private async broadcast(): Promise<void> {
    const score = await this.scorer.getScore();
    const payload = serialize({
      nodeId: this.mesh.peerId,
      score,
      cpuInfo: score,
      timestamp: Date.now(),
    });
    this.mesh.broadcast(MessageType.SCORE_BROADCAST, payload);
  }
}
```

### 3.6 Tests

```typescript
describe('ResourceScorer', () => {
  it('should run benchmarks and return a score', async () => {
    const scorer = new ResourceScorer();
    const score = await scorer.getScore();
    expect(score).toBeGreaterThanOrEqual(0);
    expect(score).toBeLessThanOrEqual(100);
  });
});

describe('EntropyScore', () => {
  it('should calculate a score between 0–100', () => {
    const input: BioScoreInput = {
      cpu: { aluOpsPerSecond: 1000, fpOpsPerSecond: 300, primeBenchMs: 500 },
      memory: { readLatencyNs: 80, writeBandwidthMBs: 5000 },
      upvotes: 10,
      sessionAge: 600_000,
    };
    const score = calculateBioScore(input);
    expect(score).toBeGreaterThanOrEqual(0);
    expect(score).toBeLessThanOrEqual(100);
  });

  it('should score a balanced machine higher than an imbalanced one', () => {
    const balanced = calculateBioScore(balancedInput);
    const imbalanced = calculateBioScore(imbalancedInput);
    expect(balanced).toBeGreaterThan(imbalanced);
  });
});
```

---

## 4. Task Distribution (`packages/task-distribution`)

### 4.1 Purpose
Breaks a large computation into smaller work units using fractal (recursive divide-and-conquer) algorithms, assigns them to nodes based on their Bio-Score, and reassembles results.

### 4.2 File Structure

```
packages/task-distribution/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── distributor.ts       # TaskDistributor — main orchestrator
│   ├── splitter.ts          # Fractal task splitting strategies
│   ├── scheduler.ts         # Work unit → node assignment
│   ├── reassembler.ts       # Result collection and reassembly
│   ├── progress-tracker.ts  # Track completion, detect stalls
│   ├── queue.ts             # Priority queue for pending tasks
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── distributor.test.ts
│   ├── splitter.test.ts
│   ├── scheduler.test.ts
│   └── reassembler.test.ts
```

### 4.3 Core Implementation

```typescript
// src/types.ts
export interface DistributionConfig {
  /** Min work unit granularity (bytes of input) */
  minUnitSize: number;
  /** Target work unit count (adjusts split depth) */
  targetUnitCount: number;
  /** Default redundancy factor */
  defaultRedundancy: number;
  /** Max work units per task */
  maxUnits: number;
}

export interface DistributionPlan {
  taskId: TaskId;
  units: WorkUnit[];
  metadata: {
    totalUnits: number;
    estimatedTotalWork: number;
    splitStrategy: string;
    depth: number;
  };
}
```

#### `src/splitter.ts` — Fractal Splitting
```typescript
export type SplitStrategy = 'uniform' | 'recursive-bisect' | 'data-parallel';

export class TaskSplitter {
  /**
   * Fractal split: recursively divide input until units are at minUnitSize.
   *
   * For uniform data (arrays, matrices):
   *   - Bisect input into halves → quarters → eighths → ...
   *
   * For map-style tasks (same op, different data):
   *   - Split data into N equal chunks based on targetUnitCount.
   *
   * For recursive algorithms (merge sort, FFT):
   *   - Each split represents a sub-problem with its own range.
   */
  split(task: Task, config: DistributionConfig): DistributionPlan {
    switch (task.inputType) {
      case 'array':
        return this.splitArray(task, config);
      case 'range':
        return this.splitRange(task, config);
      case 'recursive':
        return this.splitRecursive(task, config, 0);
      default:
        return this.splitSingle(task); // indivisible
    }
  }

  private splitArray(task: Task, config: DistributionConfig): DistributionPlan {
    const input = task.input as unknown[];
    const unitSize = Math.max(config.minUnitSize, Math.ceil(input.length / config.targetUnitCount));
    const units: WorkUnit[] = [];

    for (let i = 0; i < input.length; i += unitSize) {
      const end = Math.min(i + unitSize, input.length);
      units.push({
        id: generateWorkUnitId(),
        taskId: task.id,
        input: input.slice(i, end),
        range: [i, end],
        estimatedSize: estimateSize(input.slice(i, end)),
        deadline: task.deadline,
        priority: task.priority,
        redundancyFactor: config.defaultRedundancy,
      });
    }

    return {
      taskId: task.id,
      units,
      metadata: {
        totalUnits: units.length,
        estimatedTotalWork: units.reduce((s, u) => s + u.estimatedSize, 0),
        splitStrategy: 'uniform',
        depth: Math.ceil(Math.log2(units.length)),
      },
    };
  }

  private splitRecursive(task: Task, config: DistributionConfig, depth: number): DistributionPlan {
    // Recursive bisection — returns leaf units at max depth
    if (depth >= config.maxRecursionDepth || this.isAtomic(task.input)) {
      return { taskId: task.id, units: [this.toWorkUnit(task)], metadata: { ... } };
    }
    const [left, right] = this.bisectInput(task.input);
    const leftPlan = this.splitRecursive({ ...task, input: left }, config, depth + 1);
    const rightPlan = this.splitRecursive({ ...task, input: right }, config, depth + 1);
    return { taskId: task.id, units: [...leftPlan.units, ...rightPlan.units], metadata: { ... } };
  }
}
```

#### `src/scheduler.ts` — Node Assignment
```typescript
export class WorkScheduler {
  /**
   * Assign work units to nodes using score-weighted selection.
   *
   * Algorithm:
   * 1. Get list of available nodes with their Bio-Scores.
   * 2. Create a weighted roulette wheel (higher score = more likely selected).
   * 3. For each work unit (sorted by priority then deadline):
   *    a. Select N nodes (where N = redundancy factor) using roulette.
   *    b. Mark them as assigned.
   *    c. Send TASK_ASSIGNMENT message to each node.
   * 4. Track pending assignments with deadlines.
   */
  async assignUnits(
    units: WorkUnit[],
    availableNodes: Map<NodeId, number>, // nodeId → bioScore
    mesh: MeshManager,
  ): Promise<Map<WorkUnitId, NodeId[]>> {
    const assignments = new Map<WorkUnitId, NodeId[]>();
    const sorted = [...units].sort(byPriorityAndDeadline);

    for (const unit of sorted) {
      const selected = this.selectNodes(availableNodes, unit.redundancyFactor);
      assignments.set(unit.id, selected);

      for (const nodeId of selected) {
        const envelope = createEnvelope(MessageType.TASK_ASSIGNMENT, mesh.peerId, nodeId, {
          workUnit: unit,
        });
        mesh.send(nodeId, serialize(envelope));
      }
    }

    return assignments;
  }

  private selectNodes(availableNodes: Map<NodeId, number>, count: number): NodeId[] {
    // Weighted random selection without replacement
    const entries = Array.from(availableNodes.entries());
    const totalScore = entries.reduce((s, [, score]) => s + score, 0);
    const selected: NodeId[] = [];

    while (selected.length < count && entries.length > 0) {
      let r = Math.random() * totalScore;
      for (let i = 0; i < entries.length; i++) {
        r -= entries[i][1];
        if (r <= 0) {
          selected.push(entries[i][0]);
          entries.splice(i, 1);
          break;
        }
      }
    }

    return selected;
  }
}
```

#### `src/reassembler.ts` — Result Collection
```typescript
export class ResultReassembler {
  private results = new Map<TaskId, Map<WorkUnitId, TaskResult[]>>();
  private onComplete: ((taskId: TaskId, output: unknown) => void) | null = null;

  /**
   * Handle an incoming TaskResult from a worker node.
   * Collects results per work unit (with redundancy, take first valid).
   * When all units for a task have results, reassembles.
   */
  receiveResult(result: TaskResult): void {
    const taskResults = this.results.get(result.taskId);
    if (!taskResults) return; // unknown/expired task

    const unitResults = taskResults.get(result.workUnitId) ?? [];
    unitResults.push(result);
    taskResults.set(result.workUnitId, unitResults);

    // Check if all units have at least 1 valid result
    if (this.isTaskComplete(result.taskId)) {
      const output = this.reassemble(result.taskId);
      this.onComplete?.(result.taskId, output);
      this.cleanup(result.taskId);
    }
  }

  private reassemble(taskId: TaskId): unknown {
    const taskResults = this.results.get(taskId)!;
    const sortedUnits = Array.from(taskResults.entries())
      .sort(([a], [b]) => compareWorkUnitIds(a, b));

    // For array-based tasks, concatenate in order
    const outputs = sortedUnits.map(([, results]) => results[0].output);
    if (Array.isArray(outputs[0])) {
      return (outputs as unknown[][]).flat();
    }
    // For aggregatable results (sum, count, etc.), reduce
    if (typeof outputs[0] === 'number') {
      return (outputs as number[]).reduce((a, b) => a + b, 0);
    }
    // Default: return as map
    return Object.fromEntries(
      sortedUnits.map(([unitId, results]) => [unitId, results[0].output])
    );
  }
}
```

#### `src/distributor.ts` — Main Orchestrator
```typescript
export class TaskDistributor {
  private splitter: TaskSplitter;
  private scheduler: WorkScheduler;
  private reassembler: ResultReassembler;
  private redundancy: RedundancyManager;

  constructor(
    private mesh: MeshManager,
    private nodeRegistry: NodeRegistry,
    config: DistributionConfig,
  ) { ... }

  /**
   * Submit a task for distributed execution.
   * Returns a promise that resolves with the final result.
   */
  async submitTask(task: Task): Promise<unknown> {
    // 1. Split
    const plan = this.splitter.split(task, this.config);

    // 2. Determine redundancy factors
    for (const unit of plan.units) {
      unit.redundancyFactor = this.redundancy.calculateFactor(unit, this.nodeRegistry);
    }

    // 3. Assign to nodes
    const availableNodes = this.nodeRegistry.getAvailableNodes();
    await this.scheduler.assignUnits(plan.units, availableNodes, this.mesh);

    // 4. Wait for reassembly
    return new Promise((resolve, reject) => {
      this.reassembler.onComplete = (taskId, output) => resolve(output);
      this.reassembler.onTimeout = (taskId) => reject(new Error(`Task ${taskId} timed out`));
    });
  }
}
```

---

## 5. Redundancy & Fault Tolerance (`packages/redundancy`)

### 5.1 Purpose
Uses the Poisson distribution to calculate how many redundant copies of each work unit to dispatch, based on historical node dropout rates and the criticality of the task.

### 5.2 File Structure

```
packages/redundancy/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── poisson.ts           # Poisson distribution calculator
│   ├── redundancy-manager.ts # Determines redundancy factor
│   ├── timeout-manager.ts   # Detect stalled work units
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── poisson.test.ts
│   └── redundancy-manager.test.ts
```

### 5.3 Poisson Distribution

```typescript
// src/poisson.ts
/**
 * Poisson probability mass function: P(k; λ) = (λ^k * e^-λ) / k!
 *
 * Used to model the probability that exactly k nodes will stay online
 * given an average dropout rate λ.
 */
export function poissonPMF(k: number, lambda: number): number {
  return (Math.pow(lambda, k) * Math.exp(-lambda)) / factorial(k);
}

/**
 * Compute the minimum redundancy factor `r` such that the probability
 * of at least one node completing the work is ≥ targetReliability.
 *
 * λ (lambda) = expected number of nodes that will stay online
 *            = total assigned * (1 - historical_dropout_rate)
 *
 * We want P(at least 1 success) ≥ targetReliability
 *   = 1 - P(0 successes) ≥ targetReliability
 *   = 1 - e^(-λ) ≥ targetReliability
 *   = λ ≥ -ln(1 - targetReliability)
 *   = r * (1 - dropoutRate) ≥ -ln(1 - targetReliability)
 *   = r ≥ -ln(1 - targetReliability) / (1 - dropoutRate)
 */
export function minRedundancyFactor(
  historicalDropoutRate: number,
  targetReliability: number,
): number {
  if (targetReliability >= 1) return Infinity;
  const lambda = -Math.log(1 - targetReliability);
  const effectiveRate = 1 - historicalDropoutRate;
  if (effectiveRate <= 0) return Infinity;
  return Math.ceil(lambda / effectiveRate);
}
```

```typescript
// src/redundancy-manager.ts
export class RedundancyManager {
  private dropoutHistory: number[] = [];
  private windowSize = 100; // track last 100 assignments

  /**
   * Record a node dropping out mid-task.
   */
  recordDropout(nodeId: NodeId): void {
    this.dropoutHistory.push(1);
    if (this.dropoutHistory.length > this.windowSize) {
      this.dropoutHistory.shift();
    }
  }

  /**
   * Record a successful completion.
   */
  recordSuccess(nodeId: NodeId): void {
    this.dropoutHistory.push(0);
    if (this.dropoutHistory.length > this.windowSize) {
      this.dropoutHistory.shift();
    }
  }

  /**
   * Calculate current dropout rate.
   */
  getDropoutRate(): number {
    if (this.dropoutHistory.length === 0) return 0.3; // default 30%
    return this.dropoutHistory.reduce((a, b) => a + b, 0) / this.dropoutHistory.length;
  }

  /**
   * Determine redundancy factor for a work unit.
   *
   * Factors considered:
   * - Historical dropout rate of the network
   * - Priority of the task (higher priority = more redundancy)
   * - Node's Bio-Score stability (new nodes get more redundancy)
   */
  calculateFactor(unit: WorkUnit, nodeRegistry: NodeRegistry): number {
    const dropoutRate = this.getDropoutRate();
    const baseReliability = 0.95; // 95% confidence

    // Boost reliability for high-priority tasks
    const reliabilityMap = {
      [TaskPriority.LOW]: 0.90,
      [TaskPriority.MEDIUM]: 0.95,
      [TaskPriority.HIGH]: 0.99,
      [TaskPriority.CRITICAL]: 0.999,
    };
    const targetReliability = reliabilityMap[unit.priority] ?? baseReliability;

    const factor = minRedundancyFactor(dropoutRate, targetReliability);

    // Cap at reasonable number
    return Math.min(factor, 5);
  }
}
```

### 5.4 Timeout Manager

```typescript
// src/timeout-manager.ts
export class TimeoutManager {
  private timers = new Map<WorkUnitId, ReturnType<typeof setTimeout>>();
  private onTimeout: ((unitId: WorkUnitId) => void) | null = null;

  /**
   * Start tracking a work unit deadline.
   * If the unit expires, trigger re-dispatch.
   */
  startTracking(unit: WorkUnit): void {
    const remaining = unit.deadline - Date.now();
    if (remaining <= 0) {
      this.onTimeout?.(unit.id);
      return;
    }
    const timer = setTimeout(() => {
      this.onTimeout?.(unit.id);
      this.timers.delete(unit.id);
    }, remaining);
    this.timers.set(unit.id, timer);
  }

  /** Stop tracking (unit completed) */
  stopTracking(unitId: WorkUnitId): void {
    const timer = this.timers.get(unitId);
    if (timer) clearTimeout(timer);
    this.timers.delete(unitId);
  }
}
```

### 5.5 Tests

```typescript
describe('Poisson redundancy', () => {
  it('should calculate correct PMF values', () => {
    // λ=1, k=0: P(0;1) = e^-1 ≈ 0.3679
    expect(poissonPMF(0, 1)).toBeCloseTo(0.3679, 3);
    // λ=1, k=1: P(1;1) = e^-1 ≈ 0.3679
    expect(poissonPMF(1, 1)).toBeCloseTo(0.3679, 3);
  });

  it('should compute minimum redundancy factor', () => {
    // 30% dropout, 95% reliability target
    const factor = minRedundancyFactor(0.3, 0.95);
    // r ≥ -ln(0.05) / 0.7 = 2.996 / 0.7 ≈ 4.28 → ceil = 5
    expect(factor).toBe(5);
  });

  it('should handle 0% dropout rate efficiently', () => {
    const factor = minRedundancyFactor(0, 0.95);
    expect(factor).toBe(1); // no redundancy needed
  });
});

describe('RedundancyManager', () => {
  it('should start with default dropout rate', () => {
    const mgr = new RedundancyManager();
    expect(mgr.getDropoutRate()).toBe(0.3);
  });

  it('should adapt dropout rate based on history', () => {
    const mgr = new RedundancyManager();
    mgr.recordSuccess('node1');
    mgr.recordSuccess('node2');
    mgr.recordSuccess('node3');
    mgr.recordDropout('node4');
    expect(mgr.getDropoutRate()).toBeCloseTo(0.25);
  });

  it('should increase redundancy for critical tasks', () => {
    const mgr = new RedundancyManager();
    const lowUnit = createTestUnit(TaskPriority.LOW);
    const criticalUnit = createTestUnit(TaskPriority.CRITICAL);
    const lowFactor = mgr.calculateFactor(lowUnit, mockRegistry);
    const criticalFactor = mgr.calculateFactor(criticalUnit, mockRegistry);
    expect(criticalFactor).toBeGreaterThanOrEqual(lowFactor);
  });
});
```

---

## 6. Integration: Full Distribution Pipeline

```typescript
describe('Full distribution pipeline', () => {
  it('should split, distribute, execute, and reassemble a task', async () => {
    // Setup: 3 mesh nodes with known scores
    const mesh = await createTestMesh(3);
    const registry = new NodeRegistry(mesh);
    registry.updateScore('node1', 80);
    registry.updateScore('node2', 60);
    registry.updateScore('node3', 40);

    const distributor = new TaskDistributor(mesh, registry, defaultConfig);

    // Submit task: sum of numbers 1–1000
    const task: Task = {
      id: generateTaskId(),
      code: 'return input.reduce((a, b) => a + b, 0);',
      input: Array.from({ length: 1000 }, (_, i) => i + 1),
      deadline: Date.now() + 60_000,
      priority: TaskPriority.MEDIUM,
      requireProof: false,
      requireEncryption: false,
      createdAt: Date.now(),
      ownerId: mesh.peerId,
    };

    const result = await distributor.submitTask(task);

    // Expected: sum(1..1000) = 500500
    expect(result).toBe(500500);
  });
});
```

---

## 7. Development Workflow for Phase 2

### Step-by-step build order

| Week | Focus | Components |
|------|-------|-----------|
| **6** | Wasm Executor | Sandbox, timeout, memory caps, JS fallback |
| **7** | Resource Scoring | CPU + RAM benchmarks, entropy formula, score broadcast |
| **8** | Task Distribution | Fractal splitter, scheduler, reassembler |
| **9** | Redundancy + Integration | Poisson model, timeout manager, full pipeline e2e |

### Testing strategy

| Test | Scope |
|------|-------|
| Unit | Each benchmark, entropy formula, splitter strategies |
| Integration | Executor + splitter + scheduler + reassembler with mock mesh |
| E2E | Real WebRTC mesh of 3 headless browsers, full task lifecycle |
| Performance | Benchmark execution time vs. single-node, speedup factor |

---

## 8. Deliverables Checklist (End of Phase 2)

- [ ] WasmExecutor with sandbox (Worker or iframe), timeout, and memory enforcement
- [ ] JS fallback execution when Wasm is unavailable
- [ ] CPU benchmark (ALU + FP + prime sieve)
- [ ] Memory benchmark (latency + bandwidth)
- [ ] Entropy-based Bio-Score formula, validated against balanced/imbalanced machines
- [ ] Score broadcast via signal transduction on 30s interval
- [ ] Fractal task splitter (array uniform + recursive bisect strategies)
- [ ] Score-weighted roulette scheduler for work unit assignment
- [ ] ResultReassembler with order-preserving merge and aggregation
- [ ] Poisson distribution library with PMF and min-redundancy calculation
- [ ] RedundancyManager with adaptive dropout tracking
- [ ] TimeoutManager for stalled work unit re-dispatch
- [ ] Integration test: full pipeline from split → assign → execute → reassemble
- [ ] All packages pass `pnpm build` and `pnpm test`
