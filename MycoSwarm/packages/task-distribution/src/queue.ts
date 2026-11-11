import type { Task } from '@fungal/core';
import { TaskPriority } from '@fungal/core';

export class PriorityQueue {
  private tasks: Task[] = [];

  enqueue(task: Task): void {
    const idx = this.tasks.findIndex((t) => {
      if (t.priority !== task.priority) return t.priority < task.priority;
      return t.deadline > task.deadline;
    });
    if (idx === -1) this.tasks.push(task);
    else this.tasks.splice(idx, 0, task);
  }

  dequeue(): Task | null {
    return this.tasks.shift() ?? null;
  }

  peek(): Task | null {
    return this.tasks[0] ?? null;
  }

  remove(taskId: string): boolean {
    const idx = this.tasks.findIndex((t) => t.id === taskId);
    if (idx === -1) return false;
    this.tasks.splice(idx, 1);
    return true;
  }

  get size(): number {
    return this.tasks.length;
  }

  get isEmpty(): boolean {
    return this.tasks.length === 0;
  }

  clear(): void {
    this.tasks = [];
  }

  toArray(): Task[] {
    return [...this.tasks];
  }
}