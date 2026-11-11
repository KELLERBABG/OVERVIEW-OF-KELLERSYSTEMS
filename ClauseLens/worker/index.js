// ClauseLens licence API on Cloudflare Workers: seat verification for CL- keys
// and Polar licence keys, plus the Polar order webhook.

const POLAR_API_BASE = "https://api.polar.sh/v1/customer-portal/license-keys";
const POLAR_ORGANIZATION_ID = "6898f4c5-7bc8-4ec5-bc97-a16b69b0498c";
const POLAR_BENEFIT_IDS = [
  "2911df21-0aed-41a9-807c-bff4c8f38483",
  "acbe441a-7161-42de-9a0f-4e8000d37846",
];
const TOKEN_TTL_SECONDS = 7 * 24 * 60 * 60;
const WEBHOOK_TOLERANCE_SECONDS = 300;
const RESEND_ENDPOINT = "https://api.resend.com/emails";
const SITE_ORIGIN = "https://clause.kellersystems.dev";
const TIER_LABELS = {
  pro: "ClauseLens Pro",
  team: "ClauseLens Team",
  enterprise: "ClauseLens Enterprise",
};

const CORS_HEADERS = {
  "Access-Control-Allow-Origin": "*",
  "Access-Control-Allow-Headers": "Content-Type, Authorization, X-License-Key",
  "Access-Control-Allow-Methods": "GET, POST, OPTIONS",
};

function json(status, body) {
  return new Response(JSON.stringify(body, null, 2), {
    status,
    headers: { "Content-Type": "application/json; charset=utf-8", ...CORS_HEADERS },
  });
}

// Secrets pasted via CLIs can carry a trailing newline; a stray newline would
// silently break HMAC and RSA verification, so every secret is trimmed on read.
function secretOf(value) {
  return typeof value === "string" ? value.trim() : "";
}

function bytesToBase64(bytes) {
  let binary = "";
  for (const byte of bytes) binary += String.fromCharCode(byte);
  return btoa(binary);
}

