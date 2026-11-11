import { describe, it, expect, beforeEach } from 'vitest';
import { createTestContext, asDevice, asUser, asGuest, parseJSON, type TestContext } from './helpers.js';

describe('Event Ingest', () => {
  let ctx: TestContext;

  beforeEach(() => {
    ctx = createTestContext();
  });

  // ─── Single Event Ingest ────────────────────────────────────

  describe('POST /api/v1/events', () => {
    it('accepts a valid single event from authenticated device', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(201);
      const body = await parseJSON(res);
      expect(body.device_id).toBe(ctx.data.device1.id);
      expect(body.wash_cycle).toBe(1);
      expect(body.id).toBeDefined();
      expect(body.received_at).toBeDefined();
    });

    it('rejects event from unauthenticated request', async () => {
      const res = await asGuest(ctx.app, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(401);
    });

    it('rejects event with mismatched device_id', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: 'WRONG-DEVICE-ID',
        timestamp: new Date().toISOString(),
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(403);
    });

    it('rejects duplicate wash_cycle (idempotency)', async () => {
      // Send first event
      await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      // Send duplicate
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 40.0,
        estimated_mg: 3.2,
        wash_cycle: 1,
      });

      expect(res.status).toBe(409);
      const body = await parseJSON(res);
      expect(body.error).toBe('Duplicate event');
    });

    it('rejects event with invalid water_volume_l', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: -5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(400);
    });

    it('rejects event with water_volume_l > 100', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 150,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(400);
    });

    it('rejects event with missing timestamp', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(400);
    });

    it('rejects event older than 48 hours', async () => {
      const oldDate = new Date(Date.now() - 49 * 60 * 60 * 1000).toISOString();

      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: oldDate,
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      expect(res.status).toBe(422);
      const body = await parseJSON(res);
      expect(body.error).toBe('Event too old');
    });

    it('updates device last_heartbeat on ingest', async () => {
      const before = ctx.db.prepare('SELECT last_heartbeat FROM devices WHERE id = ?')
        .get(ctx.data.device1.id) as any;

      await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 35.5,
        estimated_mg: 2.84,
        wash_cycle: 1,
      });

      const after = ctx.db.prepare('SELECT last_heartbeat FROM devices WHERE id = ?')
        .get(ctx.data.device1.id) as any;

      expect(after.last_heartbeat).not.toBe(before.last_heartbeat);
    });
  });

  // ─── Batch Event Ingest ─────────────────────────────────────

  describe('POST /api/v1/events/batch', () => {
    it('accepts a valid batch of events', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events/batch', {
        events: [
          { device_id: ctx.data.device1.id, timestamp: new Date().toISOString(), water_volume_l: 30, estimated_mg: 2.4, wash_cycle: 1 },
          { device_id: ctx.data.device1.id, timestamp: new Date().toISOString(), water_volume_l: 35, estimated_mg: 2.8, wash_cycle: 2 },
          { device_id: ctx.data.device1.id, timestamp: new Date().toISOString(), water_volume_l: 40, estimated_mg: 3.2, wash_cycle: 3 },
        ],
      });

      expect(res.status).toBe(201);
      const body = await parseJSON(res);
      expect(body.accepted).toBe(3);
      expect(body.rejected).toBe(0);
      expect(body.events).toHaveLength(3);
    });

    it('handles mixed valid/invalid events in batch', async () => {
      // Send first event for cycle 1
      await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: new Date().toISOString(),
        water_volume_l: 30,
        estimated_mg: 2.4,
        wash_cycle: 1,
      });

      // Send batch with duplicate cycle 1
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events/batch', {
        events: [
          { device_id: ctx.data.device1.id, timestamp: new Date().toISOString(), water_volume_l: 30, estimated_mg: 2.4, wash_cycle: 1 },
          { device_id: ctx.data.device1.id, timestamp: new Date().toISOString(), water_volume_l: 35, estimated_mg: 2.8, wash_cycle: 2 },
        ],
      });

      const body = await parseJSON(res);
      expect(body.accepted).toBe(1);
      expect(body.rejected).toBe(1);
      expect(body.errors[0].wash_cycle).toBe(1);
    });

    it('rejects batch with device_id mismatch', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events/batch', {
        events: [
          { device_id: 'WRONG-DEVICE', timestamp: new Date().toISOString(), water_volume_l: 30, estimated_mg: 2.4, wash_cycle: 1 },
        ],
      });

      const body = await parseJSON(res);
      expect(body.accepted).toBe(0);
      expect(body.rejected).toBe(1);
    });

    it('rejects empty batch', async () => {
      const res = await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events/batch', {
        events: [],
      });

      expect(res.status).toBe(400);
    });
  });

  // ─── Event Listing ──────────────────────────────────────────

  describe('GET /api/v1/events', () => {
    beforeEach(async () => {
      // Seed some events
      for (let i = 1; i <= 5; i++) {
        await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
          device_id: ctx.data.device1.id,
          timestamp: new Date().toISOString(),
          water_volume_l: 30 + i,
          estimated_mg: 2.4 + i * 0.1,
          wash_cycle: i,
        });
      }
    });

    it('returns paginated events for authenticated user', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/events');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data).toHaveLength(5);
      expect(body.pagination.total).toBe(5);
      expect(body.pagination.page).toBe(1);
    });

    it('supports pagination', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/events?page=1&limit=2');
      const body = await parseJSON(res);

      expect(body.data).toHaveLength(2);
      expect(body.pagination.total).toBe(5);
      expect(body.pagination.pages).toBe(3);
    });

    it('filters by device_id', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', `/api/v1/events?device_id=${ctx.data.device1.id}`);
      const body = await parseJSON(res);

      expect(body.data).toHaveLength(5);
    });

    it('returns 401 for unauthenticated user', async () => {
      const res = await asGuest(ctx.app, 'GET', '/api/v1/events');
      expect(res.status).toBe(401);
    });

    it('does not show events from other orgs', async () => {
      // Brand user should not see laundry events
      const res = await asUser(ctx.app, ctx.data.brandUser, 'GET', '/api/v1/events');
      const body = await parseJSON(res);

      expect(body.data).toHaveLength(0);
    });
  });
});
