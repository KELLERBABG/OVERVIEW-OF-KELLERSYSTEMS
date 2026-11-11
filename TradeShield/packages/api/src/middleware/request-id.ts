import { Context, Next } from 'hono';
import { randomBytes } from 'crypto';

export async function requestId(c: Context, next: Next) {
  const id = c.req.header('x-request-id') || randomBytes(8).toString('hex');
  c.set('requestId', id);
  c.header('X-Request-Id', id);
  await next();
}
