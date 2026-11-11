import { describe, it, expect } from 'vitest';
import { poissonPMF, minRedundancyFactor } from '../src/poisson.js';

describe('Poisson PMF', () => {
  it('should calculate correct PMF values', () => {
    expect(poissonPMF(0, 1)).toBeCloseTo(0.3679, 3);
    expect(poissonPMF(1, 1)).toBeCloseTo(0.3679, 3);
    expect(poissonPMF(2, 2)).toBeCloseTo(0.2707, 3);
  });

  it('should return 1 for k=0, λ=0', () => {
    expect(poissonPMF(0, 0)).toBe(1);
  });
});

describe('minRedundancyFactor', () => {
  it('should compute minimum redundancy for 30% dropout, 95% reliability', () => {
    const factor = minRedundancyFactor(0.3, 0.95);
    // r ≥ -ln(0.05) / 0.7 = 2.996 / 0.7 ≈ 4.28 → ceil = 5
    expect(factor).toBe(5);
  });

  it('should compute factor for 0% dropout, 95% reliability', () => {
    const factor = minRedundancyFactor(0, 0.95);
    // r ≥ -ln(0.05) / 1 = 2.996 → ceil = 3
    expect(factor).toBe(3);
  });

  it('should return higher factors for higher reliability targets', () => {
    const low = minRedundancyFactor(0.3, 0.90);
    const high = minRedundancyFactor(0.3, 0.99);
    expect(high).toBeGreaterThanOrEqual(low);
  });

  it('should return Infinity for impossible reliability', () => {
    const factor = minRedundancyFactor(1, 0.95);
    expect(factor).toBe(Infinity);
  });
});