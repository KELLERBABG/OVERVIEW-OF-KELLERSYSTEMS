import { describe, it, expect } from 'vitest';
import { SaturationAwareQueue } from '../src/priority-queue.js';
import { SaturationMonitor } from '../src/saturation.js';
import { TaskPriority } from '@fungal/core';
import type { Task } from '@fungal/core';

function createTask(id: string, priority: TaskPriority): Task {
  return {
    id,
    code: 'return input',
    input: {},
    deadline: Date.now() + 60_000,
    priority,
    requireProof: false,
    requireEncryption: false,
    createdAt: Date.now(),
    ownerId: 'owner',
  };
}

describe('SaturationAwareQueue', () => {
  it('should accept tasks under normal saturation', () => {
    const monitor = new SaturationMonitor();
    const queue = new SaturationAwareQueue(monitor);
    const result = queue.enqueue(createTask('t1', TaskPriority.LOW));
    expect(result.accepted).toBe(true);
  });

  it('should reject low-priority tasks when saturated', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({
      busyRatio: 1, localQueueDepth: 50, avgAvailableScore: 10, avgCompletionTime: 5000, reportedAt: Date.now(),
    });
    const queue = new SaturationAwareQueue(monitor, { highThreshold: 0.9, fullThreshold: 0.95 });
    const result = queue.enqueue(createTask('t1', TaskPriority.LOW));
    expect(result.accepted).toBe(false);
    expect(result.reason).toContain('saturated');
  });

  it('should accept high-priority tasks even when saturated', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({
      busyRatio: 0.95, localQueueDepth: 50, avgAvailableScore: 20, avgCompletionTime: 3000, reportedAt: Date.now(),
    });
    const queue = new SaturationAwareQueue(monitor);
    const result = queue.enqueue(createTask('t1', TaskPriority.CRITICAL));
    expect(result.accepted).toBe(true);
  });

  it('should reject all tasks when fully saturated', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({
      busyRatio: 1, localQueueDepth: 100, avgAvailableScore: 5, avgCompletionTime: 10000, reportedAt: Date.now(),
    });
    const queue = new SaturationAwareQueue(monitor, { fullThreshold: 0.5 });
    const result = queue.enqueue(createTask('t1', TaskPriority.CRITICAL));
    expect(result.accepted).toBe(false);
  });

  it('should dequeue in priority order', () => {
    const monitor = new SaturationMonitor();
    const queue = new SaturationAwareQueue(monitor);
    queue.enqueue(createTask('low', TaskPriority.LOW));
    queue.enqueue(createTask('high', TaskPriority.HIGH));
    queue.enqueue(createTask('medium', TaskPriority.MEDIUM));
    expect(queue.dequeue()!.id).toBe('high');
    expect(queue.dequeue()!.id).toBe('medium');
    expect(queue.dequeue()!.id).toBe('low');
  });
});