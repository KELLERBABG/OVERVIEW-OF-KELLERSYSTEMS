import { describe, it, expect } from 'vitest';
import { RedundancyManager } from '../src/redundancy-manager.js';
import { TaskPriority } from '@fungal/core';
import type { WorkUnit } from '@fungal/core';

function createTestUnit(priority: TaskPriority): WorkUnit {
  return {
    id: 'test-unit',
    taskId: 'test-task',
    input: { data: 'test' },
    range: [0, 100],
    estimatedSize: 100,
    deadline: Date.now() + 60_000,
    priority,
    redundancyFactor: 1,
  };
}

describe('RedundancyManager', () => {
  it('should start with default dropout rate', () => {
    const mgr = new RedundancyManager();
    expect(mgr.getDropoutRate()).toBe(0.3);
  });

  it('should compute factor for a work unit', () => {
    const mgr = new RedundancyManager();
    const unit = createTestUnit(TaskPriority.MEDIUM);
    const factor = mgr.calculateFactor(unit, { getScore: () => 50, getAvailableNodes: () => new Map(), getActiveNodeCount: () => 2, getTotalScore: () => 100 });
    expect(factor).toBeGreaterThanOrEqual(1);
    expect(factor).toBeLessThanOrEqual(5);
  });

  it('should increase factor for critical tasks', () => {
    const mgr = new RedundancyManager();
    const low = mgr.calculateFactor(createTestUnit(TaskPriority.LOW), null as any);
    const critical = mgr.calculateFactor(createTestUnit(TaskPriority.CRITICAL), null as any);
    expect(critical).toBeGreaterThanOrEqual(low);
  });

  it('should adapt dropout rate based on history', () => {
    const mgr = new RedundancyManager();
    for (let i = 0; i < 10; i++) mgr.recordSuccess('node1');
    expect(mgr.getDropoutRate()).toBeLessThan(0.3);
  });
});