import { describe, it, expect } from 'vitest';
import { TaskSplitter } from '../src/splitter.js';
import { TaskPriority } from '@fungal/core';
import type { Task } from '@fungal/core';
import type { DistributionConfig } from '../src/config.js';

const testConfig: DistributionConfig = {
  minUnitSize: 10,
  targetUnitCount: 4,
  defaultRedundancy: 2,
  maxUnits: 100,
  maxRecursionDepth: 4,
};

describe('TaskSplitter', () => {
  it('should split array input into units', () => {
    const splitter = new TaskSplitter();
    const task: Task = {
      id: 'test-1',
      code: 'return input',
      input: Array.from({ length: 100 }, (_, i) => i),
      deadline: Date.now() + 60_000,
      priority: TaskPriority.MEDIUM,
      requireProof: false,
      requireEncryption: false,
      createdAt: Date.now(),
      ownerId: 'owner1',
      inputType: 'array',
    };
    const plan = splitter.split(task, testConfig);
    expect(plan.units.length).toBeGreaterThan(1);
    expect(plan.metadata.splitStrategy).toBe('uniform');
  });

  it('should split range input', () => {
    const splitter = new TaskSplitter();
    const task: Task = {
      id: 'test-2',
      code: 'return input',
      input: { start: 0, end: 1000 },
      deadline: Date.now() + 60_000,
      priority: TaskPriority.LOW,
      requireProof: false,
      requireEncryption: false,
      createdAt: Date.now(),
      ownerId: 'owner1',
      inputType: 'range',
    };
    const plan = splitter.split(task, testConfig);
    expect(plan.units.length).toBeGreaterThan(1);
    expect(plan.metadata.splitStrategy).toBe('range');
  });

  it('should return single unit for atomic input', () => {
    const splitter = new TaskSplitter();
    const task: Task = {
      id: 'test-3',
      code: 'return input',
      input: { value: 42 },
      deadline: Date.now() + 60_000,
      priority: TaskPriority.HIGH,
      requireProof: false,
      requireEncryption: false,
      createdAt: Date.now(),
      ownerId: 'owner1',
      inputType: 'single',
    };
    const plan = splitter.split(task, testConfig);
    expect(plan.units).toHaveLength(1);
    expect(plan.metadata.splitStrategy).toBe('single');
  });

  it('should recursively split large arrays', () => {
    const splitter = new TaskSplitter();
    const task: Task = {
      id: 'test-4',
      code: 'return input',
      input: Array.from({ length: 1000 }, (_, i) => i),
      deadline: Date.now() + 60_000,
      priority: TaskPriority.CRITICAL,
      requireProof: false,
      requireEncryption: false,
      createdAt: Date.now(),
      ownerId: 'owner1',
      inputType: 'recursive',
    };
    const plan = splitter.split(task, { ...testConfig, maxRecursionDepth: 3 });
    expect(plan.units.length).toBeGreaterThan(0);
    expect(plan.metadata.depth).toBeLessThanOrEqual(3);
  });
});