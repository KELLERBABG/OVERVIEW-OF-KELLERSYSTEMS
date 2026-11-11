"use strict";

import os from "node:os";
import path from "node:path";
import { readFileSync, writeFileSync, mkdirSync, unlinkSync, existsSync } from "node:fs";
import { createHash } from "node:crypto";
import { verifyLicenseToken, TIER_ENTITLEMENTS, TIERS } from "./licensing.js";

export const DEFAULT_LICENSE_SERVER =
  process.env.CLAUSELENS_AUTH_SERVER || "https://license.kellersystems.dev";

// Cloudflare's bot filter answers browser-less clients of the custom domain with 403.
export const WORKERS_DEV_SERVER = "https://clauselens-license.papababg02.workers.dev";

/**
 * Generates a unique, deterministic hardware/device fingerprint for per-seat binding.
 */
export function getMachineFingerprint() {
  const parts = [
    os.platform(),
    os.arch(),
    os.hostname(),
    os.cpus()?.[0]?.model || "cpu",
    String(os.totalmem() || 0),
    os.userInfo()?.username || "user",
  ];
  return createHash("sha256").update(parts.join("|")).digest("hex").slice(0, 32);
}

/**
 * Returns the path to the local credentials store (~/.clauselens/account.json).
 */
export function getCredentialsPath() {
  const home = os.homedir() || process.env.USERPROFILE || process.cwd();
  return path.join(home, ".clauselens", "account.json");
}

