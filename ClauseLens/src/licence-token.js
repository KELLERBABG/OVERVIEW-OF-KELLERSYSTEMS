// Browser-safe half of the licence session: the worker signs, the browser verifies.
// Kept free of node:crypto so index.html and studio.html can load it.

export const LICENSE_TOKEN_PUBLIC_KEY = `-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAx4BLJOYSSuOaSXzWgyAX
xpGy5WFwUYSk2Xa8s1sQcIVhr5QoMMqdqmQWRNFENgOlj9qE4GPPVWeiFUm7pjO+
IlUr76iL7e5wU7NsQjthdggdOw/IJ876zj6rz5dnzGmUJ0uYq7/P7Aht8E+1F9Ap
LuEhEDzUK3TZewLZVIEao0YsGjx9Jf35/dRo0MmUk0OJzLfvoN1TsTXIb+Fkrrep
N/G0ZVJxe/JbBYdPRh52ONbQO+RjGEGDwsG6ffPMvQk2cY1m1Rsm69KbKmGkCXbA
yemcusQ9lSeACsnsE14uIwAjynABRyE8nLfEw5mF5fCNznwhuk5e1pg6nGITD9Cv
nQIDAQAB
-----END PUBLIC KEY-----`;

// A seat stays usable offline for a week after the last successful check.
export const LICENSE_TOKEN_TTL_SECONDS = 7 * 24 * 60 * 60;

const TIERS = ["community", "pro", "team", "enterprise"];

function base64ToBytes(text) {
  const binary = atob(text);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i += 1) bytes[i] = binary.charCodeAt(i);
  return bytes;
}

function base64UrlToBytes(value) {
  const normalized = String(value).replace(/-/g, "+").replace(/_/g, "/");
  const padded = normalized + "=".repeat((4 - (normalized.length % 4)) % 4);
  return base64ToBytes(padded);
}

function pemToDer(pem) {
  return base64ToBytes(String(pem).replace(/-----[^-]+-----/g, "").replace(/\s+/g, ""));
}

export async function verifyLicenseTokenWeb(token, options = {}) {
  const { machineId, now = Date.now(), publicKey = LICENSE_TOKEN_PUBLIC_KEY } = options;
  const invalid = (error) => ({ valid: false, tier: "community", expiresAt: null, seatId: null, error });
  if (!token || typeof token !== "string") return invalid("Keine Lizenzsitzung auf diesem Gerät.");

  const [body, signature] = token.split(".");
  if (!body || !signature) return invalid("Lizenzsitzung ist beschädigt.");

  let signed = false;
  try {
    const key = await crypto.subtle.importKey(
      "spki",
      pemToDer(publicKey),
      { name: "RSASSA-PKCS1-v1_5", hash: "SHA-256" },
      false,
      ["verify"],
    );
    signed = await crypto.subtle.verify(
      "RSASSA-PKCS1-v1_5",
      key,
      base64UrlToBytes(signature),
      new TextEncoder().encode(body),
    );
  } catch {
    signed = false;
  }
  if (!signed) return invalid("Lizenzsitzung ist ungültig.");

  let claims;
  try {
    claims = JSON.parse(new TextDecoder().decode(base64UrlToBytes(body)));
  } catch {
    return invalid("Lizenzsitzung ist beschädigt.");
  }
  if (!claims || typeof claims.exp !== "number") return invalid("Lizenzsitzung ist beschädigt.");
  if (claims.exp * 1000 <= now) return invalid("Ihre Lizenzsitzung ist abgelaufen. Bitte erneut aktivieren.");
  if (machineId && claims.machineId && claims.machineId !== machineId) {
    return invalid("Diese Lizenzsitzung ist an ein anderes Gerät gebunden.");
  }

  const tier = TIERS.includes(claims.tier) ? claims.tier : "community";
  return {
    valid: true,
    tier,
    expiresAt: new Date(claims.exp * 1000).toISOString(),
    seatId: claims.seatId || null,
    error: null,
  };
}
