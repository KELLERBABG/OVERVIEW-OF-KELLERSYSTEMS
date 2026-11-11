# Phase 4 — Consensus & Incentives (Weeks 14–17)

**Goal:** A decentralized consensus engine determines task priority based on network saturation. A token-based incentive layer rewards compute contributions and enables a self-sustaining economic loop.

---

## 1. Packages Involved

| Package | Responsibility |
|---------|---------------|
| `@fungal/consensus` | Mycelial Consensus Engine — leaderless voting for task priority, conflict resolution |
| `@fungal/incentive-token` | L2 smart contract, micropayment channels, wallet integration |
| `@fungal/substrate-layer` | Abstraction layer for pluggable adapters (browser, Node.js, headless) |

---

## 2. Consensus Engine (`packages/consensus`)

### 2.1 Purpose
A **leaderless** (no single coordinator) voting mechanism that decides task priority based on network "saturation" — how busy nodes are, how many tasks are queued, and the Bio-Score of participating nodes. Also resolves conflicts when redundant work units return contradictory results.

### 2.2 File Structure

```
packages/consensus/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── engine.ts            # Main consensus engine
│   ├── vote.ts              # Vote creation, casting, tallying
│   ├── priority-queue.ts    # Saturation-aware priority queue
│   ├── conflict-resolver.ts # Resolve contradictory results
│   ├── saturation.ts        # Network saturation calculation
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── engine.test.ts
│   ├── vote.test.ts
│   ├── priority-queue.test.ts
│   ├── conflict-resolver.test.ts
│   └── saturation.test.ts
```

### 2.3 Core Data Structures

```typescript
// src/types.ts
export interface Proposal {
  id: string;
  type: ProposalType;
  proposerId: NodeId;
  /** Task priority change, saturation report, etc. */
  subject: string;
  payload: unknown;
  timestamp: number;
  deadline: number;
  status: 'active' | 'accepted' | 'rejected' | 'expired';
}

export enum ProposalType {
  PRIORITY_CHANGE = 'priority_change',
  TASK_QUEUE_REORDER = 'task_queue_reorder',
  NODE_EXPULSION = 'node_expulsion',
  CONFLICT_RESOLUTION = 'conflict_resolution',
  SATURATION_REPORT = 'saturation_report',
}

export interface Vote {
  proposalId: string;
  voterId: NodeId;
  weight: number; // Bio-Score based
  decision: 'approve' | 'reject' | 'abstain';
  signature: string;
  timestamp: number;
}

export interface SaturationMetrics {
  /** Percentage of known nodes currently busy (0–1) */
  busyRatio: number;
  /** Number of pending work units in local queue */
  localQueueDepth: number;
  /** Average Bio-Score of available nodes (0–100) */
  avgAvailableScore: number;
  /** Average work unit completion time (ms) */
  avgCompletionTime: number;
  /** Timestamp */
  reportedAt: number;
}
```

### 2.4 Saturation Calculation

```typescript
// src/saturation.ts
export class SaturationMonitor {
  private localMetrics: SaturationMetrics | null = null;
  private peerMetrics = new Map<NodeId, SaturationMetrics>();

  /**
   * Calculate network-wide saturation (0 = idle, 1 = fully saturated).
   *
   * Saturation = w₁ * busyRatio + w₂ * (queueDepth / maxQueue) + w₃ * (1 - avgScore / 100)
   *
   * Where weights w₁, w₂, w₃ sum to 1.0 and are configurable.
   *
   * When saturation > threshold (e.g., 0.8), the network signals
   * "too busy — reject new low-priority tasks."
   */
  calculateNetworkSaturation(): number {
    const reports = [this.localMetrics, ...this.peerMetrics.values()]
      .filter((m): m is SaturationMetrics => m !== null);

    if (reports.length === 0) return 0;

    const avgBusyRatio = average(reports.map(r => r.busyRatio));
    const avgQueueDepth = average(reports.map(r => r.localQueueDepth));
    const avgScore = average(reports.map(r => r.avgAvailableScore));

    const queueSaturation = Math.min(avgQueueDepth / 50, 1); // 50 = max queue depth
    const scoreSaturation = 1 - avgScore / 100;

    // Weights
    const w1 = 0.5, w2 = 0.3, w3 = 0.2;
    return w1 * avgBusyRatio + w2 * queueSaturation + w3 * scoreSaturation;
  }

  /** Update local metrics (called periodically) */
  updateLocalMetrics(metrics: SaturationMetrics): void {
    this.localMetrics = metrics;
  }

  /** Handle incoming saturation report from a peer */
  receivePeerReport(nodeId: NodeId, metrics: SaturationMetrics): void {
    this.peerMetrics.set(nodeId, metrics);
  }
}
```