export function loadStoredAccount() {
  const credPath = getCredentialsPath();
  if (!existsSync(credPath)) return null;
  try {
    const raw = readFileSync(credPath, "utf-8");
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

export function saveStoredAccount(accountData) {
  const credPath = getCredentialsPath();
  mkdirSync(path.dirname(credPath), { recursive: true });
  writeFileSync(credPath, JSON.stringify(accountData, null, 2), "utf-8");
}

export function clearStoredAccount() {
  const credPath = getCredentialsPath();
  if (existsSync(credPath)) {
    try {
      unlinkSync(credPath);
    } catch {
      // Ignore
    }
  }
}

function testAccount() {
  const token = process.env.CLAUSELENS_TEST_TOKEN || null;
  return {
    email: "test@clauselens.dev",
    licenseKey: "CL-PRO-test",
    tier: "pro",
    seatId: "seat_test_123",
    machineId: getMachineFingerprint(),
    activatedAt: new Date(0).toISOString(),
    expiresAt: null,
    token,
  };
}

async function requestSeat(base, payload) {
  const controller = new AbortController();
  const timeout = setTimeout(() => controller.abort(), 6000);
  try {
    const resp = await fetch(`${base}/api/v1/seats/verify`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      signal: controller.signal,
    });
    const data = await resp.json().catch(() => null);
    return { ok: resp.ok, status: resp.status, data };
  } finally {
    clearTimeout(timeout);
  }
}

function reusableSeat(stored, licenseKey, machineId, publicKey) {
  if (!stored || stored.licenseKey !== licenseKey || stored.machineId !== machineId) return null;
  const session = verifyLicenseToken(stored.token, { machineId, publicKey });
  return session.valid ? { ...stored, valid: true, offline: true } : null;
}

/**
 * Activates a seat online. The worker checks the key (CL- or Polar) and returns a
 * seven-day token, so later runs need no network.
 */
export async function verifySeatOnline({
  email,
  licenseKey,
  machineId = getMachineFingerprint(),
  authServer = DEFAULT_LICENSE_SERVER,
  publicKey,
}) {
  if (!email || !email.includes("@")) {
    return {
      valid: false,
      error: "Gültige E-Mail-Adresse für den Benutzeraccount erforderlich.",
    };
  }
  if (!licenseKey) {
    return {
      valid: false,
      error: "Lizenzschlüssel für Seat-Aktivierung erforderlich.",
    };
  }

  const cleanKey = String(licenseKey).trim();
  const payload = {
    email: email.trim().toLowerCase(),
    licenseKey: cleanKey,
    machineId,
    platform: os.platform(),
    timestamp: new Date().toISOString(),
  };

  try {
    let resp = await requestSeat(authServer, payload);
    if (!resp.ok && resp.status === 403 && authServer === DEFAULT_LICENSE_SERVER) {
      resp = await requestSeat(WORKERS_DEV_SERVER, payload);
    }

    if (resp.ok) {
      const data = resp.data || {};
      if (!data.token) {
        return { valid: false, error: "Der Lizenzserver hat keine Lizenzsitzung ausgestellt." };
      }
      return {
        valid: true,
        email: payload.email,
        licenseKey: cleanKey,
        tier: data.tier,
        seatId: data.seatId || `seat_${machineId.slice(0, 8)}`,
        machineId,
        activatedAt: data.activatedAt || new Date().toISOString(),
        expiresAt: data.expiresAt || null,
        token: data.token,
      };
    }

    // Outage or quota: keep a still valid session instead of blocking the user.
    if (resp.status === 429 || resp.status >= 500) {
      const kept = reusableSeat(loadStoredAccount(), cleanKey, machineId, publicKey);
      if (kept) return kept;
    }
    const message = resp.data && resp.data.error;
    return {
      valid: false,
      error: message || `Online-Lizenzprüfung fehlgeschlagen (HTTP ${resp.status}).`,
    };
  } catch (err) {
    const unreachable = /fetch failed|ENOTFOUND|EAI_AGAIN|ECONNREFUSED|aborted|timed out/i.test(
      String(err && err.message),
    );
    if (unreachable || process.env.CLAUSELENS_ALLOW_OFFLINE_SEAT === "1" || authServer.includes("localhost")) {
      const kept = reusableSeat(loadStoredAccount(), cleanKey, machineId, publicKey);
      if (kept) return kept;
    }
    return {
      valid: false,
      error: `Online-Verbindung zum ClauseLens-Lizenzserver (${authServer}) fehlgeschlagen: ${err.message}.`,
    };
  }
}

/**
 * Licences the local device from the stored session token.
 */
export function resolveActiveLicense({ machineId = getMachineFingerprint(), publicKey } = {}) {
  if (process.env.CLAUSELENS_TEST_ACCOUNT === "1") {
    return {
      valid: true,
      tier: TIERS.PRO,
      licensee: "test@clauselens.dev",
      expiresAt: null,
      error: null,
      entitlements: TIER_ENTITLEMENTS[TIERS.PRO],
    };
  }
  const stored = loadStoredAccount();
  if (!stored) {
    return {
      valid: false,
      tier: TIERS.COMMUNITY,
      licensee: null,
      expiresAt: null,
      error: "Keine Lizenzsitzung auf diesem Gerät. Bitte anmelden: clauselens login --email <email> --key <lizenzschlüssel>",
      entitlements: TIER_ENTITLEMENTS[TIERS.COMMUNITY],
    };
  }
  return verifyLicenseToken(stored.token, { machineId, publicKey });
}

/**
 * Enforces that an active account and valid seat are present.
 * Throws an Error if no valid account seat is registered on this machine.
 */
export function requireActiveAccount({ publicKey } = {}) {
  // Test override guard for non-interactive unit test environments
  if (process.env.CLAUSELENS_TEST_ACCOUNT === "1") {
    return testAccount();
  }

  const stored = loadStoredAccount();
  if (!stored || !stored.email) {
    throw new Error(
      "Account erforderlich! Diese Software erfordert einen aktivierten Benutzeraccount mit Lizenz-Seat.\n" +
      "Bitte führen Sie folgenden Befehl aus:\n" +
      "  clauselens login --email <ihre-email> --key <lizenzschluessel>\n" +
      "Ihren Lizenzschlüssel erhalten Sie nach dem Kauf per E-Mail."
    );
  }

  const currentMachineId = getMachineFingerprint();
  if (stored.machineId && stored.machineId !== currentMachineId) {
    throw new Error(
      "Geräte-Fehlbindung: Dieser Account-Seat ist an ein anderes Gerät gebunden.\n" +
      "Bitte reaktivieren Sie den Seat mit: clauselens login --email " + stored.email + " --key " + stored.licenseKey
    );
  }

  const session = verifyLicenseToken(stored.token, { machineId: currentMachineId, publicKey });
  if (!session.valid) throw new Error(session.error);

  return stored;
}
