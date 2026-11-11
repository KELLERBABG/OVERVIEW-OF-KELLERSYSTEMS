import { randomUUID } from "node:crypto";

// Empfaenger ohne dokumentierte Rechtsgrundlage werden nicht angeschrieben (UWG § 7).
export const LEGAL_BASIS = {
  einwilligung: "Dokumentierte Einwilligung des Empfaengers",
  bestandskunde: "Bestehende Kundenbeziehung nach § 7 Abs. 3 UWG",
};

export function splitRow(line, delimiter) {
  const cells = [];
  let current = "";
  let quoted = false;
  for (let i = 0; i < line.length; i += 1) {
    const char = line[i];
    if (quoted) {
      if (char === '"') {
        if (line[i + 1] === '"') {
          current += '"';
          i += 1;
        } else {
          quoted = false;
        }
      } else {
        current += char;
      }
    } else if (char === '"') {
      quoted = true;
    } else if (char === delimiter) {
      cells.push(current);
      current = "";
    } else {
      current += char;
    }
  }
  cells.push(current);
  return cells;
}

export function parseCsv(text) {
  const clean = String(text).replace(/^\uFEFF/, "");
  const lines = clean.split(/\r?\n/).filter((line) => line.trim() !== "");
  if (lines.length === 0) return { headers: [], rows: [] };
  const delimiter = lines[0].includes(";") ? ";" : ",";
  const table = lines.map((line) => splitRow(line, delimiter));
  const headers = table[0].map((header) => header.trim().toLowerCase());
  const rows = table.slice(1).map((cells) => {
    const row = {};
    headers.forEach((header, index) => {
      row[header] = (cells[index] || "").trim();
    });
    return row;
  });
  return { headers, rows };
}

const EMAIL_RE = /^[^@\s]+@[^@\s]+\.[^@\s]+$/;
const ROLE_RE = /^(noreply|no-reply|postmaster|abuse|mailer-daemon)@/i;

export function checkRecipient(row) {
  const email = String(row.email || "").trim().toLowerCase();
  if (!EMAIL_RE.test(email)) return { ok: false, reason: "keine gueltige E-Mail-Adresse" };
  if (ROLE_RE.test(email)) return { ok: false, reason: "Rollenadresse" };
  const basis = String(row.rechtsgrundlage || "").trim().toLowerCase();
  if (!Object.hasOwn(LEGAL_BASIS, basis)) {
    return { ok: false, reason: "rechtsgrundlage fehlt (einwilligung oder bestandskunde)" };
  }
  if (!String(row.quelle || "").trim()) return { ok: false, reason: "quelle fehlt" };
  if (basis === "einwilligung" && !/^\d{4}-\d{2}-\d{2}$/.test(String(row.einwilligung_am || "").trim())) {
    return { ok: false, reason: "einwilligung_am fehlt (JJJJ-MM-TT)" };
  }
  return { ok: true, email, basis };
}

export function templateVariables(row, config = {}) {
  const name = String(row.name || "").trim();
  const parts = name.split(/\s+/).filter(Boolean);
  return {
    ...row,
    name,
    vorname: parts[0] || "",
    nachname: parts.slice(1).join(" "),
    anrede: String(row.anrede || "").trim() || "Guten Tag",
    absender: config.fromName || "",
    firma_absender: config.company || "",
    abmelde_adresse: config.unsubscribe || "",
    website: config.site || "",
  };
}

export function renderTemplate(template, variables) {
  return String(template).replace(/\{\{\s*([a-z0-9_]+)\s*\}\}/gi, (match, key) => {
    const normalized = String(key).toLowerCase();
    if (!Object.hasOwn(variables, normalized)) return match;
    const value = variables[normalized];
    return value === undefined || value === null ? "" : String(value);
  });
}

export function buildFooter(config) {
  const lines = ["", "-- ", config.fromName || "", config.company || "", config.address || ""];
  lines.push("Website: " + (config.site || ""));
  if (config.unsubscribe) {
    lines.push("Abmeldung: Antwort mit \"abmelden\" oder eine Mail an " + config.unsubscribe);
    lines.push("Anschrift fuer den Widerspruch: " + (config.address || ""));
  }
  return lines.filter((line) => line !== "").join("\n");
}

function encodeHeader(value) {
  const text = String(value ?? "");
  if (!/[^\x20-\x7E]/.test(text)) return text;
  return "=?UTF-8?B?" + Buffer.from(text, "utf8").toString("base64") + "?=";
}

function formatAddress(name, address) {
  const label = String(name || "").trim();
  if (!label) return address;
  const encoded = /[^\x20-\x7E]/.test(label) ? encodeHeader(label) : '"' + label.replace(/"/g, '\\"') + '"';
  return encoded + " <" + address + ">";
}

export function buildMessage({ fromName, fromAddress, to, subject, text, unsubscribe }) {
  const domain = String(fromAddress).split("@")[1] || "localhost";
  const headers = [
    "From: " + formatAddress(fromName, fromAddress),
    "To: " + to,
    "Subject: " + encodeHeader(subject),
    "Date: " + new Date().toUTCString(),
    "Message-ID: <" + randomUUID() + "@" + domain + ">",
    "MIME-Version: 1.0",
    "Content-Type: text/plain; charset=utf-8",
    "Content-Transfer-Encoding: base64",
  ];
  if (unsubscribe) {
    headers.push("List-Unsubscribe: <mailto:" + unsubscribe + "?subject=abmelden>");
  }
  const body = Buffer.from(String(text).replace(/\r?\n/g, "\r\n"), "utf8")
    .toString("base64")
    .replace(/(.{1,76})/g, "$1\r\n");
  return headers.join("\r\n") + "\r\n\r\n" + body;
}

export function dotStuff(message) {
  return String(message)
    .split("\r\n")
    .map((line) => (line.startsWith(".") ? "." + line : line))
    .join("\r\n");
}
