import { test } from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
  LEGAL_BASIS,
  buildFooter,
  buildMessage,
  checkRecipient,
  dotStuff,
  parseCsv,
  renderTemplate,
  splitRow,
  templateVariables,
} from "../tools/outreach/compose.mjs";
import { SmtpError, sendSmtp } from "../tools/outreach/smtp.mjs";
import { loadState, loadSuppressed, plan, writeState } from "../tools/outreach.mjs";

const repoRoot = fileURLToPath(new URL("..", import.meta.url));

const config = {
  fromName: "Lukas Negenborn",
  fromEmail: "info@example.de",
  company: "Keller Systems Software Development",
  address: "Stuttgarter Str. 106, 70736 Fellbach",
  site: "https://example.de",
  unsubscribe: "info@example.de",
};

function tempDir() {
  const dir = mkdtempSync(join(tmpdir(), "clauselens-outreach-"));
  return { dir, cleanup: () => rmSync(dir, { recursive: true, force: true }) };
}

function startSmtpMock({ rejectRcpt = false } = {}) {
  const received = { mailFrom: [], rcpt: [], data: "", auth: [] };
  const server = net.createServer((socket) => {
    let buffer = "";
    let inData = false;
    socket.write("220 mock ESMTP\r\n");
    socket.on("data", (chunk) => {
      buffer += chunk.toString("utf8");
      let index = buffer.indexOf("\r\n");
      while (index !== -1) {
        const line = buffer.slice(0, index);
        buffer = buffer.slice(index + 2);
        if (inData) {
          if (line === ".") {
            inData = false;
            socket.write("250 OK queued\r\n");
          } else {
            received.data += line + "\r\n";
          }
        } else if (/^EHLO/i.test(line)) {
          socket.write("250-mock\r\n250-AUTH PLAIN\r\n250 OK\r\n");
        } else if (/^AUTH PLAIN/i.test(line)) {
          received.auth.push(line);
          socket.write("235 authenticated\r\n");
        } else if (/^MAIL FROM:/i.test(line)) {
          received.mailFrom.push(line);
          socket.write("250 OK\r\n");
        } else if (/^RCPT TO:/i.test(line)) {
          received.rcpt.push(line);
          socket.write(rejectRcpt ? "550 no such user\r\n" : "250 OK\r\n");
        } else if (/^DATA$/i.test(line)) {
          inData = true;
          socket.write("354 go ahead\r\n");
        } else if (/^QUIT/i.test(line)) {
          socket.write("221 bye\r\n");
          socket.end();
        } else {
          socket.write("250 OK\r\n");
        }
        index = buffer.indexOf("\r\n");
      }
    });
    socket.on("error", () => {});
  });
  return new Promise((resolve) => {
    server.listen(0, "127.0.0.1", () => {
      const port = server.address().port;
      resolve({
        port,
        received,
        close: () => new Promise((done) => server.close(done)),
      });
    });
  });
}

test("parseCsv erkennt Semikolon, Quotes und BOM", () => {
  const text = "\uFEFFemail;name;anmerkung\r\nmax@example.de;\"Mustermann, Max\";\"sagt; hallo\"\r\n";
  const table = parseCsv(text);
  assert.deepEqual(table.headers, ["email", "name", "anmerkung"]);
  assert.equal(table.rows.length, 1);
  assert.equal(table.rows[0].name, "Mustermann, Max");
  assert.equal(table.rows[0].anmerkung, "sagt; hallo");
});

test("parseCsv faellt ohne Semikolon auf Komma zurueck", () => {
  const table = parseCsv("email,name\nmax@example.de,Max\n");
  assert.deepEqual(table.headers, ["email", "name"]);
  assert.equal(table.rows[0].email, "max@example.de");
});

test("splitRow behandelt doppelte Quotes", () => {
  assert.deepEqual(splitRow('a;"b""c";d', ";"), ["a", 'b"c', "d"]);
});

