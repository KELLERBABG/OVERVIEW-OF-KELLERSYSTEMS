import type { Task, WorkUnit } from '@fungal/core';
import { generateWorkUnitId, estimateSize } from '@fungal/core';
import type { DistributionConfig } from './config.js';
import type { DistributionPlan } from './types.js';

export class TaskSplitter {
  split(task: Task, config: DistributionConfig): DistributionPlan {
    const inputType = task.inputType ?? 'single';

    switch (inputType) {
      case 'array':
        return this.splitArray(task, config);
      case 'range':
        return this.splitRange(task, config);
      case 'recursive':
        return this.splitRecursive(task, config, 0);
      default:
        return this.splitSingle(task);
    }
  }

  private splitArray(task: Task, config: DistributionConfig): DistributionPlan {
    const input = task.input as unknown[];
    const unitSize = Math.max(config.minUnitSize, Math.ceil(input.length / config.targetUnitCount));
    const units: WorkUnit[] = [];

    for (let i = 0; i < input.length; i += unitSize) {
      const end = Math.min(i + unitSize, input.length);
      units.push({
        id: generateWorkUnitId(),
        taskId: task.id,
        input: input.slice(i, end),
        range: [i, end],
        estimatedSize: estimateSize(input.slice(i, end)),
        deadline: task.deadline,
        priority: task.priority,
        redundancyFactor: config.defaultRedundancy,
      });
    }

    return {
      taskId: task.id,
      units,
      metadata: {
        totalUnits: units.length,
        estimatedTotalWork: units.reduce((s, u) => s + u.estimatedSize, 0),
        splitStrategy: 'uniform',
        depth: Math.ceil(Math.log2(units.length)) || 1,
      },
    };
  }

  private splitRange(task: Task, config: DistributionConfig): DistributionPlan {
    const input = task.input as { start: number; end: number };
    const range = input.end - input.start;
    const chunkSize = Math.max(1, Math.ceil(range / config.targetUnitCount));
    const units: WorkUnit[] = [];

    for (let i = input.start; i < input.end; i += chunkSize) {
      const chunkEnd = Math.min(i + chunkSize, input.end);
      units.push({
        id: generateWorkUnitId(),
        taskId: task.id,
        input: { start: i, end: chunkEnd },
        range: [i, chunkEnd],
        estimatedSize: 64,
        deadline: task.deadline,
        priority: task.priority,
        redundancyFactor: config.defaultRedundancy,
      });
    }

    return {
      taskId: task.id,
      units,
      metadata: {
        totalUnits: units.length,
        estimatedTotalWork: range,
        splitStrategy: 'range',
        depth: Math.ceil(Math.log2(units.length)) || 1,
      },
    };
  }

  private splitRecursive(task: Task, config: DistributionConfig, depth: number): DistributionPlan {
    if (depth >= config.maxRecursionDepth || this.isAtomic(task.input)) {
      return this.splitSingle(task);
    }

    const bisected = this.bisectInput(task.input);
    if (!bisected) return this.splitSingle(task);

    const [left, right] = bisected;
    const leftPlan = this.splitRecursive({ ...task, input: left }, config, depth + 1);
    const rightPlan = this.splitRecursive({ ...task, input: right }, config, depth + 1);

    return {
      taskId: task.id,
      units: [...leftPlan.units, ...rightPlan.units],
      metadata: {
        totalUnits: leftPlan.units.length + rightPlan.units.length,
        estimatedTotalWork: leftPlan.metadata.estimatedTotalWork + rightPlan.metadata.estimatedTotalWork,
        splitStrategy: 'recursive-bisect',
        depth,
      },
    };
  }

  private splitSingle(task: Task): DistributionPlan {
    return {
      taskId: task.id,
      units: [
        {
          id: generateWorkUnitId(),
          taskId: task.id,
          input: task.input,
          range: [0, 1],
          estimatedSize: estimateSize(task.input),
          deadline: task.deadline,
          priority: task.priority,
          redundancyFactor: 1,
        },
      ],
      metadata: {
        totalUnits: 1,
        estimatedTotalWork: estimateSize(task.input),
        splitStrategy: 'single',
        depth: 0,
      },
    };
  }

  private isAtomic(input: unknown): boolean {
    if (input === null || input === undefined) return true;
    if (typeof input === 'string') return input.length < 100;
    if (Array.isArray(input)) return input.length < 10;
    if (typeof input === 'object') {
      const keys = Object.keys(input as Record<string, unknown>);
      return keys.length < 3;
    }
    return true;
  }

  private bisectInput(input: unknown): [unknown, unknown] | null {
    if (Array.isArray(input)) {
      const mid = Math.ceil(input.length / 2);
      return [input.slice(0, mid), input.slice(mid)];
    }
    if (typeof input === 'object' && input !== null) {
      const entries = Object.entries(input as Record<string, unknown>);
      const mid = Math.ceil(entries.length / 2);
      return [
        Object.fromEntries(entries.slice(0, mid)),
        Object.fromEntries(entries.slice(mid)),
      ];
    }
    return null;
  }
}