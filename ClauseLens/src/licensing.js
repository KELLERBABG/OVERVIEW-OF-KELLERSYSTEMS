"use strict";

import { createHmac, createPublicKey, sign as signData, verify as verifyData } from "node:crypto";
import { LICENSE_TOKEN_PUBLIC_KEY, LICENSE_TOKEN_TTL_SECONDS } from "./licence-token.js";

export { LICENSE_TOKEN_PUBLIC_KEY, LICENSE_TOKEN_TTL_SECONDS };

export const TIERS = Object.freeze({
  COMMUNITY: "community",
  PRO: "pro",
  TEAM: "team",
  ENTERPRISE: "enterprise",
});

export const TIER_ENTITLEMENTS = Object.freeze({
  [TIERS.COMMUNITY]: Object.freeze({
    tier: TIERS.COMMUNITY,
    displayName: "ClauseLens Community (Trial)",
    htmlReports: true, // basic preview
    batchAnalysis: false,
    comparisonDiff: true, // basic diff
    customConfig: false,
    restApi: true, // rate-limited local use
    maxBatchDocs: 1,
    seats: 1,
    exportJsonSchema: true,
  }),
  [TIERS.PRO]: Object.freeze({
    tier: TIERS.PRO,
    displayName: "ClauseLens Professional",
    htmlReports: true,
    batchAnalysis: true,
    comparisonDiff: true,
    customConfig: true,
    restApi: true,
    maxBatchDocs: 50,
    seats: 1,
    exportJsonSchema: true,
  }),
  [TIERS.TEAM]: Object.freeze({
    tier: TIERS.TEAM,
    displayName: "ClauseLens Team & Kanzlei",
    htmlReports: true,
    batchAnalysis: true,
    comparisonDiff: true,
    customConfig: true,
    restApi: true,
    maxBatchDocs: 250,
    seats: 5,
    exportJsonSchema: true,
  }),
  [TIERS.ENTERPRISE]: Object.freeze({
    tier: TIERS.ENTERPRISE,
    displayName: "ClauseLens Enterprise",
    htmlReports: true,
    batchAnalysis: true,
    comparisonDiff: true,
    customConfig: true,
    restApi: true,
    maxBatchDocs: 1000,
    seats: Infinity,
    exportJsonSchema: true,
  }),
});

function signPayload(payloadStr, secret) {
  return createHmac("sha256", secret).update(payloadStr).digest("hex").slice(0, 16);
}

/**
 * Splits CL-<TIER>-<PAYLOAD>-<SIG>. The payload is base64url, so it may itself
 * contain "-"; the signature is always the 16 hex chars after the last dash.
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

/**
 * Issues a CL- key. Only the licence worker holds the secret these days.
 */
export function generateLicense({
  tier = TIERS.PRO,
  licensee = "ClauseLens Customer",
  expiresAt = "2099-12-31",
  secret,
} = {}) {
  if (!secret) throw new Error("Lizenzschlüssel können nur mit dem Server-Secret ausgestellt werden.");
  const normTier = String(tier).toLowerCase();
  if (!Object.values(TIERS).includes(normTier)) {
    throw new Error(`Ungültiger Lizenz-Tier: ${tier}`);
  }
  const payloadObj = {
    tier: normTier,
    licensee: String(licensee).trim(),
    expiresAt: String(expiresAt).trim(),
  };
  const payloadStr = JSON.stringify(payloadObj);
  const payloadB64 = Buffer.from(payloadStr, "utf-8").toString("base64url");
  const signature = signPayload(payloadB64, secret);
  return `CL-${normTier.toUpperCase()}-${payloadB64}-${signature}`;
}

/**
 * Checks a CL- key signature. The shipped CLI has no secret, so only the licence worker can do this.
 */
