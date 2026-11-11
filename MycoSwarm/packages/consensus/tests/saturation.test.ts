import { describe, it, expect } from 'vitest';
import { SaturationMonitor } from '../src/saturation.js';

describe('SaturationMonitor', () => {
  it('should start at 0 saturation', () => {
    const monitor = new SaturationMonitor();
    expect(monitor.calculateNetworkSaturation()).toBe(0);
  });

  it('should compute weighted saturation correctly', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({
      busyRatio: 0.8,
      localQueueDepth: 10,
      avgAvailableScore: 40,
      avgCompletionTime: 1000,
      reportedAt: Date.now(),
    });

    const saturation = monitor.calculateNetworkSaturation();
    // w1=0.5*0.8 + w2=0.3*(10/50) + w3=0.2*(1-40/100)
    // = 0.4 + 0.06 + 0.12 = 0.58
    expect(saturation).toBeCloseTo(0.58, 2);
  });

  it('should incorporate peer reports', () => {
    const monitor = new SaturationMonitor();
    monitor.updateLocalMetrics({
      busyRatio: 0.5, localQueueDepth: 5, avgAvailableScore: 60, avgCompletionTime: 800, reportedAt: Date.now(),
    });
    monitor.receivePeerReport('peer1', {
      busyRatio: 0.3, localQueueDepth: 2, avgAvailableScore: 80, avgCompletionTime: 500, reportedAt: Date.now(),
    });
    const saturation = monitor.calculateNetworkSaturation();
    expect(saturation).toBeGreaterThan(0);
    expect(saturation).toBeLessThan(1);
  });

  it('should remove peer reports', () => {
    const monitor = new SaturationMonitor();
    monitor.receivePeerReport('peer1', {
      busyRatio: 0.9, localQueueDepth: 50, avgAvailableScore: 10, avgCompletionTime: 5000, reportedAt: Date.now(),
    });
    monitor.removePeer('peer1');
    expect(monitor.reportCount).toBe(0);
  });
});