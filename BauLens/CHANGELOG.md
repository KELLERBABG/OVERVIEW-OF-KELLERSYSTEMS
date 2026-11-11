# Änderungsverlauf

Alle wesentlichen Änderungen an BauLens. Die Versionsnummer in `package.json` und im CLI-Kopf
folgt diesem Verlauf; die Fußzeile des Studios nennt zusätzlich die Engine-Version.

Format nach [Keep a Changelog](https://keepachangelog.com/de/1.1.0/),
Versionierung nach [Semantic Versioning](https://semver.org/lang/de/).

---

## [Unveröffentlicht]

### Hinzugefügt

- **Lizenzierung und 14-Tage-Testphase in der Desktop-Anwendung.** Der monetäre Kern des Produkts
  ist jetzt real: `src/licensing.js` entscheidet getrennt von der Anwendung, ob sie frei läuft —
  Testphase (Startzeitpunkt im Konfigurationsverzeichnis der App, sofort persistiert), aktivierte
  Lizenz oder Sperre. Die Aktivierung prüft den Schlüssel gegen die öffentliche, authentifizierungs-
  freie Polar-Schnittstelle `POST /v1/customer-portal/license-keys/validate`; gesendet wird nur der
  Schlüssel, nie Plandaten. Ein ungültiger Schlüssel wird abgewiesen, ein nicht erreichbares Polar
  wird nicht mit einem ungültigen Schlüssel verwechselt (`valid: null` statt `valid: false`), und
  eine zuvor geprüfte Lizenz bleibt offline 30 Tage gültig, bevor eine erneute Prüfung fällig ist.
  Ist die Testphase abgelaufen und kein brauchbarer Schlüssel hinterlegt, sperrt ein Zahlenschirm
  die Arbeitsfläche („Testphase abgelaufen“) mit Eingabe, Kauf-Link und ehrlicher Fehlermeldung;
  Projekte und Plandaten bleiben erhalten. Die Sperre ist nicht nur die Oberfläche: eine einzige
  Schranke (`assertUnlocked()`) verweigert im gesperrten Zustand die Arbeitsbefehle (Ordner öffnen,
  Stapelprüfung, Exporte, Drop-Routing, Druck) ebenso wie die im Studio-Seitencode verdrahteten
  Aktionen (Auswertung, Datei-Importe, Downloads, Druck) und schluckt die Kameratasten 1/2/3/R,
  bevor die Seite sie sieht — statt unter dem Zahlenschirm weiterzuarbeiten. Ein gesperrter
  Zahlenschirm lässt sich nicht schließen, auch der Offline-Aktivierungsversuch nicht; darauf
  verlässt sich die Schranke als reines Prädikat. Neuer Shell-Befehl `license_state_read` /
  `license_state_write` schreibt den Zustand nach `<app-config>/license-state.json` — absichtlich
  außerhalb des Webview-Speichers. Die CSP der Desktop-Hülle lässt nur `https://api.polar.sh`
  zusätzlich zu. Die Kommandozeile bleibt unbetroffen: die Regeln kennen weiterhin keine Lizenz.
- **Tests: `test/licensing.test.js`.** 22 Fälle — Testphasenbeginn, Ablauf auf den Tag, zurück-
  gesetzte Uhr, Sperrgründe, Polar-Sukzess (200), Ablehnung (404), `revoked` bei 200, offline-
  Grace, Ablaufdatum gegen Grace-Fenster und die Konsistenz von `site.config.json` mit dem Modul.
- **Tests: `test/desktop_licensing.test.js`.** 18 Fälle, die das ausgelieferte Inject-Skript in
  einer VM mit minimalem DOM fahren: Erststart, gesperrter Zahlenschirm (nicht schließbar — auch
  offline nicht), Aktivierung (Erfolg, Ablehnung, Doppel-Enter, offline mit Cache), Flucht-Taste,
  lesbarer Speicher — und die Schranke selbst auf beiden Hosts: im gesperrten Zustand erreichen
  Ordner- öffnen, Prüfung, Export, Drop, die Kameratasten und die Studio-Aktionen der Seite die
  Shell nicht, in der Testphase laufen sie durch.

### Geändert

- **Interne Struktur des Inject-Skripts, Verhalten unverändert.** Die Lizenz-Sektion gliedert sich
  in Daten, Schranke, Darstellung und Abläufe: `setLicenseState()` ist der einzige Schreiber von
  Zustand und Entscheidung, `gated()` der einzige Ort, an dem die Schranke auf eine Funktion
  angewandt wird (Stapelbefehle und Studio-Aktionen registrieren durch ihn), und beide Drop-Pfade
  teilen sich `feedPageInput()`. Der Aufbau steht im Kopf der Sektion.

### Behoben

- **Die Desktop-Injektion landete im Studio im Seitenquelltext statt am Dokumentende.**
  `build-desktop-assets.mjs` ersetzte die erste `</body>` der Datei — im Studio steht die erste
  in einem Template-Literal (der Dossier-Export), sodass das injizierte Skript mitten im
  Skript-String der Seite landete: dessen `</script>` schnitt das Seitenskript mitten im
  Template ab, und weder Seitencode noch Schranke liefen je in der Auslieferung (im Gegenunterschied
  zum Workhorse, der genau eine `</body>` hat). Eingefügt wird jetzt an der letzten `</body>`,
  `test/staging.test.js` pinnt die Skript-Reihenfolge der gestapelten Seiten.

## [0.10.0] — 2026-10-06

### Hinzugefügt

- **DIN-A4-Befund als echte PDF-Datei.** Die Engine schreibt den Vorprüfungsbefund jetzt selbst als
  PDF: `check spec.json --pdf befund.pdf` auf der Kommandozeile, die IPC-Methode `dossier_pdf` für
  die Shell, im Desktop-Workhorse die Schaltfläche „PDF“ mit nativem Speichern-Dialog — kein
  Druckdialog mehr nötig. Geschrieben von `src/dossierpdf.js` über einen eigenen, abhängigkeitsfreien
  PDF-Schreiber (`src/pdf.js`); die Glyphenbreiten stammen aus den Adobe-AFM-Dateien der Core-14-
  Schriften (`src/pdfmetrics.js`), nicht aus Schätzungen. Die Prüfziffer des Siegels ist mit der
  HTML-Fassung identisch, sodass beide Ausgaben vergleichbar sind. Fußzeile mit Seitenzahl auf jeder
  Seite, Kopfzeile und Tabellenkopf werden bei Seitenumbruch wiederholt.
- **Hauskörper aus den Plandaten statt aus einem erfundenen Rechteck.** Das Lageplan-Blatt zeichnet
  die überbaute Fläche aus den Angaben des Vorhabens: `GRZ × Grundstücksfläche` (`grz_ist` ist per
  Definition der überbaute Anteil, und so bleibt die gezeichnete Bebauung mit dem Grundstück stimmig,
  das dasselbe Blatt daneben zeigt), sonst `Nutzfläche / Geschosse`; ohne Geschossangabe gilt die
  Nutzfläche als Grundfläche, genau so trägt der DXF-Import die gemessene überbaute Fläche ein. Das
  Grundstücksrechteck ist dann Hauskörper plus die vier geprüften Grenzabstände, und die Legende nennt
  die Rechnung, aus der die Maßzahlen stammen (`Grundfläche 280 m² · GRZ 0,35 × 800 m²`). Nur ohne
  verwertbare Angabe bleibt das schematische Bezugsrechteck — dann steht das auch so auf dem Blatt.
  Weil der Plan aus diesen Werten zeichnet, deckt die Prüfziffer sie jetzt mit ab: Kommandozeile und
  Desktop-Schicht übergeben `gebaeude` an den Renderer, sonst trügen zwei Vorhaben, die sich nur in
  der GRZ unterscheiden, dasselbe Siegel. Ein Test vergleicht beide Wege bis aufs Byte.
- **Blattübersicht und Blattnummern.** Das Dossier ist jetzt eine Mappe: mit dem Lageplan-Blatt
  zerfällt es in Blätter (Blatt 1 Befund, Blatt 2 Lageplan, Blatt 3 Unterschriften und Prüfsiegel),
  und eine Registertafel auf der ersten Seite nennt jedes Blatt mit Inhalt und den Seiten, auf denen
  es liegt. Jede Fußzeile trägt `Blatt 2/3 · Lageplan · Seite 3 von 4`, sodass ein einzelnes Blatt
  sich selbst zuordnet; das Schriftfeld des Lageplans wiederholt seine Blattnummer. Das Blatt ist
  die Ablageeinheit, die Seite die Zählung — ein Befund über zwei Seiten bleibt Blatt 1. Die
  Seitenbereiche der Tafel sind erst nach dem Umbruch bekannt und werden im selben späten Durchgang
  gesetzt wie die Fußzeilen. Die Übersicht gibt es nur im PDF: die HTML-Fassung ist ein einziger
  Druckfluss ohne Blätter, und erfundene Blattnummern wären dort eine Struktur, die das Dokument
  nicht hat.
- **Lageplan als eigenes Zeichnungsblatt.** Der Plan verlässt den Textfluss und bekommt ein
  DIN-A4-Querblatt mit Blattrahmen und Schriftfeld: Maßketten in Metern an allen vier Seiten,
  Maßstabsbalken, Legende und Nordpfeil, dazu ein angegebener Maßstab (1:100 bis 1:1000), der aus
  dem Platzbedarf der Abstandsflächen gewählt wird. Die Maßzahlen sind die geprüften Grenzabstände,
  das Bezugsrechteck ist als schematisch gekennzeichnet; die Fußzeile mit Seitenzahl sitzt auch auf
  dem Querblatt, und der Blattwechsel ist im Layoutaudit als eigenes Blatt überprüfbar.
- **Prüfsumme des Bündels über die RPC-Schnittstelle** (`scripts/smoke-bundle.mjs`). Die Suite prüft
  nun auch `ping`, `describe`, `check_file` und `dossier_pdf` durch das gebaute Einzeldatei-Artefakt
  und vergleicht die Antworten mit der Quelle.
- **Lagepläne eines ganzen Projektordners in einem Durchgang.** Im Desktop-Workhorse schreibt die
  Schaltfläche „Alle Lagepläne exportieren“ für jedes Vorhaben des geöffneten Ordners ein Blatt als
  DXF und SVG in einen im Dialog gewählten Zielordner. Die Engine-Methode ist `plan_export`
  (`{ files: [{ path, label }], outDir, formats }`), die die Schleife selbst fährt, weil der
  Dateiname, der Maßstab und die Prüfziffer je Blatt Engine-Tatsachen sind — die Shell liefert nur
  das Suchergebnis und den Ordner. Je Projekt wird die erste Plandatei mit Geometrie gewählt (DXF,
  IFC, XPlanung, B-Plan), sonst die erste Datei; benannt wird nach dem Vorhaben
  (`Wohnanlage-Musterweg_Lageplan.dxf`). Ein unlesbarer Plan wird mit Namen gemeldet und nimmt die
  übrigen nicht mit, ein fehlendes Zielverzeichnis wird nie erfunden, und zwei Vorhaben mit gleichem
  Namen bekommen `-2`-Blätter statt einander zu überschreiben — die Umbenennung steht im Bericht.
  Die Namen werden aus Verzeichnisnamen gebildet und deshalb entschärft: Trennzeichen,
  Laufwerksbuchstaben, Steuerzeichen und `..` können den gewählten Ordner nicht verlassen
  (`src/planexport.js`). Der Ordnerdialog kann jetzt eine Beschriftung tragen, damit „Zielordner für
  die Lagepläne wählen“ nicht wie „Projektordner öffnen“ aussieht.
- **Das Lageplan-Blatt als DXF und SVG.** Das Zeichnungsblatt verlässt die Mappe auch als eigene
  Datei: `--svg lageplan.svg` und `--dxf lageplan.dxf` auf der Kommandozeile, die IPC-Methoden
  `plan_svg` und `plan_dxf` für die Shell, im Desktop-Workhorse die Schaltflächen „Lageplan als SVG
  speichern“ und „Lageplan als DXF speichern“. Beide Formate entstehen aus **einer** Aufzeichnung
  (`recordPlanSheet` in `src/dossierpdf.js`): die PDF-Seite spielt genau dieselben Primitive ab, die
  `src/plansvg.js` und `src/plandxf.js` als Datei schreiben — zwei Zeichenwege wären zwei
  Zeichnungen, und die erste Korrektur landete nur in einer. Das DXF ist ein konservatives ASCII-DXF
  im AutoCAD-2000-Dialekt (AC1015): eine Zeichnungseinheit = ein Meter (`$INSUNITS 6`), Rechtecke als
  geschlossene `LWPOLYLINE` (der eigene CAD-Import liest sie als Grundstücksgrenze und Hauskörper
  zurück, Test in `test/planio.test.js`), Beschriftung als `TEXT`, ein Bauteil je Ebene, dazu ein
  eigener `DASHED`-Linientyp für die Abstandsflächen; Blattrahmen, Blattkopf, Legende,
  Maßstabsbalken und Schriftfeld liegen auf eigenen Ebenen und lassen sich einfrieren. Das SVG ist
  DIN A4 quer in Punkten (gleiche viewBox, keine Umrechnung nötig), mit einer Gruppe je Ebene und
  ohne eingebettete Schrift oder externe Verweise. Beide nennen Maßstab und Prüfziffer, und beide
  sind byteweise reproduzierbar.

### Behoben

- **`check_file` war im Bündel funktionsunfähig.** Der Bundler band Node-Builtins über eine von Hand
  gepflegte Namensliste; `extname` fehlte darauf, und `import { basename, extname } from "node:path"`
  wurde still verworfen. Das gebaute Artefakt warf beim Öffnen einer Plandatei „extname is not
  defined“, während alle CLI-Fälle weiter bestanden — die Desktop-Anwendung hätte keine einzige Datei
  einlesen können. Die Bindungen werden jetzt aus den Importen abgeleitet, und die Prüfsumme testet
  die RPC-Schnittstelle, über die der Fehler erreichbar war.
- **Metadaten des PDFs in der richtigen Kodierung.** Strings im Info-Dictionary sind PDF-Textstrings
  (PDFDocEncoding), nicht WinAnsi wie der Seiteninhalt; ein Halbgeviertstrich im Titel wurde dadurch
  in jedem Betrachter als „Œ“ angezeigt. Sie werden nun als UTF-16BE mit Byte-Order-Mark geschrieben.
- **Gemessene statt geschätzter Breiten.** Der Schreiber maß Courier-Text mit Helvetica-Breiten und
  benutzte für alle Zeichen außerhalb ASCII pauschal 600 Einheiten — für Geviertstrich und Auslassung
  (je 1000) zu schmal, sodass rechtsbündige Zeilen den Satzspiegel um bis zu 15 pt überschritten
  und umbrochene Zeilen zu spät umbrachen. Gemessen wird jetzt der tatsächlich gezeichnete Byte-Strom
  mit den echten Breiten je Schriftschnitt.
- **Ein Vorhaben, eine Prüfziffer.** Das Siegel deckte die Projektfelder ab, und die wurden an den
  beiden Aufrufstellen unterschiedlich gefüllt: fehlende Angaben waren in der Kommandozeile `null`
  und in der Desktop-Schicht `undefined`, und `JSON.stringify` behält das eine und verwirft das
  andere. Dieselbe Plandatei trug dadurch je nach Weg zwei verschiedene Prüfziffern. Der Hash
  entfernt jetzt nicht gefüllte Felder und sortiert Schlüssel, sodass er nur von den Tatsachen
  abhängt; Kommandozeile und Desktop erzeugen für dasselbe Vorhaben dasselbe Dokument.
- **Der Projektname aus dem Plan gewinnt gegen den Dateinamen.** Die Ingestion beschriftet ein
  Vorhaben mit seinem Dateinamen, und dieser Name stand bisher im Kopf des Befunds: das Dossier zu
  „Am Hang 3“ hieß „haus.dxf“. Der im Plan angegebene Name wird nun übernommen, der Dateiname dient
  nur als Rückfall für Pläne ohne eigene Bezeichnung.
- **Gewahrte Grenzabstände wurden rot gezeichnet.** Das Lageplan-Blatt verglich jeden Abstand mit dem
  Vergleichswert 3,5 m, aber mit umgekehrter Richtung: 4,0 m Abstand — also gewahrt, wie es die
  Befundtabelle prüft (`ist < erforderlich` ist der Mangel) — wurden rot gezeichnet, ein
  unterschrittener Abstand von 2,5 m grün. In einem Einreichungsordner ist ein Blatt, das geprüfte
  Abstände rot malt, während der Befund „ok“ sagt, schlimmer als keine Farbe. Die Richtung ist
  gedreht, das Maß auf dem Vergleichswert gilt wie in der Regel als gewahrt, und ein Test hält beides
  fest.
- **Das Bündel verwarf mehrzeilige Import-Listen.** Muster im Bundler lasen die Namen zwischen den
  geschweiften Klammern mit `.`, das keinen Zeilenumbruch trifft; ein über mehrere Zeilen geschriebener
  Import — gültiges ESM — scheiterte als „unlesbar“. Die Muster erfassen jetzt auch Zeilenumbrüche.
- **`desktop/README.md`** beschreibt den PDF-Export statt der früheren Annahme, ein PDF sei
  ausschließlich über den Druckdialog möglich.## [0.9.0] — 2026-10-05

Der Meilenstein trennt die beiden Säulen: Die Website verkauft und erklärt, die Desktop-Anwendung
arbeitet. Das offene Web-Studio ist aus der Navigation verschwunden, an seine Stelle treten ein
Download-Hub und eine Showcase-Animation.

### Hinzugefügt

- **Desktop-Workhorse statt Einzelvorhaben-Studio** (`desktop/workhorse.html`). Drei-Spalten-Layout
  nach VS-Code-Vorbild: Projekt-Explorer mit Status-Ampel je Vorhaben, Center-Stage mit Modell,
  Inspector mit Verdikt, Kennzahlen, Mängelliste und Export. Szenario A (eine per Drag & Drop
  eingeworfene Datei startet direkt in der Detailansicht) und Szenario B (Ordner öffnen indiziert
  den Buchordner rekursiv) sind beide umgesetzt. Ohne verbundene Engine läuft die Seite in einem
  ausgewiesenen Demo-Modus weiter.
- **Interaktives 3D/2D-Modell** (`studio.html`, `desktop/workhorse.html`). Die feste 30-Grad-Isometrie
  ist durch eine echte Kamera ersetzt: Orbit per linker Maustaste, Pan per rechter oder mittlerer
  Maustaste, stufenloser Zoom per Mausrad, Presets auf `1` (Isometrie Süd-West), `2` (Draufsicht),
  `3` (Nord-Ansicht) und `R` (zurück). Ein Mangel in der Liste dreht die Kamera auf die betroffene
  Fassade, die verletzte Abstandsfläche pulsiert rot, unkritische bleiben transparent.
- **Download-Hub** (`downloads/index.html`, erzeugt aus `site.config.json`). Automatische
  Betriebssystem-Erkennung, direkte Verweise auf die signierten Dateien für Windows, macOS (universell
  für Apple Silicon und Intel), Linux (AppImage und Debian-Paket) sowie die eigenständigen CLI-Binaries
  und die Engine als Skript. Dazu Signaturhinweis, Prüfsummen-Anleitung und Systemvoraussetzungen.
- **Interaktive Showcase-Animation** im Hero-Bereich von `index.html`. Ein Baukörper wird vermessen,
  ein Laser tastet die vier Fassaden ab, die verletzte Abstandsfläche blinkt rot, der
  Heilungsvorschlag verschiebt den Baukörper um 45 cm und die Ampel springt auf Grün; der
  Prüfbericht-Stempel trägt eine deterministische Prüfziffer. Ein Schieberegler wechselt zwischen
  Bayern, Nordrhein-Westfalen und Wien und ändert damit die Abstandsflächenformel unmittelbar.
- **Batch-Prüfungen über Ordner.** `src/ingest.js` bildet Plandateien auf ein Spec ab (IFC, DXF,
  XPlanung-GML, B-Plan-GeoJSON, Textteil, Baubeschreibung, JSON-Spec) und meldet ausdrücklich, welche
  Werte geschätzt statt gemessen sind. Die neue IPC-Methode `check_file` prüft eine Datei, die
  Shell-Methoden `scan_workspace`, `pick_workspace_folder`, `read_workspace_file`,
  `write_workspace_file`, `batch_check` und `export_file` bedienen den Rest.

### Geändert

- **Neues Preismodell in Euro** (`site.config.json`): Starter 49 € einmalig als Projekt-Pass über
  30 Tage, Pro Architekt 89 € / Monat (890 € / Jahr), Büro / Kanzlei 199 € / Monat für drei
  Arbeitsplätze (1.990 € / Jahr), Enterprise ab 1.490 € / Monat. Preise werden an einer Stelle
  formatiert (`scripts/pricing-format.mjs`), damit Seite und Prüfung nie auseinanderlaufen.
- **Kein offenes Web-Studio mehr.** Alle Studio-Verweise in Navigation, Hero, Abschlussbereich,
  Footer und den erzeugten Rechts- und Preisseiten sind entfernt; `_redirects` leitet
  `/studio.html`, `/studio` und `/studio/` auf `/downloads/` um. `studio.html` bleibt im Repository,
  weil der Desktop-Build es als Engine-Ansicht einbindet.
- **Release-Pipeline für alle drei Plattformen** (`.github/workflows/release.yml`): eigener Build je
  Betriebssystem (NSIS, universelles DMG, AppImage und Deb), Prüfung, dass die auf der
  Download-Seite beworbenen Dateinamen tatsächlich entstehen, und eine aus den Artefakten erzeugte
  `SHA256SUMS.txt`.
- `scripts/build-desktop-assets.mjs` legt `desktop/workhorse.html` als `index.html` der Anwendung ab
  und bindet `desktop/inject/desktop.js` in beide Seiten ein.

### Entfernt

- USD-Preise, die Jahreslizenz-Bindung und die kostenlose Vollversion im Browser.

## [0.8.1] — 2026-10-05

### Hinzugefügt

- **Desktop-Integration & JSON-Line IPC-Protokoll** (`src/ipc.js`, `test/ipc.test.js`). Die Engine stellt ein robustes JSON-Line IPC-Protokoll (`ping`, `describe`, `check`) über `stdin` und `stdout` bereit (`baucheck --rpc`). Ermöglicht nativen Desktop-Shells (Tauri, Rust, Electron), die geprüfte JavaScript-Engine im Hintergrund als Subprozess anzusteuern, ohne eine Zweitimplementierung der Regeln zu benötigen.
- **Lokale Offline-Schriften.** IBM Plex Sans und IBM Plex Mono wurden direkt unter `assets/fonts/` lokal im Repository hinterlegt und per CSS eingebunden, sodass alle Seiten und das Desktop-Fenster 100 % offline ohne Internetverbindung funktionieren.
- **Top-Level-Await-Unterstützung im Bundler.** `scripts/bundle.js` wickelt asynchrone Initialisierungen und Top-Level-Await-Aufrufe nun sauber für den CommonJS-Build (`dist/baulens.cjs`) ab.
- 11 neue IPC-Tests integriert; die Testsuite umfasst nun 157 Testfälle (`node --test`).

## [0.8.0] — 2026-10-05

### Hinzugefügt

- **Geschärfte Landesbauordnungs-Profile.** Nordrhein-Westfalen (`NW`, § 6 BauO NRW), Hessen (`HE`, § 6 HBO), Sachsen (`SN`, § 6 SächsBO), Thüringen (`TH`, § 6 ThürBO) und Hamburg (`HH`, § 6 HBauO) wenden nun realitätsgetreu den Abstandsflächenfaktor **0,4 H** (mind. 3,0 m) sowie verifizierte Paragraphenketten an (Vertrauensstufe `hoch`). Niedersachsen (`NI`, § 5 NBauO) wendet 0,5 H an.
- **Kommunale Satzungen von 10 auf 20 Städte erweitert.** Neu aufgenommen: Hannover, Augsburg, Essen, Dortmund, Bonn, Münster (mit erhöhtem Fahrradstellplatzfaktor 3,0 je WE), Mannheim, Karlsruhe, Wiesbaden und Bremen in `src/municipalities.js`.
- **Grenzgaragen- und Nebengebäude-Freistellungsprivileg** (§ 6 Abs. 8 MBO / Art. 6 Abs. 7 BayBO / § 6 Abs. 8 BauO NRW). Garagen und Carports an der Grundstücksgrenze bis 3,0 m Wandhöhe und 9,0 m Grenzlänge (max. 15 m insgesamt) sind nun von regulären Abstandsflächen freigestellt.
- **Dachgeschoss-Vollgeschossprüfung** nach 2/3- und 3/4-LBO-Regelung: Erreicht die lichte Fläche über 2,30 m mindestens 67 % des darunterliegenden Geschosses, erfolgt ein Warnhinweis bezüglich B-Plan-Vollgeschosshöchstgrenzen.
- Gesetzestexte für BauO NRW, HBO, NBauO und HBauO in die Wissensdatenbank und den Paragraphen-Drawer (`src/statutes.js`) integriert.

## [0.7.0] — 2026-10-02

### Hinzugefügt

- **Rechtsseiten.** Impressum (§ 5 DDG), Datenschutzerklärung (Art. 13/14 DSGVO), AGB,
  Lizenzvereinbarung und Widerrufsbelehrung inklusive der Besonderheiten bei digitalen Inhalten
  (§ 356 Abs. 5 BGB). Alle Seiten lesen die Anbieterangaben aus `site.config.json`, sodass eine
  Adressänderung überall zugleich wirkt.
- **Preisseite und Checkout über Polar** (`pricing/index.html`). Polar tritt als Merchant of Record
  auf und übernimmt Zahlung, Rechnung und Umsatzsteuer; es gibt kein Backend und keinen geheimen
  Schlüssel im Repository. Die Umstellung auf Jahreslizenzen (Pro Architekt 1.499 $ pro Jahr,
  Büro/Kanzlei 2.750 $ pro Jahr und Arbeitsplatz) ist auf allen Oberflächen vollzogen.
- **Kaufbestätigung** (`checkout/success/index.html`) mit den nächsten Schritten und einer
  Anlaufstelle, falls die Lizenz-E-Mail ausbleibt.
- **Optionale Lizenzaktivierung im Studio.** Ein hinterlegter Schlüssel wird einmalig gegen die
  öffentliche Prüfschnittstelle von Polar validiert (`api.polar.sh/v1/customer-portal/license-keys/validate`,
  laut Polar-Dokumentation ohne Authentifizierung für öffentliche Clients vorgesehen) und lokal
  gespeichert. Ist Polar nicht erreichbar, bleibt eine frühere Lizenz gültig und die Prüf-Engine
  läuft unverändert weiter.
- `site.config.json` als einzige Quelle für Preise, Checkout-Links und Anbieterangaben, mit
  `scripts/build-site.mjs` als Generator, `scripts/check-links.mjs` und `scripts/check-prices.mjs`
  als Prüfungen (`npm run build`).
- `_redirects` für Cloudflare Pages mit den kanonischen Routen `/impressum`, `/datenschutz`,
  `/agb`, `/lizenz`, `/widerruf` und `/pricing`.
- **Dokumentationsbrowser `docs.html`.** Alle Markdown-Dokumente des Repositories lassen sich im
  Browser lesen, mit Navigationsleiste und automatisch erzeugtem Inhaltsverzeichnis. Über `http(s)`
  holt die Seite die jeweils aktuelle Fassung nach; beim Öffnen von der Festplatte greift eine
  eingebettete Kopie, weil `fetch()` auf `file://` blockiert ist. Erzeugt und geprüft über
  `scripts/build-docs.mjs` und `scripts/verify-docs.mjs` (`npm run docs:verify`).
- **Einheitliches Erscheinungsbild.** `studio.html` nutzt dieselben Design-Tokens wie die
  Produktseite: eine neutrale Flächenrampe, ein reservierter Akzent, IBM Plex Sans und Mono in zwei
  Rollen. Die Klassennamen und die Druckansicht des Dossiers blieben unverändert.

### Geändert

- Preise von Monats- auf Jahreslizenzen umgestellt und die widersprüchlichen Prüfungskontingente
  vereinheitlicht (die Produktseite nannte 20 Prüfungen im Monat, das Whitepaper 15).
- Produktseite, Whitepaper und README nennen dieselben Preise; der Preisblock der Startseite wird
  aus `site.config.json` erzeugt.
- `assets/banner.svg` auf dieselbe Palette umgestellt (vorher Amber, außerhalb des Farbsystems).

### Behoben

- **Studio-CSS referenzierte eine nie definierte Variable.** `.drawer-title` nutzte
  `var(--font-sans)`, das im Studio nicht existierte; der Titel fiel auf die Browser-Vorgabe
  zurück. Die Variable ist jetzt Teil des Token-Blocks.
- `src/version.js` als einzige Quelle für Produktname und Version. CLI-Kopf, Dossier, BCF-Export
  und XBau-XML lesen die Version aus `package.json`, statt drei verschiedene Werte zu führen.## [0.6.0] — 2026-10-02

### Hinzugefügt

- **Digitaler Bauantrags-Assistent (XBau).** Amtliche XML-Antragsdatei nach XBau 2.3 und
  Onlinezugangsgesetz (`baugenehmigung.antrag.0200`) in `src/xbau.js`, inklusive Bauantragsformular,
  Baubeschreibung und Antrag auf Abweichung oder Befreiung nach §67 MBO, Art. 63 BayBO bzw.
  §31 Abs. 2 BauGB. Ausgabe über `--xbau antrag.xml` oder im Studio-Tab.
- **Compliance Repair Engine** (`src/repair.js`, `--repair`). Drei Lösungswege bei festgestellten
  Abweichungen: geometrische Verschiebung des Baukörpers innerhalb zulässiger Pufferzonen, bauliche
  Anpassung der Trauf- und Wandhöhe, sowie eine formulierte rechtliche Begründung für den
  Befreiungsantrag.
- **BIM-Schnittstelle (BCF 2.1)** in `src/bcf.js`. Mängel und Auflagen werden als BCF-Themen mit
  3D-Kollisionskoordinaten exportiert (`--bcf issues.bcfjson`) und im Studio visualisiert.
  Vorgesehen für Revit, ArchiCAD und Vectorworks.

## [0.5.0] — 2026-10-02

### Hinzugefügt

- **Bebauungsplan-Parser** (`src/bplanparser.js`) für drei Eingabeformate:
  - **XPlanung GML** (Standard 5.x / 6.0) mit Baugebieten (`WA`, `WR`, `MI`, `GE`), Festsetzungen zu
    GRZ, GFZ, Vollgeschossen, First- und Traufhöhe, Bauweise und Baugrenze (`BP_Baugrenze`)
  - **GeoJSON** für Vektor-Baugrenzen und Attribute
  - **Textteil-Heuristik** für amtliche Textfestsetzungen
- **Geometrische Baugrenzenprüfung** (`checkBuildingWithinBaugrenze`) per 2D-Ray-Casting und
  Segment-Intersection: liegt der Gebäudeumriss vollständig innerhalb der festgesetzten Baugrenze
  (§23 Abs. 3 BauNVO)?
- **Festsetzungs-Prüfmodul** `bplan_festsetzungen` für First- und Traufhöhenüberschreitung
  (§9 Abs. 1 Nr. 1 BauGB, §16/§18 BauNVO), Vollgeschoss-Obergrenze und Dachneigungs-Korridor.
- **Katastermodell** mit Gemarkung, Flur, Flurstücksnummer, Katasterfläche und Grenzpunkten.
- **CLI:** `--bplan bplan.gml` gleicht den Entwurf automatisch gegen den Bebauungsplan ab.
- **Studio:** neuer B-Plan-Tab mit Drag-and-Drop für XPlanung und Festsetzungs-Ampel.

## [0.4.1] — 2026-10-02

### Hinzugefügt

- **`brandwand`** — grenzständige Wände unter 2,50 m Abstand zur Grundstücksgrenze, Anforderung
  feuerbeständiger Gebäudeabschlusswände (REI 90-M / EI 90). §30 MBO, OIB RL 2, VKF.
- **`barrierefreiheit`** — Aufzugspflicht ab vier Vollgeschossen oder über 13,0 m Gebäudehöhe,
  stufenloser Zugang, maximale Rampenneigung von 6,0 %. §50 MBO, OIB RL 4, SIA 500, DIN 18040-2.
- **`schallschutz`** — Grenzabstand für Luft-/Wasser-Wärmepumpen im Außenbereich, Lärmpegelbereiche
  I–VII zur Bemessung der Schalldämmung von Außenbauteilen. TA Lärm, DIN 4109.

## [0.4.0] — 2026-10-02

### Hinzugefügt

- **Vollständige DACH-Expansion** (`src/dach.js`):
  - **Deutschland** — MBO plus 16 Landesbauordnungen und Kommunalsatzungen
  - **Österreich** — harmonisierte OIB-Richtlinien 1–6 (Ausgabe 2023) und neun Bundesländer, darunter
    Wien (Bauwich und Bauklassen I–VI), Niederösterreich, Salzburg, Tirol und die Steiermark
  - **Schweiz** — IVHB-Konkordat mit den Leitkantonen Zürich, Bern, Basel-Stadt, Luzern, St. Gallen
    und Genf, ergänzt um SIA 416 und SIA 500
- **DACH-Radar** über 34 Profile gleichzeitig, filterbar nach Land (`--country DE|AT|CH`, `--dach`).
- **Behördenkonformes DIN-A4-Dossier** (`src/dossier.js`) mit national angepasster Anschrift,
  Vektor-CAD-Lageplan als SVG und deterministischem SHA-256-Prüfsiegel.
- **Interaktiver Paragraphen-Drawer** mit verbatimten Gesetzestexten, Bauamt-Kommentierung und
  Abweichungswegen für OIB-RL 2/3, Wiener BO §79, IVHB Art. 6, PBG ZH §293 und SIA 500
  (`src/statutes.js`).

## [0.3.2] — 2026-10-02

### Hinzugefügt

- Interaktiver Rechts-Explorer mit verbatimten Statustexten und Handlungsleitfaden für das Bauamt.

## [0.3.1] — 2026-10-02

### Hinzugefügt

- Quervergleichs-Matrix über alle Bundeslandprofile (`src/matrix.js`).

## [0.3.0] — 2026-10-02

### Hinzugefügt

- **Kommunale Satzungen** für zehn Städte (München, Nürnberg, Berlin, Hamburg, Köln, Düsseldorf,
  Frankfurt, Stuttgart, Leipzig, Dresden) inklusive Stellplatz-, Gründach- und Milieuschutzvorgaben
  (`src/municipalities.js`, `--stadt`).
- **3D-Abstandsflächen-Viewer** im Studio mit isometrischer Baumassen-Darstellung, dynamischer
  Hüllkurve und Dachform.
- **`gruendach`** — Gründachpflicht bei Flachdächern bis 15° aus Kommunalsatzungen.
- **`stellplaetze`** — Kfz- und Fahrradstellplätze mit kommunalen Freistellungen.
- **`gfz`** — Geschossflächenzahl gegen das zulässige Höchstmaß (§20 BauNVO).
- **Baubeschreibungs-Parser** (`src/textparser.js`) für deutsche Exposés und DIN-277-Flächen.
- **BIM- und CAD-Ingestion** (`src/cadparser.js`) für IFC 2x3/IFC4 und DXF.

## [0.2.0]

### Hinzugefügt

- **Landesprofile** (`src/profiles.js`) für alle 16 Bundesländer plus MBO-Referenz, mit
  hinterlegten Strukturabweichungen für Bayern, Baden-Württemberg und Berlin. Alle übrigen Länder
  laufen auf als „MBO-ähnlich (default)" gekennzeichneten Werten.
- **`grenzbebauung`** — Zulässigkeit von Bauen bis zur Grundstücksgrenze, aktiv in Bayern und Berlin.
- Regelwahl über `--rules`, Markdown-Bericht über `--report`.

## [0.1.0]

### Hinzugefügt

- Deterministischer Regelkern über strukturierte JSON-Eingabe: `gebaeudeklasse`, `abstandsflaeche`,
  `grz`, `tageslicht`, `raumhoehen`, `rettungswege`, `vollgeschosse`.
- CLI mit den Exit-Codes 0 (bestanden), 1 (nicht bestanden) und 2 (Bedienungsfehler).
- Markdown-Bericht mit Ist-Wert, Soll-Wert und Normzitat je Regel.
- Testsuite auf Basis des Node-Test-Runners, keine externen Abhängigkeiten.

---

[0.6.0]: https://github.com/KELLERBABG/BauLens/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/KELLERBABG/BauLens/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/KELLERBABG/BauLens/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/KELLERBABG/BauLens/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/KELLERBABG/BauLens/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/KELLERBABG/BauLens/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/KELLERBABG/BauLens/compare/v0.2.0...v0.3.0
[0.7.0]: https://github.com/KELLERBABG/BauLens/releases/tag/v0.7.0
[0.8.0]: https://github.com/KELLERBABG/BauLens/releases/tag/v0.8.0
[0.8.1]: https://github.com/KELLERBABG/BauLens/releases/tag/v0.8.1
[0.9.0]: https://github.com/KELLERBABG/BauLens/releases/tag/v0.9.0
[Unveröffentlicht]: https://github.com/KELLERBABG/BauLens/compare/v0.10.0...HEAD
[0.10.0]: https://github.com/KELLERBABG/BauLens/releases/tag/v0.10.0
