"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { createClauseLensServer } from "../src/server.js";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");

test("server: serves intact index.html at root and API endpoints", async () => {
  const server = createClauseLensServer({ rootDir: root });

  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;
  const baseUrl = `http://localhost:${port}`;

  try {
    // 1. Verify GET / serves untouched index.html
    const rootRes = await fetch(`${baseUrl}/`);
    assert.equal(rootRes.status, 200);
    assert.match(rootRes.headers.get("content-type"), /text\/html/);
    const rootBody = await rootRes.text();
    const diskIndexHtml = readFileSync(path.join(root, "index.html"), "utf-8");
    assert.equal(rootBody, diskIndexHtml, "GET / must serve the EXACT intact index.html from disk");

    // 2. Verify GET /api/v1/health
    const healthRes = await fetch(`${baseUrl}/api/v1/health`);
    assert.equal(healthRes.status, 200);
    const healthData = await healthRes.json();
    assert.equal(healthData.status, "ok");
    assert.equal(healthData.engine, "ClauseLens");
    assert.equal(healthData.version, "1.0.0");

    // 3. Verify GET /api/v1/rules
    const rulesRes = await fetch(`${baseUrl}/api/v1/rules`);
    assert.equal(rulesRes.status, 200);
    const rulesData = await rulesRes.json();
    assert.equal(rulesData.count, 47);
    assert.ok(rulesData.profiles.includes("b2b_saas"));

    // 4. Verify POST /api/v1/analyze
    const analyzeRes = await fetch(`${baseUrl}/api/v1/analyze`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        text: "Die Haftung des Anbieters ist für sämtliche Schäden ausgeschlossen.",
      }),
    });
    assert.equal(analyzeRes.status, 200);
    const analyzeData = await analyzeRes.json();
    assert.ok(analyzeData.riskScore > 0);
    assert.equal(analyzeData.findings.length, 1);
    assert.equal(analyzeData.findings[0].type, "liability");

    // 5. Verify GET /app (Studio) & GET /pricing
    const appRes = await fetch(`${baseUrl}/app`);
    assert.equal(appRes.status, 200);
    const appBody = await appRes.text();
    assert.ok(appBody.includes("ClauseLens Studio"));

    const pricingRes = await fetch(`${baseUrl}/pricing`);
    assert.equal(pricingRes.status, 200);
    const pricingBody = await pricingRes.text();
    assert.ok(pricingBody.includes("Preise") && pricingBody.includes("Professional"));

    const impRes = await fetch(`${baseUrl}/impressum`);
    assert.equal(impRes.status, 200);
    const impBody = await impRes.text();
    assert.ok(impBody.includes("Lukas Negenborn"));

    const privRes = await fetch(`${baseUrl}/privacy`);
    assert.equal(privRes.status, 200);
    const privBody = await privRes.text();
    assert.ok(privBody.includes("Polar.sh"));

    const termsRes = await fetch(`${baseUrl}/terms`);
    assert.equal(termsRes.status, 200);
    const termsBody = await termsRes.text();
    assert.ok(termsBody.includes("Polar.sh"));

    // 6. Verify GET /api/v1/account/status
    const accRes = await fetch(`${baseUrl}/api/v1/account/status`);
    assert.equal(accRes.status, 200);
    const accData = await accRes.json();
    assert.ok(typeof accData.authenticated === "boolean");
    assert.ok(accData.machineFingerprint.length === 32);

    // 7. Verify POST /api/v1/account/logout
    const logoutRes = await fetch(`${baseUrl}/api/v1/account/logout`, { method: "POST" });
    assert.equal(logoutRes.status, 200);
    const logoutData = await logoutRes.json();
    assert.equal(logoutData.success, true);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
