"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { LICENSE_TOKEN_PUBLIC_KEY, LICENSE_TOKEN_TTL_SECONDS, verifyLicenseTokenWeb } from "../src/licence-token.js";
import { claims, testKeyPair } from "./helpers/license_fixture.mjs";

const { publicPem, sign } = testKeyPair();
const machineId = "web-0123456789abcdef";

test("licence token: a signature from the signing key unlocks the tier", async () => {
  const token = sign(claims());
  const result = await verifyLicenseTokenWeb(token, { machineId, publicKey: publicPem });
  assert.equal(result.valid, true);
  assert.equal(result.tier, "pro");
  assert.equal(result.seatId, "seat_1");
  assert.ok(result.expiresAt);
  assert.equal(result.error, null);
});

test("licence token: a token signed by somebody else is rejected", async () => {
  const other = testKeyPair();
  const result = await verifyLicenseTokenWeb(other.sign(claims()), { machineId, publicKey: publicPem });
  assert.equal(result.valid, false);
  assert.equal(result.error, "Lizenzsitzung ist ungültig.");
  assert.equal(result.tier, "community");
});

test("licence token: tampering with the body invalidates the signature", async () => {
  const token = sign(claims({ tier: "community" }));
  const parts = token.split(".");
  const upgraded = Buffer.from(JSON.stringify(claims({ tier: "enterprise" }))).toString("base64url");
  const result = await verifyLicenseTokenWeb(upgraded + "." + parts[1], { machineId, publicKey: publicPem });
  assert.equal(result.valid, false);
});

test("licence token: expired sessions stop working", async () => {
  const token = sign(claims({ exp: Math.floor(Date.now() / 1000) - 1 }));
  const result = await verifyLicenseTokenWeb(token, { machineId, publicKey: publicPem });
  assert.equal(result.valid, false);
  assert.match(result.error, /abgelaufen/);
});

test("licence token: a session is bound to one machine", async () => {
  const token = sign(claims());
  const result = await verifyLicenseTokenWeb(token, { machineId: "web-ffffffffffffffff", publicKey: publicPem });
  assert.equal(result.valid, false);
  assert.match(result.error, /anderes Gerät/);
});

test("licence token: malformed input is reported, never accepted", async () => {
  assert.equal((await verifyLicenseTokenWeb("", { machineId, publicKey: publicPem })).error, "Keine Lizenzsitzung auf diesem Gerät.");
  assert.equal((await verifyLicenseTokenWeb("abc", { machineId, publicKey: publicPem })).error, "Lizenzsitzung ist beschädigt.");
  assert.equal((await verifyLicenseTokenWeb("abc.def", { machineId, publicKey: publicPem })).error, "Lizenzsitzung ist ungültig.");
});

test("licence token: unknown tiers fall back to community", async () => {
  const token = sign(claims({ tier: "platinum" }));
  const result = await verifyLicenseTokenWeb(token, { machineId, publicKey: publicPem });
  assert.equal(result.valid, true);
  assert.equal(result.tier, "community");
});

test("licence token: the shipped public key is an RSA SPKI key and the TTL is a week", () => {
  assert.match(LICENSE_TOKEN_PUBLIC_KEY, /^-----BEGIN PUBLIC KEY-----/);
  assert.equal(LICENSE_TOKEN_TTL_SECONDS, 604800);
});