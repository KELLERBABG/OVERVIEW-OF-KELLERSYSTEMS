/**
 * Licensing core for the BauLens desktop application.
 *
 * The trial runs from the first start; a license is a Polar license key. The
 * entitlement decision is pure: state in, decision out, with the clock and the
 * network injected, so the same code runs in the webview, and the test suite
 * drives it without waiting or fetching.
 *
 * Contract with Polar (see docs/PAYMENTS_AND_LEGAL.md):
 *
 *   POST https://api.polar.sh/v1/customer-portal/license-keys/validate
 *   { "key": "…", "organization_id": "…" }
 *
 * The endpoint is deliberately unauthenticated — Polar documents it as safe
 * for public clients such as desktop applications, so no secret lives in this
 * repository and no backend has to be operated. A valid key answers 200 with
 * the license object; an unknown, revoked or expired key answers 404. Network
 * failure is not an answer: a previously validated license stays usable for a
 * grace window (OFFLINE_GRACE_DAYS) and the app keeps working offline, which
 * is a supported state, not an error.
 *
 * The engine itself stays unrestricted — this module only decides whether the
 * desktop *application* is unlocked. Rules exist exactly once, and no rule
 * consults a license.
 */

/** Trial length, in days, from the first start. Mirrors billing.trialDays in
    site.config.json; test/licensing.test.js pins the two together. */
export const TRIAL_DAYS = 14;

/** How long a validated license keeps working without a network. Beyond this
    window the app needs one successful re-validation to stay unlocked. */
export const OFFLINE_GRACE_DAYS = 30;

export const POLAR_VALIDATE_ENDPOINT =
  "https://api.polar.sh/v1/customer-portal/license-keys/validate";

/** Fallback organization id; the staged desktop config carries the same value
    from site.config.json and takes precedence. The test suite pins them. */
export const DEFAULT_ORGANIZATION_ID = "6898f4c5-7bc8-4ec5-bc97-a16b69b0498c";

const DAY_MS = 24 * 60 * 60 * 1000;

/** The persisted state, as written by the desktop shell into the app's own
    config directory. Everything else in the file is ignored. */
export function emptyState(now = Date.now()) {
  return { version: 1, firstStartAt: now, license: null };
}

/**
 * Trial window for a state.
 *
 * A first-start timestamp in the future (clock rolled back after the trial
 * began) is clamped rather than punished: the elapsed time is never negative,
 * so the trial neither shortens nor restarts.
 */
export function trialState(state, now = Date.now(), trialDays = TRIAL_DAYS) {
  const firstStartAt =
    state && typeof state.firstStartAt === "number" && isFinite(state.firstStartAt)
      ? state.firstStartAt
      : now;
  const elapsed = Math.max(0, now - firstStartAt);
  const expiresAt = firstStartAt + trialDays * DAY_MS;
  /* With a rolled-back clock the raw remainder can exceed the trial length;
     never promise more days than the trial ever had. */
  const daysLeft = Math.max(0, Math.min(trialDays, Math.ceil((expiresAt - now) / DAY_MS)));
  return {
    firstStartAt,
    expiresAt,
    elapsedMs: Math.min(elapsed, expiresAt - firstStartAt),
    daysLeft,
    status: now < expiresAt ? "active" : "expired",
  };
}

/**
 * Standing of a stored license.
 *
 *   none          no license was ever validated
 *   valid         validated, within the offline grace window
 *   grace-expired validated, but longer ago than the grace window; the app
 *                 needs one successful re-validation to stay unlocked
 *   expired       the license itself carries an expiry date (Polar
 *                 `expires_at`) that has passed — the grace window cannot
 *                 outlive the license
 */
export function evaluateLicense(license, now = Date.now(), graceDays = OFFLINE_GRACE_DAYS) {
  if (!license || typeof license.validatedAt !== "number" || !isFinite(license.validatedAt)) {
    return { status: "none", ageMs: null, expiresAt: null };
  }
  const ageMs = Math.max(0, now - license.validatedAt);
  const expiresAt = typeof license.expiresAt === "number" && isFinite(license.expiresAt)
    ? license.expiresAt
    : null;

  if (expiresAt !== null && now >= expiresAt) {
    return { status: "expired", ageMs, expiresAt };
  }
  if (ageMs > graceDays * DAY_MS) {
    return { status: "grace-expired", ageMs, expiresAt };
  }
  return { status: "valid", ageMs, expiresAt };
}

