import { describe, it, expect } from 'vitest';
import { ConflictResolver } from '../src/conflict-resolver.js';
import type { TaskResult } from '@fungal/core';

function makeResult(nodeId: string, output: unknown): TaskResult {
  return {
    workUnitId: 'wu-1',
    taskId: 'task-1',
    nodeId,
    output,
    executionTime: 100,
    memoryUsed: 1024,
    completedAt: Date.now(),
  };
}

describe('ConflictResolver', () => {
  it('should return single result directly', async () => {
    const resolver = new ConflictResolver();
    const result = makeResult('node1', 42);
    const resolved = await resolver.resolve('task-1', 'wu-1', [result], {
      getScore: () => 50,
    });
    expect(resolved.output).toBe(42);
  });

  it('should use majority vote', async () => {
    const resolver = new ConflictResolver();
    const results = [
      makeResult('node1', 'majority-value'),
      makeResult('node2', 'majority-value'),
      makeResult('node3', 'minority-value'),
    ];
    const resolved = await resolver.resolve('task-1', 'wu-1', results, {
      getScore: () => 50,
    });
    expect(resolved.output).toBe('majority-value');
  });

  it('should use score-weighted selection when no majority', async () => {
    const resolver = new ConflictResolver();
    const results = [
      makeResult('low-score-node', 'bad'),
      makeResult('high-score-node', 'good'),
    ];
    const resolved = await resolver.resolve('task-1', 'wu-1', results, {
      getScore: (id: string) => (id === 'high-score-node' ? 100 : 10),
    });
    expect(resolved.output).toBe('good');
  });
});