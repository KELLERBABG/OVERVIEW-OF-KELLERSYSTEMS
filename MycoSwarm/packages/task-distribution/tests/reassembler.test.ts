import { describe, it, expect } from 'vitest';
import { ResultReassembler } from '../src/reassembler.js';
import type { TaskResult } from '@fungal/core';

describe('ResultReassembler', () => {
  it('should collect results and call onComplete', () => {
    const reassembler = new ResultReassembler();
    let completedOutput: unknown = null;

    reassembler.onComplete = (taskId, output) => {
      completedOutput = output;
    };

    reassembler.registerTask('task-1', 3);

    reassembler.receiveResult(makeResult('task-1', 'wu-0', [1, 2]));
    reassembler.receiveResult(makeResult('task-1', 'wu-1', [3, 4]));
    reassembler.receiveResult(makeResult('task-1', 'wu-2', [5, 6]));

    expect(completedOutput).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it('should ignore results for unknown tasks', () => {
    const reassembler = new ResultReassembler();
    expect(() =>
      reassembler.receiveResult(makeResult('unknown', 'wu-0', [1])),
    ).not.toThrow();
  });

  it('should track progress', () => {
    const reassembler = new ResultReassembler();
    reassembler.registerTask('task-2', 5);
    reassembler.receiveResult(makeResult('task-2', 'wu-0', [1]));
    reassembler.receiveResult(makeResult('task-2', 'wu-1', [2]));

    const progress = reassembler.getProgress('task-2');
    expect(progress.completed).toBe(2);
    expect(progress.total).toBe(5);
  });
});

function makeResult(
  taskId: string,
  workUnitId: string,
  output: unknown,
): TaskResult {
  return {
    workUnitId,
    taskId,
    nodeId: 'node1',
    output,
    executionTime: 100,
    memoryUsed: 1024,
    completedAt: Date.now(),
  };
}