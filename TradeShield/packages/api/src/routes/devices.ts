import { Hono } from 'hono';
import { zValidator } from '@hono/zod-validator';
import { RegisterDeviceSchema, UpdateDeviceSchema, DeviceHeartbeatSchema } from '@washtrap/shared';

export const devicesRouter = new Hono();

// Valid status transitions
const VALID_TRANSITIONS: Record<string, string[]> = {
  active: ['inactive', 'maintenance'],
  inactive: ['active', 'maintenance'],
  maintenance: ['active', 'inactive'],
};

// ─── GET / — List devices for organization ───────────────────────
devicesRouter.get(
  '/',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const devices = db.prepare(`
      SELECT
        d.*,
        COALESCE(stats.total_cycles, 0) as total_cycles,
        COALESCE(stats.total_mg, 0) as total_mg,
        COALESCE(stats.total_water_l, 0) as total_water_l
      FROM devices d
      LEFT JOIN (
        SELECT
          device_id,
          COUNT(*) as total_cycles,
          SUM(estimated_mg) as total_mg,
          SUM(water_volume_l) as total_water_l
        FROM capture_events
        GROUP BY device_id
      ) stats ON d.id = stats.device_id
      WHERE d.org_id = ?
      ORDER BY d.created_at DESC
    `).all(orgId);

    return c.json({ data: devices });
  }
);

// ─── GET /:id — Device detail with recent events + stats ─────────
devicesRouter.get(
  '/:id',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const deviceId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const device = db.prepare(`
      SELECT * FROM devices
      WHERE id = ? AND org_id = ?
    `).get(deviceId, orgId);

    if (!device) {
      return c.json({ error: 'Device not found' }, 404);
    }

    const recentEvents = db.prepare(`
      SELECT * FROM capture_events
      WHERE device_id = ?
      ORDER BY timestamp DESC
      LIMIT 10
    `).all(deviceId);

    const stats = db.prepare(`
      SELECT
        COUNT(*) as total_cycles,
        COALESCE(SUM(water_volume_l), 0) as total_water_l,
        COALESCE(SUM(estimated_mg), 0) as total_mg,
        COALESCE(AVG(water_volume_l), 0) as avg_water_l,
        COALESCE(AVG(estimated_mg), 0) as avg_mg,
        MIN(timestamp) as first_event,
        MAX(timestamp) as last_event
      FROM capture_events
      WHERE device_id = ?
    `).get(deviceId) as any;

    // Last 7 days daily breakdown
    const dailyBreakdown = db.prepare(`
      SELECT
        date(timestamp) as date,
        COUNT(*) as cycles,
        SUM(estimated_mg) as total_mg,
        SUM(water_volume_l) as total_water_l
      FROM capture_events
      WHERE device_id = ? AND timestamp >= datetime('now', '-7 days')
      GROUP BY date(timestamp)
      ORDER BY date
    `).all(deviceId);

    return c.json({
      data: {
        ...device,
        recent_events: recentEvents,
        stats: {
          ...stats,
          avg_water_l: Math.round(stats.avg_water_l * 100) / 100,
          avg_mg: Math.round(stats.avg_mg * 1000) / 1000,
        },
        daily_breakdown: dailyBreakdown,
      },
    });
  }
);

// ─── POST / — Register new device ────────────────────────────────
devicesRouter.post(
  '/',
  zValidator('json', RegisterDeviceSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const data = c.req.valid('json');

    // Check for duplicate device ID
    const existing = db.prepare('SELECT id FROM devices WHERE id = ?').get(data.id);
    if (existing) {
      return c.json({ error: 'Device with this ID already exists' }, 409);
    }

    const now = new Date().toISOString();
    db.prepare(`
      INSERT INTO devices (id, org_id, name, location, installed_at, status, created_at)
      VALUES (?, ?, ?, ?, ?, 'active', ?)
    `).run(data.id, orgId, data.name || null, data.location || null, data.installed_at || null, now);

    const device = db.prepare('SELECT * FROM devices WHERE id = ?').get(data.id);

    return c.json({ data: device }, 201);
  }
);

// ─── PUT /:id — Update device metadata ───────────────────────────
devicesRouter.put(
  '/:id',
  zValidator('json', UpdateDeviceSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const deviceId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const data = c.req.valid('json');
    const device = db.prepare('SELECT * FROM devices WHERE id = ? AND org_id = ?').get(deviceId, orgId) as any;

    if (!device) {
      return c.json({ error: 'Device not found' }, 404);
    }

    // Validate status transition
    if (data.status && data.status !== device.status) {
      const allowed = VALID_TRANSITIONS[device.status] || [];
      if (!allowed.includes(data.status)) {
        return c.json({
          error: 'Invalid status transition',
          message: `Cannot transition from '${device.status}' to '${data.status}'. Allowed: ${allowed.join(', ')}`,
        }, 400);
      }
    }

    const updates: string[] = [];
    const params: any[] = [];

    if (data.name !== undefined) {
      updates.push('name = ?');
      params.push(data.name);
    }
    if (data.location !== undefined) {
      updates.push('location = ?');
      params.push(data.location);
    }
    if (data.status !== undefined) {
      updates.push('status = ?');
      params.push(data.status);
    }

    if (updates.length === 0) {
      return c.json({ error: 'No fields to update' }, 400);
    }

    params.push(deviceId, orgId);
    db.prepare(`
      UPDATE devices SET ${updates.join(', ')}
      WHERE id = ? AND org_id = ?
    `).run(...params);

    const updated = db.prepare('SELECT * FROM devices WHERE id = ?').get(deviceId);

    return c.json({ data: updated });
  }
);

// ─── DELETE /:id — Soft-delete (set inactive) ────────────────────
devicesRouter.delete(
  '/:id',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const user = c.get('user') as any;
    const deviceId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    if (!user || user.role !== 'admin') {
      return c.json({ error: 'Admin access required' }, 403);
    }

    const device = db.prepare('SELECT * FROM devices WHERE id = ? AND org_id = ?').get(deviceId, orgId);

    if (!device) {
      return c.json({ error: 'Device not found' }, 404);
    }

    // Soft delete: set status to inactive
    db.prepare(`
      UPDATE devices SET status = 'inactive' WHERE id = ? AND org_id = ?
    `).run(deviceId, orgId);

    return c.json({ deleted: true, id: deviceId });
  }
);

// ─── POST /:id/heartbeat — Device heartbeat (Zod validated) ─────
devicesRouter.post(
  '/:id/heartbeat',
  zValidator('json', DeviceHeartbeatSchema),
  async (c) => {
    const db = c.get('db');
    const deviceId = c.req.param('id');
    const data = c.req.valid('json');

    if (data.device_id !== deviceId) {
      return c.json({ error: 'Device ID mismatch' }, 403);
    }

    const device = db.prepare('SELECT * FROM devices WHERE id = ?').get(deviceId) as any;

    if (!device) {
      return c.json({ error: 'Device not found' }, 404);
    }

    db.prepare(`
      UPDATE devices
      SET last_heartbeat = CURRENT_TIMESTAMP,
          status = 'active',
          firmware_version = COALESCE(?, firmware_version),
          battery_level = COALESCE(?, battery_level)
      WHERE id = ?
    `).run(data.firmware_version || null, data.battery_level ?? null, deviceId);

    return c.json({
      status: 'ok',
      server_time: new Date().toISOString(),
      next_check_in_ms: 5 * 60 * 1000,
    });
  }
);
