import Database from 'better-sqlite3';
import { readFileSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { nanoid } from 'nanoid';
import { createApp } from '../src/app.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

// ─── Test Database ──────────────────────────────────────────────

export function createTestDatabase(): Database.Database {
  const db = new Database(':memory:');
  db.pragma('foreign_keys = ON');

  // Run schema
  const schemaPath = join(__dirname, '..', 'src', 'db', 'schema.sql');
  const schema = readFileSync(schemaPath, 'utf-8');
  db.exec(schema);

  return db;
}

// ─── Seed Data ──────────────────────────────────────────────────

export interface TestOrg {
  id: string;
  name: string;
  type: 'laundry' | 'brand' | 'admin';
}

export interface TestUser {
  id: string;
  org_id: string;
  email: string;
  name: string;
  role: string;
}

export interface TestDevice {
  id: string;
  org_id: string;
  name: string;
  status: string;
}

export interface TestData {
  laundry: TestOrg;
  brand: TestOrg;
  admin: TestOrg;
  laundryUser: TestUser;
  brandUser: TestUser;
  device1: TestDevice;
  device2: TestDevice;
}

export function seedTestData(db: Database.Database): TestData {
  const laundry = { id: nanoid(), name: 'Test Laundry', type: 'laundry' as const };
  const brand = { id: nanoid(), name: 'Test Brand', type: 'brand' as const };
  const admin = { id: nanoid(), name: 'Test Admin', type: 'admin' as const };

  db.prepare('INSERT INTO organizations (id, name, type) VALUES (?, ?, ?)').run(laundry.id, laundry.name, laundry.type);
  db.prepare('INSERT INTO organizations (id, name, type) VALUES (?, ?, ?)').run(brand.id, brand.name, brand.type);
  db.prepare('INSERT INTO organizations (id, name, type) VALUES (?, ?, ?)').run(admin.id, admin.name, admin.type);

  const laundryUser = { id: nanoid(), org_id: laundry.id, email: 'laundry@test.com', name: 'Laundry User', role: 'admin' };
  const brandUser = { id: nanoid(), org_id: brand.id, email: 'brand@test.com', name: 'Brand User', role: 'admin' };

  db.prepare('INSERT INTO users (id, org_id, email, name, role, password_hash) VALUES (?, ?, ?, ?, ?, ?)').run(
    laundryUser.id, laundryUser.org_id, laundryUser.email, laundryUser.name, laundryUser.role, 'password'
  );
  db.prepare('INSERT INTO users (id, org_id, email, name, role, password_hash) VALUES (?, ?, ?, ?, ?, ?)').run(
    brandUser.id, brandUser.org_id, brandUser.email, brandUser.name, brandUser.role, 'password'
  );

  const device1 = { id: 'TEST-DEVICE-001', org_id: laundry.id, name: 'Test Washer 1', status: 'active' };
  const device2 = { id: 'TEST-DEVICE-002', org_id: laundry.id, name: 'Test Washer 2', status: 'active' };

  db.prepare('INSERT INTO devices (id, org_id, name, status, last_heartbeat) VALUES (?, ?, ?, ?, ?)').run(
    device1.id, device1.org_id, device1.name, device1.status, new Date().toISOString()
  );
  db.prepare('INSERT INTO devices (id, org_id, name, status, last_heartbeat) VALUES (?, ?, ?, ?, ?)').run(
    device2.id, device2.org_id, device2.name, device2.status, new Date().toISOString()
  );

  return { laundry, brand, admin, laundryUser, brandUser, device1, device2 };
}

// ─── App Factory ────────────────────────────────────────────────

export interface TestContext {
  db: Database.Database;
  app: ReturnType<typeof createApp>;
  data: TestData;
}

export function createTestContext(): TestContext {
  const db = createTestDatabase();
  const data = seedTestData(db);
  const app = createApp(db);
  return { db, app, data };
}

// ─── Request Helpers ────────────────────────────────────────────

/**
 * Make a request as a user (via session cookie).
 * We simulate the cookie by looking up the user in the DB
 * and setting the cookie header directly.
 */
export async function asUser(
  app: ReturnType<typeof createApp>,
  user: TestUser,
  method: string,
  path: string,
  body?: any,
) {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'Cookie': `session=${user.id}`,
  };

  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
  }

  return app.request(path, init);
}

/**
 * Make a request as a device (via X-API-Key header).
 */
export async function asDevice(
  app: ReturnType<typeof createApp>,
  device: TestDevice,
  method: string,
  path: string,
  body?: any,
) {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
    'X-API-Key': device.id,
  };

  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
  }

  return app.request(path, init);
}

/**
 * Make an unauthenticated request.
 */
export async function asGuest(
  app: ReturnType<typeof createApp>,
  method: string,
  path: string,
  body?: any,
) {
  const headers: Record<string, string> = {
    'Content-Type': 'application/json',
  };

  const init: RequestInit = { method, headers };
  if (body !== undefined) {
    init.body = JSON.stringify(body);
  }

  return app.request(path, init);
}

// ─── Assertion Helpers ──────────────────────────────────────────

export async function parseJSON(res: Response): Promise<any> {
  return res.json();
}

export function expectOk(res: Response, status = 200) {
  if (res.status !== status) {
    throw new Error(`Expected status ${status}, got ${res.status}`);
  }
}
