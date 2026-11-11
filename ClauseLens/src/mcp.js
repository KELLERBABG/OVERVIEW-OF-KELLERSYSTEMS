"use strict";

/**
 * src/mcp.js
 * ClauseLens MCP (Model Context Protocol) Server.
 * Ermöglicht LLMs & Agenten (Claude Desktop, Cursor, Zed, Antigravity)
 * die direkte, lokale und 100% DSGVO-sichere Auditierung deutscher Verträge nach BGB.
 * Zero external dependencies — pure Node.js stdio JSON-RPC 2.0.
 */

import readline from "node:readline";
import { resolve, isAbsolute } from "node:path";
import { existsSync } from "node:fs";
import { analyzeContract } from "./analyze.js";
import { extractText } from "./textio.js";

const SERVER_NAME = "clauselens-mcp";
const SERVER_VERSION = "1.0.0";
const PROTOCOL_VERSION = "2024-11-05";

// Kompakte BGB-Wissensbasis für Schnellauskünfte im MCP
const BGB_KNOWLEDGE = [
  {
    type: "auto_renewal",
    title: "Automatische Vertragsverlängerung",
    basis: "§ 309 Nr. 9 lit. b BGB, § 307 Abs. 1 BGB",
    b2bScope: "Im B2B sind Verlängerungen um > 1 Jahr nach BGH (§ 307 BGB) regelmäßig unwirksam. Im B2C nur noch unbestimmte Zeit mit 1 Monat Kündigungsfrist.",
    trap: "Der Vertrag verlängert sich automatisch um jeweils 24 Monate, wenn er nicht 6 Monate vor Ablauf gekündigt wird.",
    redline: "Der Vertrag verlängert sich nach Ablauf der Mindestlaufzeit auf unbestimmte Zeit und kann jederzeit mit einer Frist von einem Monat gekündigt werden."
  },
  {
    type: "liability",
    title: "Haftungsausschluss & Kardinalpflichten",
    basis: "§ 309 Nr. 7 lit. a, b BGB, § 307 Abs. 2 Nr. 2 BGB",
    b2bScope: "Haftung für Körperschäden, Vorsatz und grobe Fahrlässigkeit darf NIE ausgeschlossen werden. Bei Kardinalpflichten haftet der Verwender zwingend auch für leichte Fahrlässigkeit (Deckelung auf vertragstypischen Schaden zulässig).",
    trap: "Jegliche Haftung für leichte und grobe Fahrlässigkeit sowie für Mangelfolgeschäden ist ausgeschlossen.",
    redline: "Der Anbieter haftet unbeschränkt für Schäden aus der Verletzung des Lebens, des Körpers oder der Gesundheit sowie für Vorsatz und grobe Fahrlässigkeit. Bei leichter Fahrlässigkeit haftet er nur bei Verletzung wesentlicher Vertragspflichten (Kardinalpflichten), begrenzt auf den vorhersehbaren, vertragstypischen Schaden."
  },
  {
    type: "penalty",
    title: "Vertragsstrafen & Schadenspauschalen",
    basis: "§ 309 Nr. 5, Nr. 6 BGB, § 307 Abs. 1 BGB",
    b2bScope: "Im B2C formularmäßig absolut unzulässig. Im B2B nur wirksam bei klar begrenzten Tatbeständen, Verschuldenserfordernis und angemessener Obergrenze (z. B. 5 % der Auftragssumme).",
    trap: "Für jeden Verstoß zahlt der Kunde eine verschuldensunabhängige Vertragsstrafe in Höhe von 15.000 EUR ohne Nachweis eines Schadens.",
    redline: "Verletzt der Auftragnehmer diese Pflicht schuldhaft, kann der Auftraggeber eine angemessene Vertragsstrafe verlangen, maximal jedoch 5 % des Nettoauftragswerts. Dem Auftragnehmer bleibt der Nachweis eines geringeren Schadens vorbehalten."
  },
  {
    type: "written_form_trap",
    title: "Schriftformklausel (Kündigung nur per Brief/Einschreiben)",
    basis: "§ 309 Nr. 13 BGB, § 305b BGB",
    b2bScope: "Seit 2016 sind Klauseln unwirksam, die für Anzeigen/Kündigungen eine strengere Form als die Textform (§ 126b BGB, z.B. E-Mail) verlangen.",
    trap: "Kündigungen bedürfen zu ihrer Wirksamkeit der Schriftform per Einschreiben. Eine Kündigung per E-Mail ist ausgeschlossen.",
    redline: "Kündigungen und wesentliche Erklärungen bedürfen der Textform (z. B. per E-Mail)."
  },
  {
    type: "setoff_restriction",
    title: "Aufrechnungs- und Zurückbehaltungsverbot",
    basis: "§ 309 Nr. 3 BGB, § 307 Abs. 1 BGB",
    b2bScope: "Aufrechnungsverbot ist unwirksam, wenn unbestrittene, entscheidungsreife oder rechtskräftig festgestellte Gegenforderungen nicht ausgenommen werden.",
    trap: "Die Aufrechnung mit Gegenansprüchen des Kunden ist vollständig ausgeschlossen.",
    redline: "Eine Aufrechnung ist nur mit unbestrittenen, rechtskräftig festgestellten oder entscheidungsreifen Gegenforderungen zulässig."
  },
  {
    type: "deemed_acceptance",
    title: "Abnahmefiktion & Rügeverkürzung",
    basis: "§ 308 Nr. 5 BGB, § 640 Abs. 2 BGB",
    b2bScope: "Eine Abnahmefiktion verlangt zwingend, dass der Verwender den Kunden bei Fristbeginn gesondert und ausdrücklich auf die Fiktionsfolge hinweist.",
    trap: "Das Werk gilt als vorbehaltlos abgenommen, wenn der Kunde nicht innerhalb von 5 Tagen schriftlich Mängel anzeigt.",
    redline: "Das Werk gilt als abgenommen, wenn der Auftragnehmer dem Auftraggeber nach Fertigstellung eine angemessene Frist zur Abnahme von mindestens 14 Werktagen gesetzt hat und der Auftraggeber die Abnahme nicht innerhalb dieser Frist unter Angabe mindestens eines Mangels verweigert hat, sofern der Auftragnehmer bei Fristbeginn auf diese Rechtsfolge gesondert hingewiesen hat."
  },
  {
    type: "salvatorische_klausel_falle",
    title: "Geltungserhaltende Reduktion in Salvatorischer Klausel",
    basis: "§ 306 Abs. 2 BGB (BGH-Standard)",
    b2bScope: "Klauseln, die anordnen, dass eine unwirksame AGB-Bestimmung durch die maximal zulässige Regelung ersetzt wird, verstoßen gegen das Verbot der geltungserhaltenden Reduktion und sind selbst unwirksam.",
    trap: "Sollte eine Bestimmung unwirksam sein, so gilt an ihrer Stelle diejenige wirksame Bestimmung als vereinbart, die dem wirtschaftlichen Zweck am nächsten kommt.",
    redline: "Sollten einzelne Bestimmungen dieses Vertrags unwirksam sein, bleibt die Wirksamkeit der übrigen Bestimmungen unberührt. Anstelle der unwirksamen Bestimmung treten die gesetzlichen Vorschriften (§ 306 Abs. 2 BGB)."
  }
];

