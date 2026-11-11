-- ================================================================
-- WashTrap Ledger — Database Schema
-- ================================================================
-- SQLite with WAL mode, foreign keys enabled.
-- All IDs use nanoid (TEXT). Timestamps are ISO 8601 strings.
-- Created via CREATE TABLE IF NOT EXISTS for safe re-runs.
-- ================================================================

-- ─── Organizations ──────────────────────────────────────────────
-- Laundries own devices and generate capture data.
-- Brands receive credits and buy offsets.
-- Admin org is for WashTrap internal use.
CREATE TABLE IF NOT EXISTS organizations (
  id          TEXT PRIMARY KEY,
  name        TEXT NOT NULL,
  type        TEXT NOT NULL CHECK (type IN ('laundry', 'brand', 'admin')),
  created_at  TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ─── Users ──────────────────────────────────────────────────────
-- Each user belongs to one organization.
-- On org deletion, remove all its users.
CREATE TABLE IF NOT EXISTS users (
  id              TEXT PRIMARY KEY,
  org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  email           TEXT UNIQUE NOT NULL,
  name            TEXT,
  role            TEXT NOT NULL DEFAULT 'member' CHECK (role IN ('admin', 'member', 'viewer')),
  password_hash   TEXT NOT NULL DEFAULT 'password',  -- MVP: plain string. Production: bcrypt.
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ─── Devices ────────────────────────────────────────────────────
-- ESP32-based microfiber filter units installed on washing machines.
-- The device ID matches the ESP32 hardware ID.
-- On org deletion, remove all its devices (cascades to events).
CREATE TABLE IF NOT EXISTS devices (
  id              TEXT PRIMARY KEY,
  org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  name            TEXT,                    -- human label: "Waschmaschine #3"
  location        TEXT,                    -- street address
  installed_at    TEXT,                    -- when hardware was installed
  status          TEXT NOT NULL DEFAULT 'active'
                    CHECK (status IN ('active', 'inactive', 'maintenance')),
  last_heartbeat  TEXT,                    -- last time device checked in
  firmware_version TEXT,
  battery_level   INTEGER CHECK (battery_level >= 0 AND battery_level <= 100),
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ─── Capture Events ─────────────────────────────────────────────
-- One row per wash cycle, as reported by the ESP32.
-- UNIQUE(device_id, wash_cycle) enforces idempotency: a device
-- can never submit the same cycle number twice.
CREATE TABLE IF NOT EXISTS capture_events (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id       TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  timestamp       TEXT NOT NULL,           -- ESP32 RTC timestamp (ISO 8601)
  water_volume_l  REAL NOT NULL CHECK (water_volume_l > 0 AND water_volume_l <= 100),
  estimated_mg    REAL NOT NULL CHECK (estimated_mg >= 0 AND estimated_mg <= 100),
  wash_cycle      INTEGER NOT NULL,
  received_at     TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(device_id, wash_cycle)
);

-- ─── Credit Batches ─────────────────────────────────────────────
-- Represents a block of microplastic offset credits.
-- A brand (org_id) receives credits from a laundry (source_org_id).
-- UNIQUE constraint on (source_org_id, period_start, period_end)
-- prevents double-issuing the same period.
CREATE TABLE IF NOT EXISTS credit_batches (
  id              TEXT PRIMARY KEY,
  org_id          TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  source_org_id   TEXT NOT NULL REFERENCES organizations(id) ON DELETE CASCADE,
  period_start    TEXT NOT NULL,           -- YYYY-MM-DD
  period_end      TEXT NOT NULL,           -- YYYY-MM-DD
  total_mg        REAL NOT NULL CHECK (total_mg > 0),
  total_eur       REAL NOT NULL CHECK (total_eur >= 0),
  status          TEXT NOT NULL DEFAULT 'pending'
                    CHECK (status IN ('pending', 'issued', 'redeemed')),
  issued_at       TEXT,
  redeemed_at     TEXT,
  notes           TEXT,
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
  UNIQUE(source_org_id, period_start, period_end)
);

-- ─── Credit Audit Log ───────────────────────────────────────────
-- Immutable append-only log of every status change on a credit batch.
CREATE TABLE IF NOT EXISTS credit_audit_log (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  credit_id       TEXT NOT NULL REFERENCES credit_batches(id) ON DELETE CASCADE,
  action          TEXT NOT NULL,           -- 'created', 'issued', 'redeemed', 'voided'
  from_status     TEXT,
  to_status       TEXT,
  performed_by    TEXT,                    -- user ID or 'system'
  notes           TEXT,
  created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- ─── Device Health Checks ───────────────────────────────────────
-- One row per device per day. Populated by a cron job or on-read.
-- Used by the /dashboard/alerts endpoint for offline detection.
CREATE TABLE IF NOT EXISTS device_health_checks (
  id              INTEGER PRIMARY KEY AUTOINCREMENT,
  device_id       TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
  check_date      TEXT NOT NULL,           -- YYYY-MM-DD
  event_count     INTEGER NOT NULL DEFAULT 0,
  is_online       INTEGER NOT NULL DEFAULT 1,
  UNIQUE(device_id, check_date)
);

-- ================================================================
-- Indexes
-- ================================================================
-- Events: most queries filter by device + timestamp range
CREATE INDEX IF NOT EXISTS idx_events_device_ts
  ON capture_events(device_id, timestamp);
CREATE INDEX IF NOT EXISTS idx_events_received
  ON capture_events(received_at);
-- Events: org-scoped queries join through devices
CREATE INDEX IF NOT EXISTS idx_events_timestamp
  ON capture_events(timestamp);

-- Devices: org lookup and status filtering
CREATE INDEX IF NOT EXISTS idx_devices_org
  ON devices(org_id);
CREATE INDEX IF NOT EXISTS idx_devices_status
  ON devices(status);
CREATE INDEX IF NOT EXISTS idx_devices_heartbeat
  ON devices(last_heartbeat);

-- Credits: brand-scoped and source-scoped queries
CREATE INDEX IF NOT EXISTS idx_credits_org
  ON credit_batches(org_id);
CREATE INDEX IF NOT EXISTS idx_credits_source
  ON credit_batches(source_org_id);
CREATE INDEX IF NOT EXISTS idx_credits_status
  ON credit_batches(status);

-- Users: email lookup (login) and org scoping
CREATE INDEX IF NOT EXISTS idx_users_email
  ON users(email);
CREATE INDEX IF NOT EXISTS idx_users_org
  ON users(org_id);

-- Audit log: credit history lookups
CREATE INDEX IF NOT EXISTS idx_audit_credit
  ON credit_audit_log(credit_id);

-- Health checks: alert queries
CREATE INDEX IF NOT EXISTS idx_health_device
  ON device_health_checks(device_id, check_date);
