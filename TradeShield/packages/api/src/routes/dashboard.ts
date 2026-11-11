import { Hono } from 'hono';
import { zValidator } from '@hono/zod-validator';
import { TimelineSchema } from '@washtrap/shared';
import { DEVICE_TIMEOUT_MS } from '@washtrap/shared';

export const dashboardRouter = new Hono();

// ─── GET /summary — KPI summary with optional comparison ─────────
dashboardRouter.get(
  '/summary',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    // Current period stats (30 days)
    const deviceStats = db.prepare(`
      SELECT
        COUNT(*) as total_devices,
        SUM(CASE WHEN status = 'active' THEN 1 ELSE 0 END) as active_devices,
        SUM(CASE WHEN status = 'inactive' THEN 1 ELSE 0 END) as inactive_devices,
        SUM(CASE WHEN status = 'maintenance' THEN 1 ELSE 0 END) as maintenance_devices
      FROM devices
      WHERE org_id = ?
    `).get(orgId) as any;

    const eventStats = db.prepare(`
      SELECT
        COUNT(*) as total_events_30d,
        COALESCE(SUM(ce.estimated_mg), 0) as total_mg_30d,
        COALESCE(SUM(ce.water_volume_l), 0) as total_water_l_30d,
        COALESCE(AVG(ce.estimated_mg), 0) as avg_mg_per_cycle,
        COALESCE(AVG(ce.water_volume_l), 0) as avg_water_per_cycle
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      WHERE d.org_id = ?
        AND ce.timestamp >= datetime('now', '-30 days')
    `).get(orgId) as any;

    const creditStats = db.prepare(`
      SELECT
        COALESCE(SUM(total_mg), 0) as total_credits_mg,
        COALESCE(SUM(total_eur), 0) as total_credits_eur,
        COUNT(*) as total_batches,
        SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) as pending_batches,
        SUM(CASE WHEN status = 'issued' THEN 1 ELSE 0 END) as issued_batches
      FROM credit_batches
      WHERE org_id = ?
    `).get(orgId) as any;

    // Previous period comparison (30-60 days ago)
    const prevEventStats = db.prepare(`
      SELECT
        COUNT(*) as total_events_prev,
        COALESCE(SUM(ce.estimated_mg), 0) as total_mg_prev
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      WHERE d.org_id = ?
        AND ce.timestamp >= datetime('now', '-60 days')
        AND ce.timestamp < datetime('now', '-30 days')
    `).get(orgId) as any;

    const eventsChangePct = prevEventStats.total_events_prev > 0
      ? ((eventStats.total_events_30d - prevEventStats.total_events_prev) / prevEventStats.total_events_prev) * 100
      : 0;
    const mgChangePct = prevEventStats.total_mg_prev > 0
      ? ((eventStats.total_mg_30d - prevEventStats.total_mg_prev) / prevEventStats.total_mg_prev) * 100
      : 0;

    return c.json({
      total_devices: deviceStats.total_devices,
      active_devices: deviceStats.active_devices,
      inactive_devices: deviceStats.inactive_devices || 0,
      maintenance_devices: deviceStats.maintenance_devices || 0,
      total_events_30d: eventStats.total_events_30d,
      total_mg_30d: eventStats.total_mg_30d,
      total_water_l_30d: eventStats.total_water_l_30d,
      avg_mg_per_cycle: Math.round(eventStats.avg_mg_per_cycle * 1000) / 1000,
      avg_water_per_cycle: Math.round(eventStats.avg_water_per_cycle * 100) / 100,
      total_credits_mg: creditStats.total_credits_mg,
      total_credits_eur: creditStats.total_credits_eur,
      total_batches: creditStats.total_batches,
      pending_batches: creditStats.pending_batches || 0,
      issued_batches: creditStats.issued_batches || 0,
      comparison: {
        total_events_prev: prevEventStats.total_events_prev,
        total_mg_prev: prevEventStats.total_mg_prev,
        events_change_pct: Math.round(eventsChangePct * 10) / 10,
        mg_change_pct: Math.round(mgChangePct * 10) / 10,
      },
    });
  }
);

// ─── GET /timeline — Time-series for charts ──────────────────────
dashboardRouter.get(
  '/timeline',
  zValidator('query', TimelineSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const { period, start_date, end_date } = c.req.valid('query');

    let dateTrunc: string;
    switch (period) {
      case 'week':
        dateTrunc = "date(ce.timestamp, 'weekday 1', '-6 days')";
        break;
      case 'month':
        dateTrunc = "date(ce.timestamp, 'start of month')";
        break;
      default:
        dateTrunc = "date(ce.timestamp)";
    }

    let whereClause = 'WHERE d.org_id = ?';
    const params: any[] = [orgId];

    if (start_date) {
      whereClause += ' AND ce.timestamp >= ?';
      params.push(start_date);
    }
    if (end_date) {
      whereClause += ' AND ce.timestamp <= ?';
      params.push(end_date + ' 23:59:59');
    }

    const timeline = db.prepare(`
      SELECT
        ${dateTrunc} as date,
        COALESCE(SUM(ce.estimated_mg), 0) as total_mg,
        COALESCE(SUM(ce.water_volume_l), 0) as total_water_l,
        COUNT(*) as event_count
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      ${whereClause}
      GROUP BY date
      ORDER BY date
    `).all(...params);

    return c.json({ data: timeline });
  }
);