const TOOLS_MANIFEST = [
  {
    name: "audit_contract",
    description: "Auditiert einen deutschen Vertragstext oder AGB-Abschnitt nach §§ 305–310 BGB. Erkennt 47 Klauselfallen (Haftungsausschluss, Vertragsstrafen, automatische Verlängerung, Kündigungsfristen, Abwerbeverbote, etc.), berechnet einen Risikoscore (0–100) und liefert sofort BGB-konforme Redlines.",
    inputSchema: {
      type: "object",
      properties: {
        text: {
          type: "string",
          description: "Der deutsche Vertragstext, AGB-Abschnitt oder Klauseltext, der geprüft werden soll."
        },
        profile: {
          type: "string",
          enum: ["default", "b2b_saas", "freelance_consulting", "procurement_strict"],
          description: "Prüfprofil: 'default' (ausgewogen), 'b2b_saas' (Cloud/Software), 'freelance_consulting' (Dienstleistung), 'procurement_strict' (strenger Einkauf)."
        }
      },
      required: ["text"]
    }
  },
  {
    name: "audit_file",
    description: "Liest eine lokale Vertragsdatei (.pdf, .docx, .txt) 100 % offline ein und auditiert sie nach deutschem AGB-Recht. Es werden keinerlei Daten ins Internet übertragen.",
    inputSchema: {
      type: "object",
      properties: {
        filePath: {
          type: "string",
          description: "Absoluter oder relativer Dateipfad zum Vertrag (.pdf, .docx oder .txt)."
        },
        profile: {
          type: "string",
          enum: ["default", "b2b_saas", "freelance_consulting", "procurement_strict"],
          description: "Optionales Prüfprofil."
        }
      },
      required: ["filePath"]
    }
  },
  {
    name: "get_bgb_clause_knowledge",
    description: "Liefert juristisches Fachwissen zu einem bestimmten Klauseltyp (Haftung, Vertragsstrafe, Kündigungsfrist, Aufrechnung, Abwerbeverbot, Abnahmefiktion etc.) inkl. BGB-Norm, B2B-Geltungsbereich, Fallstricken und rechtssicherer Muster-Redline.",
    inputSchema: {
      type: "object",
      properties: {
        query: {
          type: "string",
          description: "Suchbegriff oder Klauseltyp (z. B. 'vertragsstrafe', 'haftung', 'kuendigung', 'salvatorisch', 'schriftform')."
        }
      },
      required: ["query"]
    }
  },
  {
    name: "compare_contracts",
    description: "Vergleicht zwei Versionen eines Vertragstextes (z. B. Erstentwurf vs. Gegenvorschlag) und ermittelt das Risikodelta: Welche Klauselfallen wurden entschärft und welche Risiken sind neu entstanden?",
    inputSchema: {
      type: "object",
      properties: {
        originalText: {
          type: "string",
          description: "Der ursprüngliche Vertragstext."
        },
        revisedText: {
          type: "string",
          description: "Der überarbeitete oder nachverhandelte Vertragstext."
        }
      },
      required: ["originalText", "revisedText"]
    }
  }
];