/**
 * The decision the UI acts on.
 *
 * reason:
 *   license               a valid license (possibly from the offline cache)
 *   trial                 the trial is running, daysLeft says for how long
 *   trial-expired         no license, trial over — paywall
 *   license-expired       the license itself expired; trial also over — paywall
 *   revalidation-needed   validated once, offline for longer than the grace
 *                         window — paywall until one successful re-validation
 */
export function entitlement(state, now = Date.now(), options = {}) {
  const trialDays = options.trialDays || TRIAL_DAYS;
  const graceDays = options.graceDays || OFFLINE_GRACE_DAYS;
  const trial = trialState(state, now, trialDays);
  const license = evaluateLicense(state && state.license, now, graceDays);

  if (license.status === "valid") {
    return { unlocked: true, reason: "license", trial, license };
  }
  if (trial.status === "active") {
    return { unlocked: true, reason: "trial", trial, license };
  }
  if (license.status === "grace-expired") {
    return { unlocked: false, reason: "revalidation-needed", trial, license };
  }
  if (license.status === "expired") {
    return { unlocked: false, reason: "license-expired", trial, license };
  }
  return { unlocked: false, reason: "trial-expired", trial, license };
}

/**
 * Validates a key against Polar's public endpoint.
 *
 * Returns exactly one of:
 *   { valid: true,  license: { key, validatedAt, expiresAt, licenseId } }
 *   { valid: false, reason: "empty" | "invalid", detail }
 *   { valid: null,  reason: "offline", detail }   network or server failure
 *
 * `valid: null` is deliberately distinct from `false`: the caller must not
 * treat "Polar could not be reached" as "this key is bad" — a typo'd key and a
 * dead uplink deserve different messages, and only the second one may fall
 * back to a cached license.
 */
export async function validateLicenseKey(key, options = {}) {
  const normalized = typeof key === "string" ? key.trim() : "";
  if (!normalized) {
    return { valid: false, reason: "empty", detail: "no key supplied" };
  }

  const fetchImpl = options.fetchImpl || (typeof fetch === "function" ? fetch : null);
  if (!fetchImpl) {
    return { valid: null, reason: "offline", detail: "no fetch implementation" };
  }

  const endpoint = options.endpoint || POLAR_VALIDATE_ENDPOINT;
  const organizationId = options.organizationId || DEFAULT_ORGANIZATION_ID;
  const timeoutMs = options.timeoutMs || 8000;
  const now = typeof options.now === "number" ? options.now : Date.now();

  const controller = typeof AbortController === "function" ? new AbortController() : null;
  const timer = controller ? setTimeout(() => controller.abort(), timeoutMs) : null;

  let response;
  try {
    response = await fetchImpl(endpoint, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ key: normalized, organization_id: organizationId }),
      signal: controller ? controller.signal : undefined,
    });
  } catch (error) {
    return { valid: null, reason: "offline", detail: String((error && error.message) || error) };
  } finally {
    if (timer) clearTimeout(timer);
  }

  if (response.status === 404) {
    return {
      valid: false,
      reason: "invalid",
      detail: "unknown, revoked or expired key",
    };
  }
  if (!response || !response.ok) {
    return {
      valid: null,
      reason: "offline",
      detail: "validate endpoint answered HTTP " + (response ? response.status : "?"),
    };
  }

  let body = null;
  try {
    body = await response.json();
  } catch (error) {
    return { valid: null, reason: "offline", detail: "unreadable validate response" };
  }

  /* Polar can answer 200 with a non-granted status; believe the status field
     over the status code when both are present. */
  const status = body && typeof body.status === "string" ? body.status : null;
  if (status === "revoked" || status === "expired") {
    return { valid: false, reason: "invalid", detail: "key is " + status };
  }

  const expiresAt =
    body && typeof body.expires_at === "string" && body.expires_at
      ? Date.parse(body.expires_at)
      : body && typeof body.expires_at === "number"
        ? body.expires_at
        : null;

  return {
    valid: true,
    license: {
      key: normalized,
      validatedAt: now,
      expiresAt: expiresAt !== null && isFinite(expiresAt) ? expiresAt : null,
      licenseId: body && typeof body.id === "string" ? body.id : null,
    },
  };
}
