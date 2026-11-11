import { verifyLicenseTokenWeb } from "./licence-token.js";

// The licence worker validates Polar keys server-side and signs the session token.
export const LICENSE_SERVER = "https://license.kellersystems.dev";
// Cloudflare's Bot Fight Mode answers scripted calls on the custom domain with 403.
export const LICENSE_SERVER_FALLBACK = "https://clauselens-license.papababg02.workers.dev";
export const REQUEST_TIMEOUT_MS = 8000;

const MESSAGES = {
  invalid: "Bitte geben Sie einen Lizenzschlüssel ein.",
  network: "Der Lizenzserver ist nicht erreichbar. Prüfen Sie die Internetverbindung und versuchen Sie es erneut.",
  http: "Der Lizenzserver konnte den Schlüssel nicht prüfen.",
  seat_limit: "Alle Seats dieses Schlüssels sind belegt. Geben Sie einen Seat im Polar-Kundenportal frei oder erwerben Sie eine weitere Lizenz.",
  token: "Der Lizenzserver hat keine gültige Sitzung ausgestellt. Bitte erneut versuchen.",
};

export function polarStatusMessage(reason) {
  return MESSAGES[reason] || MESSAGES.http;
}

// Two FNV-1a passes, enough to keep two "devices" from colliding in a seat label.
export function hashLabel(input) {
  const text = String(input || "");
  let a = 0x811c9dc5;
  let b = 0x01000193;
  for (let i = 0; i < text.length; i += 1) {
    const code = text.charCodeAt(i);
    a = Math.imul(a ^ code, 0x01000193) >>> 0;
    b = Math.imul(b + code, 0x85ebca6b) >>> 0;
  }
  return a.toString(16).padStart(8, "0") + b.toString(16).padStart(8, "0");
}

export function describeDevice(env = undefined) {
  const nav = env && env.navigator ? env.navigator : (typeof navigator !== "undefined" ? navigator : {});
  const scr = env && env.screen ? env.screen : (typeof screen !== "undefined" ? screen : {});
  let timeZone = env && env.timeZone;
  if (!timeZone) {
    try {
      timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone || "";
    } catch {
      timeZone = "";
    }
  }
  return [
    nav.userAgent || "",
    nav.language || "",
    nav.platform || "",
    nav.hardwareConcurrency || "",
    timeZone,
    scr.width || "",
    scr.height || "",
    scr.colorDepth || "",
  ].join("|");
}

export function deviceLabel(env = undefined) {
  return "web-" + hashLabel(describeDevice(env));
}

function maskKey(key) {
  const text = String(key || "");
  if (text.length <= 14) return text;
  return text.slice(0, 9) + "..." + text.slice(-4);
}

async function postSeat(base, payload, options = {}) {
  const doFetch = options.fetch || (typeof fetch === "function" ? fetch : null);
  if (!doFetch) return { networkError: true };
  const controller = typeof AbortController === "function" ? new AbortController() : null;
  const timer = controller ? setTimeout(() => controller.abort(), options.timeoutMs || REQUEST_TIMEOUT_MS) : null;
  try {
    const response = await doFetch(base + "/api/v1/seats/verify", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
      signal: controller ? controller.signal : undefined,
    });
    const data = await response.json().catch(() => null);
    return { ok: response.ok, httpStatus: response.status, data };
  } catch (err) {
    return { networkError: true, error: (err && err.message) || String(err) };
  } finally {
    if (timer) clearTimeout(timer);
  }
}

export async function verifyLicenseToken(token, options = {}) {
  const machineId = options.machineId || deviceLabel(options.env);
  return verifyLicenseTokenWeb(token, { ...options, machineId });
}

// Activation goes through the licence server, which checks Polar and signs the session.
export async function activateLicenseSeat(key, label, options = {}) {
  const trimmed = String(key || "").trim();
  if (!trimmed) return { ok: false, reason: "invalid", message: polarStatusMessage("invalid") };
  const seatLabel = String(label || "").trim();
  if (!seatLabel) return { ok: false, reason: "invalid", message: "Geräte-ID fehlt." };

  const servers = [options.server || LICENSE_SERVER, LICENSE_SERVER_FALLBACK];
  for (let i = 0; i < servers.length; i += 1) {
    const result = await postSeat(servers[i], { licenseKey: trimmed, machineId: seatLabel }, options);
    if (result.networkError) continue;
    // 403 on the custom domain means the bot fight mode answered, not a licence error.
    if (result.httpStatus === 403 && i < servers.length - 1) continue;
    if (!result.ok || !result.data) {
      const message = (result.data && result.data.error) || polarStatusMessage("http") + " (HTTP " + result.httpStatus + ")";
      return { ok: false, reason: "http", message };
    }
    const session = await verifyLicenseTokenWeb(result.data.token, {
      machineId: seatLabel,
      publicKey: options.publicKey,
    });
    if (!session.valid) return { ok: false, reason: "token", message: polarStatusMessage("token") };
    return {
      ok: true,
      reason: "granted",
      key: trimmed,
      tier: session.tier,
      token: result.data.token,
      seatId: result.data.seatId || session.seatId,
      expiresAt: session.expiresAt,
      label: seatLabel,
      displayKey: maskKey(trimmed),
      message: "Lizenz aktiviert.",
    };
  }
  return { ok: false, reason: "network", message: polarStatusMessage("network") };
}