function formatAnalysisMarkdown(res, titleLabel = "Vertragsprüfung") {
  const lines = [];
  lines.push(`## ClauseLens BGB-Risikoanalyse: ${titleLabel}`);
  lines.push(`- **Risiko-Score:** ${res.riskScore}/100 (**${res.riskLevel.toUpperCase()}**)`);
  lines.push(`- **Erkannte Klauselfallen:** ${res.findings.length} (${res.counts.hoch}x hoch, ${res.counts.mittel}x mittel, ${res.counts.niedrig}x niedrig)`);
  lines.push(`- **Fazit:** ${res.summary}`);
  lines.push("");

  if (res.findings.length === 0) {
    lines.push("### Ergebnis: Keine auffälligen AGB-Fallen gefunden");
    lines.push("Der geprüfte Text enthält keine der 47 bekannten unwirksamen Klauselstrukturen.");
    return lines.join("\n");
  }

  lines.push("### Gefundene Klauselfallen im Detail:");
  lines.push("");

  for (let i = 0; i < res.findings.length; i++) {
    const f = res.findings[i];
    const sevBadge = f.severity === "hoch" ? "🔴 HOCH" : f.severity === "mittel" ? "🟡 MITTEL" : "⚪ NIEDRIG";
    const [from, to] = f.lineNumbers;
    const lineStr = from === to ? `Zeile ${from}` : `Zeilen ${from}–${to}`;

    lines.push(`#### ${i + 1}. [${sevBadge}] ${f.titlePlainGerman} (${lineStr})`);
    lines.push(`- **Gesetzliche Grundlage:** \`${f.citation || f.type}\``);
    lines.push(`- **Gefundener Vertragstext:**`);
    lines.push(`  > "${f.excerpt}"`);
    lines.push(`- **Rechtliche Einordnung:** ${f.why}`);
    if (f.suggestion) {
      lines.push(`- **Rechtssicherer Gegenvorschlag (Redline):**`);
      lines.push("```text");
      lines.push(f.suggestion);
      lines.push("```");
    }
    lines.push("");
  }

  lines.push("---");
  lines.push("💡 *Tipp: Für einen versiegelten DIN-A4 PDF-Auditbericht oder Portfolio-Audits nutze ClauseLens Studio: https://clause.kellersystems.dev/studio.html*");

  return lines.join("\n");
}

