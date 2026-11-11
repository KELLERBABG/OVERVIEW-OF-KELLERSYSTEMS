import { describe, it, expect, beforeEach } from 'vitest';
import { createTestContext, asUser, parseJSON, type TestContext } from './helpers.js';

describe('Dashboard', () => {
  let ctx: TestContext;

  beforeEach(() => {
    ctx = createTestContext();
  });

  // Seed events for dashboard tests
  async function seedEvents(count: number) {
    for (let i = 1; i <= count; i++) {
      await ctx.app.request('/api/v1/events', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'X-API-Key': ctx.data.device1.id },
        body: JSON.stringify({
          device_id: ctx.data.device1.id,
          timestamp: new Date().toISOString(),
          water_volume_l: 30 + (i % 10),
          estimated_mg: 2.4 + (i % 10) * 0.1,
          wash_cycle: i,
        }),
      });
    }
  }

  // ─── Summary ────────────────────────────────────────────────

  describe('GET /api/v1/dashboard/summary', () => {
    it('returns zeroed summary when no events exist', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/summary');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.total_devices).toBe(2);
      expect(body.active_devices).toBe(2);
      expect(body.total_events_30d).toBe(0);
      expect(body.total_mg_30d).toBe(0);
      expect(body.total_water_l_30d).toBe(0);
      expect(body.total_credits_mg).toBe(0);
      expect(body.total_credits_eur).toBe(0);
      expect(body.comparison).toBeDefined();
    });

    it('returns correct stats after seeding events', async () => {
      await seedEvents(10);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/summary');
      const body = await parseJSON(res);

      expect(body.total_devices).toBe(2);
      expect(body.active_devices).toBe(2);
      expect(body.total_events_30d).toBe(10);
      expect(body.total_mg_30d).toBeGreaterThan(0);
      expect(body.total_water_l_30d).toBeGreaterThan(0);
      expect(body.avg_mg_per_cycle).toBeGreaterThan(0);
      expect(body.avg_water_per_cycle).toBeGreaterThan(0);
    });

    it('includes comparison with previous 30 days', async () => {
      await seedEvents(5);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/summary');
      const body = await parseJSON(res);

      expect(body.comparison).toBeDefined();
      expect(body.comparison.total_events_prev).toBe(0);
      expect(body.comparison.total_mg_prev).toBe(0);
      expect(body.comparison.events_change_pct).toBeDefined();
      expect(body.comparison.mg_change_pct).toBeDefined();
    });

    it('returns 401 for unauthenticated user', async () => {
      const res = await ctx.app.request('/api/v1/dashboard/summary');
      expect(res.status).toBe(401);
    });

    it('does not show data from other orgs', async () => {
      await seedEvents(10);

      const res = await asUser(ctx.app, ctx.data.brandUser, 'GET', '/api/v1/dashboard/summary');
      const body = await parseJSON(res);

      // Brand has no devices
      expect(body.total_devices).toBe(0);
      expect(body.total_events_30d).toBe(0);
    });
  });

  // ─── Timeline ───────────────────────────────────────────────

  describe('GET /api/v1/dashboard/timeline', () => {
    it('returns timeline data points', async () => {
      await seedEvents(5);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/timeline');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data).toBeDefined();
      expect(Array.isArray(body.data)).toBe(true);
      expect(body.data.length).toBeGreaterThan(0);

      const point = body.data[0];
      expect(point.date).toBeDefined();
      expect(point.total_mg).toBeGreaterThanOrEqual(0);
      expect(point.total_water_l).toBeGreaterThanOrEqual(0);
      expect(point.event_count).toBeGreaterThan(0);
    });

    it('supports week aggregation', async () => {
      await seedEvents(5);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/timeline?period=week');
      const body = await parseJSON(res);

      expect(body.data.length).toBeGreaterThan(0);
    });

    it('supports month aggregation', async () => {
      await seedEvents(5);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/timeline?period=month');
      const body = await parseJSON(res);

      expect(body.data.length).toBeGreaterThan(0);
    });
  });

  // ─── Device Summary ─────────────────────────────────────────

  describe('GET /api/v1/dashboard/devices', () => {
    it('returns devices with stats', async () => {
      await seedEvents(5);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/devices');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data).toHaveLength(2);

      const device = body.data.find((d: any) => d.id === ctx.data.device1.id);
      expect(device).toBeDefined();
      expect(device.total_cycles).toBe(5);
      expect(device.total_mg).toBeGreaterThan(0);
      expect(device.total_water_l).toBeGreaterThan(0);
      expect(device.event_count_24h).toBeDefined();
    });
  });

  // ─── Device Ranking ─────────────────────────────────────────

  describe('GET /api/v1/dashboard/ranking', () => {
    it('returns devices ranked by capture volume', async () => {
      await seedEvents(10);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/ranking?days=30');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data.length).toBeGreaterThan(0);
      expect(body.data[0].rank).toBe(1);
      expect(body.data[0].total_mg).toBeGreaterThan(0);
    });
  });

  // ─── Alerts ─────────────────────────────────────────────────

  describe('GET /api/v1/dashboard/alerts', () => {
    it('returns alerts for the org', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/alerts');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data).toBeDefined();
      expect(body.summary).toBeDefined();
      expect(typeof body.summary.total).toBe('number');
      expect(typeof body.summary.critical).toBe('number');
      expect(typeof body.summary.warning).toBe('number');
      expect(typeof body.summary.info).toBe('number');
    });

    it('detects offline devices', async () => {
      // Set device last_heartbeat to 30 minutes ago (past timeout)
      ctx.db.prepare(`
        UPDATE devices SET last_heartbeat = datetime('now', '-30 minutes')
        WHERE id = ?
      `).run(ctx.data.device1.id);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/alerts');
      const body = await parseJSON(res);

      const offlineAlert = body.data.find((a: any) => a.type === 'device_offline');
      expect(offlineAlert).toBeDefined();
      expect(offlineAlert.device_id).toBe(ctx.data.device1.id);
    });

    it('detects low battery devices', async () => {
      ctx.db.prepare('UPDATE devices SET battery_level = 15 WHERE id = ?')
        .run(ctx.data.device1.id);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/alerts');
      const body = await parseJSON(res);

      const batteryAlert = body.data.find((a: any) => a.type === 'low_battery');
      expect(batteryAlert).toBeDefined();
      expect(batteryAlert.severity).toBe('critical');
    });

    it('detects maintenance devices', async () => {
      ctx.db.prepare("UPDATE devices SET status = 'maintenance' WHERE id = ?")
        .run(ctx.data.device1.id);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/dashboard/alerts');
      const body = await parseJSON(res);

      const maintAlert = body.data.find((a: any) => a.type === 'maintenance');
      expect(maintAlert).toBeDefined();
    });
  });
});