### 2.5 Priority Queue

```typescript
// src/priority-queue.ts
export class SaturationAwareQueue {
  private pendingTasks: Task[] = [];
  private saturationMonitor: SaturationMonitor;

  constructor(saturationMonitor: SaturationMonitor) {
    this.saturationMonitor = saturationMonitor;
  }

  /**
   * Enqueue a new task.
   * If saturation is high and task priority is low, reject immediately.
   */
  enqueue(task: Task): { accepted: boolean; reason?: string } {
    const saturation = this.saturationMonitor.calculateNetworkSaturation();

    // Admission control
    if (saturation > 0.9 && task.priority <= TaskPriority.MEDIUM) {
      return { accepted: false, reason: 'Network saturated, try again later' };
    }
    if (saturation > 0.95) {
      return { accepted: false, reason: 'Network fully saturated' };
    }

    // Insert sorted: higher priority first, then earlier deadline
    const idx = this.pendingTasks.findIndex(t => {
      if (t.priority !== task.priority) return t.priority < task.priority;
      return t.deadline > task.deadline;
    });
    if (idx === -1) this.pendingTasks.push(task);
    else this.pendingTasks.splice(idx, 0, task);

    return { accepted: true };
  }

  /** Dequeue the next task to process */
  dequeue(): Task | null {
    return this.pendingTasks.shift() ?? null;
  }

  /** Peek at queue depth */
  get depth(): number {
    return this.pendingTasks.length;
  }

  /** Re-order queue based on consensus vote */
  reorder(proposal: Proposal): void {
    if (proposal.type !== ProposalType.TASK_QUEUE_REORDER) return;
    const { taskId, newPriority } = proposal.payload as any;
    const task = this.pendingTasks.find(t => t.id === taskId);
    if (!task) return;
    task.priority = newPriority as TaskPriority;
    // Re-sort
    this.pendingTasks.sort((a, b) => {
      if (a.priority !== b.priority) return b.priority - a.priority;
      return a.deadline - b.deadline;
    });
  }
}
```

### 2.6 Voting Engine

```typescript
// src/vote.ts
export class VotingEngine {
  private activeProposals = new Map<string, Proposal>();
  private votes = new Map<string, Vote[]>();
  private nodeRegistry: NodeRegistry;

  constructor(nodeRegistry: NodeRegistry) {
    this.nodeRegistry = nodeRegistry;
  }

  /**
   * Create a new proposal for the network to vote on.
   */
  createProposal(type: ProposalType, subject: string, payload: unknown, proposerId: NodeId): Proposal {
    const proposal: Proposal = {
      id: generateProposalId(),
      type,
      proposerId,
      subject,
      payload,
      timestamp: Date.now(),
      deadline: Date.now() + 30_000, // 30s voting window
      status: 'active',
    };
    this.activeProposals.set(proposal.id, proposal);
    return proposal;
  }

  /**
   * Cast a vote on an active proposal.
   * Vote weight = node's Bio-Score (higher score = more influence).
   */
  castVote(proposalId: string, voterId: NodeId, decision: 'approve' | 'reject' | 'abstain'): void {
    const proposal = this.activeProposals.get(proposalId);
    if (!proposal || proposal.status !== 'active') return;

    const weight = this.nodeRegistry.getScore(voterId) ?? 1;
    const vote: Vote = {
      proposalId,
      voterId,
      weight,
      decision,
      signature: '', // TODO: sign with node's private key
      timestamp: Date.now(),
    };

    const proposalVotes = this.votes.get(proposalId) ?? [];
    proposalVotes.push(vote);
    this.votes.set(proposalId, proposalVotes);

    // Check if quorum reached
    this.checkProposal(proposal);
  }

  /**
   * Check if proposal has enough votes to resolve.
   * Quorum: >50% of active nodes by weight must vote.
   * Acceptance: >66% of non-abstain votes must approve.
   */
  private checkProposal(proposal: Proposal): void {
    const votes = this.votes.get(proposal.id) ?? [];
    const activeNodes = this.nodeRegistry.getActiveNodeCount();
    const totalWeight = this.nodeRegistry.getTotalScore();
    const votedWeight = votes.reduce((s, v) => s + v.weight, 0);

    // Quorum: >50% of total weight
    if (votedWeight / totalWeight < 0.5) return;

    // Count approvals vs rejections (exclude abstain)
    const nonAbstain = votes.filter(v => v.decision !== 'abstain');
    const approvals = nonAbstain.filter(v => v.decision === 'approve');
    const approvalWeight = approvals.reduce((s, v) => s + v.weight, 0);
    const nonAbstainWeight = nonAbstain.reduce((s, v) => s + v.weight, 0);
    const approvalRatio = approvalWeight / nonAbstainWeight;

    proposal.status = approvalRatio >= 0.66 ? 'accepted' : 'rejected';
    this.onProposalResolved(proposal);
  }

  private onProposalResolved(proposal: Proposal): void {
    // Emit event — the engine listens for this
    this.eventBus.emit('proposal-resolved', proposal);
    this.activeProposals.delete(proposal.id);
    this.votes.delete(proposal.id);
  }
}
```

