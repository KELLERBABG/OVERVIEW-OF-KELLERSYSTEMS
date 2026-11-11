import { nanoid } from 'nanoid';
import { initDatabase, closeDatabase } from './connection.js';

// ─── Configuration ──────────────────────────────────────────────
const MONTHS_BACK = 6;
const CREDIT_MONTHS_BACK = 6; // Credit periods to generate

// Microplastic constants (from shared/constants.ts, duplicated to avoid import issues at seed time)
const CONSERVATIVE_MG_PER_LITER = 0.08;
const CREDIT_PRICE_PER_MG_EUR = 0.001;

// ─── Deterministic-ish random helpers ────────────────────────────
// Seeded PRNG so seed data is reproducible across runs.
let _seed = 42;
function seededRandom(): number {
  _seed = (_seed * 16807 + 0) % 2147483647;
  return (_seed - 1) / 2147483646;
}

function randomBetween(min: number, max: number): number {
  return min + seededRandom() * (max - min);
}

function randomInt(min: number, max: number): number {
  return Math.floor(randomBetween(min, max + 1));
}

function dateToISO(d: Date): string {
  return d.toISOString();
}

function dateOnly(d: Date): string {
  return d.toISOString().split('T')[0];
}

// ─── Seed ───────────────────────────────────────────────────────
async function seed() {
  console.log('Seeding database...\n');

  const db = initDatabase();

  try {
    // ── Clear existing data (fresh seed) ───────────────────────
    console.log('  Clearing existing data...');
    db.exec(`
      DELETE FROM credit_audit_log;
      DELETE FROM device_health_checks;
      DELETE FROM credit_batches;
      DELETE FROM capture_events;
      DELETE FROM devices;
      DELETE FROM users;
      DELETE FROM organizations;
      DELETE FROM _migrations;
    `);

    // ── Organizations ──────────────────────────────────────────
    console.log('  Creating organizations...');
    const laundry1 = { id: nanoid(), name: 'Berlin Laundromat GmbH', type: 'laundry' };
    const laundry2 = { id: nanoid(), name: 'München Wash Co.', type: 'laundry' };
    const brand1 = { id: nanoid(), name: 'EcoTextile GmbH', type: 'brand' };
    const brand2 = { id: nanoid(), name: 'GreenFashion AG', type: 'brand' };
    const admin = { id: nanoid(), name: 'WashTrap Admin', type: 'admin' };
    const orgs = [laundry1, laundry2, brand1, brand2, admin];

    const insertOrg = db.prepare(
      'INSERT INTO organizations (id, name, type) VALUES (?, ?, ?)'
    );
    for (const org of orgs) {
      insertOrg.run(org.id, org.name, org.type);
    }

    // ── Users ──────────────────────────────────────────────────
    console.log('  Creating users...');
    const users = [
      { id: nanoid(), org_id: laundry1.id, email: 'admin@berlin-laundry.de', name: 'Hans Schmidt', role: 'admin' },
      { id: nanoid(), org_id: laundry1.id, email: 'maria@berlin-laundry.de', name: 'Maria Weber', role: 'member' },
      { id: nanoid(), org_id: laundry1.id, email: 'karl@berlin-laundry.de', name: 'Klaus Fischer', role: 'viewer' },
      { id: nanoid(), org_id: laundry2.id, email: 'info@muenchen-wash.de', name: 'Thomas Müller', role: 'admin' },
      { id: nanoid(), org_id: laundry2.id, email: 'sophie@muenchen-wash.de', name: 'Sophie Bauer', role: 'member' },
      { id: nanoid(), org_id: brand1.id, email: 'csr@ecotextile.de', name: 'Lisa Fischer', role: 'admin' },
      { id: nanoid(), org_id: brand1.id, email: 'jana@ecotextile.de', name: 'Jana Krause', role: 'member' },
      { id: nanoid(), org_id: brand2.id, email: 'sustainability@greenfashion.de', name: 'Anna Becker', role: 'admin' },
      { id: nanoid(), org_id: admin.id, email: 'admin@washtrap.io', name: 'WashTrap Admin', role: 'admin' },
    ];

    const insertUser = db.prepare(
      'INSERT INTO users (id, org_id, email, name, role, password_hash) VALUES (?, ?, ?, ?, ?, ?)'
    );
    for (const user of users) {
      insertUser.run(user.id, user.org_id, user.email, user.name, user.role, 'password');
    }

    // ── Devices ────────────────────────────────────────────────
    console.log('  Creating devices...');
    const now = new Date();
    const devices = [
      // Berlin Laundromat — 3 commercial washers, different ages and battery states
      {
        id: 'ESP32-BER-001', org_id: laundry1.id, name: 'Waschmaschine #1',
        location: 'Hauptstraße 12, Berlin', installed_days_ago: 150,
        status: 'active', battery: 72, firmware: '1.2.0',
      },
      {
        id: 'ESP32-BER-002', org_id: laundry1.id, name: 'Waschmaschine #2',
        location: 'Hauptstraße 12, Berlin', installed_days_ago: 120,
        status: 'active', battery: 85, firmware: '1.2.0',
      },
      {
        id: 'ESP32-BER-003', org_id: laundry1.id, name: 'Waschmaschine #3',
        location: 'Hauptstraße 12, Berlin', installed_days_ago: 45,
        status: 'maintenance', battery: 15, firmware: '1.1.0',
      },
      // München Wash Co. — 2 industrial washers
      {
        id: 'ESP32-MUC-001', org_id: laundry2.id, name: 'Industrial Washer A',
        location: 'Marienplatz 5, München', installed_days_ago: 100,
        status: 'active', battery: 90, firmware: '1.2.0',
      },
      {
        id: 'ESP32-MUC-002', org_id: laundry2.id, name: 'Industrial Washer B',
        location: 'Marienplatz 5, München', installed_days_ago: 60,
        status: 'active', battery: 58, firmware: '1.2.0',
      },
    ];

    const insertDevice = db.prepare(`
      INSERT INTO devices (id, org_id, name, location, installed_at, status, firmware_version, battery_level, last_heartbeat, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    for (const d of devices) {
      const installedAt = new Date(now);
      installedAt.setDate(installedAt.getDate() - d.installed_days_ago);

      // Last heartbeat: active devices checked in recently, maintenance one hasn't
      let lastHeartbeat: Date;
      if (d.status === 'maintenance') {
        lastHeartbeat = new Date(now);
        lastHeartbeat.setDate(lastHeartbeat.getDate() - 2); // 2 days ago
      } else {
        lastHeartbeat = new Date(now);
        lastHeartbeat.setHours(lastHeartbeat.getHours() - randomInt(1, 12)); // 1-12 hours ago
      }

      insertDevice.run(
        d.id, d.org_id, d.name, d.location,
        dateToISO(installedAt), d.status, d.firmware,
        d.battery, dateToISO(lastHeartbeat), dateToISO(installedAt)
      );
    }

    // ── Capture Events ─────────────────────────────────────────
    console.log('  Generating 6 months of capture events...');
    const insertEvent = db.prepare(`
      INSERT INTO capture_events (device_id, timestamp, water_volume_l, estimated_mg, wash_cycle, received_at)
      VALUES (?, ?, ?, ?, ?, ?)
    `);

    let totalEvents = 0;

    // Per-device cycle counters (each device maintains its own counter)
    const deviceCycles: Record<string, number> = {};
    for (const d of devices) {
      deviceCycles[d.id] = 0;
    }

    const startDate = new Date(now);
    startDate.setMonth(startDate.getMonth() - MONTHS_BACK);
    startDate.setHours(0, 0, 0, 0);

    const currentDate = new Date(startDate);

    while (currentDate <= now) {
      const dayOfWeek = currentDate.getDay(); // 0=Sun, 6=Sat
      const isWeekend = dayOfWeek === 0 || dayOfWeek === 6;
      const month = currentDate.getMonth(); // 0-11

      // Seasonal modifier: summer has slightly more laundry
      const seasonalMod = (month >= 4 && month <= 8) ? 1.15 : 1.0;

      for (const d of devices) {
        // Skip events for devices not yet installed
        const installedAt = new Date();
        installedAt.setDate(installedAt.getDate() - d.installed_days_ago);
        if (currentDate < installedAt) continue;

        // Skip events for maintenance days (intermittent)
        if (d.status === 'maintenance' && seededRandom() < 0.3) continue;

        // Events per day: weekdays have more, weekends less
        // Commercial laundry runs ~8-14 cycles/weekday, ~3-6 on weekends
        let baseCycles = isWeekend ? randomBetween(3, 6) : randomBetween(8, 14);
        baseCycles *= seasonalMod;

        // Some days a machine might be down (5% chance)
        if (seededRandom() < 0.05) baseCycles = 0;

        const numEvents = Math.round(baseCycles);

        for (let i = 0; i < numEvents; i++) {
          deviceCycles[d.id]++;

          // Time distribution: commercial laundry peaks 7-10am and 1-4pm
          let hour: number;
          const timeRoll = seededRandom();
          if (timeRoll < 0.35) {
            hour = randomInt(7, 10);   // Morning peak
          } else if (timeRoll < 0.70) {
            hour = randomInt(13, 16);  // Afternoon peak
          } else {
            hour = randomInt(6, 20);   // Spread throughout day
          }
          const minute = randomInt(0, 59);
          const second = randomInt(0, 59);

          const timestamp = new Date(currentDate);
          timestamp.setHours(hour, minute, second, 0);

          // Water volume: industrial washers use more
          // Commercial front-loader: 15-50L typical, industrial: 40-80L
          const isIndustrial = d.name?.includes('Industrial');
          const waterMin = isIndustrial ? 40 : 15;
          const waterMax = isIndustrial ? 80 : 55;
          const water_volume_l = Math.round(randomBetween(waterMin, waterMax) * 100) / 100;

          // Microplastic estimation: conservative ratio
          const estimated_mg = Math.round(water_volume_l * CONSERVATIVE_MG_PER_LITER * 1000) / 1000;

          // received_at is shortly after timestamp (simulates network delay)
          const receivedAt = new Date(timestamp.getTime() + randomInt(500, 5000));

          insertEvent.run(
            d.id,
            dateToISO(timestamp),
            water_volume_l,
            estimated_mg,
            deviceCycles[d.id],
            dateToISO(receivedAt)
          );
          totalEvents++;
        }
      }

      currentDate.setDate(currentDate.getDate() + 1);
    }

    console.log(`  ✓ ${totalEvents.toLocaleString()} capture events created`);

    // ── Credit Batches ─────────────────────────────────────────
    console.log('  Creating credit batches...');
    const insertCredit = db.prepare(`
      INSERT INTO credit_batches (id, org_id, source_org_id, period_start, period_end, total_mg, total_eur, status, issued_at, redeemed_at, created_at)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
    `);

    // Generate monthly credit periods
    const creditStart = new Date(now);
    creditStart.setMonth(creditStart.getMonth() - CREDIT_MONTHS_BACK);
    creditStart.setDate(1); // Start of month

    let totalCredits = 0;

    for (let m = 0; m < CREDIT_MONTHS_BACK; m++) {
      const periodStart = new Date(creditStart);
      periodStart.setMonth(periodStart.getMonth() + m);

      const periodEnd = new Date(periodStart);
      periodEnd.setMonth(periodEnd.getMonth() + 1);
      periodEnd.setDate(0); // Last day of month

      // Skip future months
      if (periodStart > now) break;

      // For each laundry, create credits for EcoTextile (brand1)
      for (const laundry of [laundry1, laundry2]) {
        // Calculate actual capture for this period
        const stats = db.prepare(`
          SELECT
            COALESCE(SUM(ce.estimated_mg), 0) as total_mg,
            COUNT(*) as total_cycles,
            COALESCE(SUM(ce.water_volume_l), 0) as total_water_l
          FROM capture_events ce
          JOIN devices d ON ce.device_id = d.id
          WHERE d.org_id = ?
            AND ce.timestamp >= ?
            AND ce.timestamp <= ?
        `).get(laundry.id, dateToISO(periodStart), dateOnly(periodEnd) + 'T23:59:59') as any;

        if (stats.total_mg <= 0) continue;

        const total_eur = Math.round(stats.total_mg * CREDIT_PRICE_PER_MG_EUR * 100) / 100;
        const creditId = nanoid();

        // Determine credit status based on age
        const ageMonths = (now.getTime() - periodEnd.getTime()) / (30 * 24 * 60 * 60 * 1000);
        let status: string;
        let issuedAt: string | null = null;
        let redeemedAt: string | null = null;

        if (ageMonths > 4) {
          status = 'redeemed';
          issuedAt = dateToISO(new Date(periodEnd.getTime() + 3 * 24 * 60 * 60 * 1000));
          redeemedAt = dateToISO(new Date(periodEnd.getTime() + 25 * 24 * 60 * 60 * 1000));
        } else if (ageMonths > 1) {
          status = 'issued';
          issuedAt = dateToISO(new Date(periodEnd.getTime() + 2 * 24 * 60 * 60 * 1000));
        } else {
          status = 'pending';
        }

        insertCredit.run(
          creditId, brand1.id, laundry.id,
          dateOnly(periodStart), dateOnly(periodEnd),
          stats.total_mg, total_eur,
          status, issuedAt, redeemedAt,
          dateToISO(periodEnd)
        );

        // Audit log entries
        db.prepare(`
          INSERT INTO credit_audit_log (credit_id, action, from_status, to_status, performed_by, notes, created_at)
          VALUES (?, 'created', NULL, 'pending', 'system', ?, ?)
        `).run(
          creditId,
          `Auto-generated for ${dateOnly(periodStart)}–${dateOnly(periodEnd)}: ${stats.total_cycles} cycles, ${stats.total_mg.toFixed(1)} mg`,
          dateToISO(periodEnd)
        );

        if (status === 'issued' || status === 'redeemed') {
          db.prepare(`
            INSERT INTO credit_audit_log (credit_id, action, from_status, to_status, performed_by, notes, created_at)
            VALUES (?, 'issued', 'pending', 'issued', ?, NULL, ?)
          `).run(creditId, users.find(u => u.org_id === brand1.id && u.role === 'admin')?.id || 'system', issuedAt!);
        }

        if (status === 'redeemed') {
          db.prepare(`
            INSERT INTO credit_audit_log (credit_id, action, from_status, to_status, performed_by, notes, created_at)
            VALUES (?, 'redeemed', 'issued', 'redeemed', ?, 'CSR Q-report verified', ?)
          `).run(creditId, users.find(u => u.org_id === brand1.id && u.role === 'admin')?.id || 'system', redeemedAt!);
        }

        totalCredits++;
      }
    }

    console.log(`  ✓ ${totalCredits} credit batches created with audit entries`);

    // ── Device Health Checks ───────────────────────────────────
    console.log('  Generating device health checks...');
    const insertHealth = db.prepare(`
      INSERT OR IGNORE INTO device_health_checks (device_id, check_date, event_count, is_online)
      VALUES (?, ?, ?, ?)
    `);

    let totalHealthChecks = 0;

    for (const d of devices) {
      const healthStart = new Date(now);
      healthStart.setDate(healthStart.getDate() - 30); // Last 30 days only

      const healthDate = new Date(healthStart);
      while (healthDate <= now) {
        // Check if device was installed by this date
        const installedAt = new Date();
        installedAt.setDate(installedAt.getDate() - d.installed_days_ago);
        if (healthDate < installedAt) {
          healthDate.setDate(healthDate.getDate() + 1);
          continue;
        }

        const dateStr = dateOnly(healthDate);

        // Count events for this device on this day
        const dayStats = db.prepare(`
          SELECT COUNT(*) as count FROM capture_events
          WHERE device_id = ? AND date(timestamp) = ?
        `).get(d.id, dateStr) as any;

        // Maintenance device goes offline sometimes
        const isOnline = d.status === 'maintenance' && seededRandom() < 0.4 ? 0 : 1;

        insertHealth.run(d.id, dateStr, dayStats.count, isOnline);
        totalHealthChecks++;

        healthDate.setDate(healthDate.getDate() + 1);
      }
    }

    console.log(`  ✓ ${totalHealthChecks} device health checks created`);

    // ── Summary ────────────────────────────────────────────────
    console.log('\nSeeding complete!\n');

    const summary = {
      organizations: db.prepare('SELECT COUNT(*) as c FROM organizations').get() as any,
      users: db.prepare('SELECT COUNT(*) as c FROM users').get() as any,
      devices: db.prepare('SELECT COUNT(*) as c FROM devices').get() as any,
      capture_events: db.prepare('SELECT COUNT(*) as c FROM capture_events').get() as any,
      credit_batches: db.prepare('SELECT COUNT(*) as c FROM credit_batches').get() as any,
      audit_entries: db.prepare('SELECT COUNT(*) as c FROM credit_audit_log').get() as any,
      health_checks: db.prepare('SELECT COUNT(*) as c FROM device_health_checks').get() as any,
    };

    console.log('┌─────────────────────────┬──────────┐');
    console.log('│ Table                   │ Rows     │');
    console.log('├─────────────────────────┼──────────┤');
    for (const [table, data] of Object.entries(summary)) {
      console.log(`│ ${table.padEnd(24)}│ ${String(data.c).padStart(8)} │`);
    }
    console.log('└─────────────────────────┴──────────┘');

    // Show per-device event totals
    console.log('\nPer-device capture totals:');
    const deviceTotals = db.prepare(`
      SELECT
        d.name,
        d.id,
        COUNT(ce.id) as cycles,
        ROUND(COALESCE(SUM(ce.estimated_mg), 0), 1) as total_mg,
        ROUND(COALESCE(SUM(ce.water_volume_l), 0), 0) as total_water_l
      FROM devices d
      LEFT JOIN capture_events ce ON d.id = ce.device_id
      GROUP BY d.id
      ORDER BY total_mg DESC
    `).all() as any[];

    for (const dt of deviceTotals) {
      console.log(`  ${dt.name || dt.id}: ${dt.cycles} cycles, ${dt.total_mg} mg, ${dt.total_water_l}L`);
    }

  } catch (error) {
    console.error('Seeding failed:', error);
    process.exit(1);
  } finally {
    closeDatabase();
  }
}

seed();
