import type { TaskResult, WorkUnitId, NodeId } from '@fungal/core';

export interface NodeRegistry {
  getScore(nodeId: string): number;
}

export class ConflictResolver {
  async resolve(
    _taskId: string,
    _unitId: WorkUnitId,
    results: TaskResult[],
    nodeRegistry: NodeRegistry,
  ): Promise<TaskResult> {
    if (results.length === 1) return results[0]!;

    // Strategy 1: Majority vote (by value)
    const valueGroups = groupBy(results, (r) => JSON.stringify(r.output));
    const majority = valueGroups.find((g) => g.length > results.length / 2);
    if (majority) return majority[0]!;

    // Strategy 2: Bio-Score weighted
    const scored = results
      .map((r) => ({ result: r, score: nodeRegistry.getScore(r.nodeId) ?? 0 }))
      .sort((a, b) => b.score - a.score);
    const bestScored = scored[0];
    if (bestScored) return bestScored.result;

    // Strategy 3: First valid result as fallback
    return results[0]!;
  }
}

function groupBy<T>(arr: T[], keyFn: (item: T) => string): T[][] {
  const groups = new Map<string, T[]>();
  for (const item of arr) {
    const key = keyFn(item);
    const group = groups.get(key) ?? [];
    group.push(item);
    groups.set(key, group);
  }
  return Array.from(groups.values());
}