### 2.7 Conflict Resolution

```typescript
// src/conflict-resolver.ts
export class ConflictResolver {
  /**
   * Resolve contradictory results from redundant work units.
   *
   * Strategies (in order of preference):
   * 1. Majority vote — if N/2+1 nodes agree, accept that result
   * 2. Bio-Score weighted — trust the higher-scoring node's result
   * 3. zk-Proof — if available, verify proof to determine correctness
   * 4. Re-execute — dispatch to a new high-score node for fresh run
   */
  async resolve(
    taskId: string,
    unitId: WorkUnitId,
    results: TaskResult[],
    nodeRegistry: NodeRegistry,
  ): Promise<TaskResult> {
    if (results.length === 1) return results[0];

    // Strategy 1: Majority vote (by value)
    const valueGroups = groupBy(results, r => JSON.stringify(r.output));
    const majority = valueGroups.find(g => g.length > results.length / 2);
    if (majority) return majority[0];

    // Strategy 2: Bio-Score weighted
    const scored = results
      .map(r => ({ result: r, score: nodeRegistry.getScore(r.nodeId) ?? 0 }))
      .sort((a, b) => b.score - a.score);
    const bestScored = scored[0];

    // Strategy 3: If any result has a valid zk-proof, trust it
    const withProof = results.find(r => r.proof);
    if (withProof) {
      const isValid = await verifyProof(withProof);
      if (isValid) return withProof;
    }

    // Strategy 4: Re-execute on highest-score node
    // (return best guess for now, the Reassember will re-dispatch)
    return bestScored.result;
  }
}
```

### 2.8 Main Engine

```typescript
// src/engine.ts
export class ConsensusEngine {
  private voting: VotingEngine;
  private saturation: SaturationMonitor;
  private queue: SaturationAwareQueue;
  private conflictResolver: ConflictResolver;
  private mesh: MeshManager;
  private eventBus: EventBus;

  constructor(mesh: MeshManager, nodeRegistry: NodeRegistry, eventBus: EventBus) {
    this.saturation = new SaturationMonitor();
    this.queue = new SaturationAwareQueue(this.saturation);
    this.voting = new VotingEngine(nodeRegistry);
    this.conflictResolver = new ConflictResolver();
    this.mesh = mesh;
    this.eventBus = eventBus;

    // Listen for incoming votes and proposals
    eventBus.on('message', this.handleMessage.bind(this));

    // Periodically report saturation
    setInterval(() => this.broadcastSaturation(), 30_000);
  }

  /**
   * Submit a task — goes through admission control then queue.
   */
  submitTask(task: Task): { accepted: boolean; reason?: string } {
    const result = this.queue.enqueue(task);
    if (result.accepted) {
      this.eventBus.emit('task-queued', task);
    }
    return result;
  }

  /**
   * Get next task to execute (called by local node when idle).
   */
  getNextTask(): Task | null {
    return this.queue.dequeue();
  }

  /**
   * Propose a priority change for a task.
   * Goes to network vote.
   */
  proposePriorityChange(taskId: string, newPriority: TaskPriority, proposerId: NodeId): void {
    const proposal = this.voting.createProposal(
      ProposalType.PRIORITY_CHANGE,
      `Change priority of task ${taskId}`,
      { taskId, newPriority },
      proposerId,
    );
    // Broadcast to mesh
    this.mesh.broadcast(serialize(proposal));
  }

  /**
   * Handle conflicting results — use consensus to resolve.
   * If can't resolve automatically, create a conflict resolution proposal.
   */
  async resolveConflict(taskId: string, unitId: WorkUnitId, results: TaskResult[]): Promise<TaskResult> {
    return this.conflictResolver.resolve(taskId, unitId, results, this.nodeRegistry);
  }

  private handleMessage(envelope: MessageEnvelope): void {
    // Parse consensus-related messages
    switch (envelope.type) {
      case MessageType.PROPOSAL:
        this.voting.registerProposal(envelope.payload as Proposal);
        // Auto-cast vote based on local policy
        this.autoVote(envelope.payload as Proposal);
        break;
      case MessageType.VOTE:
        this.voting.castVote(...);
        break;
      case MessageType.STATUS_UPDATE:
        if (envelope.payload.saturation) {
          this.saturation.receivePeerReport(envelope.senderId, envelope.payload.saturation);
        }
        break;
    }
  }

  private autoVote(proposal: Proposal): void {
    // Default voting policy: approve priority increases, reject decreases
    if (proposal.type === ProposalType.PRIORITY_CHANGE) {
      const { newPriority } = proposal.payload as any;
      const decision = newPriority >= TaskPriority.MEDIUM ? 'approve' : 'reject';
      this.voting.castVote(proposal.id, this.mesh.peerId, decision);
    }
  }

  private broadcastSaturation(): void {
    const metrics = this.saturation['localMetrics']; // current local metrics
    if (!metrics) return;
    this.mesh.broadcast(serialize({
      type: MessageType.STATUS_UPDATE,
      saturation: metrics,
    }));
  }
}
```

