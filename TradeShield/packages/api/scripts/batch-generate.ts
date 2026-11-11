#!/usr/bin/env tsx
// ═══════════════════════════════════════════════════════════════════
// WashTrap Batch Data Generator
// ═══════════════════════════════════════════════════════════════════
//
// Rapidly generates a large number of wash cycle events by
// backfilling historical data. Uses the batch API endpoint
// to send 50 events per request for maximum throughput.
//
// Usage:
//   tsx scripts/batch-generate.ts                         # 500 events, all devices
//   tsx scripts/batch-generate.ts --count 2000 --days 7   # 2000 events over 7 days
//   tsx scripts/batch-generate.ts --device ESP32-BER-001   # single device

import { parseArgs } from 'node:util';

const { values: args } = parseArgs({
  options: {
    'api-url':    { type: 'string',  default: 'http://localhost:4000' },
    'device':     { type: 'string',  default: '' },
    'count':      { type: 'string',  default: '500' },
    'days':       { type: 'string',  default: '7' },
  },
  strict: false,
});

const API_URL = args['api-url']!;
const TARGET_COUNT = parseInt(args['count']!, 10);
const SPAN_DAYS = parseInt(args['days']!, 10);
const BATCH_SIZE = 50;

const DEVICES = [
  { id: 'ESP32-BER-001', type: 'commercial' as const },
  { id: 'ESP32-BER-002', type: 'commercial' as const },
  { id: 'ESP32-BER-003', type: 'commercial' as const },
  { id: 'ESP32-MUC-001', type: 'industrial' as const },
  { id: 'ESP32-MUC-002', type: 'industrial' as const },
];

// ─── Get current max cycle per device ───────────────────────────
async function getMaxCycle(deviceId: string): Promise<number> {
  try {
    const res = await fetch(`${API_URL}/api/v1/events?device_id=${deviceId}&limit=1`, {
      headers: { 'X-API-Key': deviceId },
    });
    const body = await res.json();
    if (body.data && body.data.length > 0) {
      return body.data[0].wash_cycle || 0;
    }
  } catch {}
  return 0;
}

// ─── Generate a batch of historical events ──────────────────────
function generateBatch(
  deviceId: string,
  deviceType: 'commercial' | 'industrial',
  startCycle: number,
  startDate: Date,
  count: number,
): Record<string, any>[] {
  const events: Record<string, any>[] = [];

  for (let i = 0; i < count; i++) {
    // Spread events across the time span
    const offset = Math.random() * SPAN_DAYS * 24 * 60 * 60 * 1000;
    const timestamp = new Date(startDate.getTime() - SPAN_DAYS * 24 * 60 * 60 * 1000 + offset);

    // Only generate events during business hours (6am-10pm)
    const hour = timestamp.getHours();
    if (hour < 6 || hour > 22) {
      timestamp.setHours(6 + Math.floor(Math.random() * 16));
    }
    timestamp.setMinutes(Math.floor(Math.random() * 60));
    timestamp.setSeconds(Math.floor(Math.random() * 60));

    const waterMin = deviceType === 'industrial' ? 40 : 15;
    const waterMax = deviceType === 'industrial' ? 80 : 55;
    const water_volume_l = Math.round((waterMin + Math.random() * (waterMax - waterMin)) * 100) / 100;
    const estimated_mg = Math.round(water_volume_l * 0.08 * 1000) / 1000;

    events.push({
      device_id: deviceId,
      timestamp: timestamp.toISOString(),
      water_volume_l,
      estimated_mg,
      wash_cycle: startCycle + i + 1,
    });
  }

  // Sort by timestamp
  events.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());

  return events;
}

// ─── Send batch to API ──────────────────────────────────────────
async function sendBatch(
  events: Record<string, any>[],
  deviceId: string,
): Promise<{ accepted: number; rejected: number }> {
  try {
    const res = await fetch(`${API_URL}/api/v1/events/batch`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': deviceId,
      },
      body: JSON.stringify({ events }),
    });
    const body = await res.json();
    return { accepted: body.accepted || 0, rejected: body.rejected || 0 };
  } catch (err: any) {
    console.error(`  Network error: ${err.message}`);
    return { accepted: 0, rejected: events.length };
  }
}

// ─── Main ───────────────────────────────────────────────────────
async function main() {
  console.log('WashTrap Batch Data Generator\n');
  console.log(`API: ${API_URL}`);
  console.log(`Target: ${TARGET_COUNT} events over ${SPAN_DAYS} days\n`);

  // Check API
  try {
    const res = await fetch(`${API_URL}/health`);
    if (!res.ok) throw new Error('not ok');
  } catch {
    console.error('Cannot connect to API. Make sure it\'s running: pnpm dev');
    process.exit(1);
  }

  // Select devices
  let devices = DEVICES;
  if (args.device) {
    devices = DEVICES.filter((d) => d.id === args.device);
    if (devices.length === 0) {
      console.error(`Unknown device: ${args.device}`);
      process.exit(1);
    }
  }

  // Get current cycle counts
  console.log('Fetching current cycle counts...');
  const startCycles: Record<string, number> = {};
  for (const d of devices) {
    startCycles[d.id] = await getMaxCycle(d.id);
    console.log(`  ${d.id}: current cycle = ${startCycles[d.id]}`);
  }

  const eventsPerDevice = Math.ceil(TARGET_COUNT / devices.length);
  const startDate = new Date();
  let totalAccepted = 0;
  let totalRejected = 0;

  for (const d of devices) {
    console.log(`\nGenerating ${eventsPerDevice} events for ${d.id}...`);

    const events = generateBatch(d.id, d.type, startCycles[d.id], startDate, eventsPerDevice);

    // Send in batches of BATCH_SIZE
    for (let i = 0; i < events.length; i += BATCH_SIZE) {
      const batch = events.slice(i, i + BATCH_SIZE);
      const batchNum = Math.floor(i / BATCH_SIZE) + 1;
      const totalBatches = Math.ceil(events.length / BATCH_SIZE);

      const result = await sendBatch(batch, d.id);
      totalAccepted += result.accepted;
      totalRejected += result.rejected;

      const progress = Math.round(((i + batch.length) / events.length) * 100);
      process.stdout.write(`\r  Batch ${batchNum}/${totalBatches} — ${progress}% — accepted: ${result.accepted}, rejected: ${result.rejected}`);

      // Small delay between batches to avoid rate limiting
      await new Promise((r) => setTimeout(r, 100));
    }

    process.stdout.write('\n');
  }

  console.log(`\n\nDone!`);
  console.log(`  Total accepted: ${totalAccepted}`);
  console.log(`  Total rejected: ${totalRejected}`);
}

main().catch((err) => {
  console.error(`Fatal: ${err.message}`);
  process.exit(1);
});
