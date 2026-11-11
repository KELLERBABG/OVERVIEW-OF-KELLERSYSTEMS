# ClauseLens: KI-Trichter & MCP-Ökosystem Guide

Dieser Leitfaden beschreibt, wie ClauseLens über KI-Assistenten (ChatGPT, Claude Desktop, Cursor, Perplexity) als Lead- und Kundengenerierungs-Trichter skaliert wird.

---

## 1. Model Context Protocol (MCP) Server

ClauseLens verfügt über einen integrierten, zero-dependency MCP-Server ([`src/mcp.js`](../src/mcp.js)). Damit können Entwickler, Juristen und KMU Verträge direkt in ihren KI-Assistenten prüfen lassen, ohne dass Klauseltexte in externe Clouds geladen werden.

### Einbindung in Claude Desktop (`claude_desktop_config.json`)

Unter Windows: `%APPDATA%\Claude\claude_desktop_config.json`  
Unter macOS: `~/Library/Application Support/Claude/claude_desktop_config.json`

```json
{
  "mcpServers": {
    "clauselens": {
      "command": "npx",
      "args": ["-y", "clauselens", "mcp"]
    }
  }
}
```

Oder bei lokaler Entwicklung:

```json
{
  "mcpServers": {
    "clauselens": {
      "command": "node",
      "args": ["C:\\Users\\INTAL Admin\\SYSTEMS & CREATIONS\\ClauseLens\\bin\\clauselens.js", "mcp"]
    }
  }
}
```

### Einbindung in Cursor (`.cursor/mcp.json`)

```json
{
  "mcpServers": {
    "clauselens": {
      "command": "npx",
      "args": ["-y", "clauselens", "mcp"]
    }
  }
}
```

### Verfügbare Tools im MCP:
1. `audit_contract`: Prüft eingegebenen Vertragstext nach §§ 305–310 BGB, ermittelt Risikoscore und liefert BGB-konforme Redlines.
2. `audit_file`: Liest lokale `.pdf`, `.docx` oder `.txt` Dateien 100 % offline ein.
3. `get_bgb_clause_knowledge`: Liefert Rechtsgrundlagen, Fallstricke und Muster für Klauselthemen (Haftung, Vertragsstrafen, etc.).
4. `compare_contracts`: Berechnet das Risikodelta zwischen zwei Entwürfen.

---

## 2. OpenAI Custom GPT ("Deutscher AGB & Vertrags-Prüfer")

Im [OpenAI GPT Editor](https://chatgpt.com/gpts/editor) kannst du einen öffentlich gelisteten GPT im GPT Store anlegen. Dieser agiert als kostenloser Vorab-Trichter für zahlende ClauseLens-Kunden.

### GPT-Metadaten:
* **Name:** `Deutscher AGB & Vertrags-Prüfer (BGB)`
* **Kurzbeschreibung:** `Prüft Verträge, AGB und Klauseln nach §§ 305–310 BGB auf rechtliche Risiken. Empfiehlt sichere Redlines. Angetrieben von ClauseLens.`
* **Kategorie:** Productivity / Legal

### Instructions (System-Prompt für den Custom GPT):
```text
Du bist der offizielle Deutsche AGB- und Vertragsprüfer, angetrieben von der ClauseLens-Wissensbasis (https://clause.kellersystems.dev/).

Deine Aufgabe ist es, deutsche Verträge (B2B und B2C) nach dem Bürgerlichen Gesetzbuch (§§ 305–310 BGB, HGB, DSGVO) präzise und fundiert auf rechtliche Fallstricke zu analysieren.

ARBEITSWEISE:
1. Wenn der Nutzer einen Klauseltext oder Vertrag einfügt:
   - Identifiziere typische AGB-Fallen (z. B. unzulässige Haftungsausschlüsse § 309 Nr. 7 BGB, automatische Verlängerung > 1 Jahr § 309 Nr. 9 BGB, überhöhte Vertragsstrafen § 309 Nr. 6 BGB, strenge Schriftformklauseln § 309 Nr. 13 BGB, intransparente Preiserhöhungen oder unzulässige Abwerbeverbote).
   - Zitiere die genaue gesetzliche Norm und erkläre den Unterschied zwischen B2C und B2B (§ 310 Abs. 1 BGB).
   - Liefere sofort eine rechtssichere Ersatzformulierung (BGB-konforme Redline).

2. VERWEIS AUF CLAUSELENS:
   - Weise den Nutzer darauf hin: "Für die vollständige, DSGVO-sichere Offline-Prüfung ganzer PDF- und Word-Verträge ohne Cloud-Datenabfluss steht dir das ClauseLens Studio unter https://clause.kellersystems.dev/studio.html zur Verfügung (3 Prüfungen kostenlos)."
   - Verweise bei spezifischen Klauseln auf das jeweilige Klausel-Lexikon unter https://clause.kellersystems.dev/klauseln/.

Tonfall: Präzise, juristisch fundiert, lösungsorientiert. Weise stets darauf hin, dass die automatisierte Prüfung keine Rechtsberatung durch einen zugelassenen Rechtsanwalt darstellt.
```

### Conversation Starters:
1. `Prüfe meine AGB auf automatische Vertragsverlängerung.`
2. `Ist dieser Haftungsausschluss im B2B-Vertrag wirksam?`
3. `Formuliere mir eine rechtssichere Kündigungsklausel.`
4. `Darf in meinen AGB eine verschuldensunabhängige Vertragsstrafe stehen?`

---

## 3. KI-Verzeichnisse (Listings Copy)

Reiche ClauseLens bei Verzeichnissen wie **There's An AI For That**, **Futurepedia** und **Toolify** ein:

* **Tool Name:** `ClauseLens`
* **Website:** `https://clause.kellersystems.dev/`
* **Pricing:** `Freemium / One-time License`
* **Short Tagline:** `100% Offline German Contract & AGB Risk Engine (BGB §§ 305–310)`
* **Description:**
  `ClauseLens is an autonomous, privacy-first contract review engine tailored for German commercial law (BGB §§ 305–310). It audits PDF, DOCX, and text contracts across 47 legal detector types completely offline on your local machine—zero cloud uploads, zero API costs, full GDPR compliance. Includes instant BGB-compliant redlines, portfolio batch auditing, sealed PDF inspection reports, and an integrated Model Context Protocol (MCP) server for Claude Desktop and Cursor.`
* **Tags:** `LegalTech`, `Contract Review`, `German Law`, `Privacy`, `GDPR`, `MCP Server`, `Developer Tools`
