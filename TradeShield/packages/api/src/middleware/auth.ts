import { Context, Next } from 'hono';
import { getCookie } from 'hono/cookie';
import { API_KEY_HEADER } from '@washtrap/shared';

// ─── Combined auth middleware ────────────────────────────────────
// Checks session cookie first, then API key header.
// Sets context variables: user, device, org_id
export async function authMiddleware(c: Context, next: Next) {
  const db = c.get('db');

  // 1. Try session cookie auth
  const session = getCookie(c, 'session');
  if (session) {
    const user = db.prepare(`
      SELECT u.*, o.name as org_name, o.type as org_type
      FROM users u
      JOIN organizations o ON u.org_id = o.id
      WHERE u.id = ?
    `).get(session) as any;

    if (user) {
      c.set('user', user);
      c.set('org_id', user.org_id);
      await next();
      return;
    }
  }

  // 2. Try API key auth (device)
  const apiKey = c.req.header(API_KEY_HEADER);
  if (apiKey) {
    const device = db.prepare(`
      SELECT d.*, o.name as org_name
      FROM devices d
      JOIN organizations o ON d.org_id = o.id
      WHERE d.id = ?
    `).get(apiKey) as any;

    if (device) {
      c.set('device', device);
      c.set('org_id', device.org_id);
      await next();
      return;
    }
  }

  return c.json({ error: 'Unauthorized', message: 'Valid session cookie or API key required' }, 401);
}

// ─── Requires authenticated user (not device) ────────────────────
export async function requireAuth(c: Context, next: Next) {
  const user = c.get('user');

  if (!user) {
    return c.json({ error: 'Authentication required', message: 'User session required for this endpoint' }, 401);
  }

  await next();
}

// ─── Requires admin role ─────────────────────────────────────────
export async function requireAdmin(c: Context, next: Next) {
  const user = c.get('user');

  if (!user) {
    return c.json({ error: 'Authentication required' }, 401);
  }

  if (user.role !== 'admin') {
    return c.json({ error: 'Admin access required' }, 403);
  }

  await next();
}

// ─── Requires device auth (ESP32) ────────────────────────────────
export async function requireDeviceAuth(c: Context, next: Next) {
  const device = c.get('device');

  if (!device) {
    return c.json({ error: 'Device authentication required', message: 'X-API-Key header with valid device ID required' }, 401);
  }

  if (device.status === 'inactive') {
    return c.json({ error: 'Device is inactive', message: 'This device has been deactivated' }, 403);
  }

  await next();
}
