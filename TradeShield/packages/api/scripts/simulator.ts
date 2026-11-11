#!/usr/bin/env tsx
// ═══════════════════════════════════════════════════════════════════
// WashTrap ESP32 Firmware Simulator
// ═══════════════════════════════════════════════════════════════════
//
// Simulates one or more ESP32 filter devices sending wash cycle
// events and heartbeats to the WashTrap API.
//
// Usage:
//   tsx scripts/simulator.ts                        # real-time, all devices
//   tsx scripts/simulator.ts --fast 10              # 10x speed
//   tsx scripts/simulator.ts --device ESP32-BER-001 # single device
//   tsx scripts/simulator.ts --cycles 50            # generate 50 cycles then stop
//
// Requires the API to be running on http://localhost:4000 (or --api-url).

import { parseArgs } from 'node:util';

// ─── CLI Arguments ──────────────────────────────────────────────
const { values: args } = parseArgs({
  options: {
    'api-url':    { type: 'string',  default: 'http://localhost:4000' },
    'device':     { type: 'string',  default: '' },       // empty = all devices
    'fast':       { type: 'string',  default: '1' },      // speed multiplier
    'cycles':     { type: 'string',  default: '0' },      // 0 = unlimited
    'heartbeat':  { type: 'string',  default: '300' },    // heartbeat interval (seconds)
    'help':       { type: 'boolean', default: false },
  },
  strict: false,
});

if (args.help) {
  console.log(`
WashTrap ESP32 Simulator

Options:
  --api-url <url>     API base URL (default: http://localhost:4000)
  --device <id>       Simulate a single device (default: all 5 seeded devices)
  --fast <n>          Speed multiplier (default: 1, use 60 for 1-minute-per-hour)
  --cycles <n>        Stop after N total cycles (default: unlimited)
  --heartbeat <s>     Heartbeat interval in seconds (default: 300 = 5 min)
  --help              Show this help

Examples:
  tsx scripts/simulator.ts                          # real-time, all devices
  tsx scripts/simulator.ts --fast 120               # 2 hours → 1 minute
  tsx scripts/simulator.ts --device ESP32-BER-001 --cycles 20
`);
  process.exit(0);
}

const API_URL = args['api-url']!;
const SPEED = parseInt(args['fast']!, 10);
const MAX_CYCLES = parseInt(args['cycles']!, 10);
const HEARTBEAT_SEC = parseInt(args['heartbeat']!, 10);

// ─── Device Definitions ─────────────────────────────────────────
// Matches the seeded devices in seed.ts
interface DeviceProfile {
  id: string;
  name: string;
  location: string;
  type: 'commercial' | 'industrial';  // affects water volume range
  firmware: string;
  initialBattery: number;
  batteryDrainPerCycle: number;       // % per wash cycle
}

const DEVICE_PROFILES: DeviceProfile[] = [
  {
    id: 'ESP32-BER-001', name: 'Waschmaschine #1', location: 'Berlin',
    type: 'commercial', firmware: '1.2.0', initialBattery: 72, batteryDrainPerCycle: 0.3,
  },
  {
    id: 'ESP32-BER-002', name: 'Waschmaschine #2', location: 'Berlin',
    type: 'commercial', firmware: '1.2.0', initialBattery: 85, batteryDrainPerCycle: 0.25,
  },
  {
    id: 'ESP32-BER-003', name: 'Waschmaschine #3', location: 'Berlin',
    type: 'commercial', firmware: '1.1.0', initialBattery: 15, batteryDrainPerCycle: 0.35,
  },
  {
    id: 'ESP32-MUC-001', name: 'Industrial Washer A', location: 'München',
    type: 'industrial', firmware: '1.2.0', initialBattery: 90, batteryDrainPerCycle: 0.2,
  },
  {
    id: 'ESP32-MUC-002', name: 'Industrial Washer B', location: 'München',
    type: 'industrial', firmware: '1.2.0', initialBattery: 58, batteryDrainPerCycle: 0.2,
  },
];

// ─── Simulation State ───────────────────────────────────────────
interface DeviceState {
  profile: DeviceProfile;
  cycleCount: number;
  battery: number;
  lastHeartbeat: number;
  totalEvents: number;
  totalErrors: number;
  consecutiveErrors: number;
}

// ─── Random Helpers ─────────────────────────────────────────────
function randomBetween(min: number, max: number): number {
  return min + Math.random() * (max - min);
}