function handleAuditContract(args) {
  if (!args || typeof args.text !== "string" || !args.text.trim()) {
    throw new Error("Parameter 'text' muss ein nicht-leerer String sein.");
  }
  const profile = args.profile || "default";
  const result = analyzeContract(args.text, { profile });
  return formatAnalysisMarkdown(result, `Textprüfung (${args.text.length} Zeichen)`);
}

function handleAuditFile(args) {
  if (!args || typeof args.filePath !== "string" || !args.filePath.trim()) {
    throw new Error("Parameter 'filePath' ist erforderlich.");
  }
  const absPath = isAbsolute(args.filePath) ? args.filePath : resolve(process.cwd(), args.filePath);
  if (!existsSync(absPath)) {
    throw new Error(`Datei nicht gefunden: ${absPath}`);
  }
  const extracted = extractText(absPath, "auto");
  const profile = args.profile || "default";
  const result = analyzeContract(extracted.text, { profile });
  return formatAnalysisMarkdown(result, `${extracted.format.toUpperCase()}-Datei: ${absPath}`);
}

function handleGetBgbKnowledge(args) {
  const q = (args.query || "").toLowerCase().trim();
  if (!q) {
    throw new Error("Parameter 'query' ist erforderlich.");
  }

  const matches = BGB_KNOWLEDGE.filter(k =>
    k.type.includes(q) ||
    k.title.toLowerCase().includes(q) ||
    k.basis.toLowerCase().includes(q)
  );

  if (matches.length === 0) {
    return `Kein direkter Treffer für '${args.query}'. Verfügbare Hauptthemen: ${BGB_KNOWLEDGE.map(k => k.title).join(", ")}.\nAusführliches Klausel-Lexikon: https://clause.kellersystems.dev/klauseln/`;
  }

  const out = [];
  for (const item of matches) {
    out.push(`## BGB-Klauselwissen: ${item.title}`);
    out.push(`- **Rechtsnorm:** ${item.basis}`);
    out.push(`- **B2B- vs. B2C-Geltung:** ${item.b2bScope}`);
    out.push(`- **Typische unwirksame Formulierung:**`);
    out.push(`  > "${item.trap}"`);
    out.push(`- **BGB-konforme Muster-Alternative:**`);
    out.push("```text");
    out.push(item.redline);
    out.push("```");
    out.push(`- **Vertiefung im Lexikon:** https://clause.kellersystems.dev/klauseln/`);
    out.push("");
  }
  return out.join("\n");
}

