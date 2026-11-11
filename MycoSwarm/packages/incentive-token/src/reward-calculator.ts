import type { WorkUnit, TaskResult } from '@fungal/core';
import { TaskPriority } from '@fungal/core';

export interface NodeInfo {
  id: string;
  uptimeHours: number;
  score: number;
}

export class RewardCalculator {
  calculateReward(unit: WorkUnit, result: TaskResult, _nodeInfo: NodeInfo): bigint {
    const baseRate = 1_000_000n;
    const instructions = BigInt(Math.floor(result.executionTime * 1000));

    const difficultyMap: Record<number, bigint> = {
      [TaskPriority.LOW]: 1n,
      [TaskPriority.MEDIUM]: 2n,
      [TaskPriority.HIGH]: 5n,
      [TaskPriority.CRITICAL]: 10n,
    };
    const difficulty = difficultyMap[unit.priority] ?? 1n;

    return (baseRate * instructions * difficulty) / 1_000_000n;
  }
}