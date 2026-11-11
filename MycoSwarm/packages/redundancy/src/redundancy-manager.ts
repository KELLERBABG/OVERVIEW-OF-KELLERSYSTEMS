import { TaskPriority } from '@fungal/core';
import type { WorkUnit } from '@fungal/core';
import { minRedundancyFactor } from './poisson.js';
import type { RedundancyConfig } from './config.js';
import { defaultConfig } from './config.js';

export interface NodeRegistry {
  getScore(nodeId: string): number;
  getAvailableNodes(): Map<string, number>;
  getActiveNodeCount(): number;
  getTotalScore(): number;
}

export class RedundancyManager {
  private dropoutHistory: number[] = [];
  private config: RedundancyConfig;

  constructor(config?: Partial<RedundancyConfig>) {
    this.config = { ...defaultConfig, ...config };
  }

  recordDropout(_nodeId: string): void {
    this.dropoutHistory.push(1);
    if (this.dropoutHistory.length > this.config.windowSize) {
      this.dropoutHistory.shift();
    }
  }

  recordSuccess(_nodeId: string): void {
    this.dropoutHistory.push(0);
    if (this.dropoutHistory.length > this.config.windowSize) {
      this.dropoutHistory.shift();
    }
  }

  getDropoutRate(): number {
    if (this.dropoutHistory.length === 0) return this.config.defaultDropoutRate;
    return this.dropoutHistory.reduce((a, b) => a + b, 0) / this.dropoutHistory.length;
  }

  calculateFactor(unit: WorkUnit, _nodeRegistry: NodeRegistry): number {
    const dropoutRate = this.getDropoutRate();

    const reliabilityMap: Record<number, number> = {
      [TaskPriority.LOW]: 0.90,
      [TaskPriority.MEDIUM]: 0.95,
      [TaskPriority.HIGH]: 0.99,
      [TaskPriority.CRITICAL]: 0.999,
    };
    const targetReliability = reliabilityMap[unit.priority] ?? this.config.baseReliability;

    const factor = minRedundancyFactor(dropoutRate, targetReliability);
    return Math.min(factor, this.config.maxFactor);
  }
}