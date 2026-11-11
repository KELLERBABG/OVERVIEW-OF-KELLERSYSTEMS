import { describe, it, expect } from 'vitest';
import { calculateBioScore, calculateBioScoreSimple } from '../src/entropy-score.js';
import type { BioScoreInput } from '../src/types.js';

describe('EntropyScore', () => {
  it('should calculate a score between 0–100', () => {
    const input: BioScoreInput = {
      cpu: { aluOpsPerSecond: 1000, fpOpsPerSecond: 300, primeBenchMs: 500 },
      memory: { readLatencyNs: 80, writeBandwidthMBs: 5000 },
      upvotes: 10,
      sessionAge: 600_000,
    };
    const score = calculateBioScore(input);
    expect(score).toBeGreaterThanOrEqual(0);
    expect(score).toBeLessThanOrEqual(100);
  });

  it('should score a balanced machine higher than an imbalanced one', () => {
    const balanced: BioScoreInput = {
      cpu: { aluOpsPerSecond: 1500, fpOpsPerSecond: 400, primeBenchMs: 300 },
      memory: { readLatencyNs: 60, writeBandwidthMBs: 8000 },
      upvotes: 10,
      sessionAge: 600_000,
    };
    const imbalanced: BioScoreInput = {
      cpu: { aluOpsPerSecond: 50, fpOpsPerSecond: 10, primeBenchMs: 10000 },
      memory: { readLatencyNs: 500, writeBandwidthMBs: 500 },
      upvotes: 0,
      sessionAge: 10_000,
    };
    const balancedScore = calculateBioScore(balanced);
    const imbalancedScore = calculateBioScore(imbalanced);
    expect(balancedScore).toBeGreaterThan(imbalancedScore);
  });

  it('should increase with session age', () => {
    const input: BioScoreInput = {
      cpu: { aluOpsPerSecond: 1000, fpOpsPerSecond: 300, primeBenchMs: 500 },
      memory: { readLatencyNs: 80, writeBandwidthMBs: 5000 },
      upvotes: 0,
      sessionAge: 0,
    };
    const youngScore = calculateBioScore(input);
    const oldScore = calculateBioScore({ ...input, sessionAge: 600_000 });
    expect(oldScore).toBeGreaterThanOrEqual(youngScore);
  });

  it('calculateBioScoreSimple works with any number of components', () => {
    const score = calculateBioScoreSimple([0.8, 0.5, 0.3]);
    expect(score).toBeGreaterThanOrEqual(0);
    expect(score).toBeLessThanOrEqual(100);
  });
});