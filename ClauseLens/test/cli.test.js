"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { execSync, spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { createInterface } from "node:readline";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

const here = path.dirname(fileURLToPath(import.meta.url));
const root = path.resolve(here, "..");
const node = JSON.stringify(process.execPath);

function exec(cmd, opts = {}) {
  return execSync(cmd, {
    cwd: root,
    encoding: "utf8",
    env: { ...process.env, CLAUSELENS_TEST_ACCOUNT: "1", ...opts.env },
    ...opts,
  });
}

test("CLI smoke test: analyze fixture, writes JSON + Markdown report, exit code 0", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "clauselens-test-"));
  try {
    const fixture = path.join(root, "test", "fixtures", "sample_agb.txt");
    const outJson = path.join(tmp, "out.json");
    const outMd = path.join(tmp, "out.md");
    const cmd = `${node} bin/clauselens.js analyze "${fixture}" --json "${outJson}" --report "${outMd}"`;
    const stdout = exec(cmd);

    assert.match(stdout, /Risiko-Score/);
    assert.match(stdout, /Befunde:/);
    assert.ok(existsSync(outJson), "JSON-Datei fehlt");
    assert.ok(existsSync(outMd), "Markdown-Report fehlt");

    const parsed = JSON.parse(readFileSync(outJson, "utf8"));
    assert.ok(Array.isArray(parsed.findings));
    assert.ok(parsed.findings.length >= 9);
    assert.equal(typeof parsed.riskScore, "number");
    assert.ok(parsed.riskScore >= 0 && parsed.riskScore <= 100);
    for (const f of parsed.findings) {
      assert.ok(f.type && f.severity && f.excerpt && Array.isArray(f.lineNumbers));
    }

    const md = readFileSync(outMd, "utf8");
    assert.match(md, /^# ClauseLens Risikobericht/m);
    assert.match(md, /keine Rechtsberatung/i);
    assert.match(md, /Zeilen:/);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

test("CLI fails with nonzero exit code for bad usage and missing files", () => {
  const run = (args) => {
    try {
      execSync(`${node} bin/clauselens.js ${args}`, {
        cwd: root,
        encoding: "utf8",
        stdio: "pipe",
        env: { ...process.env, CLAUSELENS_TEST_ACCOUNT: "1" },
      });
      return 0;
    } catch (err) {
      return err.status;
    }
  };
  assert.notEqual(run("frobnicate"), 0, "unbekannter Befehl muss fehlschlagen");
  assert.notEqual(run(""), 0, "fehlender Befehl muss fehlschlagen");
  assert.notEqual(
    run(`analyze "${path.join(root, "existiert-nicht.txt")}"`),
    0,
    "fehlende Datei muss fehlschlagen"
  );
});

test("CLI supports --version and -v flags", () => {
  const v1 = exec(`${node} bin/clauselens.js --version`);
  assert.equal(v1.trim(), "ClauseLens v1.0.0");

  const v2 = exec(`${node} bin/clauselens.js -v`);
  assert.equal(v2.trim(), "ClauseLens v1.0.0");
});

test("CLI exports SARIF report via --sarif flag", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "cl-sarifcli-"));
  try {
    const fixture = path.join(root, "test", "fixtures", "sample_agb.txt");
    const outSarif = path.join(tmp, "audit.sarif");
    const cmd = `${node} bin/clauselens.js analyze "${fixture}" --sarif "${outSarif}"`;
    exec(cmd);

    assert.ok(existsSync(outSarif));
    const content = JSON.parse(readFileSync(outSarif, "utf8"));
    assert.equal(content.version, "2.1.0");
    assert.ok(content.runs[0].results.length > 0);
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

test("CLI exports PDF report via --pdf flag", () => {
  const tmp = mkdtempSync(path.join(tmpdir(), "cl-pdfcli-"));
  try {
    const fixture = path.join(root, "test", "fixtures", "sample_agb.txt");
    const outPdf = path.join(tmp, "audit.pdf");
    const cmd = `${node} bin/clauselens.js analyze "${fixture}" --pdf "${outPdf}"`;
    exec(cmd);

    assert.ok(existsSync(outPdf));
    const buf = readFileSync(outPdf);
    assert.ok(buf.toString("utf8").startsWith("%PDF-1.4"));
  } finally {
    rmSync(tmp, { recursive: true, force: true });
  }
});

test("CLI blocks execution without active account when test guard is off", () => {
  const fixture = path.join(root, "test", "fixtures", "sample_agb.txt");
  const home = mkdtempSync(path.join(tmpdir(), "clauselens-nohome-"));
  try {
    execSync(`${node} bin/clauselens.js analyze "${fixture}"`, {
      cwd: root,
      encoding: "utf8",
      stdio: "pipe",
      env: { ...process.env, HOME: home, USERPROFILE: home, CLAUSELENS_TEST_ACCOUNT: "0" },
    });
    assert.fail("Should have thrown account required error");
  } catch (err) {
    assert.match(err.stderr, /Account erforderlich/);
    assert.equal(err.status, 1);
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});

test("CLI account commands: login, whoami, and logout lifecycle", async () => {
  const home = mkdtempSync(path.join(tmpdir(), "clauselens-account-"));
  // execSync blocks this process, so the mock licence server runs as its own process.
  const script = path.join(home, "mock-license.mjs");
  writeFileSync(
    script,
    [
      'import { createServer } from "node:http";',
      "const server = createServer((req, res) => {",
      '  res.writeHead(200, { "Content-Type": "application/json" });',
      '  res.end(JSON.stringify({ valid: true, seatId: "seat_cli_1", tier: "pro", token: "seat-token" }));',
      "});",
      'server.listen(0, "127.0.0.1", () => console.log(server.address().port));',
    ].join("\n"),
  );
  const mock = spawn(process.execPath, [script], { stdio: ["ignore", "pipe", "inherit"] });
  const port = await new Promise((resolve) => {
    createInterface({ input: mock.stdout }).once("line", (line) => resolve(Number(line.trim())));
  });
  const authServer = `http://127.0.0.1:${port}`;
  const env = {
    env: {
      ...process.env,
      HOME: home,
      USERPROFILE: home,
      CLAUSELENS_TEST_ACCOUNT: "0",
      CLAUSELENS_AUTH_SERVER: authServer,
    },
  };
  try {
    // 1. Activate a seat against the licence server
    const login = exec(`${node} bin/clauselens.js login --email test.user@firma.de --key CL-PRO-test`, env);
    assert.match(login, /Seat erfolgreich aktiviert/);

    // 2. Whoami should show active account
    const whoami = exec(`${node} bin/clauselens.js whoami`, env);
    assert.ok(whoami.includes("test.user@firma.de"));
    assert.ok(whoami.includes("PRO"));

    // 3. Logout should clear seat
    const logout = exec(`${node} bin/clauselens.js logout`, env);
    assert.match(logout, /abgemeldet/);
  } finally {
    mock.kill();
    rmSync(home, { recursive: true, force: true });
  }
});

test("CLI no longer mints licence keys locally", () => {
  const home = mkdtempSync(path.join(tmpdir(), "clauselens-mint-"));
  const env = { ...process.env, HOME: home, USERPROFILE: home, CLAUSELENS_TEST_ACCOUNT: "1" };
  const run = (args) => {
    try {
      execSync(`${node} bin/clauselens.js ${args}`, { cwd: root, encoding: "utf8", stdio: "pipe", env });
      return 0;
    } catch (err) {
      return err.status;
    }
  };
  try {
    assert.notEqual(run("license-gen --tier enterprise"), 0, "license-gen darf nicht mehr existieren");
    assert.notEqual(run("register --email test@firma.de"), 0, "register darf nicht mehr existieren");
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});
