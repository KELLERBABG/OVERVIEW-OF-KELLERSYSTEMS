import { Hono } from 'hono';
import { zValidator } from '@hono/zod-validator';
import { CreateEventSchema, CreateBatchEventsSchema, EventFilterSchema } from '@washtrap/shared';
import { requireDeviceAuth } from '../middleware/auth.js';
import { deviceIngestRateLimit } from '../middleware/rate-limit.js';

export const eventsRouter = new Hono();

// Maximum hours an event can be "in the past" before we reject it
const MAX_EVENT_AGE_HOURS = 48;
// Maximum events per device per day
const MAX_EVENTS_PER_DEVICE_PER_DAY = 100;

// ─── POST / — Single event ingest ────────────────────────────────
eventsRouter.post(
  '/',
  deviceIngestRateLimit,
  requireDeviceAuth,
  zValidator('json', CreateEventSchema),
  async (c) => {
    const db = c.get('db');
    const device = c.get('device') as any;
    const data = c.req.valid('json');

    if (data.device_id !== device.id) {
      return c.json({ error: 'Device ID mismatch' }, 403);
    }

    // Reject stale events
    const eventAge = Date.now() - new Date(data.timestamp).getTime();
    if (eventAge > MAX_EVENT_AGE_HOURS * 3600 * 1000) {
      return c.json({
        error: 'Event too old',
        message: `Events older than ${MAX_EVENT_AGE_HOURS} hours are rejected`,
      }, 422);
    }

    // Check daily cap
    const { count: todayCount } = db.prepare(`
      SELECT COUNT(*) as count FROM capture_events
      WHERE device_id = ? AND date(timestamp) = date(?)
    `).get(data.device_id, data.timestamp) as any;

    if (todayCount >= MAX_EVENTS_PER_DEVICE_PER_DAY) {
      return c.json({
        error: 'Daily event limit reached',
        message: `Maximum ${MAX_EVENTS_PER_DEVICE_PER_DAY} events per device per day`,
        limit: MAX_EVENTS_PER_DEVICE_PER_DAY,
      }, 429);
    }

    try {
      const stmt = db.prepare(`
        INSERT INTO capture_events (device_id, timestamp, water_volume_l, estimated_mg, wash_cycle)
        VALUES (?, ?, ?, ?, ?)
      `);

      const result = stmt.run(
        data.device_id,
        data.timestamp,
        data.water_volume_l,
        data.estimated_mg,
        data.wash_cycle
      );

      // Update device last heartbeat
      db.prepare(`
        UPDATE devices SET last_heartbeat = CURRENT_TIMESTAMP, status = 'active'
        WHERE id = ?
      `).run(data.device_id);

      return c.json({
        id: result.lastInsertRowid,
        device_id: data.device_id,
        wash_cycle: data.wash_cycle,
        received_at: new Date().toISOString(),
      }, 201);
    } catch (error: any) {
      if (error.code === 'SQLITE_CONSTRAINT_UNIQUE') {
        return c.json({
          error: 'Duplicate event',
          message: `Event for wash_cycle ${data.wash_cycle} already exists`,
        }, 409);
      }
      throw error;
    }
  }
);

