"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import {
  generateLicense,
  verifyLicense,
  signLicenseToken,
  verifyLicenseToken,
  TIERS,
} from "../src/licensing.js";

const SECRET = "unit-test-secret";

const { privateKey, publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
const PUBLIC_PEM = publicKey.export({ type: "spki", format: "pem" }).toString();

function seatToken(claims = {}, options = {}) {
  return signLicenseToken(
    { tier: TIERS.PRO, seatId: "seat_test", machineId: "machine-1", email: "kunde@firma.de", ...claims },
    privateKey,
    options,
  );
}

test("licensing: generate and verify pro license", () => {
  const key = generateLicense({ tier: TIERS.PRO, licensee: "Test GmbH", expiresAt: "2099-01-01", secret: SECRET });
  assert.ok(key.startsWith("CL-PRO-"));

  const verified = verifyLicense(key, SECRET);
  assert.equal(verified.valid, true);
  assert.equal(verified.tier, TIERS.PRO);
  assert.equal(verified.licensee, "Test GmbH");
  assert.equal(verified.entitlements.maxBatchDocs, 50);
});

test("licensing: generate and verify enterprise license", () => {
  const key = generateLicense({ tier: TIERS.ENTERPRISE, licensee: "Enterprise AG", expiresAt: "2099-01-01", secret: SECRET });
  assert.ok(key.startsWith("CL-ENTERPRISE-"));

  const verified = verifyLicense(key, SECRET);
  assert.equal(verified.valid, true);
  assert.equal(verified.tier, TIERS.ENTERPRISE);
  assert.equal(verified.entitlements.maxBatchDocs, 1000);
});

test("licensing: generate and verify team license", () => {
  const key = generateLicense({ tier: TIERS.TEAM, licensee: "Kanzlei Schmidt & Partner", expiresAt: "2099-01-01", secret: SECRET });
  assert.ok(key.startsWith("CL-TEAM-"));

  const verified = verifyLicense(key, SECRET);
  assert.equal(verified.valid, true);
  assert.equal(verified.tier, TIERS.TEAM);
  assert.equal(verified.entitlements.seats, 5);
});

test("licensing: keys stay valid when the base64url payload contains a dash", () => {
  // A base64url payload can contain "-", which the old split("-") parser truncated.
  // This licensee is chosen so its base64url payload really contains a dash.
  const licensee = "€Lct~<GXo5#B";
  const key = generateLicense({ tier: TIERS.PRO, licensee, expiresAt: "2099-01-01", secret: SECRET });
  const payloadB64 = key.slice("CL-PRO-".length, -17);
  assert.ok(payloadB64.includes("-"), "Testschlüssel muss einen Bindestrich in der Nutzlast haben");

  const verified = verifyLicense(key, SECRET);
  assert.equal(verified.valid, true, verified.error);
  assert.equal(verified.licensee, licensee);

  const tampered = key.slice(0, -4) + "0000";
  assert.equal(verifyLicense(tampered, SECRET).valid, false);
});

test("licensing: keys cannot be minted or checked without a secret", () => {
  assert.throws(() => generateLicense({ tier: TIERS.PRO }), /Server-Secret/);

  const key = generateLicense({ tier: TIERS.PRO, licensee: "Ohne Secret", secret: SECRET });
  const checked = verifyLicense(key);
  assert.equal(checked.valid, false);
  assert.match(checked.error, /nur vom Lizenzserver/);
});

test("licensing: reject tampered licence key", () => {
  const key = generateLicense({ tier: TIERS.PRO, licensee: "Manipuliert", secret: SECRET });
  const tampered = key.slice(0, -4) + "XXXX";
  const verified = verifyLicense(tampered, SECRET);

  assert.equal(verified.valid, false);
  assert.equal(verified.tier, TIERS.COMMUNITY);
  assert.match(verified.error, /Signatur|manipuliert/i);
});

test("licensing: reject expired licence", () => {
  const expiredKey = generateLicense({ tier: TIERS.PRO, licensee: "Abgelaufen", expiresAt: "2020-01-01", secret: SECRET });
  const verified = verifyLicense(expiredKey, SECRET);

  assert.equal(verified.valid, false);
  assert.match(verified.error, /abgelaufen/i);
});

test("licensing: seat token verifies offline and carries the tier", () => {
  const session = verifyLicenseToken(seatToken({ tier: TIERS.TEAM }), {
    machineId: "machine-1",
    publicKey: PUBLIC_PEM,
  });

  assert.equal(session.valid, true);
  assert.equal(session.tier, TIERS.TEAM);
  assert.equal(session.licensee, "kunde@firma.de");
  assert.equal(session.seatId, "seat_test");
  assert.equal(session.entitlements.seats, 5);
});

test("licensing: seat token expires after the grace window", () => {
  const soon = seatToken({}, { ttlSeconds: 60 });
  assert.equal(verifyLicenseToken(soon, { publicKey: PUBLIC_PEM, now: Date.now() + 61_000 }).valid, false);
  assert.match(
    verifyLicenseToken(soon, { publicKey: PUBLIC_PEM, now: Date.now() + 61_000 }).error,
    /abgelaufen/,
  );
});

test("licensing: seat token is machine bound", () => {
  const session = verifyLicenseToken(seatToken(), { machineId: "machine-2", publicKey: PUBLIC_PEM });
  assert.equal(session.valid, false);
  assert.match(session.error, /anderes Gerät/);
});

test("licensing: forged or foreign seat tokens are rejected", () => {
  assert.equal(verifyLicenseToken(null, { publicKey: PUBLIC_PEM }).valid, false);
  assert.equal(verifyLicenseToken("kaputt", { publicKey: PUBLIC_PEM }).valid, false);

  const [body] = seatToken().split(".");
  assert.equal(verifyLicenseToken(body + ".AAAA", { publicKey: PUBLIC_PEM }).valid, false);

  const foreign = generateKeyPairSync("rsa", { modulusLength: 2048 });
  const foreignToken = signLicenseToken(
    { tier: TIERS.ENTERPRISE, seatId: "seat_x", machineId: "machine-1" },
    foreign.privateKey,
  );
  assert.equal(verifyLicenseToken(foreignToken, { publicKey: PUBLIC_PEM }).valid, false);
});
