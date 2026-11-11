"use strict";

import { RULES } from "./analyze.js";
import { POLICY_PROFILES } from "./config.js";

export function getOpenApiSpec() {
  return {
    openapi: "3.0.3",
    info: {
      title: "ClauseLens Contract Risk Audit API",
      version: "1.0.0",
      description:
        "Autonome, deterministische Vertragsanalyse & AGB-Prüfung nach BGB §§ 305–310. Zero external dependencies, 100% offline & datenschutzkonform.",
      contact: {
        name: "ClauseLens Team",
        url: "https://clauselens.kellersystems.dev",
      },
      license: {
        name: "MIT",
        url: "https://opensource.org/licenses/MIT",
      },
    },
    servers: [
      {
        url: "http://localhost:3000",
        description: "Lokaler ClauseLens Server",
      },
    ],
    paths: {
      "/api/v1/health": {
        get: {
          summary: "System-Gesundheits- & Statusprüfung",
          description: "Gibt den aktuellen Engine-Status, Version und Uptime zurück.",
          responses: {
            "200": {
              description: "Erfolgreicher Status",
              content: {
                "application/json": {
                  schema: {
                    type: "object",
                    properties: {
                      status: { type: "string", example: "ok" },
                      engine: { type: "string", example: "ClauseLens" },
                      version: { type: "string", example: "1.0.0" },
                      uptimeSeconds: { type: "integer", example: 42 },
                    },
                  },
                },
              },
            },
          },
        },
      },
      "/api/v1/rules": {
        get: {
          summary: "Katalog der aktiven BGB-Prüfregeln",
          description: "Listet alle 35 spezialisierten Prüfdetektoren und verfügbaren Policy-Profile.",
          responses: {
            "200": {
              description: "Regelkatalog",
              content: {
                "application/json": {
                  schema: {
                    type: "object",
                    properties: {
                      count: { type: "integer", example: 35 },
                      profiles: {
                        type: "array",
                        items: { type: "string" },
                        example: ["default", "b2b_saas", "freelance_consulting", "procurement_strict"],
                      },
                      rules: {
                        type: "array",
                        items: {
                          type: "object",
                          properties: {
                            type: { type: "string", example: "liability" },
                            title: { type: "string", example: "Pauschaler Haftungsausschluss" },
                            severity: { type: "string", example: "hoch" },
                            why: { type: "string" },
                            suggestion: { type: "string" },
                            tags: { type: "array", items: { type: "string" } },
                          },
                        },
                      },
                    },
                  },
                },
              },
            },
          },
        },
      },
      "/api/v1/analyze": {
        post: {
          summary: "Vertragstext analysieren",
          description: "Analysiert Vertragsklauseln und liefert Risikoscore, Befunde und Gegenvorschläge.",
          parameters: [
            {
              name: "X-License-Key",
              in: "header",
              required: false,
              schema: { type: "string" },
              description: "Optionaler HMAC-SHA256 Lizenzschlüssel",
            },
          ],
          requestBody: {
            required: true,
            content: {
              "application/json": {
                schema: {
                  type: "object",
                  required: ["text"],
                  properties: {
                    text: {
                      type: "string",
                      description: "Vollständiger Vertragstext oder Klauselausschnitt",
                      example: "Die Haftung des Anbieters ist für sämtliche Schäden ausgeschlossen.",
                    },
                    profile: {
                      type: "string",
                      default: "default",
                      enum: ["default", "b2b_saas", "freelance_consulting", "procurement_strict"],
                    },
                    filename: {
                      type: "string",
                      default: "api_input.txt",
                    },
                    format: {
                      type: "string",
                      enum: ["json", "html", "pdf", "sarif"],
                      default: "json",
                      description: "Ausgabeformat (json, html, pdf oder sarif)",
                    },
                  },
                },
              },
            },
          },
          responses: {
            "200": {
              description: "Analyseergebnis",
              content: {
                "application/json": {
                  schema: {
                    type: "object",
                    properties: {
                      riskScore: { type: "integer", example: 85 },
                      riskLevel: { type: "string", enum: ["hoch", "mittel", "niedrig"], example: "hoch" },
                      counts: {
                        type: "object",
                        properties: {
                          hoch: { type: "integer" },
                          mittel: { type: "integer" },
                          niedrig: { type: "integer" },
                        },
                      },
                      findings: {
                        type: "array",
                        items: {
                          type: "object",
                          properties: {
                            type: { type: "string" },
                            severity: { type: "string" },
                            titlePlainGerman: { type: "string" },
                            excerpt: { type: "string" },
                            why: { type: "string" },
                            suggestion: { type: "string" },
                            lineNumbers: { type: "array", items: { type: "integer" } },
                          },
                        },
                      },
                      summary: { type: "string" },
                    },
                  },
                },
                "application/pdf": {
                  schema: { type: "string", format: "binary" },
                },
                "text/html": {
                  schema: { type: "string" },
                },
              },
            },
            "400": {
              description: "Ungültige Eingabe",
            },
          },
        },
      },
      "/api/v1/extract": {
        post: {
          summary: "Text aus PDF, DOCX oder TXT Datei extrahieren",
          description: "Natives Zero-Dependency Parsing von Binärdokumenten.",
          requestBody: {
            required: true,
            content: {
              "application/json": {
                schema: {
                  type: "object",
                  required: ["fileBase64"],
                  properties: {
                    fileBase64: { type: "string", description: "Base64-kodierte Datei" },
                    filename: { type: "string", example: "muster_vertrag.pdf" },
                    format: { type: "string", enum: ["auto", "pdf", "docx", "txt"], default: "auto" },
                  },
                },
              },
            },
          },
          responses: {
            "200": {
              description: "Extrahierter Text",
              content: {
                "application/json": {
                  schema: {
                    type: "object",
                    properties: {
                      text: { type: "string" },
                      format: { type: "string" },
                      charCount: { type: "integer" },
                      lineCount: { type: "integer" },
                    },
                  },
                },
              },
            },
          },
        },
      },
      "/api/v1/license": {
        get: {
          summary: "Lizenzschlüssel validieren",
          parameters: [
            {
              name: "key",
              in: "query",
              required: false,
              schema: { type: "string" },
            },
            {
              name: "X-License-Key",
              in: "header",
              required: false,
              schema: { type: "string" },
            },
          ],
          responses: {
            "200": {
              description: "Lizenzstatus",
              content: {
                "application/json": {
                  schema: {
                    type: "object",
                    properties: {
                      valid: { type: "boolean" },
                      tier: { type: "string", enum: ["community", "pro", "enterprise"] },
                      licensee: { type: "string" },
                      expiresAt: { type: "string" },
                    },
                  },
                },
              },
            },
          },
        },
      },
    },
  };
}

