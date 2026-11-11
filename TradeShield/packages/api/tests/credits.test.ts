import { describe, it, expect, beforeEach } from 'vitest';
import { createTestContext, asUser, asDevice, parseJSON, type TestContext } from './helpers.js';

describe('Credit Issuance', () => {
  let ctx: TestContext;

  beforeEach(() => {
    ctx = createTestContext();
  });

  // Seed recent events (within 48h) so credits can be issued.
  // Returns the period start/end dates that cover these events.
  async function seedRecentEvents(count: number): Promise<{ start: string; end: string }> {
    const now = new Date();
    const periodStart = new Date(now);
    periodStart.setHours(0, 0, 0, 0);

    for (let i = 0; i < count; i++) {
      const timestamp = new Date(now.getTime() - (count - i) * 60 * 60 * 1000); // spaced 1hr apart in the past

      await asDevice(ctx.app, ctx.data.device1, 'POST', '/api/v1/events', {
        device_id: ctx.data.device1.id,
        timestamp: timestamp.toISOString(),
        water_volume_l: 30 + (i % 10),
        estimated_mg: 2.4 + (i % 10) * 0.1,
        wash_cycle: i + 1,
      });
    }

    return {
      start: periodStart.toISOString().split('T')[0],
      end: now.toISOString().split('T')[0],
    };
  }

  // ─── Issue Credits ──────────────────────────────────────────

  describe('POST /api/v1/credits/issue', () => {
    it('issues credits for a period with capture events', async () => {
      const { start, end } = await seedRecentEvents(10);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });

      expect(res.status).toBe(201);
      const body = await parseJSON(res);
      expect(body.data.id).toBeDefined();
      expect(body.data.total_mg).toBeGreaterThan(0);
      expect(body.data.total_eur).toBeGreaterThan(0);
      expect(body.data.status).toBe('pending');
      expect(body.data.period).toBeDefined();
      expect(body.data.cycles).toBe(10);
    });

    it('rejects credit issuance for period with no events', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        source_org_id: ctx.data.laundry.id,
        period_start: '2020-01-01',
        period_end: '2020-01-31',
      });

      expect(res.status).toBe(400);
      const body = await parseJSON(res);
      expect(body.error).toContain('No capture events');
    });

    it('rejects duplicate credit issuance for same period', async () => {
      const { start, end } = await seedRecentEvents(5);

      const period = {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      };

      // First issuance
      const res1 = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', period);
      expect(res1.status).toBe(201);

      // Duplicate
      const res2 = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', period);
      expect(res2.status).toBe(409);
      const body = await parseJSON(res2);
      expect(body.existing_credit_id).toBeDefined();
    });

    it('rejects issuance with invalid date range', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        source_org_id: ctx.data.laundry.id,
        period_start: '2026-01-31',
        period_end: '2026-01-01',
      });

      expect(res.status).toBe(400);
    });

    it('creates audit log entry on issuance', async () => {
      const { start, end } = await seedRecentEvents(5);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });

      const body = await parseJSON(res);
      const auditLog = ctx.db.prepare('SELECT * FROM credit_audit_log WHERE credit_id = ?')
        .all(body.data.id) as any[];

      expect(auditLog.length).toBe(1);
      expect(auditLog[0].action).toBe('created');
      expect(auditLog[0].from_status).toBeNull();
      expect(auditLog[0].to_status).toBe('pending');
    });
  });

  // ─── Issue (Mark as Issued) ─────────────────────────────────

  describe('POST /api/v1/credits/:id/issue', () => {
    it('marks pending credit as issued', async () => {
      const { start, end } = await seedRecentEvents(5);

      // Create credit
      const createRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });
      const credit = await parseJSON(createRes);

      // Mark as issued
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'POST', `/api/v1/credits/${credit.data.id}/issue`);
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data.status).toBe('issued');
      expect(body.data.issued_at).toBeDefined();

      // Audit trail
      const auditLog = ctx.db.prepare('SELECT * FROM credit_audit_log WHERE credit_id = ? ORDER BY id')
        .all(credit.data.id) as any[];
      expect(auditLog.length).toBe(2);
      expect(auditLog[1].action).toBe('issued');
      expect(auditLog[1].from_status).toBe('pending');
      expect(auditLog[1].to_status).toBe('issued');
    });

    it('rejects issuing already-issued credit', async () => {
      const { start, end } = await seedRecentEvents(5);

      const createRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });
      const credit = await parseJSON(createRes);

      // Issue once
      await asUser(ctx.app, ctx.data.laundryUser, 'POST', `/api/v1/credits/${credit.data.id}/issue`);

      // Try to issue again
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'POST', `/api/v1/credits/${credit.data.id}/issue`);
      expect(res.status).toBe(400);
    });
  });

  // ─── Redeem Credits ─────────────────────────────────────────

  describe('POST /api/v1/credits/:id/redeem', () => {
    it('redeems an issued credit', async () => {
      const { start, end } = await seedRecentEvents(5);

      // Create and issue
      const createRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });
      const credit = await parseJSON(createRes);

      await asUser(ctx.app, ctx.data.laundryUser, 'POST', `/api/v1/credits/${credit.data.id}/issue`);

      // Redeem
      const res = await asUser(ctx.app, ctx.data.brandUser, 'POST', `/api/v1/credits/${credit.data.id}/redeem`, {
        notes: 'CSR Q2 report verified',
      });

      expect(res.status).toBe(200);
      const body = await parseJSON(res);
      expect(body.data.status).toBe('redeemed');

      // Audit trail
      const auditLog = ctx.db.prepare('SELECT * FROM credit_audit_log WHERE credit_id = ? ORDER BY id')
        .all(credit.data.id) as any[];
      expect(auditLog.length).toBe(3);
      expect(auditLog[2].action).toBe('redeemed');
      expect(auditLog[2].notes).toBe('CSR Q2 report verified');
    });

    it('rejects redeeming pending credit', async () => {
      const { start, end } = await seedRecentEvents(5);

      const createRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });
      const credit = await parseJSON(createRes);

      // Try to redeem without issuing first
      const res = await asUser(ctx.app, ctx.data.brandUser, 'POST', `/api/v1/credits/${credit.data.id}/redeem`);
      expect(res.status).toBe(400);
    });
  });

  // ─── Credit Listing ─────────────────────────────────────────

  describe('GET /api/v1/credits', () => {
    it('lists credits visible to both brand and laundry', async () => {
      const { start, end } = await seedRecentEvents(5);

      await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });

      // Laundry sees credits
      const laundryRes = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/credits');
      const laundryBody = await parseJSON(laundryRes);
      expect(laundryBody.data.length).toBe(1);

      // Brand also sees credits (as receiver)
      const brandRes = await asUser(ctx.app, ctx.data.brandUser, 'GET', '/api/v1/credits');
      const brandBody = await parseJSON(brandRes);
      expect(brandBody.data.length).toBe(1);
    });
  });

  // ─── Credit Stats ───────────────────────────────────────────

  describe('GET /api/v1/credits/stats', () => {
    it('returns credit statistics', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/credits/stats');
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data.total).toBeDefined();
      expect(body.data.by_status).toBeDefined();
      expect(body.data.by_status.pending).toBeDefined();
      expect(body.data.by_status.issued).toBeDefined();
      expect(body.data.by_status.redeemed).toBeDefined();
      expect(body.data.monthly).toBeDefined();
    });
  });

  // ─── Credit Detail ──────────────────────────────────────────

  describe('GET /api/v1/credits/:id', () => {
    it('returns credit detail with audit log', async () => {
      const { start, end } = await seedRecentEvents(5);

      const createRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });
      const credit = await parseJSON(createRes);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', `/api/v1/credits/${credit.data.id}`);
      expect(res.status).toBe(200);

      const body = await parseJSON(res);
      expect(body.data.id).toBe(credit.data.id);
      expect(body.data.source_org_name).toBe('Test Laundry');
      expect(body.data.org_name).toBe('Test Brand');
      expect(body.data.audit_log).toBeDefined();
      expect(body.data.audit_log.length).toBe(1);
    });
  });

  // ─── PDF Certificate ───────────────────────────────────────

  describe('GET /api/v1/credits/:id/certificate', () => {
    it('generates a PDF certificate for a credit batch', async () => {
      const { start, end } = await seedRecentEvents(5);

      const createRes = await asUser(ctx.app, ctx.data.laundryUser, 'POST', '/api/v1/credits/issue', {
        org_id: ctx.data.brand.id,
        source_org_id: ctx.data.laundry.id,
        period_start: start,
        period_end: end,
      });
      const credit = await parseJSON(createRes);

      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', `/api/v1/credits/${credit.data.id}/certificate`);
      expect(res.status).toBe(200);

      // Verify PDF content type
      const contentType = res.headers.get('content-type');
      expect(contentType).toBe('application/pdf');

      // Verify content disposition
      const disposition = res.headers.get('content-disposition');
      expect(disposition).toContain('attachment');
      expect(disposition).toContain('.pdf');

      // Verify PDF starts with %PDF
      const buffer = Buffer.from(await res.arrayBuffer());
      const header = buffer.slice(0, 5).toString('ascii');
      expect(header).toBe('%PDF-');

      // PDF should have reasonable size (> 1KB)
      expect(buffer.length).toBeGreaterThan(1000);
    });

    it('returns 404 for non-existent credit certificate', async () => {
      const res = await asUser(ctx.app, ctx.data.laundryUser, 'GET', '/api/v1/credits/nonexistent/certificate');
      expect(res.status).toBe(404);
    });
  });
});