### 2.9 Tests

```typescript
describe('SaturationMonitor', () => {
  it('should start at 0 saturation', () => {
    const monitor = new SaturationMonitor();
    expect(monitor.calculateNetworkSaturation()).toBe(0);
  });

  it('should compute weighted saturation correctly', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({
      busyRatio: 0.8,
      localQueueDepth: 10,
      avgAvailableScore: 40,
      avgCompletionTime: 1000,
      reportedAt: Date.now(),
    });
    const saturation = monitor.calculateNetworkSaturation();
    // w1=0.5*0.8 + w2=0.3*(10/50) + w3=0.2*(1-40/100)
    // = 0.4 + 0.06 + 0.12 = 0.58
    expect(saturation).toBeCloseTo(0.58, 1);
  });
});

describe('SaturationAwareQueue', () => {
  it('should reject low-priority tasks when saturated', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({ busyRatio: 1, localQueueDepth: 50, avgAvailableScore: 10, ... });
    const queue = new SaturationAwareQueue(monitor);
    const result = queue.enqueue(createTask(TaskPriority.LOW));
    expect(result.accepted).toBe(false);
  });

  it('should accept high-priority tasks even when saturated', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({ busyRatio: 0.95, localQueueDepth: 50, avgAvailableScore: 20, ... });
    const queue = new SaturationAwareQueue(monitor);
    const result = queue.enqueue(createTask(TaskPriority.CRITICAL));
    expect(result.accepted).toBe(true);
  });
});

describe('VotingEngine', () => {
  it('should accept proposal with >66% approval', () => {
    const registry = createMockRegistry([{ id: 'a', score: 100 }, { id: 'b', score: 100 }]);
    const engine = new VotingEngine(registry);
    const proposal = engine.createProposal(ProposalType.PRIORITY_CHANGE, 'test', {}, 'a');
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'b', 'approve');
    expect(proposal.status).toBe('accepted');
  });

  it('should reject proposal with insufficient approval', () => {
    const registry = createMockRegistry([{ id: 'a', score: 100 }, { id: 'b', score: 100 }]);
    const engine = new VotingEngine(registry);
    const proposal = engine.createProposal(ProposalType.PRIORITY_CHANGE, 'test', {}, 'a');
    engine.castVote(proposal.id, 'a', 'approve');
    engine.castVote(proposal.id, 'b', 'reject');
    expect(proposal.status).toBe('rejected');
  });
});
```

---

## 3. Incentive / Token Layer (`packages/incentive-token`)

### 3.1 Purpose
A Layer-2 blockchain-based micropayment system that converts contributed compute time into tokens ("digital nutrients"). Nodes stake tokens to participate; task submitters pay tokens to use the network.

### 3.2 File Structure