function base64ToBytes(text) {
  const binary = atob(String(text));
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function base64UrlToText(value) {
  const normalized = String(value).replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized + "=".repeat((4 - (normalized.length % 4)) % 4);
  return new TextDecoder().decode(base64ToBytes(padded));
}

function bytesToBase64Url(bytes) {
  return bytesToBase64(bytes).replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

function bytesToHex(bytes) {
  return [...bytes].map((b) => b.toString(16).padStart(2, "0")).join("");
}

async function hmacKey(secret) {
  return crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
}

// Matches signPayload() in src/licensing.js: first 16 hex chars of HMAC-SHA256.
async function signPayload(payloadB64, secret) {
  const signature = await crypto.subtle.sign(
    "HMAC",
    await hmacKey(secret),
    new TextEncoder().encode(payloadB64),
  );
  return bytesToHex(new Uint8Array(signature)).slice(0, 16);
}

function timingSafeEqual(a, b) {
  const left = String(a);
  const right = String(b);
  if (left.length !== right.length) return false;
  let diff = 0;
  for (let i = 0; i < left.length; i += 1) diff |= left.charCodeAt(i) ^ right.charCodeAt(i);
  return diff === 0;
}

/**
 * Splits CL-<TIER>-<PAYLOAD>-<SIG>. The payload is base64url, so it may itself
 * contain "-"; the signature is always the 16 hex chars after the last dash.
 * Mirrors parseLicenseKey() in src/licensing.js.
 */
export function parseLicenseKey(licenseKey) {
  const trimmed = String(licenseKey || "").trim();
  if (!trimmed.startsWith("CL-")) return null;
  const rest = trimmed.slice(3);
  const firstDash = rest.indexOf("-");
  if (firstDash <= 0) return null;
  const tier = rest.slice(0, firstDash);
  const remainder = rest.slice(firstDash + 1);
  if (remainder.length < 18) return null;
  const expectedSig = remainder.slice(-16);
  if (remainder.charAt(remainder.length - 17) !== "-") return null;
  const payloadB64 = remainder.slice(0, remainder.length - 17);
  if (!payloadB64) return null;
  return { tier: tier.toLowerCase(), payloadB64, expectedSig };
}

export async function verifyClLicense(licenseKey, secret) {
  if (!secret) return { valid: false, error: "Der Lizenzserver hat kein Signatur-Secret." };
  const parsedKey = parseLicenseKey(licenseKey);
  if (!parsedKey) {
    return { valid: false, error: "Ungültiges Lizenzschlüssel-Format (erwartet CL-<TIER>-<DATA>-<SIG>)." };
  }
  const { payloadB64, expectedSig } = parsedKey;
  const actualSig = await signPayload(payloadB64, secret);
  if (!timingSafeEqual(actualSig, expectedSig)) {
    return { valid: false, error: "Lizenzsignatur ungültig oder manipuliert." };
  }
  let parsed;
  try {
    parsed = JSON.parse(base64UrlToText(payloadB64));
  } catch {
    return { valid: false, error: "Lizenz konnte nicht gelesen werden." };
  }
  const expiresAt = parsed.expiresAt || null;
  if (expiresAt && new Date(expiresAt) < new Date()) {
    return { valid: false, expiresAt, error: `Lizenz ist am ${expiresAt} abgelaufen.` };
  }
  const tier = ["community", "pro", "team", "enterprise"].includes(parsed.tier)
    ? parsed.tier
    : "community";
  return { valid: true, tier, licensee: parsed.licensee || "Kunde", expiresAt };
}

export async function generateLicense({ tier, licensee, expiresAt, secret }) {
  if (!secret) throw new Error("LICENSE_SECRET ist nicht konfiguriert.");
  const payload = JSON.stringify({ tier, licensee, expiresAt });
  const payloadB64 = bytesToBase64Url(new TextEncoder().encode(payload));
  const signature = await signPayload(payloadB64, secret);
  return `CL-${String(tier).toUpperCase()}-${payloadB64}-${signature}`;
}

// Mirrors verifyLicenseToken() in src/licensing.js, which checks this signature offline.
export async function issueLicenseToken(claims, signingKey) {
  if (!signingKey) throw new Error("LICENSE_SIGNING_KEY ist nicht konfiguriert.");
  const payload = { ...claims, exp: Math.floor(Date.now() / 1000) + TOKEN_TTL_SECONDS };
  const body = bytesToBase64Url(new TextEncoder().encode(JSON.stringify(payload)));
  const key = await crypto.subtle.importKey(
    "pkcs8",
    base64ToBytes(signingKey),
    { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const signature = await crypto.subtle.sign("RSASSA-PKCS1-v1_5", key, new TextEncoder().encode(body));
  return `${body}.${bytesToBase64Url(new Uint8Array(signature))}`;
}

async function polarFetch(path, body) {
  try {
    const response = await fetch(POLAR_API_BASE + path, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(body),
    });
    const data = await response.json().catch(() => null);
    return { ok: response.ok, status: response.status, data };
  } catch {
    return { ok: false, status: 0, data: null };
  }
}

// Mirrors validateLicenseKey() + activateLicenseSeat() in src/polar.js.
export async function verifyPolarSeat(licenseKey, machineId) {
  const key = String(licenseKey).trim();
  const validation = await polarFetch("/validate", { key, organization_id: POLAR_ORGANIZATION_ID });
  if (validation.status === 0) {
    return { valid: false, error: "Polar ist nicht erreichbar. Prüfen Sie die Internetverbindung." };
  }
  if (validation.status === 404) {
    return { valid: false, error: "Dieser Lizenzschlüssel ist nicht bekannt." };
  }
  if (!validation.ok || !validation.data) {
    return { valid: false, error: `Polar konnte den Schlüssel nicht prüfen (HTTP ${validation.status}).` };
  }
  const data = validation.data;
  const status = String(data.status || "").toLowerCase();
  const expired = data.expires_at ? Date.parse(data.expires_at) <= Date.now() : false;
  if (status !== "granted" || expired) {
    return { valid: false, error: expired ? "Dieser Lizenzschlüssel ist abgelaufen." : "Dieser Lizenzschlüssel ist nicht gültig." };
  }
  if (!POLAR_BENEFIT_IDS.includes(data.benefit_id)) {
    return { valid: false, error: "Dieser Lizenzschlüssel gehört zu einem anderen Produkt." };
  }
  const tier = POLAR_BENEFIT_IDS.indexOf(data.benefit_id) === 1 ? "team" : "pro";
  let activationId = null;
  if (Number.isInteger(data.limit_activations)) {
    const activation = await polarFetch("/activate", {
      key,
      organization_id: POLAR_ORGANIZATION_ID,
      label: String(machineId || "unknown").slice(0, 32),
    });
    // Some benefits reject activations outright; validation alone still unlocks.
    if (activation.status === 422 || activation.status === 409) {
      const detail = activation.data && activation.data.detail;
      const message = Array.isArray(detail) && detail[0] && detail[0].msg ? detail[0].msg : null;
      return { valid: false, error: message || "Alle Seats dieses Schlüssels sind belegt." };
    }
    if (activation.ok && activation.data) activationId = activation.data.id || null;
  }
  return { valid: true, tier, activationId, expiresAt: data.expires_at || null };
}

async function sha256Short(value) {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(String(value || "")));
  return bytesToHex(new Uint8Array(digest)).slice(0, 8);
}

// Standard Webhooks (Svix) verification, the format Polar sends.
export async function verifyWebhookSignature(headers, rawBody, secret) {
  const id = headers.get("webhook-id");
  const timestamp = headers.get("webhook-timestamp");
  const signatureHeader = headers.get("webhook-signature");
  if (!id || !timestamp || !signatureHeader) return false;
  const age = Math.abs(Date.now() / 1000 - Number(timestamp));
  if (!Number.isFinite(age) || age > WEBHOOK_TOLERANCE_SECONDS) return false;
  const raw = String(secret).startsWith("whsec_") ? String(secret).slice(6) : String(secret);
  let keyBytes;
  try {
    keyBytes = base64ToBytes(raw);
  } catch {
    return false;
  }
  const key = await crypto.subtle.importKey("raw", keyBytes, { name: "HMAC", hash: "SHA-256" }, false, ["sign"]);
  const signedContent = `${id}.${timestamp}.${rawBody}`;
  const signature = bytesToBase64(
    new Uint8Array(await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(signedContent))),
  );
  return String(signatureHeader)
    .split(" ")
    .map((entry) => entry.trim())
    .some((entry) => {
      const [version, value] = entry.split(",");
      return version === "v1" && timingSafeEqual(value, signature);
    });
}

function tierFromOrder(order) {
  const product = String((order && order.product && order.product.name) || "").toLowerCase();
  if (product.includes("enterprise")) return "enterprise";
  if (product.includes("team") || product.includes("kanzlei")) return "team";
  return "pro";
}

function expiresInOneYear() {
  const date = new Date();
  date.setFullYear(date.getFullYear() + 1);
  return date.toISOString().slice(0, 10);
}

function escapeHtml(value) {
  return String(value).replace(/[&<>"]/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;" })[char]);
}

