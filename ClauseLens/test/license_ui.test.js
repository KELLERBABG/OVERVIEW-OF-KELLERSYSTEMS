"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const pages = ["index.html", "studio.html"];

test("license ui: both pages verify a signed session through the licence module", () => {
  for (const page of pages) {
    const html = readFileSync(page, "utf-8");
    assert.match(html, /import \* as PolarLicensing from "\.\/src\/polar\.js";/, page);
    assert.match(html, /window\.PolarLicensing = PolarLicensing;/, page);
    assert.match(html, /activateLicenseSeat\(/, page);
    assert.match(html, /verifyLicenseToken\(/, page);
    assert.match(html, /async function verifyStoredSession\(\)/, page);
    assert.match(html, /await verifyStoredSession\(\);/, page);
    assert.match(html, /clauselens_license_record/, page);
  }
});

test("license ui: a stored record alone never unlocks a page", () => {
  for (const page of pages) {
    const html = readFileSync(page, "utf-8");
    assert.doesNotMatch(html, /isOfflineError/, page);
    assert.doesNotMatch(html, /validateLicenseKey/, page);
    assert.doesNotMatch(html, /tierFromLicenseKey/, page);
    assert.doesNotMatch(html, /startsWith\(["']CL-/, page);
    assert.match(html, /lett? licenceSession = \{ valid: false/, page);
  }
});

test("license ui: hasLicense()/hasValidLicense() only trust the verified session", () => {
  const index = readFileSync("index.html", "utf-8");
  const studio = readFileSync("studio.html", "utf-8");
  assert.match(index, /function hasLicense\(\) \{\s*return licenceSession\.valid === true;/, "index");
  assert.match(studio, /function hasValidLicense\(\) \{\s*return licenceSession\.valid === true;/, "studio");
});