```
packages/incentive-token/
├── package.json
├── tsconfig.json
├── contracts/
│   ├── FungalToken.sol        # ERC-20 token
│   ├── ComputeMarket.sol      # Task payment escrow
│   ├── Staking.sol            # Node staking & slashing
│   └── Migration.sol          # Upgradeability
├── src/
│   ├── index.ts
│   ├── token-client.ts        # TS client for interacting with contracts
│   ├── micropayments.ts       # Off-chain payment channel for frequent tx
│   ├── wallet.ts              # Wallet connection (MetaMask, WalletConnect)
│   ├── reward-calculator.ts   # Calculate token reward per work unit
│   ├── types.ts
│   └── config.ts
├── tests/
│   ├── token-client.test.ts
│   ├── micropayments.test.ts
│   └── reward-calculator.test.ts
├── hardhat.config.ts
└── scripts/
    ├── deploy.ts
    └── verify.ts
```

### 3.3 Smart Contracts

#### `contracts/FungalToken.sol` (ERC-20)
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/token/ERC20/ERC20.sol";
import "@openzeppelin/contracts/access/Ownable.sol";

contract FungalToken is ERC20, Ownable {
    uint256 public constant INITIAL_SUPPLY = 1_000_000_000 * 10**18; // 1B tokens

    constructor() ERC20("Fungal Compute", "FUNGAL") Ownable(msg.sender) {
        _mint(msg.sender, INITIAL_SUPPLY);
    }

    function mint(address to, uint256 amount) external onlyOwner {
        _mint(to, amount);
    }

    function burn(uint256 amount) external {
        _burn(msg.sender, amount);
    }
}
```

#### `contracts/ComputeMarket.sol`
```solidity
// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import "@openzeppelin/contracts/access/Ownable.sol";
import "./FungalToken.sol";

contract ComputeMarket is Ownable {
    FungalToken public token;

    struct TaskPayment {
        address requester;
        uint256 totalReward;
        uint256 unitReward;
        uint256 unitsCompleted;
        bool finalized;
    }

    mapping(bytes32 => TaskPayment) public payments;

    event TaskDeposited(bytes32 indexed taskId, address requester, uint256 amount);
    event UnitRewarded(bytes32 indexed taskId, address worker, uint256 amount);
    event TaskFinalized(bytes32 indexed taskId);

    constructor(address tokenAddress) Ownable(msg.sender) {
        token = FungalToken(tokenAddress);
    }

    /// Requester deposits tokens for a task
    function depositForTask(bytes32 taskId, uint256 totalUnits) external {
        uint256 amount = msg.value; // assume native token or use ERC20
        // In production: use token.transferFrom
        payments[taskId] = TaskPayment({
            requester: msg.sender,
            totalReward: amount,
            unitReward: amount / totalUnits,
            unitsCompleted: 0,
            finalized: false
        });
        emit TaskDeposited(taskId, msg.sender, amount);
    }

    /// Worker claims reward for completing a work unit
    function claimUnitReward(bytes32 taskId, address worker) external {
        TaskPayment storage payment = payments[taskId];
        require(!payment.finalized, "Task already finalized");
        require(payment.unitsCompleted < payment.totalReward / payment.unitReward, "All units paid");

        payment.unitsCompleted++;
        token.transfer(worker, payment.unitReward);
        emit UnitRewarded(taskId, worker, payment.unitReward);
    }

    /// Finalize task, refund unused tokens to requester
    function finalizeTask(bytes32 taskId) external {
        TaskPayment storage payment = payments[taskId];
        require(!payment.finalized, "Already finalized");
        payment.finalized = true;
        uint256 remaining = payment.totalReward - (payment.unitsCompleted * payment.unitReward);
        if (remaining > 0) {
            token.transfer(payment.requester, remaining);
        }
        emit TaskFinalized(taskId);
    }
}
```

### 3.4 TypeScript Client

```typescript
// src/token-client.ts
import { ethers } from 'ethers';
import FungalToken from '../contracts/artifacts/FungalToken.json';
import ComputeMarket from '../contracts/artifacts/ComputeMarket.json';

export class TokenClient {
  private provider: ethers.BrowserProvider | ethers.JsonRpcProvider;
  private signer: ethers.Signer | null = null;
  private token: ethers.Contract;
  private market: ethers.Contract;

  constructor(
    provider: ethers.BrowserProvider | ethers.JsonRpcProvider,
    tokenAddress: string,
    marketAddress: string,
  ) {
    this.provider = provider;
    this.token = new ethers.Contract(tokenAddress, FungalToken.abi, provider);
    this.market = new ethers.Contract(marketAddress, ComputeMarket.abi, provider);
  }

  async connectWallet(): Promise<string> {
    this.signer = await this.provider.getSigner();
    return this.signer.getAddress();
  }

