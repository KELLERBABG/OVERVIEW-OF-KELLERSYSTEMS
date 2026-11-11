"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { resolve } from "node:path";

const BIN_PATH = resolve("bin", "clauselens.js");

function runMcpSession(messages) {
  return new Promise((resolvePromise, rejectPromise) => {
    const child = spawn(process.execPath, [BIN_PATH, "mcp"], {
      stdio: ["pipe", "pipe", "pipe"],
    });

    let stdout = "";
    let stderr = "";

    child.stdout.on("data", (chunk) => {
      stdout += chunk.toString("utf8");
    });

    child.stderr.on("data", (chunk) => {
      stderr += chunk.toString("utf8");
    });

    child.on("close", (code) => {
      const lines = stdout
        .split("\n")
        .map((l) => l.trim())
        .filter(Boolean);
      const responses = lines.map((l) => JSON.parse(l));
      resolvePromise({ responses, stderr, code });
    });

    child.on("error", rejectPromise);

    for (const msg of messages) {
      child.stdin.write(JSON.stringify(msg) + "\n");
    }
    child.stdin.end();
  });
}

test("mcp: initialize and tools/list", async () => {
  const initMsg = {
    jsonrpc: "2.0",
    id: 1,
    method: "initialize",
    params: {
      protocolVersion: "2024-11-05",
      capabilities: {},
      clientInfo: { name: "test-client", version: "1.0.0" },
    },
  };

  const listMsg = {
    jsonrpc: "2.0",
    id: 2,
    method: "tools/list",
    params: {},
  };

  const { responses } = await runMcpSession([initMsg, listMsg]);
  assert.equal(responses.length, 2);

  // Check initialize response
  assert.equal(responses[0].id, 1);
  assert.equal(responses[0].result.serverInfo.name, "clauselens-mcp");
  assert.equal(responses[0].result.protocolVersion, "2024-11-05");

  // Check tools/list response
  assert.equal(responses[1].id, 2);
  const toolNames = responses[1].result.tools.map((t) => t.name);
  assert.ok(toolNames.includes("audit_contract"));
  assert.ok(toolNames.includes("audit_file"));
  assert.ok(toolNames.includes("get_bgb_clause_knowledge"));
  assert.ok(toolNames.includes("compare_contracts"));
});

test("mcp: audit_contract detects illegal clauses and suggests redline", async () => {
  const trapText = `
§ 5 Haftung
Jegliche Haftung fuer leichte und grobe Fahrlaessigkeit sowie fuer Folgeschaeden wird vollstaendig ausgeschlossen.
Die Haftung ist in jedem Fall auf 1.000 EUR begrenzt.

§ 8 Laufzeit
Der Vertrag verlaengert sich nach Ablauf der Mindestlaufzeit jeweils stillschweigend um weitere 24 Monate.
  `.trim();

  const callMsg = {
    jsonrpc: "2.0",
    id: 42,
    method: "tools/call",
    params: {
      name: "audit_contract",
      arguments: {
        text: trapText,
        profile: "default",
      },
    },
  };

  const { responses } = await runMcpSession([callMsg]);
  assert.equal(responses.length, 1);
  assert.equal(responses[0].id, 42);
  assert.equal(responses[0].result.isError, false);

  const reportText = responses[0].result.content[0].text;
  assert.ok(reportText.includes("ClauseLens BGB-Risikoanalyse"));
  assert.ok(reportText.includes("HOCH") || reportText.includes("Haftung"));
  assert.ok(reportText.includes("Redline") || reportText.includes("Gegenvorschlag"));
});

test("mcp: get_bgb_clause_knowledge returns legal basis and pattern", async () => {
  const callMsg = {
    jsonrpc: "2.0",
    id: 99,
    method: "tools/call",
    params: {
      name: "get_bgb_clause_knowledge",
      arguments: {
        query: "vertragsstrafe",
      },
    },
  };

  const { responses } = await runMcpSession([callMsg]);
  assert.equal(responses.length, 1);
  assert.equal(responses[0].id, 99);
  assert.equal(responses[0].result.isError, false);

  const text = responses[0].result.content[0].text;
  assert.ok(text.includes("Vertragsstrafen"));
  assert.ok(text.includes("309"));
});

test("mcp: compare_contracts computes risk delta between drafts", async () => {
  const original = `
§ 1 Haftung
Die Haftung wird fuer alle Schaeden ausgeschlossen.
  `.trim();

  const revised = `
§ 1 Haftung
Der Anbieter haftet unbeschraenkt fuer Vorsatz und grobe Fahrlaessigkeit sowie bei Koerperschaeden.
  `.trim();

  const callMsg = {
    jsonrpc: "2.0",
    id: 101,
    method: "tools/call",
    params: {
      name: "compare_contracts",
      arguments: {
        originalText: original,
        revisedText: revised,
      },
    },
  };

  const { responses } = await runMcpSession([callMsg]);
  assert.equal(responses.length, 1);
  assert.equal(responses[0].id, 101);
  assert.equal(responses[0].result.isError, false);

  const text = responses[0].result.content[0].text;
  assert.ok(text.includes("Vertragsvergleich & Risikodelta"));
});