function randomInt(min: number, max: number): number {
  return Math.floor(randomBetween(min, max + 1));
}

// ─── Wash Cycle Generation ──────────────────────────────────────
// Simulates realistic wash cycle parameters based on time of day,
// day of week, and machine type.

function isWeekend(): boolean {
  const day = new Date().getDay();
  return day === 0 || day === 6;
}

function getHour(): number {
  return new Date().getHours();
}

/** Generate a single wash cycle event */
function generateCycleEvent(state: DeviceState): {
  device_id: string;
  timestamp: string;
  water_volume_l: number;
  estimated_mg: number;
  wash_cycle: number;
} {
  const hour = getHour();
  const weekend = isWeekend();

  // Water volume: depends on machine type and time
  let waterMin: number, waterMax: number;
  if (state.profile.type === 'industrial') {
    waterMin = 40;
    waterMax = 80;
  } else {
    waterMin = 15;
    waterMax = 55;
  }

  // Morning cycles tend to use more water (heavier loads)
  if (hour >= 7 && hour <= 10) {
    waterMin += 5;
    waterMax += 10;
  }

  const water_volume_l = Math.round(randomBetween(waterMin, waterMax) * 100) / 100;

  // Microplastic estimation: conservative ratio (0.08 mg/L)
  const estimated_mg = Math.round(water_volume_l * 0.08 * 1000) / 1000;

  state.cycleCount++;

  return {
    device_id: state.profile.id,
    timestamp: new Date().toISOString(),
    water_volume_l,
    estimated_mg,
    wash_cycle: state.cycleCount,
  };
}

/** Calculate delay (ms) until next wash cycle, based on time of day */
function getCycleDelay(): number {
  const hour = getHour();

  // Commercial laundry: cycles take 20-45 minutes
  // But we adjust frequency based on time of day
  let baseMinMinutes: number;
  let baseMaxMinutes: number;

  if (hour >= 6 && hour <= 10) {
    // Morning peak: back-to-back cycles
    baseMinMinutes = 15;
    baseMaxMinutes = 30;
  } else if (hour >= 11 && hour <= 14) {
    // Midday: moderate
    baseMinMinutes = 25;
    baseMaxMinutes = 45;
  } else if (hour >= 15 && hour <= 19) {
    // Afternoon: moderate-high
    baseMinMinutes = 20;
    baseMaxMinutes = 40;
  } else if (hour >= 20 && hour <= 22) {
    // Evening: winding down
    baseMinMinutes = 30;
    baseMaxMinutes = 60;
  } else {
    // Night (23-5): very few cycles
    baseMinMinutes = 60;
    baseMaxMinutes = 180;
  }

  // Weekend: fewer cycles
  if (isWeekend()) {
    baseMinMinutes *= 1.5;
    baseMaxMinutes *= 1.5;
  }

  const minutes = randomBetween(baseMinMinutes, baseMaxMinutes);
  return Math.round(minutes * 60 * 1000);
}

// ─── API Communication ──────────────────────────────────────────

async function postEvent(event: Record<string, any>, apiKey: string): Promise<{
  ok: boolean;
  status: number;
  data?: any;
  error?: string;
}> {
  try {
    const res = await fetch(`${API_URL}/api/v1/events`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': apiKey,
      },
      body: JSON.stringify(event),
    });

    const body = await res.json();

    if (!res.ok) {
      return { ok: false, status: res.status, error: body.error || body.message };
    }

    return { ok: true, status: res.status, data: body };
  } catch (err: any) {
    return { ok: false, status: 0, error: `Network error: ${err.message}` };
  }
}

async function postBatch(events: Record<string, any>[], apiKey: string): Promise<{
  ok: boolean;
  status: number;
  accepted: number;
  rejected: number;
  errors: string[];
}> {
  try {
    const res = await fetch(`${API_URL}/api/v1/events/batch`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': apiKey,
      },
      body: JSON.stringify({ events }),
    });

    const body = await res.json();

    if (!res.ok) {
      return { ok: false, status: res.status, accepted: 0, rejected: events.length, errors: [body.error] };
    }

    return {
      ok: true,
      status: res.status,
      accepted: body.accepted || 0,
      rejected: body.rejected || 0,
      errors: (body.errors || []).map((e: any) => `${e.wash_cycle}: ${e.error}`),
    };
  } catch (err: any) {
    return { ok: false, status: 0, accepted: 0, rejected: events.length, errors: [`Network: ${err.message}`] };
  }
}

