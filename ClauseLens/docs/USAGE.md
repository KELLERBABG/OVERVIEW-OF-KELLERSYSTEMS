# ClauseLens — Bedienungsanleitung (CLI, Web Studio & REST-API)

ClauseLens ist ein deterministischer, datenschutzkonformer Vertragsrisiko-Scanner für deutsche B2B-Verträge und AGB (§§ 305–310 BGB). Zero Dependencies, Node ≥ 18.

---

## 1. Installation & Schnellstart

```bash
# Voraussetzungen: Node.js >= 18 installiert
node -v

# 1. Einmalig: mit Lizenzschlüssel anmelden, Seat für dieses Gerät aktivieren
node bin/clauselens.js login --email kanzlei@recht.de --key CLA-PRO-...

# 2. Vertrag analysieren:
node bin/clauselens.js analyze mein_vertrag.pdf --pdf befund.pdf --sarif report.sarif
```

---

## 2. CLI-Befehlsreferenz

### 2.0 Account- & Seat-Lizenzierung (`login`, `whoami`, `logout`)
ClauseLens arbeitet 100% lokal on-premise, bindet jedoch jeden Arbeitsplatz über einen kryptografischen Hardware-Fingerprint an einen lizenzierten Nutzer-Account mit Online-Seat-Validierung.

Akzeptiert werden zwei Schlüsselarten:
- `CLA-PRO-…` / `CLA-TEAM-…` (ältere Bestellungen: `LENS-…`) aus der Polar-Bestellbestätigung. Der Schlüssel wird online bei Polar geprüft, der Seat über eine Polar-Aktivierung angelegt.
- `CL-PRO-…` / `CL-TEAM-…` vom Lizenzserver, ausgestellt vom Polar-Webhook. Er wird gegen die Lizenz-API `https://license.kellersystems.dev` geprüft (bei HTTP 403 wiederholt die CLI den Aufruf über `https://clauselens-license.papababg02.workers.dev`).

Der Server signiert bei erfolgreicher Prüfung ein Sitzungstoken (RS256, 7 Tage, an Lizenzschlüssel und Geräte-Fingerprint gebunden). Nur dieses Token liegt lokal; eine Kopie des Schlüssels allein genügt nicht. Nach Ablauf hilft ein erneutes `login`.

```bash
# Mit Lizenzschlüssel anmelden und Seat auf diesem Gerät aktivieren:
node bin/clauselens.js login --email nutzer@firma.de --key "CLA-PRO-..."

# Aktiven Account, Seat-ID und Hardware-Fingerprint abfragen:
node bin/clauselens.js whoami

# Seat auf diesem Gerät freigeben (z. B. vor Rechnerwechsel):
node bin/clauselens.js logout
```

Der Lizenzserver liegt in `worker/` (Cloudflare Worker) und wird mit `npx wrangler deploy --config worker/wrangler.toml` ausgeliefert. Ist er nicht erreichbar oder antwortet mit 429/5xx, arbeitet die CLI mit der gespeicherten Sitzung weiter, solange deren 7 Tage nicht abgelaufen sind; der Seat-Zähler wird dann nicht aktualisiert.

### 2.0a Browser und GitHub Action
`index.html` und `studio.html` aktivieren Schlüssel über denselben Weg: Der Browser ruft `/api/v1/seats/verify` des Lizenzservers auf, erhält das RS256-Sitzungstoken und prüft die Signatur lokal per WebCrypto (`src/licence-token.js`). In `localStorage` liegen nur der Schlüssel und das signierte Token; ein dort manipulierter Eintrag schaltet nichts frei, und eine an ein anderes Gerät gebundene Sitzung wird abgelehnt. Die Gültigkeit wird alle 12 Stunden erneuert; ohne Netzverbindung bleibt die Sitzung bis zum Ablauf nutzbar.

Die GitHub Action (`action.yml`) aktiviert zuerst den Seat und analysiert dann:

```yaml
- uses: KELLERBABG/ClauseLens@main
  with:
    path: contracts
    email: ${{ secrets.CLAUSELENS_EMAIL }}
    license-key: ${{ secrets.CLAUSELENS_LICENSE_KEY }}
    fail-on: high
```

Ohne `email` und `license-key` läuft die Action im unlizenzierten Modus (nur Community-Umfang). Pro CI-Lauf wird ein Seat belegt, weil jeder Runner einen neuen Hardware-Fingerprint hat.

### 2.1 Einzelvertragsanalyse (`analyze`)
Analysiert eine einzelne Datei (`.pdf`, `.docx` oder `.txt`).

```bash
# Terminal-Zusammenfassung ausgeben:
node bin/clauselens.js analyze vertrag.pdf

# Versiegelten mehrseitigen DIN-A4 Vektor-PDF-Bericht erstellen:
node bin/clauselens.js analyze vertrag.pdf --pdf audit.pdf

# OASIS SARIF 2.1.0 für GitHub Code Scanning erstellen:
node bin/clauselens.js analyze vertrag.pdf --sarif results.sarif

# Interaktiven, selbstständigen HTML-Bericht erstellen:
node bin/clauselens.js analyze vertrag.pdf --html risikobericht.html

# Strukturiertes JSON und Markdown gleichzeitig exportieren:
node bin/clauselens.js analyze vertrag.docx --json result.json --report report.md

# Spezielles Branchenprofil anwenden:
node bin/clauselens.js analyze saas_agb.txt --profile b2b_saas

# Eigene Firmen-Konfiguration einbinden:
node bin/clauselens.js analyze vertrag.pdf --config clauselens.config.json
```

