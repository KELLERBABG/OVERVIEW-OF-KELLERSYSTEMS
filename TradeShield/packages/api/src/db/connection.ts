import Database from 'better-sqlite3';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { mkdirSync, existsSync } from 'fs';

export type AnyRow = Record<string, unknown>;

// Resolve DB path relative to the project root (packages/api/),
// not relative to this source file. This avoids breakage when
// the file is compiled to dist/ or run from different cwd.
const __dirname = dirname(fileURLToPath(import.meta.url));
const PROJECT_ROOT = join(__dirname, '..', '..');

const DB_PATH = process.env.DB_PATH || join(PROJECT_ROOT, 'data', 'washtrap.db');

let dbInstance: Database.Database | null = null;

/**
 * Initialize (or return existing) SQLite connection.
 * Safe to call multiple times — returns the same instance.
 */
export function initDatabase(): Database.Database {
  if (dbInstance) {
    return dbInstance;
  }

  // Ensure data directory exists
  const dbDir = dirname(DB_PATH);
  if (!existsSync(dbDir)) {
    mkdirSync(dbDir, { recursive: true });
  }

  const db = new Database(DB_PATH);

  // Performance pragmas
  db.pragma('journal_mode = WAL');
  db.pragma('foreign_keys = ON');
  db.pragma('busy_timeout = 5000');
  db.pragma('synchronous = NORMAL');

  dbInstance = db;
  console.log(`Database connected: ${DB_PATH}`);
  return db;
}

/**
 * Get the singleton database instance (must call initDatabase first).
 */
export function getDatabase(): Database.Database {
  if (!dbInstance) {
    throw new Error('Database not initialized. Call initDatabase() first.');
  }
  return dbInstance;
}

/**
 * Close the database connection.
 */
export function closeDatabase(): void {
  if (dbInstance) {
    dbInstance.close();
    dbInstance = null;
  }
}

export function getDatabasePath(): string {
  return DB_PATH;
}
