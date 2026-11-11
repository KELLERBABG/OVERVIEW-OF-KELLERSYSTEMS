import type { TaskResult, TaskId, WorkUnitId } from '@fungal/core';

export class ResultReassembler {
  private results = new Map<TaskId, { units: Map<WorkUnitId, TaskResult[]>; expectedCount: number }>();
  onComplete: ((taskId: TaskId, output: unknown) => void) | null = null;
  onTimeout: ((taskId: TaskId) => void) | null = null;

  registerTask(taskId: TaskId, unitCount: number): void {
    this.results.set(taskId, { units: new Map(), expectedCount: unitCount });
  }

  receiveResult(result: TaskResult): void {
    const taskEntry = this.results.get(result.taskId);
    if (!taskEntry) return;

    const { units } = taskEntry;
    const unitResults = units.get(result.workUnitId) ?? [];
    unitResults.push(result);
    units.set(result.workUnitId, unitResults);

    if (result.error) return;

    if (this.isTaskComplete(result.taskId, taskEntry)) {
      try {
        const output = this.reassemble(result.taskId, taskEntry);
        this.onComplete?.(result.taskId, output);
      } finally {
        this.cleanup(result.taskId);
      }
    }
  }

  private isTaskComplete(
    taskId: TaskId,
    entry: { units: Map<WorkUnitId, TaskResult[]>; expectedCount: number },
  ): boolean {
    if (entry.units.size < entry.expectedCount) return false;

    for (const results of entry.units.values()) {
      const hasValid = results.some((r) => !r.error);
      if (!hasValid) return false;
    }
    return true;
  }

  private reassemble(
    taskId: TaskId,
    entry: { units: Map<WorkUnitId, TaskResult[]>; expectedCount: number },
  ): unknown {
    const sortedUnits = Array.from(entry.units.entries())
      .sort(([a], [b]) => compareWorkUnitIds(a, b));

    const outputs = sortedUnits.map(([, results]) => {
      const valid = results.find((r) => !r.error);
      return valid ? valid.output : undefined;
    });

    if (outputs.length > 0 && Array.isArray(outputs[0])) {
      return (outputs as unknown[][]).flat();
    }

    if (outputs.length > 0 && typeof outputs[0] === 'number') {
      return (outputs as number[]).reduce((a, b) => a + (b ?? 0), 0);
    }

    return Object.fromEntries(
      sortedUnits.map(([unitId, results]) => {
        const valid = results.find((r) => !r.error);
        return [unitId, valid ? valid.output : null];
      }),
    );
  }

  getProgress(taskId: TaskId): { completed: number; total: number } {
    const entry = this.results.get(taskId);
    if (!entry) return { completed: 0, total: 0 };
    return {
      completed: Array.from(entry.units.values()).filter((r) =>
        r.some((x) => !x.error),
      ).length,
      total: entry.expectedCount,
    };
  }

  private cleanup(taskId: TaskId): void {
    this.results.delete(taskId);
  }
}

function compareWorkUnitIds(a: string, b: string): number {
  if (a < b) return -1;
  if (a > b) return 1;
  return 0;
}