import { Hono } from 'hono';
import { zValidator } from '@hono/zod-validator';
import { nanoid } from 'nanoid';
import { IssueCreditSchema, CreditFilterSchema, RedeemCreditSchema } from '@washtrap/shared';
import { CREDIT_PRICE_PER_MG_EUR } from '@washtrap/shared';

export const creditsRouter = new Hono();

// ─── GET / — List credit batches (org-scoped correctly) ──────────
creditsRouter.get(
  '/',
  zValidator('query', CreditFilterSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const { page, limit, status, source_org_id } = c.req.valid('query');
    const offset = (page - 1) * limit;

    // FIX: Query both as brand (org_id) AND as laundry (source_org_id)
    let whereClause = `(cb.org_id = ? OR cb.source_org_id = ?)`;
    const params: any[] = [orgId, orgId];

    if (status) {
      whereClause += ' AND cb.status = ?';
      params.push(status);
    }
    if (source_org_id) {
      whereClause += ' AND cb.source_org_id = ?';
      params.push(source_org_id);
    }

    const countRow = db.prepare(`
      SELECT COUNT(*) as count
      FROM credit_batches cb
      WHERE ${whereClause}
    `).get(...params) as any;

    const credits = db.prepare(`
      SELECT
        cb.*,
        src.name as source_org_name,
        dst.name as org_name
      FROM credit_batches cb
      JOIN organizations src ON cb.source_org_id = src.id
      JOIN organizations dst ON cb.org_id = dst.id
      WHERE ${whereClause}
      ORDER BY cb.created_at DESC
      LIMIT ? OFFSET ?
    `).all(...params, limit, offset);

    return c.json({
      data: credits,
      pagination: {
        page,
        limit,
        total: countRow.count,
        pages: Math.ceil(countRow.count / limit),
      },
    });
  }
);

// ─── GET /stats — Credit summary statistics ─────────────────────
creditsRouter.get(
  '/stats',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const totalStats = db.prepare(`
      SELECT
        COALESCE(SUM(total_mg), 0) as total_mg,
        COALESCE(SUM(total_eur), 0) as total_eur,
        COUNT(*) as total_batches,
        SUM(CASE WHEN status = 'pending' THEN total_mg ELSE 0 END) as pending_mg,
        SUM(CASE WHEN status = 'pending' THEN total_eur ELSE 0 END) as pending_eur,
        SUM(CASE WHEN status = 'issued' THEN total_mg ELSE 0 END) as issued_mg,
        SUM(CASE WHEN status = 'issued' THEN total_eur ELSE 0 END) as issued_eur,
        SUM(CASE WHEN status = 'redeemed' THEN total_mg ELSE 0 END) as redeemed_mg,
        SUM(CASE WHEN status = 'redeemed' THEN total_eur ELSE 0 END) as redeemed_eur
      FROM credit_batches
      WHERE org_id = ? OR source_org_id = ?
    `).get(orgId, orgId) as any;

    // Monthly breakdown
    const monthly = db.prepare(`
      SELECT
        strftime('%Y-%m', created_at) as month,
        SUM(total_mg) as total_mg,
        SUM(total_eur) as total_eur,
        COUNT(*) as batches
      FROM credit_batches
      WHERE (org_id = ? OR source_org_id = ?)
      GROUP BY month
      ORDER BY month DESC
      LIMIT 12
    `).all(orgId, orgId);

    return c.json({
      data: {
        total: {
          mg: totalStats.total_mg,
          eur: totalStats.total_eur,
          batches: totalStats.total_batches,
        },
        by_status: {
          pending: { mg: totalStats.pending_mg, eur: totalStats.pending_eur },
          issued: { mg: totalStats.issued_mg, eur: totalStats.issued_eur },
          redeemed: { mg: totalStats.redeemed_mg, eur: totalStats.redeemed_eur },
        },
        monthly,
      },
    });
  }
);

// ─── GET /:id — Single credit batch detail ──────────────────────
creditsRouter.get(
  '/:id',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const creditId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const credit = db.prepare(`
      SELECT
        cb.*,
        src.name as source_org_name,
        dst.name as org_name
      FROM credit_batches cb
      JOIN organizations src ON cb.source_org_id = src.id
      JOIN organizations dst ON cb.org_id = dst.id
      WHERE cb.id = ? AND (cb.org_id = ? OR cb.source_org_id = ?)
    `).get(creditId, orgId, orgId);

    if (!credit) {
      return c.json({ error: 'Credit batch not found' }, 404);
    }

    // Get audit trail
    const auditLog = db.prepare(`
      SELECT * FROM credit_audit_log
      WHERE credit_id = ?
      ORDER BY created_at DESC
    `).all(creditId);

    return c.json({ data: { ...credit, audit_log: auditLog } });
  }
);

