import { describe, it, expect } from 'vitest';
import { RewardCalculator } from '../src/reward-calculator.js';
import { TaskPriority } from '@fungal/core';
import type { WorkUnit, TaskResult } from '@fungal/core';

describe('RewardCalculator', () => {
  it('should calculate non-zero reward', () => {
    const calc = new RewardCalculator();
    const unit: WorkUnit = {
      id: 'wu-1', taskId: 't-1', input: {}, range: [0, 1],
      estimatedSize: 100, deadline: Date.now() + 60000,
      priority: TaskPriority.MEDIUM, redundancyFactor: 1,
    };
    const result: TaskResult = {
      workUnitId: 'wu-1', taskId: 't-1', nodeId: 'n-1',
      output: 42, executionTime: 500, memoryUsed: 1024,
      completedAt: Date.now(),
    };
    const reward = calc.calculateReward(unit, result, { id: 'n-1', uptimeHours: 24, score: 50 });
    expect(reward).toBeGreaterThan(0n);
  });

  it('should give higher reward for critical tasks', () => {
    const calc = new RewardCalculator();
    const baseUnit: Partial<WorkUnit> = {
      id: 'wu-1', taskId: 't-1', input: {}, range: [0, 1],
      estimatedSize: 100, deadline: Date.now() + 60000, redundancyFactor: 1,
    };
    const result: TaskResult = {
      workUnitId: 'wu-1', taskId: 't-1', nodeId: 'n-1',
      output: 42, executionTime: 500, memoryUsed: 1024,
      completedAt: Date.now(),
    };
    const low = calc.calculateReward(
      { ...baseUnit, priority: TaskPriority.LOW } as WorkUnit,
      result,
      { id: 'n-1', uptimeHours: 24, score: 50 },
    );
    const critical = calc.calculateReward(
      { ...baseUnit, priority: TaskPriority.CRITICAL } as WorkUnit,
      result,
      { id: 'n-1', uptimeHours: 24, score: 50 },
    );
    expect(critical).toBeGreaterThan(low);
  });
});