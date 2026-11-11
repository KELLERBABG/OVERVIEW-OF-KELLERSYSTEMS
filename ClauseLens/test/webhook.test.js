"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { createHmac } from "node:crypto";
import { handleCheckoutWebhook, verifyWebhookSignature } from "../src/webhook.js";
import { verifyLicense, TIERS } from "../src/licensing.js";

const LICENSE_SECRET = "webhook-test-secret";

test("webhook: processes Stripe checkout and provisions pro license", () => {
  const stripeEvent = {
    type: "checkout.session.completed",
    data: {
      object: {
        customer_details: { name: "Mustermann IT GmbH" },
        metadata: { plan: "pro" },
      },
    },
  };

  const res = handleCheckoutWebhook(stripeEvent, { licenseSecret: LICENSE_SECRET });
  assert.equal(res.success, true);
  assert.equal(res.tier, TIERS.PRO);
  assert.equal(res.customer, "Mustermann IT GmbH");
  assert.ok(res.licenseKey.startsWith("CL-PRO-"));

  const lic = verifyLicense(res.licenseKey, LICENSE_SECRET);
  assert.equal(lic.valid, true);
  assert.equal(lic.tier, TIERS.PRO);
});

test("webhook: processes Polar order and provisions enterprise license", () => {
  const polarEvent = {
    type: "order.created",
    data: {
      customer: { name: "Corporate AG" },
      product: { name: "ClauseLens Enterprise Annual" },
    },
  };

  const res = handleCheckoutWebhook(polarEvent, { licenseSecret: LICENSE_SECRET });
  assert.equal(res.success, true);
  assert.equal(res.tier, TIERS.ENTERPRISE);
  assert.equal(res.customer, "Corporate AG");
  assert.ok(res.licenseKey.startsWith("CL-ENTERPRISE-"));

  const lic = verifyLicense(res.licenseKey, LICENSE_SECRET);
  assert.equal(lic.valid, true);
  assert.equal(lic.tier, TIERS.ENTERPRISE);
});

test("webhook: processes Stripe checkout and provisions team license", () => {
  const stripeEvent = {
    type: "checkout.session.completed",
    data: {
      object: {
        customer_details: { name: "Anwaltskanzlei Meyer" },
        metadata: { plan: "team" },
      },
    },
  };

  const res = handleCheckoutWebhook(stripeEvent, { licenseSecret: LICENSE_SECRET });
  assert.equal(res.success, true);
  assert.equal(res.tier, TIERS.TEAM);
  assert.equal(res.customer, "Anwaltskanzlei Meyer");
  assert.ok(res.licenseKey.startsWith("CL-TEAM-"));

  const lic = verifyLicense(res.licenseKey, LICENSE_SECRET);
  assert.equal(lic.valid, true);
  assert.equal(lic.tier, TIERS.TEAM);
  assert.equal(lic.entitlements.seats, 5);
});

test("webhook: signature verification verifies HMAC correctly", () => {
  const payload = JSON.stringify({ ping: "pong" });
  const secret = "whsec_test_secret";
  const validSig = createHmac("sha256", secret).update(payload).digest("hex");

  assert.equal(verifyWebhookSignature(payload, validSig, secret), true);
  assert.equal(verifyWebhookSignature(payload, "invalid_sig", secret), false);
});
