# BauLens

**Deterministische Bauordnungs-Vorprüfung für Deutschland, Österreich und die Schweiz.**

BauLens prüft ein Bauvorhaben gegen die tatsächlich geltenden Regelwerke — Musterbauordnung,
Landesbauordnungen, OIB-Richtlinien, IVHB-Kantonsrecht, BauNVO und kommunale Satzungen — und
liefert einen Befund mit Paragraphennachweis, bevor der Antrag beim Bauamt liegt.

Die Engine ist ein Regelwerk, kein Sprachmodell. Jede Aussage folgt einer dokumentierten Formel mit
Fundstelle und benennt, wo vereinfacht wurde. Derselbe Input ergibt immer denselben Befund.

> **Einordnung:** Vorprüfung, keine Rechtsberatung. Der Befund ersetzt weder die Prüfung durch eine
> bauvorlageberechtigte Person noch den Genehmigungsbescheid der Bauaufsichtsbehörde. Verbindlich ist
> immer der aktuelle Gesetzestext.

---

## Das Problem

Ein formal fehlerhafter Bauantrag wird nicht abgelehnt, sondern zurückgestellt. Jede Nachforderung
des Bauamts — eine übersehene Abstandsfläche, ein falsch gezähltes Vollgeschoss, eine
GRZ-Überschreitung — kostet Wochen Stillstand und Nacharbeit im Planungsbüro. Gefunden wird der
Fehler heute meist erst nach Einreichung, aus dem Bescheid.

BauLens zieht diese Prüfung in das Büro vor: Entwurfsdaten hinein, Befund mit Paragraphennachweis
heraus, in Sekunden statt Wochen.

## Was das Werkzeug prüft

**Prüfmodule.** 15 module, gruppiert in neun Kernregeln und sechs Zusatzmodule. Ein Modul ist aktiv,
wenn das gewählte Profil es vorsieht.

| Modul | Prüfinhalt | Fundstelle |
|---|---|---|
| `gebaeudeklasse` | Einordnung in Gebäudeklassen 1–5 aus Höhe, Geschossen und Einheiten | §2 MBO |
| `abstandsflaeche` | Erforderlicher Grenzabstand je Himmelsrichtung | §6 Abs. 2 MBO bzw. LBO |
| `grz` | Grundflächenzahl gegen den B-Plan-Richtwert | §19 BauNVO |
| `gfz` | Geschossflächenzahl gegen das zulässige Höchstmaß | §20 BauNVO |
| `stellplaetze` | Kfz- und Fahrradstellplätze inkl. kommunaler Freistellungen | Landesbauordnung, Satzung |
| `tageslicht` | Fensterfläche je Aufenthaltsraum | §49 MBO |
| `raumhoehen` | Lichte Raumhöhe (2,50 m Soll, 2,20 m Untergrenze) | §46 MBO |
| `rettungswege` | Zweiter Rettungsweg und notwendiger Treppenraum | §11, §35 MBO |
| `vollgeschosse` | Plausibilität von Höhe und Geschosszahl | §16 BauNVO |
| `grenzbebauung` | Zulässigkeit von Bauen bis zur Grenze (nur BY, BE) | §22 BayBO, BauO Bln |
| `gruendach` | Gründachpflicht bei Flachdächern bis 15° | Kommunale Satzung |
| `brandwand` | Grenzständige Wände unter 2,50 m Abstand, REI 90-M | §30 MBO, OIB RL 2 |
| `barrierefreiheit` | Aufzugspflicht, stufenloser Zugang, Rampenneigung | §50 MBO, OIB RL 4, SIA 500 |
| `schallschutz` | Grenzabstand für Wärmepumpen, Lärmpegelbereiche I–VII | TA Lärm, DIN 4109 |
| `bplan_festsetzungen` | First-/Traufhöhe, Vollgeschoss-Obergrenze, Dachneigungskorridor | §9 BauGB, §16/§18 BauNVO |

Zusätzlich prüft `bplan_festsetzungen` geometrisch, ob der Gebäudeumriss vollständig innerhalb der
festgesetzten Baugrenze liegt (2D-Ray-Casting und Segment-Intersection).

**Verdikte und Exit-Codes.**

| Verdikt | Bedeutung | Exit-Code |
|---|---|---|
| `bestanden` | Keine Abweichung | 0 |
| `mit Auflagen` | Warnungen, im Verfahren regelbar | 0 |
| `nicht bestanden` | Mindestens ein Fehler | 1 |
| — | Bedienungs- oder Eingabefehler | 2 |