test("checkRecipient verlangt Rechtsgrundlage, Quelle und Datum", () => {
  const basis = { email: "Max@Example.de", rechtsgrundlage: "einwilligung", einwilligung_am: "2026-10-01", quelle: "Opt-In" };
  const ok = checkRecipient(basis);
  assert.equal(ok.ok, true);
  assert.equal(ok.email, "max@example.de");
  assert.equal(ok.basis, "einwilligung");

  assert.equal(checkRecipient({ ...basis, rechtsgrundlage: "" }).reason, "rechtsgrundlage fehlt (einwilligung oder bestandskunde)");
  assert.equal(checkRecipient({ ...basis, rechtsgrundlage: "kalt" }).ok, false);
  assert.equal(checkRecipient({ email: "noreply@example.de", rechtsgrundlage: "bestandskunde", quelle: "Shop" }).reason, "Rollenadresse");
  assert.equal(checkRecipient({ email: "keine-mail", rechtsgrundlage: "bestandskunde", quelle: "Shop" }).reason, "keine gueltige E-Mail-Adresse");
  assert.equal(checkRecipient({ email: "max@example.de", rechtsgrundlage: "bestandskunde", quelle: "" }).reason, "quelle fehlt");
  assert.equal(checkRecipient({ email: "max@example.de", rechtsgrundlage: "einwilligung", quelle: "Opt-In" }).reason, "einwilligung_am fehlt (JJJJ-MM-TT)");
  assert.equal(checkRecipient({ ...basis, einwilligung_am: "01.10.2026" }).ok, false);

  const bestandskunde = checkRecipient({ email: "max@example.de", rechtsgrundlage: "bestandskunde", quelle: "Bestellung 2026-09-15" });
  assert.equal(bestandskunde.ok, true);
  assert.equal(bestandskunde.basis, "bestandskunde");
  assert.ok(LEGAL_BASIS.bestandskunde.includes("7 Abs. 3"));
});

test("templateVariables leitet Vorname und Anrede ab", () => {
  const vars = templateVariables({ name: "Erika Beispiel", anrede: "" }, config);
  assert.equal(vars.vorname, "Erika");
  assert.equal(vars.nachname, "Beispiel");
  assert.equal(vars.anrede, "Guten Tag");
  assert.equal(vars.absender, "Lukas Negenborn");
  assert.equal(vars.website, "https://example.de");
  assert.equal(vars.abmelde_adresse, "info@example.de");
});

test("renderTemplate ersetzt Platzhalter und laesst Unbekanntes stehen", () => {
  const text = renderTemplate("Hallo {{vorname}}, von {{ firma }} {{unbekannt}}", { vorname: "Max", firma: "Muster GmbH" });
  assert.equal(text, "Hallo Max, von Muster GmbH {{unbekannt}}");
});

test("buildFooter nennt Absender, Abmeldung und Widerspruchsadresse", () => {
  const footer = buildFooter(config);
  assert.ok(footer.includes("Lukas Negenborn"));
  assert.ok(footer.includes("Keller Systems Software Development"));
  assert.ok(footer.includes("abmelden"));
  assert.ok(footer.includes("info@example.de"));
  assert.ok(!footer.includes("Website: \n"));
});

test("buildMessage erzeugt encoded-word, base64-Body und List-Unsubscribe", () => {
  const subject = "Kurze Frage zu Ihren AGB und Prüfungen";
  const text = "Guten Tag Max,\n\nzeile zwei.";
  const message = buildMessage({
    fromName: "Lukas Negenborn",
    fromAddress: "info@example.de",
    to: "max@example.de",
    subject,
    text,
    unsubscribe: "info@example.de",
  });
  assert.match(message, /^From: "Lukas Negenborn" <info@example.de>\r\n/);
  assert.match(message, /\r\nTo: max@example.de\r\n/);
  assert.match(message, /\r\nContent-Transfer-Encoding: base64\r\n/);
  const subjectLine = /\r\nSubject: =\?UTF-8\?B\?([^?]+)\?=\r\n/.exec(message);
  assert.ok(subjectLine, "Subject muss als encoded-word kodiert sein");
  assert.equal(Buffer.from(subjectLine[1], "base64").toString("utf8"), subject);
  assert.match(message, /\r\nMessage-ID: <[0-9a-f-]+@example\.de>\r\n/);
  assert.match(message, /\r\nList-Unsubscribe: <mailto:info@example\.de\?subject=abmelden>\r\n/);
  const body = message.split("\r\n\r\n")[1].replace(/\r\n/g, "");
  assert.equal(Buffer.from(body, "base64").toString("utf8"), "Guten Tag Max,\r\n\r\nzeile zwei.");
});