  async getBalance(address: string): Promise<bigint> {
    return this.token.balanceOf(address);
  }

  async depositForTask(taskId: string, totalUnits: number, amountPerUnit: bigint): Promise<string> {
    if (!this.signer) throw new Error('Wallet not connected');
    const totalAmount = amountPerUnit * BigInt(totalUnits);
    const tx = await this.market.connect(this.signer).depositForTask(taskId, totalUnits, {
      value: totalAmount,
    });
    return tx.hash;
  }

  async claimReward(taskId: string): Promise<string> {
    if (!this.signer) throw new Error('Wallet not connected');
    const address = await this.signer.getAddress();
    const tx = await this.market.connect(this.signer).claimUnitReward(taskId, address);
    return tx.hash;
  }
}
```

### 3.5 Micropayment Channels

```typescript
// src/micropayments.ts
/**
 * Off-chain payment channels for high-frequency, low-value transactions.
 *
 * Instead of submitting an on-chain tx for every work unit (costly),
 * nodes batch payments off-chain and settle periodically on-chain.
 *
 * Simplified channel flow:
 * 1. Requester opens channel: deposits X tokens on-chain
 * 2. For each completed unit, requester signs a "receipt"
 * 3. Worker accumulates receipts
 * 4. Worker submits receipts to settle: gets paid in one on-chain tx
 */
export class MicropaymentChannel {
  private totalDeposited: bigint = 0n;
  private receipts: Array<{ unitId: string; amount: bigint; signature: string }> = [];
  private tokenClient: TokenClient;

  async openChannel(amount: bigint): Promise<string> {
    // Deposit tokens into channel contract
    return this.tokenClient.depositForTask('channel_' + Date.now(), 1000, amount / 1000n);
  }

  async signReceipt(unitId: string, amount: bigint, workerAddress: string): Promise<string> {
    // Sign a receipt that worker can redeem
    const message = ethers.solidityPackedKeccak256(
      ['string', 'uint256', 'address'],
      [unitId, amount, workerAddress],
    );
    return this.signer!.signMessage(ethers.getBytes(message));
  }

  async submitBatch(chainId: string): Promise<string> {
    // Submit all accumulated receipts on-chain and get paid
    const total = this.receipts.reduce((s, r) => s + r.amount, 0n);
    const tx = await this.market.batchClaim(this.receipts, { value: total });
    this.receipts = [];
    return tx.hash;
  }
}
```

### 3.6 Reward Calculator

```typescript
// src/reward-calculator.ts
export class RewardCalculator {
  /**
   * Calculate token reward for completing a work unit.
   *
   * Factors:
   * - Base reward (per million instructions)
   * - Difficulty multiplier (harder tasks pay more)
   * - Node reliability bonus (nodes with long uptime earn more)
   * - Speed bonus (faster than expected = bonus)
   *
   * formula: reward = baseRate * instructions * difficulty * reliabilityFactor * speedFactor
   */
  calculateReward(unit: WorkUnit, result: TaskResult, nodeInfo: NodeInfo): bigint {
    const baseRate = 1_000_000n; // 1 token per million "compute units"
    const instructions = BigInt(result.executionTime * 1000); // rough

    const difficultyMap = {
      [TaskPriority.LOW]: 1n,
      [TaskPriority.MEDIUM]: 2n,
      [TaskPriority.HIGH]: 5n,
      [TaskPriority.CRITICAL]: 10n,
    };
    const difficulty = difficultyMap[unit.priority];

    const reliabilityFactor = BigInt(Math.floor(nodeInfo.uptimeHours / 24) + 1);

    return baseRate * instructions * difficulty * reliabilityFactor / 1_000_000n;
  }
}
```

### 3.7 Testing

```typescript
describe('TokenClient', () => {
  it('should connect to wallet and return address', async () => {
    // Use Hardhat local network for testing
    const client = new TokenClient(ethers.provider, tokenAddress, marketAddress);
    const address = await client.connectWallet();
    expect(address).toMatch(/^0x[a-fA-F0-9]{40}$/);
  });

  it('should return token balance', async () => {
    const client = new TokenClient(...);
    const balance = await client.getBalance('0x...');
    expect(balance).toBeGreaterThanOrEqual(0n);
  });
});

