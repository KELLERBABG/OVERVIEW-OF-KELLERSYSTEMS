"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import {
  LICENSE_SERVER,
  LICENSE_SERVER_FALLBACK,
  activateLicenseSeat,
  describeDevice,
  deviceLabel,
  hashLabel,
  polarStatusMessage,
  verifyLicenseToken,
} from "../src/polar.js";
import { claims, testKeyPair } from "./helpers/license_fixture.mjs";

const { publicPem, sign } = testKeyPair();
const MACHINE = "web-0123456789abcdef";

function workerFetch(handler) {
  const calls = [];
  const fn = async (url, init) => {
    calls.push({ url, body: JSON.parse(init.body) });
    const reply = handler(calls[calls.length - 1], calls.length);
    return {
      ok: reply.status >= 200 && reply.status < 300,
      status: reply.status,
      json: async () => reply.data,
    };
  };
  return { fn, calls };
}

const granted = (token) => ({ status: 200, data: { status: "ok", token, tier: "pro", seatId: "seat_1" } });

test("polar: activation posts key and machine id to the licence server", async () => {
  const token = sign(claims());
  const { fn, calls } = workerFetch(() => granted(token));
  const result = await activateLicenseSeat(" CLA-PRO-abc ", MACHINE, { fetch: fn, publicKey: publicPem });
  assert.equal(result.ok, true);
  assert.equal(result.tier, "pro");
  assert.equal(result.key, "CLA-PRO-abc");
  assert.equal(result.token, token);
  assert.ok(result.expiresAt);
  assert.equal(calls.length, 1);
  assert.equal(calls[0].url, LICENSE_SERVER + "/api/v1/seats/verify");
  assert.deepEqual(calls[0].body, { licenseKey: "CLA-PRO-abc", machineId: MACHINE });
});

test("polar: an empty key or missing machine id is rejected locally", async () => {
  const { fn, calls } = workerFetch(() => granted(sign(claims())));
  assert.equal((await activateLicenseSeat("   ", MACHINE, { fetch: fn })).reason, "invalid");
  assert.equal((await activateLicenseSeat("CLA-PRO-abc", "", { fetch: fn })).message, "Geräte-ID fehlt.");
  assert.equal(calls.length, 0);
});

test("polar: a 403 from the custom domain falls back to workers.dev", async () => {
  const token = sign(claims());
  const { fn, calls } = workerFetch((call) =>
    call.url.startsWith(LICENSE_SERVER_FALLBACK) ? granted(token) : { status: 403, data: null },
  );
  const result = await activateLicenseSeat("CLA-PRO-abc", MACHINE, { fetch: fn, publicKey: publicPem });
  assert.equal(result.ok, true);
  assert.equal(calls.length, 2);
  assert.equal(calls[1].url, LICENSE_SERVER_FALLBACK + "/api/v1/seats/verify");
});

test("polar: an error response is surfaced and a full seat pool is explained", async () => {
  const { fn } = workerFetch(() => ({ status: 200, data: { status: "ok", token: sign(claims()) } }));
  const limited = await activateLicenseSeat("CLA-PRO-abc", MACHINE, {
    fetch: workerFetch(() => ({ status: 422, data: { error: "Alle Seats belegt." } })).fn,
    publicKey: publicPem,
  });
  assert.equal(limited.ok, false);
  assert.equal(limited.reason, "http");
  assert.equal(limited.message, "Alle Seats belegt.");
  const ok = await activateLicenseSeat("CLA-PRO-abc", MACHINE, { fetch: fn, publicKey: publicPem });
  assert.equal(ok.ok, true);
});

test("polar: unreachable server is reported, not treated as valid", async () => {
  const fn = async () => {
    throw new TypeError("Failed to fetch");
  };
  const result = await activateLicenseSeat("CLA-PRO-abc", MACHINE, { fetch: fn });
  assert.equal(result.ok, false);
  assert.equal(result.reason, "network");
  assert.equal(result.message, polarStatusMessage("network"));
});

test("polar: a server answer without a valid signature does not unlock", async () => {
  const forged = sign(claims());
  const other = testKeyPair();
  const forgedByOther = other.sign(claims());
  const { fn } = workerFetch(() => granted(forgedByOther));
  const result = await activateLicenseSeat("CLA-PRO-abc", MACHINE, { fetch: fn, publicKey: publicPem });
  assert.equal(result.ok, false);
  assert.equal(result.reason, "token");
  assert.equal(forged.length > 0, true);
});

test("polar: verifyLicenseToken delegates to the browser verifier", async () => {
  const token = sign(claims());
  const result = await verifyLicenseToken(token, { machineId: MACHINE, publicKey: publicPem });
  assert.equal(result.valid, true);
  assert.equal(result.tier, "pro");
});

test("polar: device label is stable and depends on the device", async () => {
  const envA = {
    navigator: { userAgent: "Mozilla/5.0 A", language: "de-DE", platform: "Win32", hardwareConcurrency: 8 },
    screen: { width: 1920, height: 1080, colorDepth: 24 },
    timeZone: "Europe/Berlin",
  };
  const label = deviceLabel(envA);
  assert.match(label, /^web-[0-9a-f]{16}$/);
  assert.equal(label, deviceLabel(envA));
  const other = deviceLabel({ ...envA, navigator: { ...envA.navigator, userAgent: "Mozilla/5.0 B" } });
  assert.notEqual(label, other);
  assert.match(describeDevice(envA), /Europe\/Berlin/);
  assert.match(hashLabel("abc"), /^[0-9a-f]{16}$/);
  assert.notEqual(hashLabel("abc"), hashLabel("abd"));
});