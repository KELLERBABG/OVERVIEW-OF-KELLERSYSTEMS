import type { Task, TaskResult, MessageEnvelope, NodeId, EventBus } from '@fungal/core';
import { MessageType, TaskPriority, serialize, deserialize } from '@fungal/core';
import { SaturationMonitor } from './saturation.js';
import { SaturationAwareQueue } from './priority-queue.js';
import { VotingEngine, type NodeRegistry } from './vote.js';
import { ConflictResolver } from './conflict-resolver.js';
import type { ConsensusConfig } from './config.js';
import { defaultConfig } from './config.js';

export interface MeshLike {
  peerId: string;
  broadcast(data: Uint8Array): void;
  onMessage(handler: (peerId: string, envelope: MessageEnvelope) => void): void;
  send(peerId: string, data: Uint8Array): void;
}

export class ConsensusEngine {
  private saturation: SaturationMonitor;
  private queue: SaturationAwareQueue;
  private voting: VotingEngine;
  private conflictResolver: ConflictResolver;
  private mesh: MeshLike;
  private eventBus: EventBus;
  private nodeRegistry: NodeRegistry;
  private config: ConsensusConfig;
  private broadcastTimer: ReturnType<typeof setInterval> | null = null;

  constructor(
    mesh: MeshLike,
    nodeRegistry: NodeRegistry,
    eventBus: EventBus,
    config?: Partial<ConsensusConfig>,
  ) {
    this.mesh = mesh;
    this.nodeRegistry = nodeRegistry;
    this.eventBus = eventBus;
    this.config = { ...defaultConfig, ...config };
    this.saturation = new SaturationMonitor({ maxQueueDepth: this.config.maxQueueDepth });
    this.queue = new SaturationAwareQueue(this.saturation, {
      maxQueueDepth: this.config.maxQueueDepth,
      highThreshold: this.config.highSaturationThreshold,
      fullThreshold: this.config.fullSaturationThreshold,
    });
    this.voting = new VotingEngine(nodeRegistry, eventBus, {
      votingWindowMs: this.config.votingWindowMs,
      quorumRatio: this.config.quorumRatio,
      approvalRatio: this.config.approvalRatio,
    });
    this.conflictResolver = new ConflictResolver();

    this.mesh.onMessage((peerId, envelope) => {
      this.handleMessage(peerId, envelope);
    });
  }

  start(): void {
    this.broadcastTimer = setInterval(() => {
      this.broadcastSaturation();
    }, this.config.saturationReportIntervalMs);
  }

  stop(): void {
    if (this.broadcastTimer) {
      clearInterval(this.broadcastTimer);
      this.broadcastTimer = null;
    }
  }

  submitTask(task: Task): { accepted: boolean; reason?: string } {
    return this.queue.enqueue(task);
  }

  getNextTask(): Task | null {
    return this.queue.dequeue();
  }

  proposePriorityChange(taskId: string, newPriority: TaskPriority, proposerId: NodeId): void {
    const proposal = this.voting.createProposal(
      'PRIORITY_CHANGE' as any,
      `Change priority of task ${taskId}`,
      { taskId, newPriority },
      proposerId,
    );
    const payload = serialize({ proposal });
    this.mesh.broadcast(payload);
  }

  async resolveConflict(
    taskId: string,
    unitId: string,
    results: TaskResult[],
  ): Promise<TaskResult> {
    return this.conflictResolver.resolve(taskId, unitId, results, this.nodeRegistry);
  }

  getSaturation(): number {
    return this.saturation.calculateNetworkSaturation();
  }

  getQueueDepth(): number {
    return this.queue.depth;
  }

  getActiveProposals(): number {
    return this.voting.getActiveProposalCount();
  }

  private handleMessage(peerId: string, _envelope: MessageEnvelope): void {
    // Handle incoming consensus-related messages
    this.saturation.receivePeerReport(peerId, {
      busyRatio: 0.5,
      localQueueDepth: 0,
      avgAvailableScore: 50,
      avgCompletionTime: 1000,
      reportedAt: Date.now(),
    });
  }

  private broadcastSaturation(): void {
    const metrics = {
      busyRatio: this.queue.depth > 0 ? Math.min(this.queue.depth / 10, 1) : 0,
      localQueueDepth: this.queue.depth,
      avgAvailableScore: 50,
      avgCompletionTime: 1000,
      reportedAt: Date.now(),
    };
    this.saturation.updateLocalMetrics(metrics);
    const payload = serialize({ saturation: metrics });
    this.mesh.broadcast(payload);
  }
}