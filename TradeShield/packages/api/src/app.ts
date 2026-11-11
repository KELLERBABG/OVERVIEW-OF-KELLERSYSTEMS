import { Hono } from 'hono';
import { cors } from 'hono/cors';
import { logger } from 'hono/logger';
import Database from 'better-sqlite3';
import { eventsRouter } from './routes/events.js';
import { devicesRouter } from './routes/devices.js';
import { dashboardRouter } from './routes/dashboard.js';
import { creditsRouter } from './routes/credits.js';
import { authRouter } from './routes/auth.js';
import { devRouter } from './routes/dev.js';
import { authMiddleware } from './middleware/auth.js';
import { errorHandler } from './middleware/error-handler.js';
import { requestId } from './middleware/request-id.js';
import { apiRateLimit } from './middleware/rate-limit.js';

/**
 * Create a Hono app instance wired to a specific database.
 * This allows tests to use an in-memory or temp database
 * without affecting the production singleton.
 */
export function createApp(db: Database.Database): Hono {
  const app = new Hono();

  // ─── Global middleware ──────────────────────────────────────
  app.use('*', errorHandler);
  app.use('*', requestId);
  app.use('*', cors({
    origin: ['http://localhost:5173', 'http://localhost:3000'],
    credentials: true,
    exposeHeaders: ['X-Request-Id', 'X-RateLimit-Limit', 'X-RateLimit-Remaining', 'X-RateLimit-Reset'],
  }));
  app.use('*', logger());

  // Inject database into context
  app.use('*', async (c, next) => {
    c.set('db', db);
    await next();
  });

  // ─── Public routes ──────────────────────────────────────────
  app.route('/api/v1/auth', authRouter);
  app.route('/api/v1/dev', devRouter);

  app.get('/health', (c) => {
    return c.json({ status: 'ok', timestamp: new Date().toISOString() });
  });

  // ─── Protected routes ───────────────────────────────────────
  app.use('/api/v1/dashboard/*', authMiddleware);
  app.use('/api/v1/dashboard/*', apiRateLimit);
  app.route('/api/v1/dashboard', dashboardRouter);

  app.use('/api/v1/events/*', authMiddleware);
  app.use('/api/v1/events', apiRateLimit);
  app.route('/api/v1/events', eventsRouter);

  app.use('/api/v1/devices/*', authMiddleware);
  app.use('/api/v1/devices', apiRateLimit);
  app.route('/api/v1/devices', devicesRouter);

  app.use('/api/v1/credits/*', authMiddleware);
  app.use('/api/v1/credits', apiRateLimit);
  app.route('/api/v1/credits', creditsRouter);

  // ─── 404 handler ────────────────────────────────────────────
  app.notFound((c) => {
    return c.json({ error: 'Not found', path: c.req.path }, 404);
  });

  return app;
}