function handleCompareContracts(args) {
  if (!args || typeof args.originalText !== "string" || typeof args.revisedText !== "string") {
    throw new Error("Sowohl 'originalText' als auch 'revisedText' sind erforderlich.");
  }
  const r1 = analyzeContract(args.originalText);
  const r2 = analyzeContract(args.revisedText);

  const deltaScore = r2.riskScore - r1.riskScore;
  const scoreBadge = deltaScore < 0 ? `Verbessert um ${Math.abs(deltaScore)} Punkte` : deltaScore > 0 ? `Verschlechtert um ${deltaScore} Punkte` : `Unverändert (${r1.riskScore}/100)`;

  const t1 = new Set(r1.findings.map(f => f.type));
  const t2 = new Set(r2.findings.map(f => f.type));

  const resolved = r1.findings.filter(f => !t2.has(f.type));
  const added = r2.findings.filter(f => !t1.has(f.type));

  const lines = [];
  lines.push(`## ClauseLens Vertragsvergleich & Risikodelta`);
  lines.push(`- **Ursprünglicher Risikoscore:** ${r1.riskScore}/100 (${r1.riskLevel.toUpperCase()})`);
  lines.push(`- **Neuer Risikoscore:** ${r2.riskScore}/100 (${r2.riskLevel.toUpperCase()})`);
  lines.push(`- **Veränderung:** **${scoreBadge}**`);
  lines.push("");

  if (resolved.length > 0) {
    lines.push(`### ✅ Entschärfte Klauselfallen (${resolved.length}):`);
    for (const f of resolved) {
      lines.push(`- **${f.titlePlainGerman}** (\`${f.citation || f.type}\`)`);
    }
    lines.push("");
  }

  if (added.length > 0) {
    lines.push(`### ⚠️ Neu hinzugekommene Risiken (${added.length}):`);
    for (const f of added) {
      lines.push(`- **[${f.severity.toUpperCase()}] ${f.titlePlainGerman}** (\`${f.citation || f.type}\`): "${f.excerpt}"`);
    }
    lines.push("");
  }

  if (resolved.length === 0 && added.length === 0) {
    lines.push("Keine strukturellen Änderungen der AGB-Risiken festgestellt.");
  }

  return lines.join("\n");
}

export function startMcpServer() {
  const rl = readline.createInterface({
    input: process.stdin,
    terminal: false
  });

  const send = (msg) => {
    process.stdout.write(JSON.stringify(msg) + "\n");
  };

  rl.on("line", (line) => {
    if (!line || !line.trim()) return;

    let req;
    try {
      req = JSON.parse(line.trim());
    } catch (e) {
      send({
        jsonrpc: "2.0",
        id: null,
        error: { code: -32700, message: "Parse error: Invalid JSON" }
      });
      return;
    }

    const { id, method, params } = req;

    // JSON-RPC Ping
    if (method === "ping") {
      send({ jsonrpc: "2.0", id, result: {} });
      return;
    }

    // MCP Handshake
    if (method === "initialize") {
      send({
        jsonrpc: "2.0",
        id,
        result: {
          protocolVersion: PROTOCOL_VERSION,
          capabilities: {
            tools: {}
          },
          serverInfo: {
            name: SERVER_NAME,
            version: SERVER_VERSION
          }
        }
      });
      return;
    }

    // MCP Initialized Notification (keine Antwort)
    if (method === "notifications/initialized") {
      return;
    }

    // Tools List
    if (method === "tools/list") {
      send({
        jsonrpc: "2.0",
        id,
        result: {
          tools: TOOLS_MANIFEST
        }
      });
      return;
    }

    // Tools Call
    if (method === "tools/call") {
      const toolName = params?.name;
      const args = params?.arguments || {};

      try {
        let textResult = "";
        if (toolName === "audit_contract") {
          textResult = handleAuditContract(args);
        } else if (toolName === "audit_file") {
          textResult = handleAuditFile(args);
        } else if (toolName === "get_bgb_clause_knowledge") {
          textResult = handleGetBgbKnowledge(args);
        } else if (toolName === "compare_contracts") {
          textResult = handleCompareContracts(args);
        } else {
          send({
            jsonrpc: "2.0",
            id,
            error: { code: -32601, message: `Tool nicht gefunden: ${toolName}` }
          });
          return;
        }

        send({
          jsonrpc: "2.0",
          id,
          result: {
            content: [
              {
                type: "text",
                text: textResult
              }
            ],
            isError: false
          }
        });
      } catch (err) {
        send({
          jsonrpc: "2.0",
          id,
          result: {
            content: [
              {
                type: "text",
                text: `Fehler bei der Ausführung von ${toolName}: ${err.message}`
              }
            ],
            isError: true
          }
        });
      }
      return;
    }

    // Unbekannte Methode
    if (id !== undefined && id !== null) {
      send({
        jsonrpc: "2.0",
        id,
        error: { code: -32601, message: `Methode nicht unterstützt: ${method}` }
      });
    }
  });

  process.stderr.write(`[ClauseLens MCP] Server läuft auf stdio (PID ${process.pid}).\n`);
  return 0;
}
