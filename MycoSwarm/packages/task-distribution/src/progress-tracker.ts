import type { TaskId, WorkUnitId } from '@fungal/core';

export class ProgressTracker {
  private tasks = new Map<TaskId, TaskProgress>();

  initTask(taskId: TaskId, totalUnits: number): void {
    this.tasks.set(taskId, {
      totalUnits,
      completedUnits: 0,
      failedUnits: 0,
      startedAt: Date.now(),
      status: 'in-progress',
    });
  }

  markCompleted(unitId: WorkUnitId, taskId: TaskId): void {
    const task = this.tasks.get(taskId);
    if (!task) return;
    task.completedUnits++;
    if (task.completedUnits >= task.totalUnits) {
      task.status = 'completed';
    }
  }

  markFailed(unitId: WorkUnitId, taskId: TaskId): void {
    const task = this.tasks.get(taskId);
    if (!task) return;
    task.failedUnits++;
  }

  getProgress(taskId: TaskId): { percent: number; status: string } | null {
    const task = this.tasks.get(taskId);
    if (!task) return null;
    return {
      percent: (task.completedUnits / task.totalUnits) * 100,
      status: task.status,
    };
  }

  private cleanup(taskId: TaskId): void {
    this.tasks.delete(taskId);
  }
}

interface TaskProgress {
  totalUnits: number;
  completedUnits: number;
  failedUnits: number;
  startedAt: number;
  status: 'in-progress' | 'completed' | 'failed';
}