function licenseEmail({ licensee, tier, licenseKey, expiresAt }) {
  const label = TIER_LABELS[tier] || TIER_LABELS.pro;
  const subject = `Ihr ${label} Lizenzschlüssel`;
  const text = [
    `Hallo ${licensee},`,
    "",
    "vielen Dank für Ihren Kauf. Ihr Lizenzschlüssel:",
    "",
    licenseKey,
    "",
    `Tarif: ${label}`,
    `Gültig bis: ${expiresAt}`,
    "",
    `Aktivierung im Browser: ${SITE_ORIGIN} → "Lizenz aktivieren"`,
    `Aktivierung in der CLI: clauselens login --email ${"<Ihre E-Mail>"} --key ${licenseKey}`,
    "",
    "Bewahren Sie den Schlüssel auf. Bei Fragen: info@kellersystems.dev",
  ].join("\n");
  const html = [
    `<p>Hallo ${escapeHtml(licensee)},</p>`,
    "<p>vielen Dank für Ihren Kauf. Ihr Lizenzschlüssel:</p>",
    `<p><strong style="font-family:ui-monospace,monospace">${escapeHtml(licenseKey)}</strong></p>`,
    `<p>Tarif: ${escapeHtml(label)}<br>Gültig bis: ${escapeHtml(expiresAt)}</p>`,
    `<p>Im Browser: <a href="${SITE_ORIGIN}">${SITE_ORIGIN}</a> → "Lizenz aktivieren"<br>`,
    `In der CLI: <code>clauselens login --email &lt;Ihre E-Mail&gt; --key ${escapeHtml(licenseKey)}</code></p>`,
    "<p>Bewahren Sie den Schlüssel auf. Bei Fragen: info@kellersystems.dev</p>",
  ].join("\n");
  return { subject, text, html };
}

async function sendLicenseEmail({ email, licensee, tier, licenseKey, expiresAt }, env) {
  const apiKey = secretOf(env && env.RESEND_API_KEY);
  if (!apiKey) return { sent: false, permanent: true, error: "RESEND_API_KEY ist nicht konfiguriert." };
  const mail = licenseEmail({ licensee, tier, licenseKey, expiresAt });
  let resp;
  try {
    resp = await fetch(RESEND_ENDPOINT, {
      method: "POST",
      headers: { Authorization: `Bearer ${apiKey}`, "Content-Type": "application/json" },
      body: JSON.stringify({
        from: (env && env.LICENSE_FROM_EMAIL) || "ClauseLens <lizenz@kellersystems.dev>",
        to: [email],
        subject: mail.subject,
        text: mail.text,
        html: mail.html,
      }),
    });
  } catch (err) {
    return { sent: false, error: String(err && err.message) };
  }
  if (!resp.ok) {
    const detail = await resp.text().catch(() => "");
    return { sent: false, error: `Resend HTTP ${resp.status}${detail ? ": " + detail.slice(0, 200) : ""}` };
  }
  const data = await resp.json().catch(() => ({}));
  return { sent: true, id: data.id || null };
}

