import type { ResourceScorer } from './scorer.js';
import type { MessageType, MessageEnvelope } from '@fungal/core';
import { serialize } from '@fungal/core';
import type { ScoringConfig } from './config.js';

export interface MeshLike {
  peerId: string;
  broadcast(data: Uint8Array): void;
}

export class ScoreBroadcaster {
  private scorer: ResourceScorer;
  private mesh: MeshLike;
  private intervalId: ReturnType<typeof setInterval> | null = null;

  constructor(scorer: ResourceScorer, mesh: MeshLike) {
    this.scorer = scorer;
    this.mesh = mesh;
  }

  start(intervalMs?: number): void {
    const ms = intervalMs ?? 30_000;
    this.broadcast();
    this.intervalId = setInterval(() => this.broadcast(), ms);
  }

  stop(): void {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
  }

  private async broadcast(): Promise<void> {
    try {
      const score = await this.scorer.getScore();
      const payload = serialize({
        nodeId: this.mesh.peerId,
        score: score.value,
        cpuScore: score.cpuScore,
        memoryScore: score.memoryScore,
        timestamp: Date.now(),
      });
      this.mesh.broadcast(payload);
    } catch {
      // broadcast failure is non-fatal
    }
  }
}