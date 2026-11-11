import { readFileSync, readdirSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';
import { initDatabase, closeDatabase } from './connection.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

/**
 * Migration runner with version tracking.
 *
 * - Maintains a `_migrations` table that records which files have run.
 * - On each run, executes any .sql file in the migrations/ directory
 *   that hasn't been applied yet, in alphabetical order.
 * - The base schema.sql is always applied first (CREATE IF NOT EXISTS).
 */
async function migrate() {
  console.log('Running database migrations...\n');

  const db = initDatabase();

  try {
    // ── Step 1: Ensure _migrations table exists ────────────────
    db.exec(`
      CREATE TABLE IF NOT EXISTS _migrations (
        id          INTEGER PRIMARY KEY AUTOINCREMENT,
        filename    TEXT UNIQUE NOT NULL,
        applied_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
      );
    `);

    // ── Step 2: Apply base schema (idempotent) ─────────────────
    console.log('  Applying base schema (schema.sql)...');
    const schemaPath = join(__dirname, 'schema.sql');
    const schema = readFileSync(schemaPath, 'utf-8');

    // SQLite can handle multiple statements via exec
    db.exec(schema);
    console.log('  ✓ Base schema applied\n');

    // ── Step 3: Apply incremental migrations ───────────────────
    const migrationsDir = join(__dirname, 'migrations');
    let migrationFiles: string[] = [];

    try {
      migrationFiles = readdirSync(migrationsDir)
        .filter((f) => f.endsWith('.sql'))
        .sort(); // Alphabetical = chronological with numbered prefixes
    } catch {
      // migrations/ directory doesn't exist yet — that's fine
    }

    if (migrationFiles.length === 0) {
      console.log('  No incremental migrations found.');
    } else {
      // Get already-applied migrations
      const applied = new Set(
        db.prepare('SELECT filename FROM _migrations').all().map((r: any) => r.filename)
      );

      for (const filename of migrationFiles) {
        if (applied.has(filename)) {
          continue; // Already applied
        }

        const filepath = join(migrationsDir, filename);
        const sql = readFileSync(filepath, 'utf-8');

        console.log(`  Applying migration: ${filename}`);
        db.exec(sql);

        db.prepare('INSERT INTO _migrations (filename) VALUES (?)').run(filename);
        console.log(`  ✓ ${filename} applied`);
      }
    }

    // ── Step 4: Report ─────────────────────────────────────────
    const total = db.prepare('SELECT COUNT(*) as count FROM _migrations').get() as any;
    console.log(`\nMigrations complete. Total applied: ${total.count}`);
  } catch (error) {
    console.error('\nMigration failed:', error);
    process.exit(1);
  } finally {
    closeDatabase();
  }
}

migrate();
