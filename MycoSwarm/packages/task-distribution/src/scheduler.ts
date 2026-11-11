import type { WorkUnit, NodeId, MessageEnvelope } from '@fungal/core';
import { MessageType as MT, serialize, createEnvelope } from '@fungal/core';

export interface MeshLike {
  peerId: string;
  send(peerId: string, data: Uint8Array | MessageEnvelope): void;
}

export class WorkScheduler {
  async assignUnits(
    units: WorkUnit[],
    availableNodes: Map<NodeId, number>,
    mesh: MeshLike,
  ): Promise<Map<string, NodeId[]>> {
    const assignments = new Map<string, NodeId[]>();
    const sorted = [...units].sort(byPriorityAndDeadline);

    for (const unit of sorted) {
      const selected = this.selectNodes(availableNodes, unit.redundancyFactor);
      assignments.set(unit.id, selected);

      for (const nodeId of selected) {
        const payload = serialize({ workUnit: unit });
        const envelope = createEnvelope(MT.TASK_ASSIGNMENT, mesh.peerId, nodeId, payload);
        mesh.send(nodeId, envelope);
      }
    }

    return assignments;
  }

  private selectNodes(availableNodes: Map<NodeId, number>, count: number): NodeId[] {
    const entries = Array.from(availableNodes.entries());
    const totalScore = entries.reduce((s, [, score]) => s + score, 0);
    if (totalScore <= 0 || count <= 0) return [];

    const selected: NodeId[] = [];
    const pool = [...entries];

    while (selected.length < count && pool.length > 0) {
      let r = Math.random() * totalScore;
      for (let i = 0; i < pool.length; i++) {
        r -= pool[i]![1];
        if (r <= 0) {
          selected.push(pool[i]![0]);
          pool.splice(i, 1);
          break;
        }
      }
    }

    return selected;
  }
}

function byPriorityAndDeadline(a: WorkUnit, b: WorkUnit): number {
  if (a.priority !== b.priority) return b.priority - a.priority;
  return a.deadline - b.deadline;
}