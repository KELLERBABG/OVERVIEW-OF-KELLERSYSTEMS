"use strict";

import { createHmac } from "node:crypto";
import { generateLicense, TIERS } from "./licensing.js";

/**
 * Verifies webhook signature for Stripe or Polar.
 */
export function verifyWebhookSignature(payloadRaw, signatureHeader, secret) {
  if (!secret) return true; // development mode
  if (!signatureHeader) return false;

  // Standard HMAC SHA256 verification
  const expected = createHmac("sha256", secret).update(payloadRaw).digest("hex");
  return signatureHeader === expected || signatureHeader.includes(expected);
}

/**
 * Handles incoming checkout/subscription events and provisions a signed license.
 */
export function handleCheckoutWebhook(eventData, options = {}) {
  const secret = options.licenseSecret;
  const event = typeof eventData === "string" ? JSON.parse(eventData) : eventData;

  // Extract customer and tier
  let customerName = "ClauseLens Customer";
  let tier = TIERS.PRO;
  let expiresAt = "2099-12-31";

  // Support Stripe checkout.session.completed
  if (event.type === "checkout.session.completed") {
    const session = event.data?.object || {};
    customerName = session.customer_details?.name || session.customer_email || customerName;
    const planName = (session.metadata?.plan || session.metadata?.tier || "pro").toLowerCase();
    if (planName.includes("enterprise")) tier = TIERS.ENTERPRISE;
    else if (planName.includes("team") || planName.includes("kanzlei")) tier = TIERS.TEAM;
    else tier = TIERS.PRO;
  }
  // Support Polar order.created
  else if (event.type === "order.created" || event.event === "order.created") {
    const data = event.data || {};
    customerName = data.customer?.name || data.customer?.email || customerName;
    const prodName = (data.product?.name || "").toLowerCase();
    if (prodName.includes("enterprise")) tier = TIERS.ENTERPRISE;
    else if (prodName.includes("team") || prodName.includes("kanzlei")) tier = TIERS.TEAM;
    else tier = TIERS.PRO;
  }
  // Direct provision test payload
  else if (event.customer || event.tier) {
    customerName = event.customer || customerName;
    tier = event.tier ? String(event.tier).toLowerCase() : TIERS.PRO;
  }

  // Calculate 1-year expiry by default
  const expiryDate = new Date();
  expiryDate.setFullYear(expiryDate.getFullYear() + 1);
  expiresAt = expiryDate.toISOString().slice(0, 10);

  const licenseKey = generateLicense({
    tier,
    licensee: customerName,
    expiresAt,
    secret,
  });

  return {
    success: true,
    tier,
    customer: customerName,
    expiresAt,
    licenseKey,
    welcomeMessage: `Willkommen bei ClauseLens ${tier.toUpperCase()}! Ihr Lizenzschlüssel wurde erfolgreich ausgestellt.`,
  };
}
