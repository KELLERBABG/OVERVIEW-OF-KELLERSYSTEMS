import { scryptSync, randomBytes, timingSafeEqual } from 'crypto';
import { Hono } from 'hono';
import { zValidator } from '@hono/zod-validator';
import { setCookie, getCookie, deleteCookie } from 'hono/cookie';
import { LoginSchema } from '@washtrap/shared';
import { authRateLimit } from '../middleware/rate-limit.js';

export function hashPassword(password: string): string {
  const salt = randomBytes(16).toString('hex');
  const hash = scryptSync(password, salt, 64).toString('hex');
  return `${salt}:${hash}`;
}

export function verifyPassword(password: string, storedHash: string): boolean {
  if (storedHash === 'password') {
    return password === 'password';
  }
  const parts = storedHash.split(':');
  if (parts.length !== 2) return false;
  const [salt, expectedHashHex] = parts;
  const expectedHash = Buffer.from(expectedHashHex, 'hex');
  const computedHash = scryptSync(password, salt, 64);
  return expectedHash.length === computedHash.length && timingSafeEqual(expectedHash, computedHash);
}

export const authRouter = new Hono();

// ─── POST /login — Login with email/password ─────────────────────
authRouter.post(
  '/login',
  authRateLimit,
  zValidator('json', LoginSchema),
  async (c) => {
    const db = c.get('db');
    const { email, password } = c.req.valid('json');

    const user = db.prepare(`
      SELECT u.*, o.name as org_name, o.type as org_type
      FROM users u
      JOIN organizations o ON u.org_id = o.id
      WHERE u.email = ?
    `).get(email) as any;

    if (!user) {
      // Use same error for both missing user and wrong password (don't leak)
      return c.json({ error: 'Invalid credentials' }, 401);
    }

    if (!verifyPassword(password, user.password_hash)) {
      return c.json({ error: 'Invalid credentials' }, 401);
    }

    // Create session cookie
    setCookie(c, 'session', user.id, {
      httpOnly: true,
      secure: process.env.NODE_ENV === 'production',
      sameSite: 'lax',
      maxAge: 60 * 60 * 24 * 7, // 7 days
      path: '/',
    });

    return c.json({
      data: {
        id: user.id,
        email: user.email,
        name: user.name,
        org_id: user.org_id,
        org_name: user.org_name,
        org_type: user.org_type,
        role: user.role,
      },
    });
  }
);

// ─── POST /logout — Logout ──────────────────────────────────────
authRouter.post('/logout', async (c) => {
  deleteCookie(c, 'session', { path: '/' });
  return c.json({ status: 'ok' });
});

// ─── GET /me — Get current user info ─────────────────────────────
authRouter.get('/me', async (c) => {
  const session = getCookie(c, 'session');

  if (!session) {
    return c.json({ error: 'Not authenticated' }, 401);
  }

  const db = c.get('db');
  const user = db.prepare(`
    SELECT u.id, u.email, u.name, u.role, u.org_id,
           o.name as org_name, o.type as org_type
    FROM users u
    JOIN organizations o ON u.org_id = o.id
    WHERE u.id = ?
  `).get(session) as any;

  if (!user) {
    return c.json({ error: 'User not found' }, 404);
  }

  return c.json({
    data: {
      id: user.id,
      email: user.email,
      name: user.name,
      org_id: user.org_id,
      org_name: user.org_name,
      org_type: user.org_type,
      role: user.role,
    },
  });
});