// ─── POST /issue — Generate credit batch for a period ───────────
creditsRouter.post(
  '/issue',
  zValidator('json', IssueCreditSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const user = c.get('user') as any;

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const { org_id: receiverOrgId, source_org_id, period_start, period_end } = c.req.valid('json');

    // Validate date range
    if (new Date(period_start) >= new Date(period_end)) {
      return c.json({ error: 'period_start must be before period_end' }, 400);
    }

    // Check if this period has already been issued for this source org
    const existing = db.prepare(`
      SELECT id FROM credit_batches
      WHERE source_org_id = ?
        AND period_start = ? AND period_end = ?
        AND status != 'redeemed'
    `).get(source_org_id, period_start, period_end);

    if (existing) {
      return c.json({
        error: 'Period already has credits',
        existing_credit_id: (existing as any).id,
      }, 409);
    }

    // Calculate total capture for the period
    const stats = db.prepare(`
      SELECT
        COALESCE(SUM(ce.estimated_mg), 0) as total_mg,
        COUNT(*) as total_cycles,
        COALESCE(SUM(ce.water_volume_l), 0) as total_water_l
      FROM capture_events ce
      JOIN devices d ON ce.device_id = d.id
      WHERE d.org_id = ?
        AND date(ce.timestamp) >= ?
        AND date(ce.timestamp) <= ?
    `).get(source_org_id, period_start, period_end) as any;

    if (stats.total_mg === 0) {
      return c.json({ error: 'No capture events found for this period' }, 400);
    }

    const total_eur = stats.total_mg * CREDIT_PRICE_PER_MG_EUR;
    const creditId = nanoid();
    const now = new Date().toISOString();

    // Create credit batch + audit log in a transaction
    const createCredit = db.transaction(() => {
      db.prepare(`
        INSERT INTO credit_batches (id, org_id, source_org_id, period_start, period_end, total_mg, total_eur, status, created_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, 'pending', ?)
      `).run(creditId, receiverOrgId || orgId, source_org_id, period_start, period_end, stats.total_mg, total_eur, now);

      db.prepare(`
        INSERT INTO credit_audit_log (credit_id, action, from_status, to_status, performed_by, notes, created_at)
        VALUES (?, 'created', NULL, 'pending', ?, ?, ?)
      `).run(creditId, user?.id || 'system', `Period ${period_start} to ${period_end}: ${stats.total_cycles} cycles, ${stats.total_mg.toFixed(1)} mg`, now);
    });

    createCredit();

    return c.json({
      data: {
        id: creditId,
        total_mg: stats.total_mg,
        total_eur,
        status: 'pending',
        created_at: now,
        period: { start: period_start, end: period_end },
        cycles: stats.total_cycles,
        water_l: stats.total_water_l,
      },
    }, 201);
  }
);

// ─── POST /:id/issue — Mark credit batch as issued ──────────────
creditsRouter.post(
  '/:id/issue',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const user = c.get('user') as any;
    const creditId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const credit = db.prepare(`
      SELECT * FROM credit_batches WHERE id = ? AND (org_id = ? OR source_org_id = ?)
    `).get(creditId, orgId, orgId) as any;

    if (!credit) {
      return c.json({ error: 'Credit batch not found' }, 404);
    }

    if (credit.status !== 'pending') {
      return c.json({
        error: 'Invalid status transition',
        message: `Cannot issue credit in '${credit.status}' status`,
      }, 400);
    }

    const now = new Date().toISOString();

    const issueCredit = db.transaction(() => {
      db.prepare(`
        UPDATE credit_batches
        SET status = 'issued', issued_at = ?
        WHERE id = ?
      `).run(now, creditId);

      db.prepare(`
        INSERT INTO credit_audit_log (credit_id, action, from_status, to_status, performed_by, created_at)
        VALUES (?, 'issued', 'pending', 'issued', ?, ?)
      `).run(creditId, user?.id || 'system', now);
    });

    issueCredit();

    const updated = db.prepare('SELECT * FROM credit_batches WHERE id = ?').get(creditId);

    return c.json({ data: updated });
  }
);

