import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import {
  TRIAL_DAYS,
  OFFLINE_GRACE_DAYS,
  DEFAULT_ORGANIZATION_ID,
  emptyState,
  trialState,
  evaluateLicense,
  entitlement,
  validateLicenseKey,
} from "../src/licensing.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const DAY = 24 * 60 * 60 * 1000;

/* A fetch stub returning a Polar-shaped answer. */
function polarFetch(status, body) {
  return async () => ({ ok: status >= 200 && status < 300, status, json: async () => body });
}
function offlineFetch() {
  return async () => {
    throw new Error("ECONNREFUSED");
  };
}

/* ------------------------------------------------------------------ *
 * Trial
 * ------------------------------------------------------------------ */

test("Erster Start beginnt die Testphase: 14 Tage, Status aktiv", () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const state = emptyState(now);
  assert.equal(state.firstStartAt, now);
  assert.equal(state.license, null);

  const trial = trialState(state, now);
  assert.equal(trial.status, "active");
  assert.equal(trial.daysLeft, TRIAL_DAYS);
  assert.equal(trial.expiresAt, now + TRIAL_DAYS * DAY);
});

test("Tag 13 der Testphase: noch 1 Tag, weiterhin aktiv", () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const state = emptyState(now);
  const trial = trialState(state, now + 13 * DAY);
  assert.equal(trial.status, "active");
  assert.equal(trial.daysLeft, 1);
});

test("Testphase ist nach 14 Tagen abgelaufen", () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const state = emptyState(now);
  const trial = trialState(state, now + 14 * DAY);
  assert.equal(trial.status, "expired");
  assert.equal(trial.daysLeft, 0);
});

test("Zurückgesetzte Uhr: vergangene Zeit wird geklemmt, nichts wird verlängert", () => {
  const now = Date.UTC(2026, 0, 15, 12);
  const state = emptyState(now);
  /* System clock ten days behind the stamp: must not restart or extend. The
     expiry stays anchored to the stamp, and the countdown never promises more
     days than the trial ever had. */
  const trial = trialState(state, now - 10 * DAY);
  assert.equal(trial.elapsedMs, 0);
  assert.equal(trial.daysLeft, TRIAL_DAYS);
  assert.equal(trial.expiresAt, now + 14 * DAY);
});

test("Unvollständiger State wird wie ein Erststart behandelt", () => {
  const trial = trialState(null, Date.UTC(2026, 0, 1));
  assert.equal(trial.status, "active");
  const trial2 = trialState({ firstStartAt: "nope" }, Date.UTC(2026, 0, 1));
  assert.equal(trial2.status, "active");
});

/* ------------------------------------------------------------------ *
 * Entitlement gating
 * ------------------------------------------------------------------ */

test("Entscheidung während der Testphase: entsperrt, Grund 'trial'", () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const decision = entitlement(emptyState(now), now + 2 * DAY);
  assert.equal(decision.unlocked, true);
  assert.equal(decision.reason, "trial");
  assert.equal(decision.trial.daysLeft, 12);
});

test("Entscheidung nach Ablauf ohne Lizenz: gesperrt, Grund 'trial-expired'", () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const decision = entitlement(emptyState(now), now + 15 * DAY);
  assert.equal(decision.unlocked, false);
  assert.equal(decision.reason, "trial-expired");
});

test("Gültige Lizenz entsperrt auch nach Ablauf der Testphase", () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const state = emptyState(now);
  state.license = { key: "K1", validatedAt: now + 16 * DAY, expiresAt: null };
  const decision = entitlement(state, now + 20 * DAY);
  assert.equal(decision.unlocked, true);
  assert.equal(decision.reason, "license");
});

/* ------------------------------------------------------------------ *
 * Offline grace
 * ------------------------------------------------------------------ */

test("Gültige Lizenz 20 Tage alt: innerhalb des Offline-Fensters, Status 'valid'", () => {
  const now = Date.UTC(2026, 0, 1);
  const license = { key: "K1", validatedAt: now - 20 * DAY, expiresAt: null };
  const standing = evaluateLicense(license, now);
  assert.equal(standing.status, "valid");
});

test("Offline-Fenster nach 30 Tagen überschritten: Sperre, obwohl Schlüssel einst gütig", () => {
  const now = Date.UTC(2026, 0, 1);
  const license = { key: "K1", validatedAt: now - (OFFLINE_GRACE_DAYS + 1) * DAY, expiresAt: null };
  const standing = evaluateLicense(license, now);
  assert.equal(standing.status, "grace-expired");

  const state = { version: 1, firstStartAt: now - 60 * DAY, license };
  const decision = entitlement(state, now);
  assert.equal(decision.unlocked, false);
  assert.equal(decision.reason, "revalidation-needed");
});

test("Lizenz mit Ablaufdatum, das vorbei ist: Grund 'license-expired', Grace zählt nicht", () => {
  const now = Date.UTC(2026, 0, 1);
  const license = { key: "K1", validatedAt: now - 1 * DAY, expiresAt: now - 1 };
  assert.equal(evaluateLicense(license, now).status, "expired");

  const state = { version: 1, firstStartAt: now - 60 * DAY, license };
  const decision = entitlement(state, now);
  assert.equal(decision.unlocked, false);
  assert.equal(decision.reason, "license-expired");
});