test("dotStuff verdoppelt fuehrende Punkte", () => {
  assert.equal(dotStuff("a\r\n.b\r\n..c"), "a\r\n..b\r\n...c");
});

test("plan ueberspringt Ungueltiges, Doppelte, Gesperrte und bereits Angeschriebene", () => {
  const rows = [
    { email: "max@example.de", name: "Max Mustermann", rechtsgrundlage: "einwilligung", einwilligung_am: "2026-10-01", quelle: "Opt-In", subject: "Hallo {{vorname}}" },
    { email: "max@example.de", name: "Doppelt", rechtsgrundlage: "bestandskunde", quelle: "Shop" },
    { email: "erika@example.de", name: "Erika Beispiel", rechtsgrundlage: "bestandskunde", quelle: "Bestellung" },
    { email: "gesperrt@example.de", name: "Gesperrt", rechtsgrundlage: "bestandskunde", quelle: "Shop" },
    { email: "alt@example.de", name: "Alt", rechtsgrundlage: "bestandskunde", quelle: "Shop" },
    { email: "kalt@example.de", name: "Kalt", rechtsgrundlage: "", quelle: "" },
  ];
  const result = plan(rows, {
    config,
    footer: buildFooter(config),
    template: "Hallo {{vorname}} von {{firma_absender}}",
    sent: new Set(["alt@example.de"]),
    suppressed: new Set(["gesperrt@example.de"]),
  });
  assert.deepEqual(result.ready.map((item) => item.email), ["max@example.de", "erika@example.de"]);
  assert.equal(result.ready[0].subject, "Hallo Max");
  assert.ok(result.ready[0].text.startsWith("Hallo Max von Keller Systems Software Development"));
  assert.ok(result.ready[0].text.includes("abmelden"));
  const reasons = result.skipped.map((item) => item.email + ":" + item.reason);
  assert.ok(reasons.includes("max@example.de:doppelt in der Liste"));
  assert.ok(reasons.includes("gesperrt@example.de:abgemeldet"));
  assert.ok(reasons.includes("alt@example.de:bereits angeschrieben"));
  assert.equal(result.skipped.length, 4);
});

test("writeState und loadState verwalten die Versandhistorie", () => {
  const { dir, cleanup } = tempDir();
  try {
    const statePath = join(dir, "state.jsonl");
    writeState(statePath, { email: "Max@Example.de", status: "sent", provider: "smtp" });
    writeState(statePath, { email: "fehler@example.de", status: "failed", error: "550" });
    assert.deepEqual([...loadState(statePath)], ["max@example.de"]);
    assert.equal(loadState(join(dir, "fehlt.jsonl")).size, 0);
  } finally {
    cleanup();
  }
});

test("loadSuppressed liest Adressen und ignoriert Leerzeilen", () => {
  const { dir, cleanup } = tempDir();
  try {
    const file = join(dir, "suppressed.txt");
    writeFileSync(file, "Alt@Example.de\n\n  noch@example.de  \n");
    const suppressed = loadSuppressed(file);
    assert.equal(suppressed.has("alt@example.de"), true);
    assert.equal(suppressed.has("noch@example.de"), true);
    assert.equal(loadSuppressed(join(dir, "fehlt.txt")).size, 0);
  } finally {
    cleanup();
  }
});

