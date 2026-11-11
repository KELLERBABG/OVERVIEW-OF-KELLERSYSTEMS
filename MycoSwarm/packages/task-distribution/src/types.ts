import type { TaskId, WorkUnit } from '@fungal/core';

export interface DistributionPlan {
  taskId: TaskId;
  units: WorkUnit[];
  metadata: {
    totalUnits: number;
    estimatedTotalWork: number;
    splitStrategy: string;
    depth: number;
  };
}