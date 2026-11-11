import type { Task } from '@fungal/core';
import { TaskPriority } from '@fungal/core';
import type { SaturationMonitor } from './saturation.js';
import type { ProposalType, Proposal } from '@fungal/core';

export class SaturationAwareQueue {
  private pendingTasks: Task[] = [];
  private saturationMonitor: SaturationMonitor;
  private config: { maxQueueDepth: number; highThreshold: number; fullThreshold: number };

  constructor(
    saturationMonitor: SaturationMonitor,
    config?: Partial<{ maxQueueDepth: number; highThreshold: number; fullThreshold: number }>,
  ) {
    this.saturationMonitor = saturationMonitor;
    this.config = {
      maxQueueDepth: 50,
      highThreshold: 0.9,
      fullThreshold: 0.95,
      ...config,
    };
  }

  enqueue(task: Task): { accepted: boolean; reason?: string } {
    const saturation = this.saturationMonitor.calculateNetworkSaturation();

    if (saturation > this.config.fullThreshold) {
      return { accepted: false, reason: 'Network fully saturated' };
    }
    if (saturation > this.config.highThreshold && task.priority <= TaskPriority.MEDIUM) {
      return { accepted: false, reason: 'Network saturated, try again later' };
    }

    const idx = this.pendingTasks.findIndex((t) => {
      if (t.priority !== task.priority) return t.priority < task.priority;
      return t.deadline > task.deadline;
    });
    if (idx === -1) this.pendingTasks.push(task);
    else this.pendingTasks.splice(idx, 0, task);

    return { accepted: true };
  }

  dequeue(): Task | null {
    return this.pendingTasks.shift() ?? null;
  }

  get depth(): number {
    return this.pendingTasks.length;
  }

  reorder(proposal: Proposal): void {
    if (proposal.type !== ('TASK_QUEUE_REORDER' as any)) return;
    const payload = proposal.payload as { taskId: string; newPriority: TaskPriority };
    const task = this.pendingTasks.find((t) => t.id === payload.taskId);
    if (!task) return;
    task.priority = payload.newPriority;
    this.pendingTasks.sort((a, b) => {
      if (a.priority !== b.priority) return b.priority - a.priority;
      return a.deadline - b.deadline;
    });
  }

  getTasks(): Task[] {
    return [...this.pendingTasks];
  }

  clear(): void {
    this.pendingTasks = [];
  }
}