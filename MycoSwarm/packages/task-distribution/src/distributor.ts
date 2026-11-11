import type { Task, WorkUnit, TaskResult, MessageType, NodeId, MessageEnvelope } from '@fungal/core';
import { MessageType as MT, generateTaskId } from '@fungal/core';
import { TaskSplitter } from './splitter.js';
import { WorkScheduler } from './scheduler.js';
import { ResultReassembler } from './reassembler.js';
import { PriorityQueue } from './queue.js';
import type { DistributionConfig } from './config.js';
import { defaultConfig } from './config.js';

export interface MeshLike {
  peerId: string;
  send(peerId: string, data: Uint8Array | MessageEnvelope): void;
  broadcast(data: Uint8Array): void;
  onMessage(handler: (peerId: string, envelope: MessageEnvelope) => void): void;
  getConnectedPeers(): Array<{ id: string; connectionState: string }>;
}

export interface NodeRegistryLike {
  getScore(nodeId: string): number;
  getAvailableNodes(): Map<string, number>;
}

export class TaskDistributor {
  private splitter: TaskSplitter;
  private scheduler: WorkScheduler;
  private reassembler: ResultReassembler;
  private queue: PriorityQueue;
  private mesh: MeshLike;
  private nodeRegistry: NodeRegistryLike;
  private config: DistributionConfig;
  private pendingTasks = new Map<string, { resolve: (v: unknown) => void; reject: (e: Error) => void }>();

  constructor(mesh: MeshLike, nodeRegistry: NodeRegistryLike, config?: Partial<DistributionConfig>) {
    this.mesh = mesh;
    this.nodeRegistry = nodeRegistry;
    this.config = { ...defaultConfig, ...config };
    this.splitter = new TaskSplitter();
    this.scheduler = new WorkScheduler();
    this.reassembler = new ResultReassembler();
    this.queue = new PriorityQueue();

    this.reassembler.onComplete = (taskId, output) => {
      const pending = this.pendingTasks.get(taskId);
      if (pending) {
        pending.resolve(output);
        this.pendingTasks.delete(taskId);
      }
    };

    this.mesh.onMessage((peerId, envelope) => {
      if (envelope.type === MT.TASK_RESULT) {
        this.handleResult(envelope);
      }
    });
  }

  async submitTask(task: Task): Promise<unknown> {
    // Split
    const plan = this.splitter.split(task, this.config);

    // Register with reassembler
    this.reassembler.registerTask(task.id, plan.units.length);

    // Assign to nodes
    const availableNodes = this.nodeRegistry.getAvailableNodes();
    await this.scheduler.assignUnits(plan.units, availableNodes, this.mesh);

    // Return promise that resolves when complete
    return new Promise((resolve, reject) => {
      this.pendingTasks.set(task.id, { resolve, reject });

      // Timeout the entire task
      const timeout = task.deadline - Date.now();
      if (timeout > 0) {
        setTimeout(() => {
          const pending = this.pendingTasks.get(task.id);
          if (pending) {
            pending.reject(new Error(`Task ${task.id} timed out`));
            this.pendingTasks.delete(task.id);
          }
        }, timeout);
      }
    });
  }

  private handleResult(envelope: MessageEnvelope): void {
    // Parse the result from the envelope payload
    const result = JSON.parse(new TextDecoder().decode(envelope.payload)) as TaskResult;
    this.reassembler.receiveResult(result);
  }

  queueTask(task: Task): void {
    this.queue.enqueue(task);
  }

  getQueueSize(): number {
    return this.queue.size;
  }

  getPendingCount(): number {
    return this.pendingTasks.size;
  }
}