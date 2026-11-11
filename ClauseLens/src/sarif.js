"use strict";

import { writeFileSync, mkdirSync } from "node:fs";
import { dirname } from "node:path";

function mapSeverityToSarifLevel(sev) {
  if (sev === "hoch") return "error";
  if (sev === "mittel") return "warning";
  return "note";
}

/**
 * Generates OASIS SARIF 2.1.0 JSON object for integration with
 * GitHub Code Scanning, GitLab Security, VS Code SARIF viewer and CI pipelines.
 */
export function generateSarif(result, meta = {}) {
  const sourceName = meta.sourceName || meta.source || "contract.txt";
  const findings = Array.isArray(result.findings) ? result.findings : [];

  const seenRules = new Map();
  const results = [];

  for (const f of findings) {
    if (!seenRules.has(f.type)) {
      seenRules.set(f.type, {
        id: f.type,
        name: f.type.replace(/_/g, " "),
        shortDescription: {
          text: f.titlePlainGerman || f.type,
        },
        fullDescription: {
          text: f.why || f.titlePlainGerman || f.type,
        },
        defaultConfiguration: {
          level: mapSeverityToSarifLevel(f.severity),
        },
        help: {
          text: `Empfohlener Gegenentwurf: ${f.suggestion || "Klausel streichen oder nachverhandeln."}`,
        },
        properties: {
          tags: ["legal", "bgb", "agb", "contract-risk"],
        },
      });
    }

    const [startLine = 1, endLine = 1] = Array.isArray(f.lineNumbers) ? f.lineNumbers : [1, 1];

    results.push({
      ruleId: f.type,
      level: mapSeverityToSarifLevel(f.severity),
      message: {
        text: `${f.titlePlainGerman || f.type}: ${f.why}\nFundstelle: "${f.excerpt}"\nGegenvorschlag: ${f.suggestion}`,
      },
      locations: [
        {
          physicalLocation: {
            artifactLocation: {
              uri: sourceName,
              uriBaseId: "%SRCROOT%",
            },
            region: {
              startLine: Math.max(1, startLine),
              endLine: Math.max(Math.max(1, startLine), endLine),
              snippet: {
                text: f.excerpt || "",
              },
            },
          },
        },
      ],
    });
  }

  const sarif = {
    $schema: "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
    version: "2.1.0",
    runs: [
      {
        tool: {
          driver: {
            name: "ClauseLens",
            semanticVersion: "1.0.0",
            informationUri: "https://clauselens.kellersystems.dev",
            rules: Array.from(seenRules.values()),
          },
        },
        results,
      },
    ],
  };

  return JSON.stringify(sarif, null, 2);
}

export function writeSarif(sarifContent, outPath) {
  mkdirSync(dirname(outPath), { recursive: true });
  writeFileSync(outPath, sarifContent, "utf-8");
}