## Regelprofile

34 Profile in drei Ebenen:

- **Deutschland — 17 Profile.** Musterbauordnung als Referenz plus alle 16 Landesbauordnungen.
  Bayern (`BayBO`), Baden-Württemberg (`LBO BW`) und Berlin (`BauO Bln`) haben hinterlegte
  Strukturabweichungen; alle übrigen Länder laufen auf gekennzeichneten MBO-Defaults.
- **Österreich — 11 Profile.** Die harmonisierten OIB-Richtlinien 1–6 (Ausgabe 2023) plus neun
  Bundesländer, darunter Wien (`BO für Wien`, Bauwich und Bauklassen I–VI), Niederösterreich,
  Salzburg, Tirol und die Steiermark.
- **Schweiz — 6 Profile.** IVHB-Konkordat plus Leitkantone Zürich (`PBG ZH`), Bern, Basel-Stadt,
  Luzern, St. Gallen und Genf, ergänzt um SIA 416 und SIA 500.

**Kommunen.** Zehn Städte mit hinterlegten Satzungen: München, Nürnberg, Berlin, Hamburg, Köln,
Düsseldorf, Frankfurt, Stuttgart, Leipzig, Dresden — Stellplatz-, Gründach- und Milieuschutzvorgaben.

Ein Profil kann Parameter setzen, Paragraphenzitate umstellen, Module abschalten oder Zusatzmodule
aktivieren. Jeder Wert, der aus Sekundärliteratur stammt oder vereinfacht wurde, ist mit
`[prüfen]` gekennzeichnet. Details im [Regelkatalog](docs/RULES.md).

## Datenübernahme

BauLens liest vorhandene Planungsunterlagen, statt Parameter manuell abzufragen:

- **IFC (2x3 / IFC4)** — Raumbuch, Geschossigkeit, Brutto-Grundfläche, Wandstärken
- **DXF** — Layer, Grenzkoordinaten und Gebäudeumriss aus 2D-Lageplänen
- **XPlanung GML (5.x / 6.0)** — amtliche Bebauungspläne: Baugebiete, GRZ, GFZ, Vollgeschosse,
  First- und Traufhöhen, Baugrenzen
- **B-Plan GeoJSON** und **Textteil** — Vektorgrenzen bzw. amtliche Textfestsetzungen
- **Baubeschreibung** — deutsche Exposés und DIN-277-Flächenberechnungen als Text

## Oberfläche: Web und Desktop

BauLens hat zwei Säulen, und sie tun unterschiedliche Dinge.