async function postHeartbeat(state: DeviceState): Promise<boolean> {
  try {
    const res = await fetch(`${API_URL}/api/v1/devices/${state.profile.id}/heartbeat`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'X-API-Key': state.profile.id,
      },
      body: JSON.stringify({
        device_id: state.profile.id,
        timestamp: new Date().toISOString(),
        firmware_version: state.profile.firmware,
        battery_level: Math.round(state.battery),
      }),
    });
    return res.ok;
  } catch {
    return false;
  }
}

// ─── Terminal Dashboard ─────────────────────────────────────────
// ANSI escape codes for live updating terminal display.

const ESC = '\x1b';
const CLEAR = `${ESC}[2J${ESC}[H`;
const BOLD = `${ESC}[1m`;
const DIM = `${ESC}[2m`;
const RED = `${ESC}[31m`;
const GREEN = `${ESC}[32m`;
const YELLOW = `${ESC}[33m`;
const CYAN = `${ESC}[36m`;
const WHITE = `${ESC}[37m`;
const RESET = `${ESC}[0m`;

function formatDuration(ms: number): string {
  const seconds = Math.floor(ms / 1000);
  const minutes = Math.floor(seconds / 60);
  const hours = Math.floor(minutes / 60);
  if (hours > 0) return `${hours}h ${minutes % 60}m`;
  if (minutes > 0) return `${minutes}m ${seconds % 60}s`;
  return `${seconds}s`;
}

function batteryColor(level: number): string {
  if (level > 50) return GREEN;
  if (level > 20) return YELLOW;
  return RED;
}

function renderDashboard(states: DeviceState[], startTime: number, totalSent: number, totalErrors: number) {
  const elapsed = Date.now() - startTime;
  const now = new Date();

  let output = CLEAR;
  output += `${BOLD}${CYAN}╔══════════════════════════════════════════════════════════════════╗${RESET}\n`;
  output += `${BOLD}${CYAN}║  💧 WashTrap ESP32 Simulator                                    ║${RESET}\n`;
  output += `${BOLD}${CYAN}╚══════════════════════════════════════════════════════════════════╝${RESET}\n\n`;

  output += `  ${DIM}API:${RESET} ${API_URL}  ${DIM}Speed:${RESET} ${SPEED}x  ${DIM}Time:${RESET} ${now.toLocaleTimeString()}  ${DIM}Running:${RESET} ${formatDuration(elapsed)}\n\n`;

  // Header
  output += `  ${BOLD}${WHITE}Device                  Type       Battery  Cycles  Events  Errors${RESET}\n`;
  output += `  ${DIM}─────────────────────── ────────── ──────── ─────── ─────── ──────${RESET}\n`;

  for (const s of states) {
    const bat = Math.round(s.battery);
    const batColor = batteryColor(bat);
    const batBar = '█'.repeat(Math.ceil(bat / 10)) + '░'.repeat(10 - Math.ceil(bat / 10));

    output += `  ${s.profile.name.padEnd(24)} `;
    output += `${s.profile.type === 'industrial' ? '🏭 Ind.' : '🏠 Com. '}  `;
    output += `${batColor}${batBar} ${String(bat).padStart(3)}%${RESET}  `;
    output += `${String(s.cycleCount).padStart(6)}  `;
    output += `${GREEN}${String(s.totalEvents).padStart(6)}${RESET}  `;
    output += `${s.totalErrors > 0 ? RED : DIM}${String(s.totalErrors).padStart(5)}${RESET}\n`;
  }

  output += `\n  ${DIM}──────────────────────────────────────────────────────────────${RESET}\n`;
  output += `  ${BOLD}Total sent:${RESET} ${GREEN}${totalSent}${RESET}  `;
  output += `${BOLD}Errors:${RESET} ${totalErrors > 0 ? RED : DIM}${totalErrors}${RESET}  `;
  output += `${BOLD}Rate:${RESET} ${totalSent > 0 ? (totalSent / (elapsed / 1000 / 60)).toFixed(1) : 0} events/min\n`;

  output += `\n  ${DIM}Press Ctrl+C to stop${RESET}\n`;

  process.stdout.write(output);
}

// ─── Connectivity Check ─────────────────────────────────────────
async function checkApiConnection(): Promise<boolean> {
  try {
    const res = await fetch(`${API_URL}/health`);
    return res.ok;
  } catch {
    return false;
  }
}