export function verifyLicense(licenseKey, secret) {
  const invalid = (error) => ({
    valid: false,
    tier: TIERS.COMMUNITY,
    licensee: "Community Edition",
    expiresAt: null,
    error,
    entitlements: TIER_ENTITLEMENTS[TIERS.COMMUNITY],
  });

  if (!secret) return invalid("CL- Schlüssel werden nur vom Lizenzserver geprüft.");
  if (!licenseKey || typeof licenseKey !== "string") return invalid("Kein Lizenzschlüssel angegeben.");

  const parsedKey = parseLicenseKey(licenseKey);
  if (!parsedKey) return invalid("Ungültiges Lizenzschlüssel-Format.");

  const { payloadB64, expectedSig } = parsedKey;
  const actualSig = signPayload(payloadB64, secret);
  if (actualSig !== expectedSig) return invalid("Lizenzsignatur ungültig oder manipuliert.");

  try {
    const parsed = JSON.parse(Buffer.from(payloadB64, "base64url").toString("utf-8"));
    const expiry = parsed.expiresAt ? new Date(parsed.expiresAt) : null;
    if (expiry && expiry < new Date()) return invalid(`Lizenz ist am ${parsed.expiresAt} abgelaufen.`);
    const validTier = Object.values(TIERS).includes(parsed.tier) ? parsed.tier : TIERS.COMMUNITY;
    return {
      valid: true,
      tier: validTier,
      licensee: parsed.licensee || "Kunde",
      expiresAt: parsed.expiresAt || null,
      error: null,
      entitlements: TIER_ENTITLEMENTS[validTier],
    };
  } catch (err) {
    return invalid(`Lizenz konnte nicht gelesen werden: ${err.message}`);
  }
}

/**
 * Signs a short-lived seat token. Used by the worker and by tests; the CLI only verifies.
 */
export function signLicenseToken(claims, privateKey, { ttlSeconds = LICENSE_TOKEN_TTL_SECONDS, now = Date.now() } = {}) {
  if (!privateKey) throw new Error("Für Lizenzsitzungen ist ein Signaturschlüssel erforderlich.");
  const payload = { ...claims, exp: Math.floor(now / 1000) + ttlSeconds };
  const body = Buffer.from(JSON.stringify(payload), "utf-8").toString("base64url");
  const signature = signData("sha256", Buffer.from(body), privateKey);
  return `${body}.${signature.toString("base64url")}`;
}

/**
 * Verifies a seat token offline. A week of offline use per successful check.
 */
export function verifyLicenseToken(token, { machineId, publicKey = LICENSE_TOKEN_PUBLIC_KEY, now = Date.now() } = {}) {
  const invalid = (error) => ({
    valid: false,
    tier: TIERS.COMMUNITY,
    licensee: null,
    expiresAt: null,
    seatId: null,
    error,
    entitlements: TIER_ENTITLEMENTS[TIERS.COMMUNITY],
  });

  if (!token || typeof token !== "string") return invalid("Keine Lizenzsitzung auf diesem Gerät.");

  const [body, signature] = token.split(".");
  if (!body || !signature) return invalid("Lizenzsitzung ist beschädigt.");

  let signed = false;
  try {
    signed = verifyData("sha256", Buffer.from(body), createPublicKey(publicKey), Buffer.from(signature, "base64url"));
  } catch {
    signed = false;
  }
  if (!signed) return invalid("Lizenzsitzung ist ungültig.");

  let claims;
  try {
    claims = JSON.parse(Buffer.from(body, "base64url").toString("utf-8"));
  } catch {
    return invalid("Lizenzsitzung ist beschädigt.");
  }
  if (!claims || typeof claims.exp !== "number") return invalid("Lizenzsitzung ist beschädigt.");
  if (claims.exp * 1000 <= now) {
    return invalid("Ihre Lizenzsitzung ist abgelaufen. Bitte erneut anmelden: clauselens login --email <email> --key <lizenzschlüssel>");
  }
  if (machineId && claims.machineId && claims.machineId !== machineId) {
    return invalid("Diese Lizenzsitzung ist an ein anderes Gerät gebunden.");
  }

  const tier = Object.values(TIERS).includes(claims.tier) ? claims.tier : TIERS.COMMUNITY;
  return {
    valid: true,
    tier,
    licensee: claims.email || null,
    expiresAt: new Date(claims.exp * 1000).toISOString(),
    seatId: claims.seatId || null,
    error: null,
    entitlements: TIER_ENTITLEMENTS[tier],
  };
}
