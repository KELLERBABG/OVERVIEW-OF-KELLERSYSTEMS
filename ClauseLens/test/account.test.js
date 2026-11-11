"use strict";

import test from "node:test";
import assert from "node:assert/strict";
import { generateKeyPairSync } from "node:crypto";
import { createServer } from "node:http";
import {
  DEFAULT_LICENSE_SERVER,
  WORKERS_DEV_SERVER,
  getMachineFingerprint,
  verifySeatOnline,
  saveStoredAccount,
  loadStoredAccount,
  clearStoredAccount,
  requireActiveAccount,
  resolveActiveLicense,
} from "../src/account.js";
import { signLicenseToken } from "../src/licensing.js";

const { privateKey, publicKey } = generateKeyPairSync("rsa", { modulusLength: 2048 });
const PUBLIC_PEM = publicKey.export({ type: "spki", format: "pem" }).toString();

const MACHINE = "0123456789abcdef0123456789abcdef";
const KEY = "CL-PRO-test";

function token(claims = {}, options = {}) {
  return signLicenseToken(
    { tier: "pro", seatId: "seat_mock_999", machineId: MACHINE, email: "it@firma.de", ...claims },
    privateKey,
    options,
  );
}

async function withMockServer(handler, run) {
  const server = createServer(handler);
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  try {
    await run(`http://127.0.0.1:${server.address().port}`);
  } finally {
    await new Promise((resolve) => server.close(resolve));
  }
}

async function withStoredAccount(data, run) {
  const original = loadStoredAccount();
  try {
    if (data) saveStoredAccount(data);
    else clearStoredAccount();
    return await run();
  } finally {
    if (original) saveStoredAccount(original);
    else clearStoredAccount();
  }
}

test("account: getMachineFingerprint produces deterministic 32-character hex ID", () => {
  const fp1 = getMachineFingerprint();
  const fp2 = getMachineFingerprint();
  assert.equal(fp1, fp2);
  assert.equal(fp1.length, 32);
  assert.match(fp1, /^[0-9a-f]{32}$/);
});

test("account: verifySeatOnline rejects invalid email and missing key", async () => {
  const badEmail = await verifySeatOnline({ email: "invalid-email", licenseKey: KEY });
  assert.equal(badEmail.valid, false);
  assert.match(badEmail.error, /Gültige E-Mail/);

  const noKey = await verifySeatOnline({ email: "user@firma.de", licenseKey: "" });
  assert.equal(noKey.valid, false);
  assert.match(noKey.error, /Lizenzschlüssel/);
});

test("account: verifySeatOnline stores the seat token from the licence server", async () => {
  const signed = token();
  await withMockServer(
    (req, res) => {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(
        JSON.stringify({
          valid: true,
          seatId: "seat_mock_999",
          tier: "pro",
          expiresAt: "2027-01-01",
          token: signed,
        }),
      );
    },
    async (authServer) => {
      const result = await verifySeatOnline({ email: "it@firma.de", licenseKey: KEY, machineId: MACHINE, authServer });
      assert.equal(result.valid, true);
      assert.equal(result.seatId, "seat_mock_999");
      assert.equal(result.tier, "pro");
      assert.equal(result.token, signed);
      assert.equal(result.machineId, MACHINE);
    },
  );
});

test("account: a licence server answer without a token is not a session", async () => {
  await withMockServer(
    (req, res) => {
      res.writeHead(200, { "Content-Type": "application/json" });
      res.end(JSON.stringify({ valid: true, tier: "pro", seatId: "seat_1" }));
    },
    async (authServer) => {
      const result = await verifySeatOnline({ email: "it@firma.de", licenseKey: KEY, machineId: MACHINE, authServer });
      assert.equal(result.valid, false);
      assert.match(result.error, /keine Lizenzsitzung/);
    },
  );
});

test("account: unreachable licence server fails without a stored session", async () => {
  await withStoredAccount(null, async () => {
    const result = await verifySeatOnline({
      email: "offline@firma.de",
      licenseKey: KEY,
      machineId: MACHINE,
      authServer: "https://license.invalid",
      publicKey: PUBLIC_PEM,
    });
    assert.equal(result.valid, false);
    assert.match(result.error, /fehlgeschlagen/);
  });
});

test("account: unreachable licence server keeps a valid session", async () => {
  const stored = {
    email: "offline@firma.de",
    licenseKey: KEY,
    tier: "pro",
    seatId: "seat_mock_999",
    machineId: MACHINE,
    token: token(),
  };
  await withStoredAccount(stored, async () => {
    const result = await verifySeatOnline({
      email: "offline@firma.de",
      licenseKey: KEY,
      machineId: MACHINE,
      authServer: "https://license.invalid",
      publicKey: PUBLIC_PEM,
    });
    assert.equal(result.valid, true);
    assert.equal(result.offline, true);
    assert.equal(result.seatId, "seat_mock_999");
  });
});

