import { describe, it, expect } from 'vitest';
import { ProbabilisticVerification } from '../src/fallback.js';

describe('ProbabilisticVerification', () => {
  it('should verify approximately P% of units', () => {
    const pv = new ProbabilisticVerification(0.1);
    const results = Array.from({ length: 10_000 }, () => pv.shouldVerify());
    const verified = results.filter(Boolean).length;
    expect(verified).toBeGreaterThan(800);
    expect(verified).toBeLessThan(1200);
  });

  it('should increase probability when violations are detected', () => {
    const pv = new ProbabilisticVerification(0.05);
    pv.adjustForSuspicion('node1', 1);
    expect(pv.getProbability()).toBe(0.1);
    pv.adjustForSuspicion('node1', 2);
    expect(pv.getProbability()).toBe(0.2);
  });

  it('should cap probability at 1.0', () => {
    const pv = new ProbabilisticVerification(0.5);
    pv.adjustForSuspicion('node1', 10);
    expect(pv.getProbability()).toBe(1);
  });

  it('should reset to default', () => {
    const pv = new ProbabilisticVerification(0.5);
    pv.adjustForSuspicion('node1', 5);
    pv.reset();
    expect(pv.getProbability()).toBe(0.05);
  });
});