// ─── GET /devices — Device summary with stats ────────────────────
dashboardRouter.get(
  '/devices',
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
        COALESCE(stats.total_water_l, 0) as total_water_l,
        COALESCE(recent.event_count_24h, 0) as event_count_24h
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
      LEFT JOIN (
        SELECT
          device_id,
          COUNT(*) as event_count_24h
        FROM capture_events
        WHERE timestamp >= datetime('now', '-1 day')
        GROUP BY device_id
      ) recent ON d.id = recent.device_id
      WHERE d.org_id = ?
      ORDER BY d.created_at DESC
    `).all(orgId);

    return c.json({ data: devices });
  }
);

// ─── GET /ranking — Top devices by capture volume ────────────────
dashboardRouter.get(
  '/ranking',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const url = new URL(c.req.url);
    const days = parseInt(url.searchParams.get('days') || '30');
    const limit = parseInt(url.searchParams.get('limit') || '10');

    const ranking = db.prepare(`
      SELECT
        d.id as device_id,
        d.name as device_name,
        COALESCE(SUM(ce.estimated_mg), 0) as total_mg,
        COUNT(*) as total_cycles
      FROM devices d
      LEFT JOIN capture_events ce ON d.id = ce.device_id
        AND ce.timestamp >= datetime('now', '-' || ? || ' days')
      WHERE d.org_id = ?
      GROUP BY d.id
      ORDER BY total_mg DESC
      LIMIT ?
    `).all(days, orgId, limit);

    // Add rank numbers
    const ranked = (ranking as any[]).map((r, i) => ({
      ...r,
      rank: i + 1,
    }));

    return c.json({ data: ranked });
  }
);

// ─── GET /alerts — Device health alerts ──────────────────────────
dashboardRouter.get(
  '/alerts',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const alerts: any[] = [];

    // Devices that haven't checked in recently
    const timeoutMs = DEVICE_TIMEOUT_MS;
    const timeoutDate = new Date(Date.now() - timeoutMs).toISOString();

    const staleDevices = db.prepare(`
      SELECT id, name, last_heartbeat, status
      FROM devices
      WHERE org_id = ?
        AND status = 'active'
        AND (last_heartbeat IS NULL OR last_heartbeat < ?)
    `).all(orgId, timeoutDate);

    for (const device of staleDevices as any[]) {
      alerts.push({
        type: 'device_offline',
        severity: 'warning',
        device_id: device.id,
        device_name: device.name,
        message: `Device "${device.name || device.id}" has not checked in since ${device.last_heartbeat || 'never'}`,
        last_seen: device.last_heartbeat,
      });
    }

    // Devices with low battery
    const lowBattery = db.prepare(`
      SELECT id, name, battery_level
      FROM devices
      WHERE org_id = ?
        AND status = 'active'
        AND battery_level IS NOT NULL
        AND battery_level < 20
    `).all(orgId);

    for (const device of lowBattery as any[]) {
      alerts.push({
        type: 'low_battery',
        severity: 'critical',
        device_id: device.id,
        device_name: device.name,
        message: `Device "${device.name || device.id}" battery at ${device.battery_level}%`,
        battery_level: device.battery_level,
      });
    }

    // Maintenance mode devices
    const maintenanceDevices = db.prepare(`
      SELECT id, name
      FROM devices
      WHERE org_id = ? AND status = 'maintenance'
    `).all(orgId);

    for (const device of maintenanceDevices as any[]) {
      alerts.push({
        type: 'maintenance',
        severity: 'info',
        device_id: device.id,
        device_name: device.name,
        message: `Device "${device.name || device.id}" is in maintenance mode`,
      });
    }

    // Devices with no events today
    const noEventsToday = db.prepare(`
      SELECT d.id, d.name
      FROM devices d
      LEFT JOIN capture_events ce ON d.id = ce.device_id
        AND date(ce.timestamp) = date('now')
      WHERE d.org_id = ? AND d.status = 'active'
        AND ce.id IS NULL
    `).all(orgId);

    for (const device of noEventsToday as any[]) {
      // Only alert if device has had events before (skip brand-new devices)
      const hasHistory = db.prepare(`
        SELECT 1 FROM capture_events WHERE device_id = ? LIMIT 1
      `).get(device.id);

      if (hasHistory) {
        alerts.push({
          type: 'no_events_today',
          severity: 'info',
          device_id: device.id,
          device_name: device.name,
          message: `Device "${device.name || device.id}" has no events today`,
        });
      }
    }

    return c.json({
      data: alerts,
      summary: {
        total: alerts.length,
        critical: alerts.filter((a) => a.severity === 'critical').length,
        warning: alerts.filter((a) => a.severity === 'warning').length,
        info: alerts.filter((a) => a.severity === 'info').length,
      },
    });
  }
);
