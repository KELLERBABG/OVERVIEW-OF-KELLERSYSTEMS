import { describe, it, expect } from 'vitest';
import { PriorityQueue } from '../src/queue.js';
import { TaskPriority } from '@fungal/core';
import type { Task } from '@fungal/core';

function createTask(id: string, priority: TaskPriority, deadlineDelayMs = 60_000): Task {
  return {
    id,
    code: 'return input',
    input: {},
    deadline: Date.now() + deadlineDelayMs,
    priority,
    requireProof: false,
    requireEncryption: false,
    createdAt: Date.now(),
    ownerId: 'owner',
  };
}

describe('PriorityQueue', () => {
  it('should enqueue and dequeue in priority order', () => {
    const queue = new PriorityQueue();
    queue.enqueue(createTask('low', TaskPriority.LOW));
    queue.enqueue(createTask('high', TaskPriority.HIGH));
    queue.enqueue(createTask('medium', TaskPriority.MEDIUM));

    expect(queue.dequeue()!.id).toBe('high');
    expect(queue.dequeue()!.id).toBe('medium');
    expect(queue.dequeue()!.id).toBe('low');
  });

  it('should return null when empty', () => {
    const queue = new PriorityQueue();
    expect(queue.dequeue()).toBeNull();
    expect(queue.peek()).toBeNull();
  });

  it('should report correct size', () => {
    const queue = new PriorityQueue();
    expect(queue.size).toBe(0);
    queue.enqueue(createTask('t1', TaskPriority.LOW));
    queue.enqueue(createTask('t2', TaskPriority.HIGH));
    expect(queue.size).toBe(2);
    queue.dequeue();
    expect(queue.size).toBe(1);
  });

  it('should remove specific tasks', () => {
    const queue = new PriorityQueue();
    queue.enqueue(createTask('t1', TaskPriority.LOW));
    queue.enqueue(createTask('t2', TaskPriority.MEDIUM));
    expect(queue.remove('t1')).toBe(true);
    expect(queue.size).toBe(1);
    expect(queue.remove('nonexistent')).toBe(false);
  });
});