// ─── POST /:id/redeem — Mark credit batch as redeemed ───────────
creditsRouter.post(
  '/:id/redeem',
  zValidator('json', RedeemCreditSchema),
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const user = c.get('user') as any;
    const creditId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const credit = db.prepare(`
      SELECT * FROM credit_batches WHERE id = ? AND (org_id = ? OR source_org_id = ?)
    `).get(creditId, orgId, orgId) as any;

    if (!credit) {
      return c.json({ error: 'Credit batch not found' }, 404);
    }

    if (credit.status !== 'issued') {
      return c.json({
        error: 'Invalid status transition',
        message: `Cannot redeem credit in '${credit.status}' status. Must be 'issued'.`,
      }, 400);
    }

    const { notes } = c.req.valid('json');
    const now = new Date().toISOString();

    const redeemCredit = db.transaction(() => {
      db.prepare(`
        UPDATE credit_batches
        SET status = 'redeemed'
        WHERE id = ?
      `).run(creditId);

      db.prepare(`
        INSERT INTO credit_audit_log (credit_id, action, from_status, to_status, performed_by, notes, created_at)
        VALUES (?, 'redeemed', 'issued', 'redeemed', ?, ?, ?)
      `).run(creditId, user?.id || 'system', notes || null, now);
    });

    redeemCredit();

    const updated = db.prepare('SELECT * FROM credit_batches WHERE id = ?').get(creditId);

    return c.json({ data: updated });
  }
);

// ─── Helpers ───────────────────────────────────────────────────

function formatDate(iso: string): string {
  try {
    return new Date(iso).toLocaleDateString('en-GB', {
      day: 'numeric', month: 'long', year: 'numeric',
    });
  } catch {
    return iso;
  }
}