#### Verfügbare Optionen für `analyze`:
- `--pdf <pfad>`: Schreibt einen gerichtsfesten mehrseitigen DIN-A4 Vektor-PDF-Bericht mit SHA-256 Siegel.
- `--sarif <pfad>`: Schreibt einen OASIS SARIF 2.1.0 Bericht für CI/CD Security-Pipelines.
- `--html <pfad>`: Schreibt einen interaktiven HTML-Bericht mit SVG-Gauges, Filtern und Redline-Kopierfunktion.
- `--json <pfad>`: Schreibt das Analyseergebnis als schema-konformes JSON.
- `--report <pfad>`: Schreibt einen Markdown-Risikobericht.
- `--format <fmt>`: `auto` (Standard), `pdf`, `docx` oder `txt` erzwingen.
- `--profile <name>`: Prüfprofil wählen: `default`, `b2b_saas`, `freelance_consulting`, `procurement_strict`.
- `--config <pfad>`: Eigene Konfigurationsdatei laden.
- `--license <key>`: Alias für `--key` bei `login`.

---

### 2.2 Portfolio- & Batch-Audit (`batch`)
Prüft ganze Verzeichnisse oder Sammlungen von Verträgen auf einen Schlag.

```bash
# Alle Verträge im aktuellen Verzeichnis scannen:
node bin/clauselens.js batch ./vertraege --html portfolio.html --json portfolio.json
```

**Ergebnis:**
- Aggregierter Portfolio-Risiko-Score
- Übersicht über Verträge mit hohem/mittlerem Risiko
- Top-Risikoklauseln im gesamten Vertragsbestand

---

### 2.3 Versionsvergleich & Risikodelta (`compare`)
Vergleicht die ursprüngliche Fassung eines Vertrags mit dem Gegenvorschlag der Gegenseite nach Nachverhandlungen:

```bash
node bin/clauselens.js compare entwurf_v1.docx gegenentwurf_v2.docx --report delta.md --html delta.html
```

**Ergebnis:**
- Berechnet das genaue Punkte-Delta des Risikoscores (z. B. von 82/100 auf 24/100 gesenkt)
- Zeigt behobene Risiken
- Entlarvt neu eingeschleuste Risiken

---

### 2.4 Lokalen Webserver & Studio starten (`serve`)
Startet den integrierten HTTP-Server mit Zero Dependencies:

```bash
node bin/clauselens.js serve --port 3000
# oder direkt:
npm start
```

- **Landingpage:** `http://localhost:3000/` (Interaktive Sandbox mit Drag & Drop Upload)
- **ClauseLens Studio:** `http://localhost:3000/app` (Interaktives Dokumentenaudit & Redlines)
- **OpenAPI 3.0 Explorer:** `http://localhost:3000/docs`
- **REST-API:** `http://localhost:3000/api/v1/...`

---

## 3. REST-API-Endpunkte & Developer Hub

### `GET /docs` & `GET /api/v1/openapi.json`
Liefert die interaktive API-Dokumentation bzw. die maschinenlesbare OpenAPI 3.0.3 Spezifikation.

### `GET /api/v1/health`
Liefert Status, Engine-Version und Uptime.
```bash
curl http://localhost:3000/api/v1/health
```

### `GET /api/v1/rules`
Liefert den vollständigen Katalog aller 47 BGB-Klauseldetektoren und verfügbaren Prüfprofile.
```bash
curl http://localhost:3000/api/v1/rules
```

### `POST /api/v1/extract`
Extrahiert Text aus binären PDF- oder DOCX-Dateien ohne Parsing-Hürden:
```bash
curl -X POST http://localhost:3000/api/v1/extract \
  -H "Content-Type: application/octet-stream" \
  --data-binary @vertrag.pdf
```

### `POST /api/v1/analyze`
Analysiert übergebenen Vertragstext. Unterstützt optional `?format=pdf` für direkten PDF-Download:
```bash
curl -X POST http://localhost:3000/api/v1/analyze \
  -H "Content-Type: application/json" \
  -d '{"text": "Die Haftung ist für sämtliche Schäden ausgeschlossen.", "profile": "default"}'
```

### `POST /api/v1/compare`
Vergleicht zwei Vertragstexte und beziffert das Risikodelta.
```bash
curl -X POST http://localhost:3000/api/v1/compare \
  -H "Content-Type: application/json" \
  -d '{"text1": "Die Haftung ist ausgeschlossen.", "text2": "Die Haftung gilt gesetzlich."}'
```

### `POST /api/v1/batch`
Auditiert eine Sammlung von Verträgen in einem einzigen Request.

### `GET /api/v1/account/status`
Fragt den Status des aktuellen Geräte-Seats und den Hardware-Fingerprint ab.

### `POST /api/v1/account/login`
Aktiviert einen Lizenzschlüssel online für diesen Rechner.
```bash
curl -X POST http://localhost:3000/api/v1/account/login \
  -H "Content-Type: application/json" \
  -d '{"email": "kanzlei@recht.de", "licenseKey": "CL-PRO-..."}'
```
