import { Context, Next } from 'hono';

export async function errorHandler(c: Context, next: Next) {
  try {
    await next();
  } catch (err: any) {
    // Zod validation errors (check shape instead of instanceof to avoid zod import)
    if (err.name === 'ZodError' && Array.isArray(err.errors)) {
      return c.json({
        error: 'Validation failed',
        details: err.errors.map((e: any) => ({
          path: e.path?.join('.') || '',
          message: e.message,
        })),
      }, 400);
    }

    // SQLite constraint errors
    if (err.code === 'SQLITE_CONSTRAINT_UNIQUE') {
      return c.json({
        error: 'Duplicate entry',
        message: err.message,
      }, 409);
    }

    if (err.code === 'SQLITE_CONSTRAINT_FOREIGNKEY') {
      return c.json({
        error: 'Referenced entity not found',
        message: err.message,
      }, 400);
    }

    // Known HTTP errors
    if (err.statusCode && err.message) {
      return c.json({ error: err.message }, err.statusCode);
    }

    // Unexpected errors — log and return generic message
    console.error('Unhandled error:', err);
    return c.json({ error: 'Internal server error' }, 500);
  }
}