function formatDateTime(iso: string): string {
  try {
    return new Date(iso).toLocaleString('en-GB', {
      day: 'numeric', month: 'short', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  } catch {
    return iso;
  }
}

// ─── GET /:id/certificate — Generate PDF certificate ───────────
creditsRouter.get(
  '/:id/certificate',
  async (c) => {
    const db = c.get('db');
    const orgId = c.get('org_id') as string;
    const creditId = c.req.param('id');

    if (!orgId) {
      return c.json({ error: 'Organization context required' }, 403);
    }

    const credit = db.prepare(`
      SELECT
        cb.*,
        src.name as source_org_name,
        dst.name as org_name
      FROM credit_batches cb
      JOIN organizations src ON cb.source_org_id = src.id
      JOIN organizations dst ON cb.org_id = dst.id
      WHERE cb.id = ? AND (cb.org_id = ? OR cb.source_org_id = ?)
    `).get(creditId, orgId, orgId) as any;

    if (!credit) {
      return c.json({ error: 'Credit batch not found' }, 404);
    }

    // Get audit trail
    const auditLog = db.prepare(`
      SELECT * FROM credit_audit_log WHERE credit_id = ? ORDER BY created_at
    `).all(creditId) as any[];

    // Generate PDF using pdfkit
    const PDFDocument = (await import('pdfkit')).default;
    const doc = new PDFDocument({ size: 'A4', margin: 50 });

    // Collect PDF chunks
    const chunks: Buffer[] = [];
    doc.on('data', (chunk: Buffer) => chunks.push(chunk));

    const pdfBuffer = await new Promise<any>((resolve) => {
      doc.on('end', () => resolve(new Uint8Array(Buffer.concat(chunks))));

      // ── Header ──────────────────────────────────────────────
      doc
        .rect(0, 0, doc.page.width, 120)
        .fill('#0369a1');

      doc
        .fontSize(28)
        .fill('#ffffff')
        .text('WashTrap', 50, 35, { continued: true })
        .fontSize(14)
        .text('  Microplastic Offset Certificate', { baseline: 'alphabetic' });

      doc
        .fontSize(10)
        .fill('#bae6fd')
        .text('EU ESPR Compliant · Circular Economy · Verified Capture', 50, 80);

      // ── Certificate Title ───────────────────────────────────
      doc
        .fill('#1e293b')
        .fontSize(20)
        .text('Credit Certificate', 50, 150);

      doc
        .moveTo(50, 178)
        .lineTo(545, 178)
        .strokeColor('#e2e8f0')
        .lineWidth(1)
        .stroke();

      // ── Certificate ID ──────────────────────────────────────
      doc
        .fontSize(9)
        .fill('#64748b')
        .text('Certificate ID', 50, 195);
      doc
        .fontSize(11)
        .fill('#334155')
        .text(credit.id, 50, 210, { width: 200 });

      // ── Period ──────────────────────────────────────────────
      doc
        .fontSize(9)
        .fill('#64748b')
        .text('Coverage Period', 300, 195);
      doc
        .fontSize(11)
        .fill('#334155')
        .text(`${formatDate(credit.period_start)} — ${formatDate(credit.period_end)}`, 300, 210, { width: 250 });

      // ── Parties ─────────────────────────────────────────────
      let y = 250;

      doc
        .fontSize(9)
        .fill('#64748b')
        .text('Source Facility', 50, y);
      doc
        .fontSize(12)
        .fill('#0f172a')
        .text(credit.source_org_name || credit.source_org_id, 50, y + 14, { width: 220 });

      doc
        .fontSize(9)
        .fill('#64748b')
        .text('Credit Recipient', 300, y);
      doc
        .fontSize(12)
        .fill('#0f172a')
        .text(credit.org_name || credit.org_id, 300, y + 14, { width: 250 });

      // ── Capture Data Table ──────────────────────────────────
      y = 320;

      doc
        .roundedRect(50, y, 495, 130, 6)
        .fill('#f8fafc');

      doc
        .fontSize(10)
        .fill('#64748b')
        .text('CAPTURE DATA', 65, y + 12);

      const col1 = 65;
      const col2 = 250;
      const col3 = 400;
      const row1 = y + 35;
      const row2 = y + 75;

      // Row 1
      doc.fontSize(9).fill('#64748b').text('Microplastic Captured', col1, row1);
      doc.fontSize(16).fill('#0ea5e9').text(`${credit.total_mg.toFixed(2)} mg`, col1, row1 + 13, { width: 170 });

      doc.fontSize(9).fill('#64748b').text('Water Filtered', col2, row1);
      doc.fontSize(16).fill('#0369a1').text(`${(credit.total_water_l || 0).toFixed(0)} L`, col2, row1 + 13, { width: 140 });

      doc.fontSize(9).fill('#64748b').text('Wash Cycles', col3, row1);
      doc.fontSize(16).fill('#334155').text(`${credit.total_cycles || '—'}`, col3, row1 + 13, { width: 130 });

      // Row 2
      doc.fontSize(9).fill('#64748b').text('Credit Value', col1, row2);
      doc.fontSize(18).fill('#16a34a').text(`€${credit.total_eur.toFixed(2)}`, col1, row2 + 12, { width: 170 });

      doc.fontSize(9).fill('#64748b').text('Status', col2, row2);
      const statusColor = credit.status === 'redeemed' ? '#2563eb' : credit.status === 'issued' ? '#16a34a' : '#d97706';
      doc.fontSize(12).fill(statusColor).text(credit.status.toUpperCase(), col2, row2 + 14, { width: 140 });

      doc.fontSize(9).fill('#64748b').text('Issue Date', col3, row2);
      doc.fontSize(11).fill('#334155').text(credit.issued_at ? formatDate(credit.issued_at) : 'Pending', col3, row2 + 14, { width: 130 });

      // ── Audit Trail ─────────────────────────────────────────
      y = 475;
      doc
        .moveTo(50, y)
        .lineTo(545, y)
        .strokeColor('#e2e8f0')
        .lineWidth(1)
        .stroke();

      doc
        .fontSize(9)
        .fill('#64748b')
        .text('AUDIT TRAIL', 50, y + 10);

      y += 28;
      for (const entry of auditLog.slice(0, 6)) {
        doc.fontSize(9).fill('#94a3b8').text(formatDateTime(entry.created_at), 65, y, { width: 130 });
        doc.fontSize(9).fill('#475569').text(entry.action.toUpperCase(), 200, y, { width: 80 });
        doc.fontSize(9).fill('#334155').text(
          entry.notes || `${entry.from_status || '—'} \u2192 ${entry.to_status || '—'}`,
          290, y, { width: 250 }
        );
        y += 18;
      }

      // ── Footer ──────────────────────────────────────────────
      const footerY = doc.page.height - 80;
      doc
        .moveTo(50, footerY)
        .lineTo(545, footerY)
        .strokeColor('#e2e8f0')
        .lineWidth(1)
        .stroke();

      doc
        .fontSize(8)
        .fill('#94a3b8')
        .text('This certificate is generated by WashTrap Ledger. Data is cryptographically linked to device telemetry.', 50, footerY + 10, { width: 495 });
      doc
        .text('For verification, visit washtrap.io/verify with the Certificate ID above.', 50, footerY + 24, { width: 495 });
      doc
        .fontSize(7)
        .fill('#cbd5e1')
        .text(`Generated: ${formatDateTime(new Date().toISOString())}`, 50, footerY + 42);

      doc.end();
    });

    // Return PDF as response
    return new Response(pdfBuffer, {
      status: 200,
      headers: {
        'Content-Type': 'application/pdf',
        'Content-Disposition': `attachment; filename="washtrap-certificate-${creditId.slice(0, 8)}.pdf"`,
        'Content-Length': String(pdfBuffer.length),
      },
    });
  }
);
