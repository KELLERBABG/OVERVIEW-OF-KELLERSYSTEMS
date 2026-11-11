#!/usr/bin/env node
import { existsSync, mkdirSync, readFileSync, appendFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  LEGAL_BASIS,
  buildFooter,
  buildMessage,
  checkRecipient,
  parseCsv,
  renderTemplate,
  templateVariables,
} from "./outreach/compose.mjs";
import { sendSmtp } from "./outreach/smtp.mjs";

const STATE_FILE = ".outreach/state.jsonl";
const SUPPRESS_FILE = ".outreach/suppressed.txt";

function loadEnvFile(path = ".env") {
  if (!existsSync(path)) return;
  readFileSync(path, "utf8")
    .split(/\r?\n/)
    .forEach((line) => {
      const match = /^([A-Z0-9_]+)=(.*)$/.exec(line.trim());
      if (!match) return;
      const key = match[1];
      if (process.env[key] !== undefined) return;
      process.env[key] = match[2].replace(/^["']|["']$/g, "");
    });
}

function config() {
  return {
    fromName: process.env.OUTREACH_FROM_NAME || "Lukas Negenborn",
    fromEmail: process.env.OUTREACH_FROM_EMAIL || "info@kellersystems.dev",
    company: process.env.OUTREACH_COMPANY || "Keller Systems Software Development",
    address: process.env.OUTREACH_ADDRESS || "c/o Block Services, Stuttgarter Str. 106, 70736 Fellbach",
    site: process.env.OUTREACH_SITE || "https://clause.kellersystems.dev",
    unsubscribe: process.env.OUTREACH_UNSUBSCRIBE || process.env.OUTREACH_FROM_EMAIL || "info@kellersystems.dev",
  };
}

export function parseArgs(argv) {
  const args = { command: argv[0] || "help", flags: {} };
  for (let i = 1; i < argv.length; i += 1) {
    const token = argv[i];
    if (!token.startsWith("--")) continue;
    const name = token.slice(2);
    const next = argv[i + 1];
    if (next === undefined || next.startsWith("--")) {
      args.flags[name] = true;
    } else {
      args.flags[name] = next;
      i += 1;
    }
  }
  return args;
}

export function loadSuppressed(path = SUPPRESS_FILE) {
  if (!existsSync(path)) return new Set();
  return new Set(
    readFileSync(path, "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim().toLowerCase())
      .filter(Boolean),
  );
}

export function loadState(path = STATE_FILE) {
  const sent = new Set();
  if (!existsSync(path)) return sent;
  readFileSync(path, "utf8")
    .split(/\r?\n/)
    .forEach((line) => {
      if (!line.trim()) return;
      try {
        const entry = JSON.parse(line);
        if (entry.email && entry.status === "sent") sent.add(String(entry.email).toLowerCase());
      } catch {
        // defekte Zeile ignorieren, der Rest der Historie bleibt nutzbar
      }
    });
  return sent;
}

export function writeState(path, entry) {
  mkdirSync(dirname(resolve(path)), { recursive: true });
  appendFileSync(path, JSON.stringify({ ...entry, at: new Date().toISOString() }) + "\n", "utf8");
}

export function plan(rows, { config: cfg, footer, template, sent, suppressed }) {
  const ready = [];
  const skipped = [];
  const seen = new Set();
  rows.forEach((row) => {
    const check = checkRecipient(row);
    if (!check.ok) {
      skipped.push({ email: row.email || "", reason: check.reason });
      return;
    }
    if (seen.has(check.email)) {
      skipped.push({ email: check.email, reason: "doppelt in der Liste" });
      return;
    }
    seen.add(check.email);
    if (suppressed.has(check.email)) {
      skipped.push({ email: check.email, reason: "abgemeldet" });
      return;
    }
    if (sent.has(check.email)) {
      skipped.push({ email: check.email, reason: "bereits angeschrieben" });
      return;
    }
    const variables = templateVariables(row, cfg);
    const text = renderTemplate(template, variables) + "\n" + footer;
    ready.push({
      email: check.email,
      basis: check.basis,
      subject: renderTemplate(row.subject || "Kurze Frage zu Ihren AGB", variables),
      text,
      variables,
    });
  });
  return { ready, skipped };
}

async function sendResend({ apiKey, fromName, fromEmail, to, subject, text, unsubscribe, fetchImpl = fetch }) {
  const response = await fetchImpl("https://api.resend.com/emails", {
    method: "POST",
    headers: { Authorization: "Bearer " + apiKey, "Content-Type": "application/json" },
    body: JSON.stringify({
      from: fromName + " <" + fromEmail + ">",
      to: [to],
      subject,
      text,
      headers: unsubscribe ? { "List-Unsubscribe": "<mailto:" + unsubscribe + "?subject=abmelden>" } : undefined,
    }),
  });
  const data = await response.json().catch(() => null);
  if (!response.ok) {
    return { ok: false, error: "Resend HTTP " + response.status + ": " + (data && data.message ? data.message : "") };
  }
  return { ok: true, id: data && data.id ? data.id : null };
}

function providerFromEnv(flags) {
  const choice = String(flags.provider || "auto").toLowerCase();
  if (choice === "resend" || (choice === "auto" && process.env.RESEND_API_KEY)) {
    if (!process.env.RESEND_API_KEY) throw new Error("RESEND_API_KEY fehlt");
    return { name: "resend" };
  }
  if (choice === "smtp" || (choice === "auto" && process.env.SMTP_HOST)) {
    if (!process.env.SMTP_HOST) throw new Error("SMTP_HOST fehlt");
    return { name: "smtp" };
  }
  throw new Error("Kein Versandweg konfiguriert: RESEND_API_KEY oder SMTP_HOST in .env setzen");
}

async function deliver(item, cfg, provider) {
  if (provider.name === "resend") {
    return sendResend({
      apiKey: process.env.RESEND_API_KEY,
      fromName: cfg.fromName,
      fromEmail: cfg.fromEmail,
      to: item.email,
      subject: item.subject,
      text: item.text,
      unsubscribe: cfg.unsubscribe,
    });
  }
  const raw = buildMessage({
    fromName: cfg.fromName,
    fromAddress: cfg.fromEmail,
    to: item.email,
    subject: item.subject,
    text: item.text,
    unsubscribe: cfg.unsubscribe,
  });
  try {
    await sendSmtp({
      host: process.env.SMTP_HOST,
      port: Number(process.env.SMTP_PORT || 587),
      secure: String(process.env.SMTP_SECURE || "false") === "true",
      user: process.env.SMTP_USER,
      pass: process.env.SMTP_PASS,
      from: cfg.fromEmail,
      to: item.email,
      raw,
      requireStartTls: String(process.env.SMTP_REQUIRE_STARTTLS || "true") !== "false",
    });
    return { ok: true };
  } catch (error) {
    return { ok: false, error: error.message };
  }
}

function readInputs(flags) {
  if (!flags.list) throw new Error("--list <kontakte.csv> fehlt");
  if (!flags.template) throw new Error("--template <vorlage.txt> fehlt");
  const csv = parseCsv(readFileSync(flags.list, "utf8"));
  const template = readFileSync(flags.template, "utf8");
  return { rows: csv.rows, template };
}

function printSummary(result, { dryRun }) {
  const { ready, skipped } = result;
  console.log("Bereit: " + ready.length + (dryRun ? " (Dry-Run, nichts gesendet)" : ""));
  if (skipped.length > 0) {
    console.log("Uebersprungen: " + skipped.length);
    skipped.slice(0, 20).forEach((item) => console.log("  - " + (item.email || "(ohne Adresse)") + ": " + item.reason));
    if (skipped.length > 20) console.log("  ... " + (skipped.length - 20) + " weitere");
  }
}

async function commandPreview(flags) {
  const cfg = config();
  const { rows, template } = readInputs(flags);
  const result = plan(rows, {
    config: cfg,
    footer: buildFooter(cfg),
    template,
    sent: new Set(),
    suppressed: loadSuppressed(),
  });
  const limit = Number(flags.limit || 3);
  result.ready.slice(0, limit).forEach((item) => {
    console.log("=== " + item.email + " (" + LEGAL_BASIS[item.basis] + ") ===");
    console.log("Betreff: " + item.subject);
    console.log(item.text);
    console.log();
  });
  printSummary(result, { dryRun: true });
}

async function commandSend(flags) {
  const cfg = config();
  const { rows, template } = readInputs(flags);
  const statePath = flags.state || STATE_FILE;
  const result = plan(rows, {
    config: cfg,
    footer: buildFooter(cfg),
    template,
    sent: loadState(statePath),
    suppressed: loadSuppressed(),
  });
  const limit = Math.max(1, Number(flags.limit || 50));
  const delayMs = Math.max(0, Number(flags.delay ?? 20)) * 1000;
  const queue = result.ready.slice(0, limit);
  const dryRun = flags.send !== true;

  printSummary({ ready: queue, skipped: result.skipped }, { dryRun });
  if (dryRun) {
    console.log("Zum Versand mit --send aufrufen.");
    return 0;
  }
  if (queue.length === 0) return 0;

  const provider = providerFromEnv(flags);
  console.log("Versand ueber " + provider.name + ".");

  let sent = 0;
  let failed = 0;
  for (let i = 0; i < queue.length; i += 1) {
    const item = queue[i];
    const outcome = await deliver(item, cfg, provider);
    if (outcome.ok) {
      sent += 1;
      writeState(statePath, { email: item.email, status: "sent", provider: provider.name });
      console.log("gesendet: " + item.email);
    } else {
      failed += 1;
      writeState(statePath, { email: item.email, status: "failed", error: outcome.error });
      console.log("fehlgeschlagen: " + item.email + " (" + outcome.error + ")");
    }
    if (i < queue.length - 1 && delayMs > 0) await new Promise((r) => setTimeout(r, delayMs));
  }
  console.log("Fertig: " + sent + " gesendet, " + failed + " fehlgeschlagen.");
  return failed > 0 ? 1 : 0;
}

function commandSuppress(flags) {
  const entries = [];
  if (flags.email) entries.push(String(flags.email));
  if (flags.file) {
    readFileSync(flags.file, "utf8")
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean)
      .forEach((line) => entries.push(line));
  }
  if (entries.length === 0) throw new Error("--email <adresse> oder --file <liste> fehlt");
  mkdirSync(dirname(resolve(SUPPRESS_FILE)), { recursive: true });
  appendFileSync(SUPPRESS_FILE, entries.join("\n") + "\n", "utf8");
  console.log(entries.length + " Adresse(n) gesperrt.");
  return 0;
}

function usage() {
  console.log("clauselens outreach");
  console.log("  preview --list <kontakte.csv> --template <vorlage.txt> [--limit 3]");
  console.log("  send    --list <kontakte.csv> --template <vorlage.txt> [--limit 50] [--delay 20] [--send]");
  console.log("  suppress --email <adresse> | --file <liste>");
  console.log("Empfaenger brauchen eine Rechtsgrundlage (einwilligung oder bestandskunde) und eine Quelle.");
  console.log("Versand: RESEND_API_KEY oder SMTP_HOST/PORT/USER/PASS in .env setzen.");
}

export async function main(argv) {
  loadEnvFile();
  const { command, flags } = parseArgs(argv);
  try {
    if (command === "preview") return await commandPreview(flags);
    if (command === "send") return await commandSend(flags);
    if (command === "suppress") return commandSuppress(flags);
    usage();
    return command === "help" ? 0 : 1;
  } catch (error) {
    console.error("Fehler: " + error.message);
    return 1;
  }
}

if (import.meta.url === pathToFileURL(process.argv[1] || "").href) {
  main(process.argv.slice(2)).then((code) => {
    process.exitCode = code;
  });
}