test("account: exhausted server quota (429) keeps a valid session", async () => {
  const stored = {
    email: "quota@firma.de",
    licenseKey: KEY,
    tier: "pro",
    seatId: "seat_mock_999",
    machineId: MACHINE,
    token: token(),
  };
  await withStoredAccount(stored, async () => {
    await withMockServer(
      (req, res) => {
        res.writeHead(429, { "Content-Type": "application/json" });
        res.end(JSON.stringify({ error: "Error 1027" }));
      },
      async (authServer) => {
        const result = await verifySeatOnline({
          email: "quota@firma.de",
          licenseKey: KEY,
          machineId: MACHINE,
          authServer,
          publicKey: PUBLIC_PEM,
        });
        assert.equal(result.valid, true);
        assert.equal(result.offline, true);
      },
    );
  });
});

test("account: a 403 from the custom domain retries the workers.dev endpoint", async () => {
  const calls = [];
  const originalFetch = globalThis.fetch;
  globalThis.fetch = async (url) => {
    calls.push(String(url));
    if (String(url).startsWith(DEFAULT_LICENSE_SERVER)) {
      return new Response("<html>challenge</html>", { status: 403, headers: { "Content-Type": "text/html" } });
    }
    return new Response(JSON.stringify({ valid: true, seatId: "seat_workers_1", tier: "pro", token: "tok" }), {
      status: 200,
      headers: { "Content-Type": "application/json" },
    });
  };
  try {
    const result = await verifySeatOnline({
      email: "fallback@firma.de",
      licenseKey: KEY,
      machineId: MACHINE,
    });

    assert.equal(result.valid, true);
    assert.equal(result.seatId, "seat_workers_1");
    assert.equal(calls.length, 2);
    assert.ok(calls[0].startsWith(DEFAULT_LICENSE_SERVER));
    assert.ok(calls[1].startsWith(WORKERS_DEV_SERVER));
  } finally {
    globalThis.fetch = originalFetch;
  }
});

test("account: requireActiveAccount accepts a valid stored session", async () => {
  const machineId = getMachineFingerprint();
  await withStoredAccount(
    {
      email: "test.dev@clauselens.de",
      licenseKey: KEY,
      tier: "pro",
      seatId: "seat_dev_1",
      machineId,
      token: signLicenseToken({ tier: "pro", seatId: "seat_dev_1", machineId, email: "test.dev@clauselens.de" }, privateKey),
    },
    async () => {
      const active = requireActiveAccount({ publicKey: PUBLIC_PEM });
      assert.equal(active.email, "test.dev@clauselens.de");
    },
  );
});

test("account: requireActiveAccount rejects an expired session", async () => {
  const machineId = getMachineFingerprint();
  await withStoredAccount(
    {
      email: "alt@clauselens.de",
      licenseKey: KEY,
      seatId: "seat_old",
      machineId,
      token: signLicenseToken({ tier: "pro", seatId: "seat_old", machineId, email: "alt@clauselens.de" }, privateKey, {
        ttlSeconds: -60,
      }),
    },
    async () => {
      assert.throws(() => requireActiveAccount({ publicKey: PUBLIC_PEM }), /abgelaufen/);
    },
  );
});

test("account: requireActiveAccount rejects a session bound to another device", async () => {
  const machineId = getMachineFingerprint();
  await withStoredAccount(
    {
      email: "fremd@clauselens.de",
      licenseKey: KEY,
      seatId: "seat_other",
      machineId,
      token: signLicenseToken({ tier: "pro", seatId: "seat_other", machineId: "other-device", email: "fremd@clauselens.de" }, privateKey),
    },
    async () => {
      assert.throws(() => requireActiveAccount({ publicKey: PUBLIC_PEM }), /anderes Gerät/);
    },
  );
});

test("account: resolveActiveLicense reports a missing session", async () => {
  await withStoredAccount(null, async () => {
    const lic = resolveActiveLicense({ publicKey: PUBLIC_PEM });
    assert.equal(lic.valid, false);
    assert.match(lic.error, /anmelden/);
  });
});

test("account: storage lifecycle (save, load, clear)", async () => {
  await withStoredAccount(null, async () => {
    const testData = { email: "test.dev@clauselens.de", licenseKey: KEY, tier: "pro", seatId: "seat_dev_1" };
    saveStoredAccount(testData);
    assert.deepEqual(loadStoredAccount(), testData);
    clearStoredAccount();
    assert.equal(loadStoredAccount(), null);
  });
});
