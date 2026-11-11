import { Context, Next } from 'hono';

interface RateLimitEntry {
  count: number;
  resetAt: number;
}

const store = new Map<string, RateLimitEntry>();

// Cleanup old entries every 5 minutes
setInterval(() => {
  const now = Date.now();
  for (const [key, entry] of store) {
    if (now > entry.resetAt) {
      store.delete(key);
    }
  }
}, 5 * 60 * 1000);

export interface RateLimitOptions {
  windowMs: number;
  max: number;
  keyGenerator?: (c: Context) => string;
}

export function rateLimit(options: RateLimitOptions) {
  const { windowMs, max, keyGenerator } = options;

  const defaultKeyGen = (c: Context) => {
    return c.req.header('x-forwarded-for') || c.req.header('x-real-ip') || 'unknown';
  };

  const gen = keyGenerator || defaultKeyGen;

  return async (c: Context, next: Next) => {
    const key = gen(c);
    const now = Date.now();
    const entry = store.get(key);

    if (entry && now < entry.resetAt) {
      if (entry.count >= max) {
        const retryAfter = Math.ceil((entry.resetAt - now) / 1000);
        c.header('Retry-After', String(retryAfter));
        c.header('X-RateLimit-Limit', String(max));
        c.header('X-RateLimit-Remaining', '0');
        c.header('X-RateLimit-Reset', String(Math.ceil(entry.resetAt / 1000)));
        return c.json({ error: 'Too many requests', retry_after: retryAfter }, 429);
      }
      entry.count++;
    } else {
      store.set(key, { count: 1, resetAt: now + windowMs });
    }

    const current = store.get(key)!;
    c.header('X-RateLimit-Limit', String(max));
    c.header('X-RateLimit-Remaining', String(Math.max(0, max - current.count)));
    c.header('X-RateLimit-Reset', String(Math.ceil(current.resetAt / 1000)));

    await next();
  };
}

// Preset configurations
export const deviceIngestRateLimit = rateLimit({
  windowMs: 60 * 1000,  // 1 minute
  max: 120,              // 120 events/min per device
  keyGenerator: (c) => `device:${c.req.header('X-API-Key') || 'unknown'}`,
});

export const apiRateLimit = rateLimit({
  windowMs: 60 * 1000,  // 1 minute
  max: 100,              // 100 requests/min per IP
});

export const authRateLimit = rateLimit({
  windowMs: 15 * 60 * 1000,  // 15 minutes
  max: 10,                     // 10 login attempts per 15 min
});
