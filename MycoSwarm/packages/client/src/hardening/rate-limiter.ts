export class RateLimiter {
  private counters = new Map<string, { count: number; windowStart: number }>();

  constructor(
    private windowMs: number = 60_000,
    private maxRequests: number = 100,
  ) {}

  allow(peerId: string): boolean {
    const now = Date.now();
    const entry = this.counters.get(peerId);

    if (!entry || now - entry.windowStart > this.windowMs) {
      this.counters.set(peerId, { count: 1, windowStart: now });
      return true;
    }

    if (entry.count >= this.maxRequests) {
      return false;
    }

    entry.count++;
    return true;
  }

  getStats(peerId: string): { count: number; remaining: number; resetAt: number } {
    const entry = this.counters.get(peerId);
    if (!entry) {
      return { count: 0, remaining: this.maxRequests, resetAt: Date.now() + this.windowMs };
    }
    return {
      count: entry.count,
      remaining: Math.max(0, this.maxRequests - entry.count),
      resetAt: entry.windowStart + this.windowMs,
    };
  }

  reset(peerId: string): void {
    this.counters.delete(peerId);
  }

  clear(): void {
    this.counters.clear();
  }
}