describe('RewardCalculator', () => {
  it('should calculate non-zero reward', () => {
    const calc = new RewardCalculator();
    const reward = calc.calculateReward(createTestUnit(), createTestResult(), createTestNode());
    expect(reward).toBeGreaterThan(0n);
  });

  it('should give higher reward for critical tasks', () => {
    const calc = new RewardCalculator();
    const low = calc.calculateReward(
      createTestUnit(TaskPriority.LOW), result, node,
    );
    const critical = calc.calculateReward(
      createTestUnit(TaskPriority.CRITICAL), result, node,
    );
    expect(critical).toBeGreaterThan(low);
  });
});
```

---

## 4. Substrate Adaptation Layer (`packages/substrate-layer`)

### 4.1 Purpose
An abstraction layer that makes the entire system runtime-agnostic. The same code runs in a browser tab, a Node.js worker, or a headless environment by swapping adapters.

### 4.2 File Structure

```
packages/substrate-layer/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── substrate.ts          # Main substrate — runtime abstraction
│   ├── adapters/
│   │   ├── browser.ts        # Browser adapter (DOM, Workers, WebRTC)
│   │   ├── node.ts           # Node.js adapter (no DOM, raw WebSocket)
│   │   └── headless.ts       # Headless/chromium adapter (puppeteer)
│   ├── types.ts              # Substrate interfaces
│   └── config.ts
```

### 4.3 Core Interfaces

```typescript
// src/types.ts
export interface SubstrateAdapter {
  /** Unique adapter type identifier */
  type: 'browser' | 'node' | 'headless';

  /** Runtime information */
  runtime: {
    name: string;
    version: string;
    hasWasm: boolean;
    hasWebRTC: boolean;
    hasIndexedDB: boolean;
    hasServiceWorker: boolean;
    memoryLimit: number;
  };

  /** Connect to the mesh */
  connect(config: MeshConfig): Promise<MeshManager>;
  /** Initialize storage */
  initStorage(namespace: string): Promise<StorageAdapter>;
  /** Spawn a background worker */
  createWorker(script: string): Promise<WorkerAdapter>;
  /** Get system resource metrics */
  getSystemMetrics(): Promise<SystemMetrics>;
}

export interface StorageAdapter {
  get(key: string): Promise<unknown | null>;
  set(key: string, value: unknown): Promise<void>;
  delete(key: string): Promise<void>;
  clear(): Promise<void>;
}

export interface WorkerAdapter {
  postMessage(msg: unknown): void;
  onMessage(handler: (msg: unknown) => void): void;
  terminate(): void;
}

export interface SystemMetrics {
  memory: { total: number; used: number; jsHeap: number };
  cpu: { cores: number; speed: number };
  network: { type: string; downlink: number };
}
```

### 4.4 Browser Adapter

```typescript
// src/adapters/browser.ts
export class BrowserAdapter implements SubstrateAdapter {
  type = 'browser' as const;

  runtime = {
    name: 'Browser',
    version: navigator.userAgent,
    hasWasm: typeof WebAssembly !== 'undefined',
    hasWebRTC: typeof RTCPeerConnection !== 'undefined',
    hasIndexedDB: typeof indexedDB !== 'undefined',
    hasServiceWorker: 'serviceWorker' in navigator,
    memoryLimit: (navigator as any).deviceMemory ?? 4 * 1024, // MB
  };

  async connect(config: MeshConfig): Promise<MeshManager> {
    const mesh = new MeshManager(config, new EventBus());
    await mesh.join();
    return mesh;
  }

  async initStorage(namespace: string): Promise<StorageAdapter> {
    return new IndexedDBStorage(namespace);
  }

  async createWorker(script: string): Promise<WorkerAdapter> {
    const blob = new Blob([script], { type: 'application/javascript' });
    const worker = new Worker(URL.createObjectURL(blob));
    return new WorkerAdapterWrapper(worker);
  }

  async getSystemMetrics(): Promise<SystemMetrics> {
    const memory = (performance as any).memory;
    return {
      memory: {
        total: (navigator as any).deviceMemory ?? 4 * 1024,
        used: memory?.usedJSHeapSize ?? 0,
        jsHeap: memory?.totalJSHeapSize ?? 0,
      },
      cpu: {
        cores: navigator.hardwareConcurrency ?? 4,
        speed: 0, // Not available in browser
      },
      network: {
        type: (navigator as any).connection?.effectiveType ?? 'unknown',
        downlink: (navigator as any).connection?.downlink ?? 0,
      },
    };
  }
}
```

### 4.5 Node.js Adapter

```typescript
// src/adapters/node.ts
export class NodeAdapter implements SubstrateAdapter {
  type = 'node' as const;