test("Frisch validierte Lizenz ist 'valid'", () => {
  const now = Date.UTC(2026, 0, 1);
  const standing = evaluateLicense({ key: "K1", validatedAt: now - 1 * DAY, expiresAt: null }, now);
  assert.equal(standing.status, "valid");
});

/* ------------------------------------------------------------------ *
 * Key validation against Polar
 * ------------------------------------------------------------------ */

test("Gültiger Schlüssel: 200 von Polar wird als Lizenz gespeichert", async () => {
  const now = Date.UTC(2026, 0, 1);
  const calls = [];
  const result = await validateLicenseKey("  abc-123  ", {
    fetchImpl: async (url, init) => {
      calls.push({ url, init });
      return polarFetch(200, { id: "lic_1", status: "granted", expires_at: null })();
    },
    organizationId: "org_test",
    now,
  });
  assert.equal(result.valid, true);
  assert.equal(result.license.key, "abc-123", "Schlüssel wird getrimmt");
  assert.equal(result.license.validatedAt, now);
  assert.equal(result.license.licenseId, "lic_1");

  assert.equal(calls.length, 1);
  const body = JSON.parse(calls[0].init.body);
  assert.equal(body.key, "abc-123");
  assert.equal(body.organization_id, "org_test", "Organisations-ID aus site.config.json wird gesendet");
});

test("Unbekannter Schlüssel: 404 von Polar wird als ungültig gemeldet", async () => {
  const result = await validateLicenseKey("nope", {
    fetchImpl: polarFetch(404, { error: "ResourceNotFound" }),
  });
  assert.equal(result.valid, false);
  assert.equal(result.reason, "invalid");
});

test("Bei 200 mit Status 'revoked' glaubt der Client dem Statusfeld, nicht dem Code", async () => {
  const result = await validateLicenseKey("bad", {
    fetchImpl: polarFetch(200, { status: "revoked" }),
  });
  assert.equal(result.valid, false);
  assert.equal(result.reason, "invalid");
});

test("Leerer Schlüssel wird gar nicht erst gesendet", async () => {
  let called = false;
  const result = await validateLicenseKey("   ", {
    fetchImpl: async () => {
      called = true;
      return polarFetch(200, {})();
    },
  });
  assert.equal(result.valid, false);
  assert.equal(result.reason, "empty");
  assert.equal(called, false);
});

test("Netzwerkfehler: valid=null ('offline'), nicht false — kein falsches Urteil über den Schlüssel", async () => {
  const result = await validateLicenseKey("good-key", { fetchImpl: offlineFetch() });
  assert.equal(result.valid, null);
  assert.equal(result.reason, "offline");
});

test("Serverfehler 503 zählt als offline, nicht als ungültig", async () => {
  const result = await validateLicenseKey("good-key", {
    fetchImpl: polarFetch(503, {}),
  });
  assert.equal(result.valid, null);
  assert.equal(result.reason, "offline");
});

test("Organisations-Id und Endpunkt kommen aus der Konfiguration, Standard als Fallback", async () => {
  let seen = null;
  await validateLicenseKey("k", {
    fetchImpl: async (url, init) => {
      seen = { url, body: JSON.parse(init.body) };
      return polarFetch(200, {})();
    },
  });
  assert.ok(seen.url.includes("api.polar.sh"));
  assert.equal(seen.body.organization_id, DEFAULT_ORGANIZATION_ID);
});

test("Ablaufdatum von Polar landet im gecachten Zustand", async () => {
  const now = Date.UTC(2026, 0, 1);
  const result = await validateLicenseKey("k", {
    fetchImpl: polarFetch(200, { status: "granted", expires_at: "2027-01-01T00:00:00Z" }),
    now,
  });
  assert.equal(result.license.expiresAt, Date.UTC(2027, 0, 1));
  /* And the stored expiry actually gates: within grace and before expiry, the
     license stands; after its expiry date, no grace window can revive it. */
  assert.equal(evaluateLicense(result.license, now + 10 * DAY).status, "valid");
  assert.equal(evaluateLicense(result.license, Date.UTC(2027, 0, 2)).status, "expired");
});

/* ------------------------------------------------------------------ *
 * Integration: activation rescues an expired trial
 * ------------------------------------------------------------------ */

test("Ablaufene Testphase wird durch Aktivierung sofort entsperrt", async () => {
  const now = Date.UTC(2026, 0, 1, 12);
  const state = emptyState(now);
  assert.equal(entitlement(state, now + 20 * DAY).unlocked, false);

  const result = await validateLicenseKey("KEY-1", {
    fetchImpl: polarFetch(200, { status: "granted" }),
    now: now + 20 * DAY,
  });
  state.license = result.license;

  const decision = entitlement(state, now + 20 * DAY);
  assert.equal(decision.unlocked, true);
  assert.equal(decision.reason, "license");
  /* The trial stamp survived: no restart, no second trial. */
  assert.equal(decision.trial.firstStartAt, now);
});

/* ------------------------------------------------------------------ *
 * The shipped configuration stays consistent with the module
 * ------------------------------------------------------------------ */

test("site.config.json nennt dieselbe Testphase und dieselbe Polar-Organisation", () => {
  const site = JSON.parse(readFileSync(join(root, "site.config.json"), "utf8"));
  assert.equal(site.billing.trialDays, TRIAL_DAYS);
  assert.equal(site.billing.organizationId, DEFAULT_ORGANIZATION_ID);
});
