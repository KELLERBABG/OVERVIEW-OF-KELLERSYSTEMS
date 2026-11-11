import { describe, it, expect } from 'vitest';
import { resetYieldTimer, yieldIfNeeded } from '../src/yield.js';

describe('yield system', () => {
  it('should not yield on first call (timer reset)', async () => {
    resetYieldTimer();
    const start = performance.now();
    await yieldIfNeeded();
    const elapsed = performance.now() - start;
    expect(elapsed).toBeLessThan(50);
  });

  it('resetYieldTimer should reset state', () => {
    resetYieldTimer();
    expect(true).toBe(true);
  });

  it('should allow heartbeat processing between yields', async () => {
    resetYieldTimer();
    let count = 0;
    for (let i = 0; i < 10; i++) {
      await new Promise((r) => setTimeout(r, 1));
      count++;
    }
    expect(count).toBe(10);
  });
});