**Die Website** ([lens.kellersystems.dev](https://lens.kellersystems.dev/)) erklärt das Verfahren, zeigt eine
interaktive Showcase-Simulation im Hero-Bereich und führt auf den **Download-Hub**
([`downloads/`](downloads/)): Betriebssystem-Erkennung, direkte Verweise auf die signierten Dateien
für Windows, macOS und Linux, Prüfsummen- und Signaturhinweise. Ein unlimitiertes Prüfstudio im
Browser gibt es dort **nicht** mehr — Plandaten gehören nicht in ein fremdes Rechenzentrum, und die
volle Prüfung ist keine Browser-Anwendung.

**Die Desktop-Anwendung** ist als Workhorse gebaut, nicht als Einzelformular:

- **Multi-Projekt-Workspace.** *Projektordner öffnen* indiziert den Büroordner rekursiv nach `.ifc`,
  `.dxf`, `.gml`, `.json` und `.bcf`; jedes Unterverzeichnis wird als eigenes Vorhaben mit
  Status-Ampel geführt, und ein Stapellauf prüft alle auf einmal.
- **Ad-hoc-Einzelprüfung.** Eine per Drag & Drop eingeworfene Datei startet direkt in der
  Detailansicht — ohne Projektordner, ohne Anlegen.
- **Interaktives 3D/2D-Modell.** Orbit per linker Maustaste, Pan per rechter oder mittlerer
  Maustaste, Zoom per Mausrad; `1` Isometrie Süd-West, `2` Draufsicht, `3` Nord-Ansicht,
  `R` zurücksetzen. Ein Klick auf einen Mangel dreht die Kamera auf die betroffene Fassade.
- **14 Tage ohne Schlüssel, dann Lizenz.** Der volle Umfang läuft ab dem ersten Start 14 Tage
  lang; der Startzeitpunkt liegt im eigenen Konfigurationsverzeichnis der Anwendung. Danach
  verlangt ein Bildschirmschirm einen Polar-Lizenzschlüssel — Aktivieren oder Kaufen. Ist die
  Testphase abgelaufen und kein brauchbarer Schlüssel hinterlegt, bleibt die Arbeitsfläche
  gesperrt; Projekte und Plandaten bleiben erhalten.

Die Desktop-Anwendung bindet dieselbe geprüfte Engine als Kindprozess an (`--rpc`, siehe
`src/ipc.js`); die Regeln existieren genau einmal.

### Dateien einlesen

`src/ingest.js` bildet Plandateien auf ein Spec ab: IFC, DXF, XPlanung-GML, B-Plan-GeoJSON,
B-Plan-Textteil, deutsche Baubeschreibungen und JSON-Specs. Jede Schätzung wird im Befund
ausgewiesen — ein Wert aus dem Grundstücksumriss ist kein gemessener Wert. Über die IPC-Methode
`check_file` prüft die Desktop-Anwendung damit ganze Ordner.

## Ausgabe

- **DIN-A4-Prüfdossier als PDF-Datei** — echte PDF-Datei, im Desktop-Workhorse per Schaltfläche
  „PDF“ über den nativen Speichern-Dialog abzulegen, ohne Druckdialog; auf der Kommandozeile mit
  `--pdf befund.pdf`. Kopf und Anschrift adaptieren die nationale Behördenstruktur (Untere
  Bauaufsichtsbehörde, MA 37 Baupolizei, Amt für Baubewilligungen), dazu ein eigenes
  **Lageplan-Blatt im Querformat** — Blattrahmen mit Schriftfeld, Maßketten in Metern an allen vier
  Seiten, Maßstabsbalken, Legende, Nordpfeil und angegebener Maßstab (1:100 bis 1:1000). Das
  Dossier ist nach Blättern geordnet wie eine Plansammlung: eine **Blattübersicht** auf der ersten
  Seite nennt jedes Blatt, jede Fußzeile liest sich `Blatt 2/3 · Lageplan · Seite 3 von 4`, sodass
  ein einzelnes Blatt sich selbst zuordnet. Der **Hauskörper** des Lageplans kommt aus den Angaben
  des Vorhabens — `GRZ × Grundstücksfläche` bzw. `Nutzfläche / Geschosse`, in der Legende
  nachgewiesen — und nicht aus einem erfundenen Rechteck. Dazu ein deterministisches
  SHA-256-Prüfsiegel über Spec und Befund; dieselbe Prüfziffer trägt die HTML-Fassung. Der Renderer ist Teil der Engine (`src/dossierpdf.js`, eigener PDF-Schreiber in
  `src/pdf.js`, keine Fremdbibliothek); die HTML-Fassung bleibt zum Drucken erhalten
- **Lageplan-Blatt als DXF und SVG** — dasselbe Blatt, das im Dossier liegt, verlässt die Mappe auch
  als eigene Datei: `--svg lageplan.svg` für Plotter und Planwerk, `--dxf lageplan.dxf` für CAD, in
  der Shell die Schaltflächen „Lageplan als SVG/DXF speichern“ (IPC: `plan_svg`, `plan_dxf`). Beide
  werden aus **derselben Aufzeichnung** geschrieben, die die PDF-Seite abspielt — ein zweiter
  Zeichenweg wäre eine zweite Zeichnung. Das **DXF** ist ein konservatives ASCII-DXF im
  AutoCAD-2000-Dialekt: eine Zeichnungseinheit ist ein Meter (`$INSUNITS 6`), Rechtecke sind
  geschlossene `LWPOLYLINE`, Beschriftungen bleiben `TEXT`, und jeder Bauteil liegt auf einer eigenen
  Ebene (`GRENZE`, `GEBAEUDE`, `ABSTANDSFLAECHE`, `MASZKETTE`, `MASSZAHL`, `NORDEN`) — Blattrahmen,
  Blattkopf, Legende, Maßstabsbalken und Schriftfeld auf `BLATTRAHMEN`, `BLATTKOPF`, `LEGENDE`,
  `MASSSTAB`, `SCHRIFTFELD`, damit ein Büro sie einfrieren und das Blatt in einen eigenen Plansatz
  einfügen kann. Das **SVG** ist DIN A4 quer in Punkten (`width="841.89pt"`, gleiche viewBox), mit
  einer Gruppe je Ebene, ohne eingebettete Schrift und ohne externe Verweise. Beide nennen Maßstab
  und Prüfziffer, damit sich die Zeichnung dem geprüften Befund zuordnen lässt.
- **Lagepläne eines ganzen Projektordners in einem Durchgang.** Im Desktop-Workhorse schreibt
  „Alle Lagepläne exportieren“ je Vorhaben des geöffneten Ordners ein Blatt als DXF **und** SVG in
  einen gewählten Zielordner, benannt nach dem Vorhaben (`Wohnanlage-Musterweg_Lageplan.dxf`); die
  Engine-Methode heißt `plan_export`, und die Kommandozeile exportiert einzelne Blätter über
  `--dxf`/`--svg`. Ein unlesbarer Plan nimmt die übrigen nicht mit (er wird mit seinem Namen
  gemeldet, der Rest wird geschrieben), ein fehlendes Zielverzeichnis wird nie erfunden, und zwei
  Vorhaben mit gleichem Namen bekommen getrennte Blätter — umbenannt und im Bericht genannt, statt
  einander stillschweigend zu überschreiben. Der Name kommt aus einem Verzeichnisnamen, also aus
  einer fremden Festplatte: Trennzeichen, Laufwerksbuchstaben und `..` werden entschärft, damit kein
  Blatt den gewählten Ordner verlässt.
- **XBau 2.3 XML** — amtlicher Bauantragsdatensatz nach Onlinezugangsgesetz
  (`baugenehmigung.antrag.0200`), inkl. Formularen für Abweichung und Befreiung nach §67 MBO,
  Art. 63 BayBO bzw. §31 Abs. 2 BauGB
- **BCF 2.1** — Mängel als BIM-Themen mit 3D-Koordinaten für Revit, ArchiCAD und Vectorworks
- **Markdown-Bericht** — Befund mit Ist-Wert, Soll-Wert und Normzitat je Zeile
- **DACH-Quervergleich** — derselbe Entwurf über alle 34 Profile, filterbar nach Land

## Compliance Repair Engine

Bei festgestellten Abweichungen berechnet `--repair` drei Lösungswege:

- **Geometrisch** — erforderliche Baukörperverschiebung innerhalb zulässiger Pufferzonen
- **Baulich** — maximal zulässige Trauf- und Wandhöhe zur Einhaltung der Abstandsfläche
- **Rechtlich** — vollständig formulierte Begründung für einen Befreiungsantrag

## Erste Schritte

Voraussetzung: Node.js ≥ 18. Keine externen Abhängigkeiten — ausschließlich Node-Standardbibliothek,
kein Installationsschritt, keine Telemetrie.

```powershell
node bin/baucheck.js --help                 # Befehlsübersicht
node bin/baucheck.js --example              # gültiges Beispiel-Spec ausgeben
node bin/baucheck.js check spec.json        # prüfen (Exit 0 / 1 / 2)
node bin/baucheck.js check spec.json --report bericht.md
node bin/baucheck.js check spec.json --pdf befund.pdf   # DIN-A4-Befund als PDF
node bin/baucheck.js check spec.json --dxf lageplan.dxf # Lageplan-Blatt für CAD (1 Einheit = 1 m)
node bin/baucheck.js check spec.json --svg lageplan.svg # Lageplan-Blatt für Plotter und Planwerk
```

**Profil wählen und grenzüberschreitend prüfen:**

```powershell
node bin/baucheck.js --lands                # alle 34 Profile als Tabelle
node bin/baucheck.js check spec.json --land BY          # BayBO erzwingen
node bin/baucheck.js check spec.json --land AT_W        # Wiener Bauordnung
node bin/baucheck.js check spec.json --land CH_ZH       # Zürcher PBG
node bin/baucheck.js check spec.json --dach             # Quervergleich DE/AT/CH
node bin/baucheck.js check spec.json --all-lands        # Quervergleich über alle Profile
node bin/baucheck.js --staedte              # unterstützte Kommunen
node bin/baucheck.js check spec.json --stadt muenchen
```

**Regeln einschränken und Planungsunterlagen einlesen:**

```powershell
node bin/baucheck.js check spec.json --rules tageslicht,raumhoehen
node bin/baucheck.js check spec.json --bplan bplan.gml
node bin/baucheck.js check spec.json --repair
node bin/baucheck.js check spec.json --xbau antrag.xml
node bin/baucheck.js check spec.json --bcf issues.bcfjson
node bin/baucheck.js check spec.json --server 3000
```

## Eingabeformat

Alle Felder sind optional, sofern sie für eine Regel nicht gebraucht werden. Fehlende Angaben
erzeugen eine Warnung, falsche Typen einen Fehler.

```jsonc
{
  "bundesland": "BY",                     // Profil, falls --land fehlt
  "grundstueck": {
    "flaeche_m2": 800,
    "grenzabstaende": { "norden": 5.0, "sueden": 4.0, "osten": 6.0, "westen": 3.5 }
  },
  "gebaeude": {
    "hoehe_m": 7.0,
    "geschosse_ueber_grund": 2,
    "geschosse_unter_grund": 1,
    "wohnungen_anzahl": 1,
    "nutzflaeche_m2": 180,
    "grz_ist": 0.35,
    "gfz_ist": 0.70
  },
  "aufenthaltsraeume": [
    { "name": "Wohnen", "flaeche_m2": 32, "fensterflaeche_m2": 6.5, "raumhoehe_m": 2.6, "etage": 0 }
  ],
  "zweiter_rettungsweg_vorhanden": true,
  "treppenraum_vorhanden": true,
  "einstellungen": { "grz_limit": 0.4, "gfz_limit": 1.2 }
}
```

Die Profilwahl folgt der Reihenfolge `--land` → `spec.bundesland` → `MBO`.

## Projektstruktur

```
bin/baucheck.js          CLI: Argumente, Berichtsausgabe, RPC-Modus
src/engine.js            Ablaufsteuerung der Prüfung
src/schema.js            Validierung und Normalisierung des Specs
src/ingest.js            Plandateien (IFC, DXF, GML, GeoJSON, Text, JSON) -> Spec
src/ipc.js               JSON-Zeilenprotokoll für die Desktop-Anwendung
src/rules/               15 Prüfmodule, je Datei eine Regel
src/profiles.js          34 DACH-Profile mit Overrides
src/municipalities.js    Kommunale Satzungen
src/statutes.js          Verbatimte Gesetzestexte für den §-Drawer
src/dach.js              Profile für Deutschland, Österreich, Schweiz
src/bplanparser.js       XPlanung GML, GeoJSON, Textteil
src/cadparser.js         DXF-Einlesen
src/textparser.js        Baubeschreibung als Text
src/dossier.js           DIN-A4-Dossier
src/xbau.js              XBau-2.3-XML
src/bcf.js               BCF-2.1-Export
src/repair.js            Compliance Repair Engine
src/matrix.js            DACH-Quervergleich
src/report.js            Markdown-Bericht
src/version.js           Produkt- und Versionsangabe aus package.json
scripts/build-docs.mjs   erzeugt docs.html aus den Markdown-Dateien
scripts/verify-docs.mjs  verifiziert die eingebettete Dokumentation
index.html               Produktseite mit Showcase-Simulation
downloads/index.html     Download-Hub mit OS-Erkennung (generiert)
studio.html              Engine-Ansicht (im Repo, vom Desktop-Build eingebunden)
docs.html                Dokumentationsbrowser
desktop/workhorse.html   Desktop-Workhorse: Multi-Projekt-Workspace
desktop/inject/desktop.js  Desktop-Verhalten, in die ausgelieferten Seiten injiziert
desktop/src-tauri/       Tauri-Shell: Ordnerdialog, Workspace-Scan, Batch-Prüfung
test/                    Testsuite und Fixtures
```

## Tests

```powershell
node --test
```

Die Suite umfasst 256 Testfälle und deckt Regelformeln, Schemavalidierung, Profil-Overrides,
kommunale Satzungen, den DACH-Quervergleich, CAD-/BIM-Ingestion, XBau-, BCF- und Dossier-Ausgabe,
das IPC-Line-Protokoll, die Datei-Ingestion (`check_file`) sowie reale Projektfälle ab.

`cli.test.js`, `matrix.test.js` und `profiles.test.js` starten das CLI als Kindprozess. In einer
Umgebung, die Kindprozesse mit Pipes blockiert, scheitern diese Dateien mit `EPERM`; das ist eine
Einschränkung der Laufzeitumgebung, kein Testfehler.

## Preise und Lizenzen

Starter 49 € einmalig (ein Vorhaben, 30 Tage), Pro Architekt 89 € pro Monat (890 € / Jahr),
Büro / Kanzlei 199 € pro Monat für drei Arbeitsplätze (1.990 € / Jahr), Enterprise ab 1.490 € pro
Monat. Alle Preise in Euro; die Desktop-Anwendung läuft 14 Tage ohne Schlüssel.

Die Preisübersicht liegt unter [pricing/](pricing/) und ist im Browser erreichbar. Die Zahlen stehen
in [`site.config.json`](site.config.json) — daraus werden die Produktseite, die Preisseite, die
Kaufbestätigung und die Rechtstexte erzeugt, damit keine Oberfläche eine andere Zahl zeigt.

Der Verkauf läuft über **Polar** als Merchant of Record. Polar ist Vertragspartner des Erwerbs,
stellt die Rechnung aus und führt die Umsatzsteuer ab; Kartendaten erreichen diese Anwendung nicht.
Ein eigener Server ist dafür nicht nötig — der Kauf ist ein Checkout-Link.

Die Desktop-Anwendung validiert den Lizenzschlüssel gegen die öffentliche, authentifizierungsfreie
Prüfschnittstelle von Polar (`POST /v1/customer-portal/license-keys/validate`). Gesendet wird nur
der Schlüssel — niemals Plandaten. Ein erfolgreich geprüfter Schlüssel gilt offline weiter
(30 Tage Grace-Fenster, danach eine erneute Prüfung), ein unbekannter Schlüssel wird abgewiesen,
ein erreichbarer Netzwerkfehler wird nicht mit einem ungültigen Schlüssel verwechselt. Die
Entscheidung tritt `src/licensing.js` (getestet, 22 Fällen in `test/licensing.test.js`); die
Anwendung speichert Startzeitpunkt und letzte Prüfung im eigenen Konfigurationsverzeichnis. Die
Prüf-Engine rechnet unabhängig davon vollständig lokal — die Regeln kennen keine Lizenz.

Wie Preise, Checkout und Rechtstexte zusammenhängen, beschreibt
[Zahlungen, Lizenzen und Rechtstexte](docs/PAYMENTS_AND_LEGAL.md).

```powershell
node scripts/build-site.mjs      # Rechts- und Preisseiten neu erzeugen
node scripts/check-links.mjs     # interne Verweise prüfen
node scripts/check-prices.mjs    # Preise gegen site.config.json prüfen
```

## Dokumentation

Die Dokumentation liegt als Markdown im Repository und ist zusätzlich als browsbare Oberfläche
verfügbar:

- **[docs.html](docs.html)** — Dokumentationsbrowser mit Navigationsleiste und Inhaltsverzeichnis
- [UPDATE_PLAN.md](UPDATE_PLAN.md) — der Transformationsplan, gegen den dieser Meilenstein gebaut wurde
- [Regelkatalog](docs/RULES.md) — jede Regel mit Formel, Fundstelle, Vereinfachung und
  Vertrauensstufe der hinterlegten Parameter
- [Whitepaper](docs/WHITEPAPER.md) — technischer Aufbau, Marktbild und Kommerzialisierung
- [Zahlungen und Rechtstexte](docs/PAYMENTS_AND_LEGAL.md) — Checkout, Lizenzaktivierung, Hosting
- [Änderungsverlauf](CHANGELOG.md) — Versionshistorie

Wird `docs.html` über `http(s)` ausgeliefert, holt der Browser die jeweils aktuelle Markdown-Fassung
nach. Beim Öffnen von der Festplatte greift eine eingebettete Kopie, weil `fetch()` auf `file://`
blockiert ist. Nach Änderungen an den Markdown-Dateien:

```powershell
node scripts/build-docs.mjs      # docs.html neu erzeugen
node scripts/verify-docs.mjs     # eingebettete Kopie gegen die Quelle prüfen
```

## Rechtliches

| Seite | Inhalt |
|---|---|
| [Impressum](legal/impressum/) | Anbieterkennzeichnung nach § 5 DDG |
| [Datenschutz](legal/datenschutz/) | Verarbeitung personenbezogener Daten, Hosting, Polar, Lizenzaktivierung |
| [AGB](legal/agb/) | Vertragsgegenstand, Preise, Haftung, Kündigung |
| [Lizenzvereinbarung](legal/lizenz/) | Nutzungsumfang der Software |
| [Widerrufsbelehrung](legal/widerruf/) | Widerrufsrecht und Erlöschen bei digitalen Inhalten |

> **Hinweis:** Die Regel-Engine erbringt eine technische Vorprüfung und keine Rechtsberatung im
> Sinne des RDG. Sie ersetzt weder die Prüfung durch eine bauvorlageberechtigte Person noch den
> Genehmigungsbescheid der Bauaufsichtsbehörde.

## Lizenz

MIT.
