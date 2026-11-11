"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { createClauseLensServer } from "../src/server.js";
import { extractTextFromBuffer } from "../src/textio.js";

test("extractTextFromBuffer: extracts plain text directly from buffer", () => {
  const buf = Buffer.from("§ 1 Vertrag\nDie Haftung ist ausgeschlossen.", "utf-8");
  const extracted = extractTextFromBuffer(buf, "txt");
  assert.equal(extracted.format, "txt");
  assert.match(extracted.text, /Die Haftung ist ausgeschlossen/);
});

test("server API /extract: extracts text via POST /api/v1/extract", async () => {
  const server = createClauseLensServer();
  await new Promise((resolve) => server.listen(0, resolve));
  const port = server.address().port;
  const baseUrl = `http://localhost:${port}`;

  try {
    // 1. Plain text extraction
    const resText = await fetch(`${baseUrl}/api/v1/extract`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        text: "Klausel 1: Vertragslaufzeit 24 Monate.",
        filename: "vertrag.txt",
      }),
    });
    assert.equal(resText.status, 200);
    const dataText = await resText.json();
    assert.equal(dataText.format, "txt");
    assert.match(dataText.text, /Klausel 1/);
    assert.ok(dataText.charCount > 0);
    assert.ok(dataText.lineCount >= 1);

    // 2. Base64 encoded upload
    const sample = "Klausel 2: Der Gerichtsstand ist München.";
    const b64 = Buffer.from(sample, "utf-8").toString("base64");
    const resB64 = await fetch(`${baseUrl}/api/v1/extract`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        fileBase64: b64,
        filename: "test.txt",
      }),
    });
    assert.equal(resB64.status, 200);
    const dataB64 = await resB64.json();
    assert.equal(dataB64.text, sample);

    // 3. Error on missing body
    const resErr = await fetch(`${baseUrl}/api/v1/extract`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({}),
    });
    assert.equal(resErr.status, 400);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
});
