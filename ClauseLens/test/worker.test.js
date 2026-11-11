import test from "node:test";
import assert from "node:assert/strict";
import { createHmac, generateKeyPairSync } from "node:crypto";
import worker, {
  generateLicense,
  verifyClLicense,
  verifyWebhookSignature,
} from "../worker/index.js";
import {
  generateLicense as generateNodeLicense,
  verifyLicense,
  verifyLicenseToken,
} from "../src/licensing.js";

const BASE = "https://license.test";
const LICENSE_SECRET = "worker-test-secret";

const { privateKey, publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
const SIGNING_KEY = privateKey.export({ type: "pkcs8", format: "der" }).toString("base64");
const PUBLIC_PEM = publicKey.export({ type: "spki", format: "pem" }).toString();
const ENV = { LICENSE_SECRET, LICENSE_SIGNING_KEY: SIGNING_KEY };

function seatRequest(body, extraEnv = {}) {
  return worker.fetch(
    new Request(BASE + "/api/v1/seats/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    }),
    { ...ENV, ...extraEnv },
  );
}

test("worker: health endpoint responds", async () => {
  const res = await worker.fetch(new Request(BASE + "/api/v1/health"), {});
  assert.equal(res.status, 200);
  const body = await res.json();
  assert.equal(body.status, "ok");
});

test("worker: unknown route returns 404", async () => {
  const res = await worker.fetch(new Request(BASE + "/nope"), {});
  assert.equal(res.status, 404);
});

test("worker: signed CL- key verifies and returns a seat with a signed token", async () => {
  const key = generateNodeLicense({ tier: "team", licensee: "Kanzlei Test", expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  const res = await seatRequest({ email: "a@example.com", licenseKey: key, machineId: "machine-abc" });
  assert.equal(res.status, 200);
  const body = await res.json();
  assert.equal(body.valid, true);
  assert.equal(body.tier, "team");
  assert.equal(body.expiresAt, "2099-12-31");
  assert.match(body.seatId, /^seat_[0-9a-f]{8}$/);

  const session = verifyLicenseToken(body.token, { machineId: "machine-abc", publicKey: PUBLIC_PEM });
  assert.equal(session.valid, true);
  assert.equal(session.tier, "team");
  assert.equal(session.seatId, body.seatId);
  assert.equal(session.licensee, "a@example.com");
  assert.ok(new Date(session.expiresAt) > new Date(Date.now() + 6 * 24 * 3600 * 1000));
});

test("worker: secrets with trailing whitespace still verify", async () => {
  const key = generateNodeLicense({ tier: "pro", licensee: "Test", expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  const res = await seatRequest({ licenseKey: key, machineId: "machine-abc" }, {
    LICENSE_SECRET: `${LICENSE_SECRET}\r\n`,
    LICENSE_SIGNING_KEY: `${SIGNING_KEY}\n`,
  });
  assert.equal(res.status, 200);
  const body = await res.json();
  const session = verifyLicenseToken(body.token, { machineId: "machine-abc", publicKey: PUBLIC_PEM });
  assert.equal(session.valid, true, session.error);
});

test("worker: seat token is bound to the machine", async () => {
  const key = generateNodeLicense({ tier: "pro", licensee: "Test", expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  const res = await seatRequest({ email: "a@example.com", licenseKey: key, machineId: "machine-abc" });
  const body = await res.json();
  const other = verifyLicenseToken(body.token, { machineId: "machine-xyz", publicKey: PUBLIC_PEM });
  assert.equal(other.valid, false);
  assert.match(other.error, /anderes Gerät/);
});

test("worker: a forged seat token is rejected", async () => {
  const [body] = "eyJ0aWVyIjoicHJvIn0".split(".");
  const forged = `${body}.AAAA`;
  const session = verifyLicenseToken(forged, { publicKey: PUBLIC_PEM });
  assert.equal(session.valid, false);
});

test("worker: tampered CL- key is rejected", async () => {
  const key = generateNodeLicense({ tier: "pro", licensee: "Test", expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  const tampered = key.slice(0, -1) + (key.endsWith("a") ? "b" : "a");
  const res = await seatRequest({ email: "a@example.com", licenseKey: tampered, machineId: "m" });
  assert.equal(res.status, 401);
});

test("worker: expired CL- key is rejected", async () => {
  const key = generateNodeLicense({ tier: "pro", licensee: "Test", expiresAt: "2020-01-01", secret: LICENSE_SECRET });
  const res = await seatRequest({ email: "a@example.com", licenseKey: key, machineId: "m" });
  assert.equal(res.status, 401);
  const body = await res.json();
  assert.match(body.error, /abgelaufen/);
});

test("worker: missing email or key is a client error", async () => {
  const res = await seatRequest({ licenseKey: "CL-PRO-x-y" });
  assert.equal(res.status, 400);
});

test("worker: seat verification fails closed without secrets", async () => {
  const key = generateNodeLicense({ tier: "pro", licensee: "Test", expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  const noSigning = await seatRequest({ email: "a@example.com", licenseKey: key, machineId: "m" }, { LICENSE_SIGNING_KEY: "" });
  assert.equal(noSigning.status, 503);

  const noSecret = await seatRequest({ email: "a@example.com", licenseKey: key, machineId: "m" }, { LICENSE_SECRET: "" });
  assert.equal(noSecret.status, 401);
});

test("worker: worker-issued keys verify with the node verifier", async () => {
  const key = await generateLicense({ tier: "pro", licensee: "Webhook Kunde", expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  const verified = verifyLicense(key, LICENSE_SECRET);
  assert.equal(verified.valid, true);
  assert.equal(verified.tier, "pro");
});

test("worker: worker keys with a dash in the base64url payload verify", async () => {
  const licensee = "€Lct~<GXo5#B";
  const key = await generateLicense({ tier: "pro", licensee, expiresAt: "2099-12-31", secret: LICENSE_SECRET });
  assert.ok(key.slice("CL-PRO-".length, -17).includes("-"), "Testschlüssel muss einen Bindestrich in der Nutzlast haben");

  const verified = await verifyClLicense(key, LICENSE_SECRET);
  assert.equal(verified.valid, true, verified.error);
  assert.equal(verified.licensee, licensee);
  assert.equal(verifyLicense(key, LICENSE_SECRET).valid, true);
});

test("worker: CL- keys cannot be verified without the secret", async () => {
  const key = generateNodeLicense({ tier: "pro", licensee: "Test", secret: LICENSE_SECRET });
  assert.equal(verifyLicense(key).valid, false);
  assert.equal((await verifyClLicense(key, "")).valid, false);
});

test("worker: Polar key activates a seat through Polar", async () => {
  const calls = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (url, options) => {
    calls.push({ url, body: JSON.parse(options.body) });
    if (url.endsWith("/validate")) {
      return new Response(
        JSON.stringify({
          status: "granted",
          benefit_id: "acbe441a-7161-42de-9a0f-4e8000d37846",
          limit_activations: 5,
          usage: 1,
          expires_at: "2099-12-31T00:00:00Z",
        }),
        { status: 200, headers: { "Content-Type": "application/json" } },
      );
    }
    return new Response(JSON.stringify({ id: "activation-1" }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
  try {
    const res = await seatRequest({ email: "a@example.com", licenseKey: "LENS-TEAM-abc", machineId: "abcdef1234567890" });
    assert.equal(res.status, 200);
    const body = await res.json();
    assert.equal(body.tier, "team");
    assert.equal(body.seatId, "activation-1");
    assert.equal(calls.length, 2);
    assert.equal(calls[0].body.organization_id, "6898f4c5-7bc8-4ec5-bc97-a16b69b0498c");
    assert.equal(calls[1].body.label, "abcdef1234567890");

    const session = verifyLicenseToken(body.token, { machineId: "abcdef1234567890", publicKey: PUBLIC_PEM });
    assert.equal(session.valid, true);
    assert.equal(session.tier, "team");
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("worker: foreign Polar benefit is rejected", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () =>
    new Response(JSON.stringify({ status: "granted", benefit_id: "bf241c3b-other", limit_activations: null }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  try {
    const res = await seatRequest({ email: "a@example.com", licenseKey: "LENS-TEAM-foreign", machineId: "m" });
    assert.equal(res.status, 401);
    const body = await res.json();
    assert.match(body.error, /anderen Produkt/);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

function signedHeaders(secret, id, timestamp, rawBody) {
  const raw = secret.startsWith("whsec_") ? secret.slice(6) : secret;
  const key = Buffer.from(raw, "base64");
  const signature = createHmac("sha256", key).update(`${id}.${timestamp}.${rawBody}`).digest("base64");
  return {
    "Content-Type": "application/json",
    "webhook-id": id,
    "webhook-timestamp": String(timestamp),
    "webhook-signature": "v1," + signature,
  };
}

const SECRET = "whsec_" + Buffer.from("polar-webhook-test-secret").toString("base64");

function polarOrder(body) {
  return JSON.stringify({
    type: "order.created",
    data: { customer: { email: "kunde@example.com", name: "Kunde GmbH" }, product: { name: "ClauseLens Pro" } },
    ...body,
  });
}

function webhookRequest(raw, { env = {}, id = "msg_1", timestamp = Math.floor(Date.now() / 1000) } = {}) {
  return worker.fetch(
    new Request(BASE + "/api/v1/webhook/polar", {
      method: "POST",
      headers: signedHeaders(SECRET, id, timestamp, raw),
      body: raw,
    }),
    { POLAR_WEBHOOK_SECRET: SECRET, LICENSE_SECRET, ...env },
  );
}

test("worker: Polar webhook is rejected without configured secret", async () => {
  const raw = polarOrder();
  const res = await worker.fetch(
    new Request(BASE + "/api/v1/webhook/polar", {
      method: "POST",
      headers: signedHeaders(SECRET, "msg_1", Math.floor(Date.now() / 1000), raw),
      body: raw,
    }),
    {},
  );
  assert.equal(res.status, 503);
});

test("worker: Polar webhook rejects a wrong signature", async () => {
  const raw = polarOrder();
  const res = await worker.fetch(
    new Request(BASE + "/api/v1/webhook/polar", {
      method: "POST",
      headers: signedHeaders(SECRET, "msg_1", Math.floor(Date.now() / 1000), raw + " "),
      body: raw,
    }),
    { POLAR_WEBHOOK_SECRET: SECRET, LICENSE_SECRET },
  );
  assert.equal(res.status, 401);
});

test("worker: Polar order issues a CL- key that the CLI verifies", async () => {
  const res = await webhookRequest(polarOrder(), { id: "msg_2" });
  assert.equal(res.status, 200);
  const body = await res.json();
  assert.equal(body.tier, "pro");
  assert.match(body.licenseKey, /^CL-PRO-/);
  assert.equal(verifyLicense(body.licenseKey, LICENSE_SECRET).valid, true);
});

test("worker: order webhook without LICENSE_SECRET fails instead of minting", async () => {
  const res = await webhookRequest(polarOrder(), { id: "msg_3", env: { LICENSE_SECRET: "" } });
  assert.equal(res.status, 503);
});

test("worker: order webhook emails the licence key via Resend", async () => {
  const calls = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (url, options) => {
    calls.push({ url: String(url), headers: options.headers, body: JSON.parse(options.body) });
    return new Response(JSON.stringify({ id: "mail_1" }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
  try {
    const res = await webhookRequest(polarOrder(), {
      id: "msg_4",
      env: { RESEND_API_KEY: "re_test", LICENSE_FROM_EMAIL: "ClauseLens <lizenz@test.dev>" },
    });
    assert.equal(res.status, 200);
    const body = await res.json();
    assert.equal(body.emailSent, true);
    assert.equal(calls.length, 1);
    assert.match(calls[0].url, /api\.resend\.com\/emails$/);
    assert.equal(calls[0].headers.Authorization, "Bearer re_test");
    assert.equal(calls[0].body.from, "ClauseLens <lizenz@test.dev>");
    assert.deepEqual(calls[0].body.to, ["kunde@example.com"]);
    assert.ok(calls[0].body.text.includes(body.licenseKey));
    assert.ok(calls[0].body.html.includes(body.licenseKey));
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("worker: Resend failure returns 502 so Polar retries the delivery", async () => {
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async () => new Response("domain not verified", { status: 403 });
  try {
    const res = await webhookRequest(polarOrder(), { id: "msg_5", env: { RESEND_API_KEY: "re_test" } });
    assert.equal(res.status, 502);
    const body = await res.json();
    assert.equal(body.emailSent, false);
    assert.match(body.emailError, /HTTP 403/);
    assert.match(body.licenseKey, /^CL-PRO-/);
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("worker: missing RESEND_API_KEY still issues the key without retry", async () => {
  const res = await webhookRequest(polarOrder(), { id: "msg_6" });
  assert.equal(res.status, 200);
  const body = await res.json();
  assert.equal(body.emailSent, false);
  assert.match(body.emailError, /RESEND_API_KEY/);
});

test("worker: webhook signature check rejects stale timestamps", async () => {
  const raw = polarOrder();
  const stale = Math.floor(Date.now() / 1000) - 3600;
  const headers = new Headers(signedHeaders(SECRET, "msg_7", stale, raw));
  assert.equal(await verifyWebhookSignature(headers, raw, SECRET), false);
});