test("sendSmtp spricht den Dialog und uebergibt die Nachricht", async () => {
  const mock = await startSmtpMock();
  try {
    const raw = "From: info@example.de\r\nTo: max@example.de\r\nSubject: Test\r\n\r\nHallo.\r\n.versteckt";
    const result = await sendSmtp({
      host: "127.0.0.1",
      port: mock.port,
      from: "info@example.de",
      to: "max@example.de",
      raw,
      user: "benutzer",
      pass: "geheim",
      requireStartTls: false,
      timeoutMs: 5000,
    });
    assert.deepEqual(result, { ok: true });
    assert.deepEqual(mock.received.mailFrom, ["MAIL FROM:<info@example.de>"]);
    assert.deepEqual(mock.received.rcpt, ["RCPT TO:<max@example.de>"]);
    assert.equal(mock.received.auth.length, 1);
    const decoded = Buffer.from(mock.received.auth[0].split(" ")[2], "base64").toString("utf8");
    assert.equal(decoded, "\u0000benutzer\u0000geheim");
    assert.ok(mock.received.data.includes("Hallo.\r\n"));
    assert.ok(mock.received.data.includes("..versteckt"), "Punktierte Zeile muss gestufft sein");
  } finally {
    await mock.close();
  }
});

test("sendSmtp meldet abgelehnte Empfaenger", async () => {
  const mock = await startSmtpMock({ rejectRcpt: true });
  try {
    await assert.rejects(
      sendSmtp({
        host: "127.0.0.1",
        port: mock.port,
        from: "info@example.de",
        to: "max@example.de",
        raw: "Subject: Test\r\n\r\nHallo",
        requireStartTls: false,
        timeoutMs: 5000,
      }),
      (error) => error instanceof SmtpError && error.code === 550 && /550/.test(error.message),
    );
  } finally {
    await mock.close();
  }
});

test("sendSmtp bricht ohne STARTTLS ab, wenn es verlangt wird", async () => {
  const mock = await startSmtpMock();
  try {
    await assert.rejects(
      sendSmtp({
        host: "127.0.0.1",
        port: mock.port,
        from: "info@example.de",
        to: "max@example.de",
        raw: "Subject: Test\r\n\r\nHallo",
        timeoutMs: 5000,
      }),
      /STARTTLS/,
    );
  } finally {
    await mock.close();
  }
});

test("outreach send ist standardmaessig ein Dry-Run", () => {
  const { dir, cleanup } = tempDir();
  try {
    const statePath = join(dir, "state.jsonl");
    const result = spawnSync(
      process.execPath,
      [
        "tools/outreach.mjs",
        "send",
        "--list",
        "tools/outreach/example-contacts.csv",
        "--template",
        "tools/outreach/example-template.txt",
        "--state",
        statePath,
      ],
      { cwd: repoRoot, encoding: "utf8", env: { ...process.env, RESEND_API_KEY: "", SMTP_HOST: "" } },
    );
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /Bereit: 2 \(Dry-Run, nichts gesendet\)/);
    assert.match(result.stdout, /Zum Versand mit --send aufrufen\./);
    assert.equal(loadState(statePath).size, 0);
  } finally {
    cleanup();
  }
});

test("outreach preview zeigt Betreff und Text", () => {
  const result = spawnSync(
    process.execPath,
    ["tools/outreach.mjs", "preview", "--list", "tools/outreach/example-contacts.csv", "--template", "tools/outreach/example-template.txt"],
    { cwd: repoRoot, encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /=== max@example\.de \(/);
  assert.match(result.stdout, /Betreff: Kurze Frage zu Ihren AGB/);
  assert.match(result.stdout, /Bereit: 2/);
});

test("outreach verweigert Versand ohne konfigurierten Weg", () => {
  const { dir, cleanup } = tempDir();
  try {
    const result = spawnSync(
      process.execPath,
      [
        "tools/outreach.mjs",
        "send",
        "--list",
        "tools/outreach/example-contacts.csv",
        "--template",
        "tools/outreach/example-template.txt",
        "--state",
        join(dir, "state.jsonl"),
        "--send",
      ],
      { cwd: repoRoot, encoding: "utf8", env: { ...process.env, RESEND_API_KEY: "", SMTP_HOST: "" } },
    );
    assert.equal(result.status, 1);
    assert.match(result.stderr, /Kein Versandweg konfiguriert/);
  } finally {
    cleanup();
  }
});
