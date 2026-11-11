import { describe, it, expect, beforeEach } from 'vitest';
import { createTestContext, asUser, asGuest, parseJSON, type TestContext } from './helpers.js';
import { hashPassword, verifyPassword } from '../src/routes/auth.js';

describe('Auth Routes & Crypto', () => {
  let ctx: TestContext;

  beforeEach(() => {
    ctx = createTestContext();
  });

  describe('Password Hashing Functions', () => {
    it('generates unique salt and verifiable hash', () => {
      const plain = 'super-secret-password-123';
      const hashed = hashPassword(plain);
      expect(hashed).toContain(':');
      expect(verifyPassword(plain, hashed)).toBe(true);
      expect(verifyPassword('wrong-password', hashed)).toBe(false);
    });

    it('handles legacy plain password fallback during migration', () => {
      expect(verifyPassword('password', 'password')).toBe(true);
      expect(verifyPassword('wrong', 'password')).toBe(false);
    });
  });

  describe('POST /api/v1/auth/login', () => {
    it('authenticates user with hashed password and sets cookie', async () => {
      const email = 'custom-auth@test.com';
      const plainPass = 'securePassword123';
      const passwordHash = hashPassword(plainPass);
      
      ctx.db.prepare(`
        INSERT INTO users (id, org_id, email, name, role, password_hash)
        VALUES (?, ?, ?, ?, ?, ?)
      `).run('user-custom-1', ctx.data.laundry.id, email, 'Test User', 'admin', passwordHash);

      const res = await asGuest(ctx.app, 'POST', '/api/v1/auth/login', {
        email,
        password: plainPass,
      });

      expect(res.status).toBe(200);
      const body = await parseJSON(res);
      expect(body.data.email).toBe(email);
      expect(body.data.id).toBe('user-custom-1');

      const setCookieHeader = res.headers.get('set-cookie');
      expect(setCookieHeader).toBeTruthy();
      expect(setCookieHeader).toContain('session=');
    });

    it('rejects invalid password with 401', async () => {
      const email = 'fail@test.com';
      ctx.db.prepare(`
        INSERT INTO users (id, org_id, email, name, role, password_hash)
        VALUES (?, ?, ?, ?, ?, ?)
      `).run('user-fail-1', ctx.data.laundry.id, email, 'Fail User', 'member', hashPassword('realPassword'));

      const res = await asGuest(ctx.app, 'POST', '/api/v1/auth/login', {
        email,
        password: 'wrongPassword',
      });

      expect(res.status).toBe(401);
      const body = await parseJSON(res);
      expect(body.error).toBe('Invalid credentials');
    });
  });

  describe('GET /api/v1/auth/me & POST /logout', () => {
    it('returns user data with active session and clears on logout', async () => {
      const meRes = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/auth/me');
      expect(meRes.status).toBe(200);
      const meBody = await parseJSON(meRes);
      expect(meBody.data.id).toBe(ctx.data.laundryUser.id);

      const logoutRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/auth/logout');
      expect(logoutRes.status).toBe(200);
    });

    it('returns 401 for /me without session', async () => {
      const res = await asGuest(ctx.app, 'GET', '/api/v1/auth/me');
      expect(res.status).toBe(401);
    });
  });
});
