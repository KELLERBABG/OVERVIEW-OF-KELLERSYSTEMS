import { describe, it, expect } from 'vitest';
import { RateLimiter } from '../src/hardening/rate-limiter.js';

describe('RateLimiter', () => {
  it('should allow requests under the limit', () => {
    const limiter = new RateLimiter(60_000, 5);
    expect(limiter.allow('peer1')).toBe(true);
    expect(limiter.allow('peer1')).toBe(true);
    expect(limiter.allow('peer1')).toBe(true);
  });

  it('should block requests over the limit', () => {
    const limiter = new RateLimiter(60_000, 3);
    limiter.allow('peer1');
    limiter.allow('peer1');
    limiter.allow('peer1');
    expect(limiter.allow('peer1')).toBe(false);
  });

  it('should return stats', () => {
    const limiter = new RateLimiter(60_000, 10);
    limiter.allow('peer1');
    limiter.allow('peer1');
    const stats = limiter.getStats('peer1');
    expect(stats.count).toBe(2);
    expect(stats.remaining).toBe(8);
  });

  it('should return empty stats for unknown peers', () => {
    const limiter = new RateLimiter();
    const stats = limiter.getStats('unknown');
    expect(stats.count).toBe(0);
  });
});