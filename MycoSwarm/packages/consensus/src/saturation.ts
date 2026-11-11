import type { SaturationMetrics } from '@fungal/core';

export class SaturationMonitor {
  private localMetrics: SaturationMetrics | null = null;
  private peerMetrics = new Map<string, SaturationMetrics>();
  private config: { maxQueueDepth: number };

  constructor(config?: Partial<{ maxQueueDepth: number }>) {
    this.config = { maxQueueDepth: 50, ...config };
  }

  calculateNetworkSaturation(): number {
    const reports = [this.localMetrics, ...this.peerMetrics.values()]
      .filter((m): m is SaturationMetrics => m !== null);

    if (reports.length === 0) return 0;

    const avgBusyRatio = average(reports.map((r) => r.busyRatio));
    const avgQueueDepth = average(reports.map((r) => r.localQueueDepth));
    const avgScore = average(reports.map((r) => r.avgAvailableScore));

    const queueSaturation = Math.min(avgQueueDepth / this.config.maxQueueDepth, 1);
    const scoreSaturation = 1 - avgScore / 100;

    const w1 = 0.5;
    const w2 = 0.3;
    const w3 = 0.2;
    return w1 * avgBusyRatio + w2 * queueSaturation + w3 * scoreSaturation;
  }

  updateLocalMetrics(metrics: SaturationMetrics): void {
    this.localMetrics = metrics;
  }

  receivePeerReport(nodeId: string, metrics: SaturationMetrics): void {
    this.peerMetrics.set(nodeId, metrics);
  }

  removePeer(nodeId: string): void {
    this.peerMetrics.delete(nodeId);
  }

  get reportCount(): number {
    return this.peerMetrics.size + (this.localMetrics ? 1 : 0);
  }

  clear(): void {
    this.localMetrics = null;
    this.peerMetrics.clear();
  }
}

function average(values: number[]): number {
  if (values.length === 0) return 0;
  return values.reduce((a, b) => a + b, 0) / values.length;
}