// ─── Main Simulation Loop ───────────────────────────────────────
async function main() {
  console.log(`${BOLD}${CYAN}WashTrap ESP32 Simulator${RESET}`);
  console.log(`${DIM}Connecting to ${API_URL}...${RESET}\n`);

  const connected = await checkApiConnection();
  if (!connected) {
    console.error(`${RED}✗ Cannot connect to API at ${API_URL}${RESET}`);
    console.error(`${DIM}  Make sure the API is running: pnpm dev${RESET}`);
    process.exit(1);
  }
  console.log(`${GREEN}✓ Connected to API${RESET}\n`);

  // Select devices to simulate
  let profiles = DEVICE_PROFILES;
  if (args.device) {
    profiles = DEVICE_PROFILES.filter((p) => p.id === args.device);
    if (profiles.length === 0) {
      console.error(`${RED}✗ Unknown device: ${args.device}${RESET}`);
      console.error(`${DIM}  Available: ${DEVICE_PROFILES.map((p) => p.id).join(', ')}${RESET}`);
      process.exit(1);
    }
  }

  // Initialize state for each device
  const states: DeviceState[] = profiles.map((p) => ({
    profile: p,
    cycleCount: 0,
    battery: p.initialBattery,
    lastHeartbeat: 0,
    totalEvents: 0,
    totalErrors: 0,
    consecutiveErrors: 0,
  }));

  const startTime = Date.now();
  let globalCycles = 0;
  let globalErrors = 0;

  // Heartbeat interval
  const heartbeatMs = HEARTBEAT_SEC * 1000 / SPEED;

  // Schedule heartbeats for all devices
  const heartbeatIntervals = states.map((s) => {
    return setInterval(async () => {
      await postHeartbeat(s);
      s.lastHeartbeat = Date.now();
    }, heartbeatMs);
  });

  // Per-device cycle generators
  const deviceGenerators = states.map(async (state) => {
    while (MAX_CYCLES === 0 || globalCycles < MAX_CYCLES) {
      const delay = getCycleDelay();

      // In fast mode, compress the delay
      const actualDelay = Math.max(50, delay / SPEED);

      await new Promise((r) => setTimeout(r, actualDelay));

      if (MAX_CYCLES > 0 && globalCycles >= MAX_CYCLES) break;

      // Generate event
      const event = generateCycleEvent(state);
      globalCycles++;

      // Send to API
      const result = await postEvent(event, state.profile.id);

      if (result.ok) {
        state.totalEvents++;
        state.consecutiveErrors = 0;
        // Drain battery slightly
        state.battery = Math.max(0, state.battery - state.profile.batteryDrainPerCycle);
      } else {
        state.totalErrors++;
        globalErrors++;
        state.consecutiveErrors++;

        // If rate limited, back off
        if (result.status === 429) {
          const backoff = 5000 / SPEED;
          await new Promise((r) => setTimeout(r, backoff));
        }

        // If too many consecutive errors, slow down
        if (state.consecutiveErrors > 5) {
          const backoff = (10000 * state.consecutiveErrors) / SPEED;
          await new Promise((r) => setTimeout(r, Math.min(backoff, 30000)));
        }
      }

      // Render dashboard every cycle (throttled in fast mode)
      if (SPEED <= 10 || globalCycles % Math.ceil(SPEED / 5) === 0) {
        renderDashboard(states, startTime, globalCycles, globalErrors);
      }
    }
  });

  // Handle Ctrl+C gracefully
  let stopping = false;
  process.on('SIGINT', () => {
    if (stopping) process.exit(0);
    stopping = true;

    // Clear heartbeat intervals
    for (const interval of heartbeatIntervals) {
      clearInterval(interval);
    }

    // Final render
    renderDashboard(states, startTime, globalCycles, globalErrors);

    console.log(`\n${YELLOW}Simulation stopped.${RESET}`);
    console.log(`${DIM}Total events: ${globalCycles}, Errors: ${globalErrors}${RESET}`);
    process.exit(0);
  });

  // Wait for all generators to finish (or Ctrl+C)
  await Promise.all(deviceGenerators);

  // Cleanup
  for (const interval of heartbeatIntervals) {
    clearInterval(interval);
  }

  renderDashboard(states, startTime, globalCycles, globalErrors);
  console.log(`\n${GREEN}Simulation complete!${RESET}`);
  console.log(`${DIM}Total events: ${globalCycles}, Errors: ${globalErrors}${RESET}`);
}

main().catch((err) => {
  console.error(`${RED}Fatal error: ${err.message}${RESET}`);
  process.exit(1);
});
