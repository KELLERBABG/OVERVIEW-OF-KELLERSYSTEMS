"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { getOpenApiSpec, renderApiDocsHtml } from "../src/openapi.js";
import { createClauseLensServer } from "../src/server.js";

test("openapi: getOpenApiSpec returns valid OpenAPI 3.0.3 specification", () => {
  const spec = getOpenApiSpec();
  assert.equal(spec.openapi, "3.0.3");
  assert.equal(spec.info.title, "ClauseLens Contract Risk Audit API");
  assert.equal(spec.info.version, "1.0.0");
  assert.ok(spec.paths["/api/v1/health"]);
  assert.ok(spec.paths["/api/v1/rules"]);
  assert.ok(spec.paths["/api/v1/analyze"]);
  assert.ok(spec.paths["/api/v1/extract"]);
  assert.ok(spec.paths["/api/v1/license"]);
});

test("openapi: renderApiDocsHtml returns intact HTML", () => {
  const html = renderApiDocsHtml();
  assert.ok(html.includes("ClauseLens REST API"));
  assert.ok(html.includes("OpenAPI 3.0.3"));
  assert.ok(html.includes("<!doctype html>"));
});

test("server API /openapi: serves spec at /api/v1/openapi.json and docs at /docs", async () => {
  const server = createClauseLensServer();
  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;
  const baseUrl = `http://localhost:${port}`;

  try {
    // 1. GET /api/v1/openapi.json
    const resSpec = await fetch(`${baseUrl}/api/v1/openapi.json`);
    assert.equal(resSpec.status, 200);
    const spec = await resSpec.json();
    assert.equal(spec.openapi, "3.0.3");

    // 2. GET /docs
    const resDocs = await fetch(`${baseUrl}/docs`);
    assert.equal(resDocs.status, 200);
    assert.match(resDocs.headers.get("content-type"), /text\/html/);
    const docsHtml = await resDocs.text();
    assert.ok(docsHtml.includes("ClauseLens REST API"));

    // 3. POST /api/v1/analyze with format=sarif
    const resSarif = await fetch(`${baseUrl}/api/v1/analyze`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        text: "Die Haftung ist ausgeschlossen.",
        format: "sarif",
      }),
    });
    assert.equal(resSarif.status, 200);
    assert.match(resSarif.headers.get("content-type"), /application\/sarif\+json/);
    const sarifData = await resSarif.json();
    assert.equal(sarifData.version, "2.1.0");
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
