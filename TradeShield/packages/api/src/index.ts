import { serve } from '@hono/node-server';
import { initDatabase } from './db/connection.js';
import { createApp } from './app.js';

const db = initDatabase();
const app = createApp(db);

// ─── Start server ───────────────────────────────────────────────
const port = parseInt(process.env.API_PORT || '4000');

serve({
  fetch: app.fetch,
  port,
}, (info) => {
  console.log(`WashTrap API running on http://localhost:${info.port}`);
  console.log(`Environment: ${process.env.NODE_ENV || 'development'}`);
});

// ─── Graceful shutdown ──────────────────────────────────────────
function shutdown(signal: string) {
  console.log(`\n${signal} received. Shutting down...`);
  db.close();
  process.exit(0);
}

process.on('SIGINT', () => shutdown('SIGINT'));
process.on('SIGTERM', () => shutdown('SIGTERM'));

// ─── Hono context type augmentation ─────────────────────────────
declare module 'hono' {
  interface ContextVariableMap {
    db: import('better-sqlite3').Database;
    user: {
      id: string;
      org_id: string;
      email: string;
      name: string | null;
      role: string;
      org_name: string;
      org_type: string;
    } | undefined;
    device: {
      id: string;
      org_id: string;
      name: string | null;
      status: string;
      org_name: string;
    } | undefined;
    org_id: string | undefined;
    requestId: string;
  }
}
