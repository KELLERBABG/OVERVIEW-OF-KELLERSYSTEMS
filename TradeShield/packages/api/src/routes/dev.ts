import { Hono } from 'hono';

export const devRouter = new Hono();

// POST /api/v1/dev/simulate - Generate simulated events
devRouter.post(
  '/simulate',
  async (c) => {
    const db = c.get('db');
    const body = await c.req.json();
    
    const { 
      device_id, 
      days = 30, 
      events_per_day = 8,
      start_date 
    } = body;
    
    if (!device_id) {
      return c.json({ error: 'device_id is required' }, 400);
    }
    
    // Check device exists
    const device = db.prepare('SELECT * FROM devices WHERE id = ?').get(device_id) as any;
    if (!device) {
      return c.json({ error: 'Device not found' }, 404);
    }
    
    // Get max wash_cycle for this device
    const maxCycle = db.prepare(`
      SELECT MAX(wash_cycle) as max_cycle FROM capture_events WHERE device_id = ?
    `).get(device_id) as any;
    
    let currentCycle = (maxCycle?.max_cycle || 0) + 1;
    const startDateObj = start_date ? new Date(start_date) : new Date();
    startDateObj.setDate(startDateObj.getDate() - days);
    
    const events = [];
    
    for (let day = 0; day < days; day++) {
      const date = new Date(startDateObj);
      date.setDate(date.getDate() + day);
      
      // Generate events for this day
      const numEvents = events_per_day + Math.floor(Math.random() * 3) - 1; // ±1 variation
      
      for (let i = 0; i < numEvents; i++) {
        const hour = 6 + Math.floor(Math.random() * 14); // 6am - 8pm
        const minute = Math.floor(Math.random() * 60);
        
        const timestamp = new Date(date);
        timestamp.setHours(hour, minute, 0, 0);
        
        // Realistic water volume: 15-80L per cycle
        const water_volume_l = 15 + Math.random() * 65;
        
        // Estimate microplastic based on conservative ratio
        const estimated_mg = water_volume_l * 0.08; // Conservative estimate
        
        events.push({
          device_id,
          timestamp: timestamp.toISOString(),
          water_volume_l: Math.round(water_volume_l * 100) / 100,
          estimated_mg: Math.round(estimated_mg * 1000) / 1000,
          wash_cycle: currentCycle++,
        });
      }
    }
    
    // Batch insert events
    const stmt = db.prepare(`
      INSERT OR IGNORE INTO capture_events (device_id, timestamp, water_volume_l, estimated_mg, wash_cycle)
      VALUES (?, ?, ?, ?, ?)
    `);
    
    const insertMany = db.transaction((events: any[]) => {
      for (const event of events) {
        stmt.run(event.device_id, event.timestamp, event.water_volume_l, event.estimated_mg, event.wash_cycle);
      }
    });
    
    insertMany(events);
    
    return c.json({
      data: {
        events_created: events.length,
        device_id,
        period: {
          start: startDateObj.toISOString().split('T')[0],
          end: new Date().toISOString().split('T')[0],
        }
      }
    });
  }
);

// GET /api/v1/dev/stats - Get database statistics
devRouter.get(
  '/stats',
  async (c) => {
    const db = c.get('db');
    
    const stats = {
      organizations: (db.prepare('SELECT COUNT(*) as count FROM organizations').get() as any).count,
      users: (db.prepare('SELECT COUNT(*) as count FROM users').get() as any).count,
      devices: (db.prepare('SELECT COUNT(*) as count FROM devices').get() as any).count,
      capture_events: (db.prepare('SELECT COUNT(*) as count FROM capture_events').get() as any).count,
      credit_batches: (db.prepare('SELECT COUNT(*) as count FROM credit_batches').get() as any).count,
    };
    
    return c.json({ data: stats });
  }
);