  runtime = {
    name: `Node.js ${process.version}`,
    version: process.version,
    hasWasm: true,
    hasWebRTC: false, // Node doesn't natively support WebRTC
    hasIndexedDB: false,
    hasServiceWorker: false,
    memoryLimit: require('os').totalmem() / (1024 * 1024),
  };

  async connect(config: MeshConfig): Promise<MeshManager> {
    // For Node.js, connect to signaling server via WebSocket
    // but use a TCP-based transport instead of WebRTC
    // This allows server nodes to participate without a browser
    throw new Error('Node.js adapter — use TCP transport');
  }

  async getSystemMetrics(): Promise<SystemMetrics> {
    const os = require('os');
    return {
      memory: { total: os.totalmem(), used: os.totalmem() - os.freemem(), jsHeap: process.memoryUsage().heapUsed },
      cpu: { cores: os.cpus().length, speed: os.cpus()[0]?.speed ?? 0 },
      network: { type: 'ethernet', downlink: 1000 },
    };
  }
}
```

### 4.6 Tests

```typescript
describe('BrowserAdapter', () => {
  it('should detect runtime capabilities', () => {
    const adapter = new BrowserAdapter();
    expect(adapter.type).toBe('browser');
    expect(adapter.runtime.hasWasm).toBe(true);
  });
});

describe('NodeAdapter', () => {
  it('should detect Node.js runtime', () => {
    const adapter = new NodeAdapter();
    expect(adapter.type).toBe('node');
    expect(adapter.runtime.name).toContain('Node.js');
  });
});
```

---

## 5. Integration: Full Economic Loop

```typescript
describe('Full incentive flow', () => {
  it('should reward workers and charge requesters', async () => {
    // 1. Requester deposits tokens for a task
    const requester = new TokenClient(provider, tokenAddress, marketAddress);
    await requester.connectWallet();
    const depositTx = await requester.depositForTask(taskId, 10, parseEther('0.1'));
    expect(depositTx).toBeDefined();

    // 2. Worker completes the task
    const worker = new TokenClient(provider, tokenAddress, marketAddress);
    await worker.connectWallet();
    const claimTx = await worker.claimReward(taskId);
    expect(claimTx).toBeDefined();

    // 3. Verify balances
    const workerBalance = await worker.getBalance(workerAddress);
    expect(workerBalance).toBeGreaterThan(initialBalance);
    const requesterBalance = await requester.getBalance(requesterAddress);
    expect(requesterBalance).toBeLessThan(initialBalance);
  });
});
```

---

## 6. Development Workflow for Phase 4

### Step-by-step build order

| Week | Focus | Components |
|------|-------|-----------|
| **14** | Saturation & Queue | SaturationMonitor, SaturationAwareQueue, admission control |
| **15** | Voting & Consensus | VotingEngine, Proposal lifecycle, conflict resolution |
| **16** | Smart Contracts | FungalToken ERC-20, ComputeMarket, Staking, Hardhat deploy |
| **17** | Integration + Substrate | Full incentive loop, wallet UI, Substrate adapters, e2e testing |

### Hardhat local development

```bash
cd packages/incentive-token
pnpm add -D hardhat @nomicfoundation/hardhat-toolbox
npx hardhat init

# Compile and deploy locally
npx hardhat compile
npx hardhat run scripts/deploy.ts --network localhost

# Run tests
npx hardhat test
```

---

## 7. Deliverables Checklist (End of Phase 4)

- [ ] `SaturationMonitor` computes network saturation from peer reports
- [ ] `SaturationAwareQueue` accepts/rejects tasks based on saturation
- [ ] `VotingEngine` with proposal lifecycle, weighted voting, quorum rules
- [ ] `ConflictResolver` with majority, score-weighted, proof-based, re-execute strategies
- [ ] `ConsensusEngine` tying it all together with message handling
- [ ] `FungalToken` ERC-20 contract (Hardhat + tests)
- [ ] `ComputeMarket` contract with deposit, claim, finalize
- [ ] `Staking` contract for node registration and slashing
- [ ] `TokenClient` TypeScript interface to contracts
- [ ] `MicropaymentChannel` for off-chain batching
- [ ] `RewardCalculator` with difficulty/reliability/speed factors
- [ ] `SubstrateAdapter` interfaces defined
- [ ] `BrowserAdapter` (full implementation)
- [ ] `NodeAdapter` (skeleton)
- [ ] Integration test: deposit → work → claim → verify balances
- [ ] All packages pass `pnpm build` and `pnpm test`