async function handleSeatVerify(request, env) {
  let body;
  try {
    body = await request.json();
  } catch {
    return json(400, { error: "JSON-Body erforderlich." });
  }
  const email = String(body.email || "").trim();
  const licenseKey = String(body.licenseKey || "").trim();
  const machineId = String(body.machineId || "").trim();
  // The browser has no e-mail to send; the CLI passes one for the session label.
  if (!licenseKey) {
    return json(400, { error: "Lizenzschlüssel ist erforderlich." });
  }
  if (!machineId) {
    return json(400, { error: "Geräte-ID ist erforderlich." });
  }
  const signingKey = secretOf(env && env.LICENSE_SIGNING_KEY);
  if (!signingKey) {
    return json(503, { error: "LICENSE_SIGNING_KEY ist nicht konfiguriert." });
  }

  // Polar issues CLA-/LENS- style keys; only CL- keys carry our own signature.
  if (!licenseKey.startsWith("CL-")) {
    const polar = await verifyPolarSeat(licenseKey, machineId);
    if (!polar.valid) return json(401, { error: polar.error });
    const seatId = polar.activationId || `seat_polar_${machineId.slice(0, 8)}`;
    const token = await issueLicenseToken(
      { tier: polar.tier, seatId, machineId, email: email || null },
      signingKey,
    );
    return json(200, {
      valid: true,
      tier: polar.tier,
      seatId,
      activatedAt: new Date().toISOString(),
      expiresAt: polar.expiresAt,
      token,
    });
  }

  const license = await verifyClLicense(licenseKey, secretOf(env && env.LICENSE_SECRET));
  if (!license.valid) return json(401, { error: `Ungültiger Lizenzschlüssel: ${license.error}` });
  const seatId = `seat_${await sha256Short(machineId)}`;
  const token = await issueLicenseToken({ tier: license.tier, seatId, machineId, email: email || null }, signingKey);
  return json(200, {
    valid: true,
    tier: license.tier,
    seatId,
    activatedAt: new Date().toISOString(),
    expiresAt: license.expiresAt,
    token,
  });
}

async function handlePolarWebhook(request, env) {
  const secret = secretOf(env && env.POLAR_WEBHOOK_SECRET);
  if (!secret) {
    return json(503, { error: "POLAR_WEBHOOK_SECRET ist nicht konfiguriert." });
  }
  const rawBody = await request.text();
  if (!(await verifyWebhookSignature(request.headers, rawBody, secret))) {
    return json(401, { error: "Webhook-Signatur ungültig." });
  }
  let event;
  try {
    event = JSON.parse(rawBody);
  } catch {
    return json(400, { error: "Webhook-Body ist kein JSON." });
  }
  if (event.type !== "order.created") {
    return json(200, { received: true, ignored: true });
  }
  const licenseSecret = secretOf(env && env.LICENSE_SECRET);
  if (!licenseSecret) {
    return json(503, { error: "LICENSE_SECRET ist nicht konfiguriert." });
  }
  const order = event.data || {};
  const licensee = (order.customer && (order.customer.name || order.customer.email)) || "ClauseLens Customer";
  const tier = tierFromOrder(order);
  const expiresAt = expiresInOneYear();
  const licenseKey = await generateLicense({
    tier,
    licensee: String(licensee),
    expiresAt,
    secret: licenseSecret,
  });
  const customerEmail = String((order.customer && order.customer.email) || "").trim();
  const delivery = customerEmail
    ? await sendLicenseEmail({ email: customerEmail, licensee: String(licensee), tier, licenseKey, expiresAt }, env)
    : { sent: false, permanent: true, error: "Bestellung enthält keine Kunden-E-Mail." };
  const response = { received: true, tier, licensee, expiresAt, licenseKey, emailSent: delivery.sent };
  if (!delivery.sent) response.emailError = delivery.error;
  // A transient send failure returns 502 so Polar retries the delivery.
  if (customerEmail && !delivery.sent && !delivery.permanent) return json(502, response);
  return json(200, response);
}

export default {
  async fetch(request, env) {
    const url = new URL(request.url);
    if (request.method === "OPTIONS") return new Response(null, { status: 204, headers: CORS_HEADERS });
    if (url.pathname === "/api/v1/health" && request.method === "GET") {
      return json(200, { status: "ok", service: "clauselens-license", version: "1.0.0" });
    }
    if (url.pathname === "/api/v1/seats/verify" && request.method === "POST") {
      return handleSeatVerify(request, env);
    }
    if (
      (url.pathname === "/api/v1/webhook/polar" || url.pathname === "/api/v1/webhook/checkout") &&
      request.method === "POST"
    ) {
      return handlePolarWebhook(request, env);
    }
    return json(404, { error: "Nicht gefunden." });
  },
};