// ─── POST /batch — Batch event ingest (offline queue replay) ─────
eventsRouter.post(
  '/batch',
  deviceIngestRateLimit,
  requireDeviceAuth,
  zValidator('json', CreateBatchEventsSchema),
  async (c) => {
    const db = c.get('db');
    const device = c.get('device') as any;
    const { events } = c.req.valid('json');

    const accepted: any[] = [];
    const errors: Array<{ wash_cycle: number; error: string }> = [];

    const insertStmt = db.prepare(`
      INSERT INTO capture_events (device_id, timestamp, water_volume_l, estimated_mg, wash_cycle)
      VALUES (?, ?, ?, ?, ?)
    `);

    const insertMany = db.transaction(() => {
      for (const event of events) {
        if (event.device_id !== device.id) {
          errors.push({ wash_cycle: event.wash_cycle, error: 'Device ID mismatch' });
          continue;
        }

        const eventAge = Date.now() - new Date(event.timestamp).getTime();
        if (eventAge > MAX_EVENT_AGE_HOURS * 3600 * 1000) {
          errors.push({ wash_cycle: event.wash_cycle, error: 'Event too old' });
          continue;
        }

        try {
          const result = insertStmt.run(
            event.device_id,
            event.timestamp,
            event.water_volume_l,
            event.estimated_mg,
            event.wash_cycle
          );
          accepted.push({
            id: result.lastInsertRowid,
            device_id: event.device_id,
            wash_cycle: event.wash_cycle,
            received_at: new Date().toISOString(),
          });
        } catch (err: any) {
          if (err.code === 'SQLITE_CONSTRAINT_UNIQUE') {
            errors.push({ wash_cycle: event.wash_cycle, error: 'Duplicate' });
          } else {
            errors.push({ wash_cycle: event.wash_cycle, error: err.message });
          }
        }
      }
    });

    insertMany();

    // Update device heartbeat
    if (accepted.length > 0) {
      db.prepare(`
        UPDATE devices SET last_heartbeat = CURRENT_TIMESTAMP, status = 'active'
        WHERE id = ?
      `).run(device.id);
    }

    return c.json({
      accepted: accepted.length,
      rejected: errors.length,
      events: accepted,
      errors,
    }, accepted.length > 0 ? 201 : 400);
  }
);

// ─── GET / — List events (paginated, filterable) ─────────────────
eventsRouter.get(
  '/',
  zValidator('query', EventFilterSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const { page, limit, device_id, start_date, end_date } = c.req.valid('query');
    const offset = (page - 1) * limit;

    let whereClause = 'WHERE d.org_id = ?';
    const params: any[] = [orgId];

    if (device_id) {
      whereClause += ' AND ce.device_id = ?';
      params.push(device_id);
    }
    if (start_date) {
      whereClause += ' AND ce.timestamp >= ?';
      params.push(start_date);
    }
    if (end_date) {
      whereClause += ' AND ce.timestamp <= ?';
      params.push(end_date + ' 23:59:59');
    }

    const countRow = db.prepare(`
      SELECT COUNT(*) as count
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      ${whereClause}
    `).get(...params) as any;

    const events = db.prepare(`
      SELECT ce.*
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      ${whereClause}
      ORDER BY ce.timestamp DESC
      LIMIT ? OFFSET ?
    `).all(...params, limit, offset);

    return c.json({
      data: events,
      pagination: {
        page,
        limit,
        total: countRow.count,
        pages: Math.ceil(countRow.count / limit),
      },
    });
  }
);

// ─── GET /:id — Single event detail ──────────────────────────────
eventsRouter.get(
  '/:id',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const eventId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const event = db.prepare(`
      SELECT ce.*, d.name as device_name
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      WHERE ce.id = ? AND d.org_id = ?
    `).get(eventId, orgId);

    if (!event) {
      return c.json({ error: 'Event not found' }, 404);
    }

    return c.json({ data: event });
  }
);

// ─── DELETE /:id — Soft delete (admin only) ─────────────────────
eventsRouter.delete(
  '/:id',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const user = c.get('user') as any;
    const eventId = c.req.param('id');

    if (!orgId || !user) {
      return c.json({ error: 'Authentication required' }, 401);
    }

    if (user.role !== 'admin') {
      return c.json({ error: 'Admin access required' }, 403);
    }

    // Verify event belongs to org
    const event = db.prepare(`
      SELECT ce.id
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      WHERE ce.id = ? AND d.org_id = ?
    `).get(eventId, orgId);

    if (!event) {
      return c.json({ error: 'Event not found' }, 404);
    }

    // Hard delete (SQLite doesn't support soft deletes well without extra columns)
    db.prepare('DELETE FROM capture_events WHERE id = ?').run(eventId);

    return c.json({ deleted: true });
  }
);