export function renderApiDocsHtml() {
  const spec = getOpenApiSpec();
  const specJsonSafe = JSON.stringify(spec).replace(/</g, "\\u003c");

  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>ClauseLens REST API & Developer Hub</title>
  <style>
    :root {
      --bg: #121110;
      --surface: #1a1918;
      --surface-2: #242220;
      --border: #33312e;
      --text: #eae6df;
      --text-muted: #9c9689;
      --red: #d1563c;
      --green: #48a05b;
      --amber: #d99b38;
      --blue: #3d8bfd;
      --mono: 'IBM Plex Mono', monospace;
      --sans: 'Inter', -apple-system, sans-serif;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body { background: var(--bg); color: var(--text); font-family: var(--sans); line-height: 1.5; padding: 32px 20px; }
    .wrap { max-width: 960px; margin: 0 auto; }
    header { border-bottom: 1px solid var(--border); padding-bottom: 24px; margin-bottom: 32px; }
    .title-row { display: flex; justify-content: space-between; align-items: baseline; flex-wrap: wrap; gap: 12px; }
    h1 { font-family: var(--mono); font-size: 24px; color: #fff; }
    .badge { display: inline-block; font-family: var(--mono); font-size: 11px; padding: 3px 8px; border-radius: 4px; background: var(--surface-2); border: 1px solid var(--border); color: var(--text-muted); }
    .badge-post { background: rgba(72,160,91,0.15); border-color: var(--green); color: var(--green); }
    .badge-get { background: rgba(61,139,253,0.15); border-color: var(--blue); color: var(--blue); }
    .endpoint { background: var(--surface); border: 1px solid var(--border); border-radius: 8px; margin-bottom: 20px; overflow: hidden; }
    .ep-header { padding: 14px 18px; display: flex; align-items: center; gap: 12px; border-bottom: 1px solid var(--border); background: var(--surface-2); font-family: var(--mono); font-size: 14px; }
    .ep-body { padding: 18px; }
    .ep-desc { color: var(--text-muted); font-size: 13.5px; margin-bottom: 14px; }
    pre { background: #0c0b0a; border: 1px solid #282624; border-radius: 6px; padding: 12px; font-family: var(--mono); font-size: 12px; overflow-x: auto; color: #c8c2b7; margin-top: 8px; }
    .nav-links { display: flex; gap: 14px; margin-top: 14px; }
    .nav-links a { color: var(--red); text-decoration: none; font-size: 13px; font-family: var(--mono); }
    .nav-links a:hover { text-decoration: underline; }
  </style>
</head>
<body>
  <div class="wrap">
    <header>
      <div class="title-row">
        <h1>ClauseLens REST API &middot; v1.0.0</h1>
        <span class="badge">OpenAPI 3.0.3 &middot; Zero Dependencies</span>
      </div>
      <p style="color: var(--text-muted); margin-top: 8px; font-size: 14px;">
        Deterministische Vertragsprüfung und AGB-Auditierung als REST-Schnittstelle.
      </p>
      <div class="nav-links">
        <a href="/">&larr; Zurück zur Übersicht</a>
        <a href="/app">ClauseLens Web Studio &rarr;</a>
        <a href="/api/v1/openapi.json" target="_blank">Rohdaten: openapi.json</a>
      </div>
    </header>

    <div id="endpointsList"></div>
  </div>

  <script>
    const spec = ${specJsonSafe};
    const container = document.getElementById('endpointsList');

    for (const [path, methods] of Object.entries(spec.paths)) {
      for (const [method, details] of Object.entries(methods)) {
        const ep = document.createElement('div');
        ep.className = 'endpoint';
        const methodUpper = method.toUpperCase();
        const badgeClass = methodUpper === 'POST' ? 'badge-post' : 'badge-get';

        let bodyExample = '';
        if (details.requestBody && details.requestBody.content['application/json']) {
          const schema = details.requestBody.content['application/json'].schema;
          bodyExample = '<div style="font-weight:600;font-size:12px;margin-top:12px;color:var(--text)">Request Body Schema:</div><pre>' +
            JSON.stringify(schema, null, 2) + '</pre>';
        }

        ep.innerHTML = \`
          <div class="ep-header">
            <span class="badge \${badgeClass}">\${methodUpper}</span>
            <span>\${path}</span>
          </div>
          <div class="ep-body">
            <div style="font-weight:600;font-size:15px;color:#fff;margin-bottom:4px;">\${details.summary || ''}</div>
            <div class="ep-desc">\${details.description || ''}</div>
            \${bodyExample}
          </div>
        \`;
        container.appendChild(ep);
      }
    }
  </script>
</body>
</html>`;
}
