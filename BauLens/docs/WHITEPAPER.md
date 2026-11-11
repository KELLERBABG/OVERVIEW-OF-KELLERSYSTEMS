# BauLens — Technisches Whitepaper

**Deterministische DACH-Bauordnungs-Vorprüfung**

| | |
|---|---|
| **Version** | 0.8.0 |
| **Regelprofil** | 34 Profile in DE, AT und CH |
| **Prüfmodule** | 15 (9 Kernmodule, 6 Zusatzmodule) |
| **Testsuite** | 157 Testfälle, Node-Test-Runner |
| **Laufzeit** | Node.js ≥ 18, keine externen Abhängigkeiten |
| **Zielgruppe** | Architektur- und Planungsbüros, Bauvorlageberechtigte, Fertighaushersteller, Bauaufsichtsbehörden im Pilotbetrieb |

---

## 1. Ausgangslage

In Deutschland, Österreich und der Schweiz ist das Baugenehmigungsverfahren der formal
fehleranfälligste Abschnitt eines Bauvorhabens. Ein Antrag wird selten abgelehnt — er wird
zurückgestellt. Jede Nachforderung des Bauamts kostet den Entwurfsverfasser Wochen Stillstand und
Nacharbeit an Unterlagen, die längst hätten geprüft sein können.

Typische Auslöser sind rechnerisch eindeutig: eine unterschrittene Abstandsfläche, ein zu hoch
gezähltes Vollgeschoss, eine GRZ- oder GFZ-Überschreitung, ein fehlender zweiter Rettungsweg,
eine Traufhöhe über der Festsetzung des Bebauungsplans.

Gefunden wird der Fehler heute meist erst nach Einreichung, aus dem Bescheid. Genau diese
Reihenfolge dreht BauLens um.

**Zwei Fehlerarten, zwei Antworten.**

| Fehlerart | Beispiel | Werkzeug |
|---|---|---|
| Rechenfehler | H/2 unterschritten, GRZ überschritten, Fensterfläche unter 1/8 | Regel-Engine mit dokumentierter Formel |
| Verfahrensfehler | Grenzbebauung ohne Planrecht, Befreiung nicht begründet | Hinweisbefund mit Fundstelle und Befreiungsweg |

Für die erste Kategorie ist ein Sprachmodell die falsche Technik: Es liefert bei gleicher Eingabe
unterschiedliche Ergebnisse und kann keine Fundstelle garantieren. Für die zweite Kategorie ist eine
deterministische Engine die falsche Technik: Sie kann keine Zulässigkeit behaupten, die vom
Planrecht abhängt. BauLens trennt beides — harte Befunde mit Zahl, Hinweise ohne.

---

## 2. Technische Architektur

### 2.1 Schichten

```
Eingabe          JSON-Spec · DXF · IFC · XPlanung GML · GeoJSON · Text-Baubeschreibung
                        │
Normalisierung   src/schema.js         Validierung, Typprüfung, Defaults
                        │
Profilauflösung  src/profiles.js       --land → spec.bundesland → MBO
                 src/municipalities.js kommunale Satzungen
                 src/dach.js           DE / AT / CH
                        │
Bewertung        src/engine.js         9 Kernmodule + 6 Zusatzmodule
                 src/rules/*.js        ein Modul je Regel, reine Funktion
                        │
Aggregation      src/report.js         Verdikt, Markdown-Bericht
                        │
Ausgabe          src/dossier.js        DIN-A4-Dossier, SHA-256-Siegel
                 src/xbau.js           XBau 2.3 XML nach OZG
                 src/bcf.js            BCF 2.1 Issues
                 src/repair.js         Lösungsvorschläge
                 src/matrix.js         DACH-Quervergleich
```

Jedes Regelmodul ist eine reine Funktion `evaluate(spec, ctx) → Finding[]`. Es gibt keinen
gemeinsamen Zustand, keine Reihenfolgeabhängigkeit und keine Seiteneffekte. Das macht Module
einzeln testbar und den Gesamtlauf reproduzierbar.

### 2.2 Garantien

- **Determinismus.** Gleicher Input, gleicher Befund. Keine Zufallskomponente, kein Modellaufruf.
- **Nullabhängigkeit.** Ausschließlich Node-Standardbibliothek. Kein `npm install`, keine
  Telemetrie, keine Netzwerkverbindung im Prüfpfad.
- **Nachvollziehbarkeit.** Jeder Befund nennt Ist-Wert, Soll-Wert und Fundstelle. Jeder
  vereinfachte Parameter ist als `[prüfen]` gekennzeichnet.
- **Lokale Verarbeitung.** Die Prüfung läuft in der Desktop-Anwendung auf dem Arbeitsplatz; es gibt
  keine Cloud-Komponente. Die Engine lässt sich über das JSON-Zeilenprotokoll (`--rpc`) auch hinter
  eine eigene Oberfläche oder in eine Pipeline hängen.

### 2.3 Regelumfang

**Kernmodule** laufen in jedem Profil:
`gebaeudeklasse`, `abstandsflaeche`, `grz`, `gfz`, `stellplaetze`, `tageslicht`, `raumhoehen`,
`rettungswege`, `vollgeschosse`.

**Zusatzmodule** werden profil- oder datenabhängig aktiviert:
`grenzbebauung` (BY, BE), `gruendach`, `brandwand`, `barrierefreiheit`, `schallschutz`,
`bplan_festsetzungen`.

Formeln, Schwellen, Vereinfachungen und Vertrauensstufen sind im
[Regelkatalog](RULES.md) dokumentiert — vollständig, inklusive der Stellen, an denen bewusst
vereinfacht wurde.

### 2.4 Profilmodell

34 Profile in drei Ebenen:

| Ebene | Umfang | Inhalt |
|---|---|---|
| Deutschland | 17 | MBO als Referenz plus 16 Landesbauordnungen; BY, BW, BE mit gesicherten Strukturabweichungen |
| Österreich | 11 | OIB-Richtlinien 1–6 (2023) plus neun Bundesländer |
| Schweiz | 6 | IVHB-Konkordat plus Leitkantone ZH, BE, BS, LU, SG, GE |
| Kommunen | 10 | Stellplatz-, Gründach- und Milieuschutzsatzungen |

Ein Profil kann vier Dinge verändern: Parameter setzen, Paragraphenzitate umstellen, Module
abschalten, Zusatzmodule aktivieren. Diese Struktur ist die Grundlage dafür, dass ein neues
Bundesland oder eine neue Satzung ohne Eingriff in die Engine ergänzt werden kann.

### 2.5 Datenübernahme

Der manuelle Parameterbogen ist die eigentliche Hürde im Alltag eines Planungsbüros. BauLens liest
deshalb vorhandene Unterlagen:

| Format | Quelle | Extrahiert |
|---|---|---|
| IFC 2x3 / IFC4 | BIM-Modell, LOD 200 | Raumbuch, Geschossigkeit, BGF, Wandstärken |
| DXF | 2D-Lageplan | Layer, Grenzkoordinaten, Gebäudeumriss |
| XPlanung GML 5.x/6.0 | amtlicher Bebauungsplan | Baugebiete, GRZ, GFZ, Z, First-/Traufhöhe, Baugrenze |
| GeoJSON | Vektor-Bebauungsplan | Baugrenzen und Attribute |
| Textteil | amtliche Textfestsetzungen | Festsetzungen per Heuristik |
| Baubeschreibung | Exposé, DIN-277-Flächenberechnung | Flächen, Höhen, Raumdaten |

### 2.6 Geometrische Prüfung

Die Baugrenzenprüfung ist der einzige Teil der Engine, der nicht auf skalierten Zahlen, sondern auf
Geometrie arbeitet. `checkBuildingWithinBaugrenze` kombiniert zwei Verfahren:

- **Ray-Casting** — liegt jeder Eckpunkt des Gebäudeumrisses innerhalb des Baugrenzenpolygons?
- **Segment-Intersection** — kreuzt eine Gebäudekante eine Grenzkante, ohne dass ein Eckpunkt
  außerhalb liegt? Dieser Fall ist bei konkaven Polygonen entscheidend und wird von reinem
  Punkt-in-Polygon nicht erfasst.

Das Ergebnis wird als Verstoß gegen §23 Abs. 3 BauNVO gemeldet, mit der Anzahl der ausgetretenen
Eckpunkte.

### 2.7 Ausgabe

| Artefakt | Format | Verwendung |
|---|---|---|
| Prüfdossier | A4-Dokument, Druckdialog → PDF | Nachweis gegenüber Bauherr, Gutachter, Bauamt |
| Prüfsiegel | SHA-256 über Spec und Befund | Nachweis, dass der Befund zu diesen Daten gehört |
| Beschlussvorlage | XBau 2.3 XML (OZG) | Einreichung bei der Bauaufsichtsbehörde |
| Mängelliste | BCF 2.1 | Übernahme in Revit, ArchiCAD, Vectorworks |
| Befund | Markdown | Ablage im Projektordner, Diff-fähig |
| Quervergleich | Tabelle über 34 Profile | Standortentscheidung, Serienprüfung |

Die Anschrift des Dossiers wird an die nationale Behördenstruktur angepasst — *Untere
Bauaufsichtsbehörde*, *Magistrat der Stadt Wien – MA 37 Baupolizei*, *Amt für Baubewilligungen
Stadt Zürich*.

### 2.8 Compliance Repair Engine

Ein Befund allein verlagert die Arbeit zum Planer zurück. `--repair` liefert zu jeder Abweichung
drei Wege:

- **Geometrisch** — erforderliche Verschiebung des Baukörpers innerhalb zulässiger Pufferzonen
  gegenüber der Grundstücksgrenze
- **Baulich** — maximal zulässige Trauf- und Wandhöhe zur Einhaltung der Abstandsfläche
- **Rechtlich** — formulierte Begründung für den Befreiungsantrag nach §31 Abs. 2 BauGB,
  §67 MBO bzw. Art. 63 BayBO

---

## 3. Betriebsmodell und Grenzen

### 3.1 Was BauLens nicht ist

- **Kein Sprachmodell über Gesetzestexten.** Es gibt keine generative Komponente im Prüfpfad.
- **Keine Rechtsberatung.** Der Befund ist eine technische Vorprüfung und erbringt keine
  Rechtsdienstleistung im Sinne des RDG.
- **Kein Ersatz für die bauvorlageberechtigte Person.** Das Dossier unterstützt die
  Qualitätssicherung; der Genehmigungsbescheid bleibt bei der Behörde.
- **Kein vollständiges Normabbild.** Was vereinfacht oder nicht modelliert ist, steht im
  Regelkatalog — sichtbar im Tool, nicht im Kleingedruckten.
- **Kein Cloud-Dienst.** Es gibt keinen Server, auf dem Plandaten liegen.

### 3.2 Haftung und Kennzeichnung

Jeder Bericht trägt auf jeder Seite den Hinweis:

> *„Dieser Prüfbericht dient ausschließlich der technischen und planerischen Vorprüfung auf Basis
> der bereitgestellten Daten. Er ersetzt weder die Prüfung durch eine bauvorlageberechtigte Person
> noch den formellen Genehmigungsbescheid der zuständigen Bauaufsichtsbehörde."*

### 3.3 Datenhaltung

Die Berechnung läuft clientseitig im Browser oder auf einem Server des Büros. Zeichnungen,
Raumdaten und Bauherrenangaben werden nicht an Dritte übertragen. Für Büros mit eigenen
IT-Vorgaben ist der Kern als lokaler REST-Dienst betreibbar (`--server`), der von
BIM-Plugin-Schnittstellen angesprochen wird.

---

## 4. Kommerzielles Modell

### 4.1 Pakete

Die Preise sind in `site.config.json` hinterlegt und werden aus dieser Quelle auf die Produktseite,
die Preisseite und in die Rechtstexte gerendert. Maßgeblich ist die
[Preisübersicht](../pricing/).

| Paket | Preis | Zielgruppe | Umfang |
|---|---|---|---|
| Testlizenz | kostenlos | Erstbewertung | Drei Projektprüfungen, alle 34 Profile, Befund mit Paragraphenzitat |
| Pro Architekt | 1.499 $ / Jahr | Einzelbüros | 180 Projektprüfungen pro Jahr, ein Arbeitsplatz, Dossier, CAD-/BIM-Ingestion, Reparaturvorschläge |
| Büro / Kanzlei | 2.750 $ / Jahr je Arbeitsplatz | Planungsbüros, Generalübernehmer | Unbegrenzte Prüfungen, eigenes Briefpapier, DACH-Quervergleich |
| Enterprise / Serien | 1.490 $ / Monat | Fertighaus- und Serienhersteller | Katalogprüfung über alle 34 Profile, API-Zugang, BCF-Anbindung |

Die Preise werden in US-Dollar geführt, weil die Produkte im Zahlungssystem in USD angelegt sind.
Die Umsatzsteuer ist enthalten; die Berechnung erfolgt im Checkout anhand der Rechnungsadresse.

**Abwicklung.** Der Verkauf läuft über Polar als **Merchant of Record**. Polar ist damit
Vertragspartner des Erwerbs, stellt die Rechnung aus und führt die Umsatzsteuer ab. Das erspart dem
Anbieter die Registrierung in jedem Kundenland und hält Kartendaten aus der eigenen Infrastruktur
heraus: Es gibt kein Backend, das Zahlungsdaten verarbeitet.

**Rechnung hinter dem Einstiegspreis.** Eine vermiedene Nachforderung entspricht zwei bis sechs
Wochen Projektverzug. Bereits eine einzelne vermiedene Verzögerung trägt die Jahresgebühr der
Pro-Lizenz. Der Vergleich auf der Produktseite stellt diesen Zusammenhang her, ohne eine
Einsparung zu behaupten, die sich nicht belegen lässt.

**Lizenzschlüssel.** Polar stellt für jeden Kauf einen Lizenzschlüssel aus und versendet ihn per
E-Mail. Die Desktop-Anwendung kann ihn optional gegen die öffentliche Prüfschnittstelle von Polar
validieren und das Ergebnis lokal speichern. Die Prüf-Engine funktioniert unabhängig davon; die Aktivierung ist
ein Komfortmerkmal, keine Zugangssperre. Details in
[Zahlungen, Lizenzen und Rechtstexte](PAYMENTS_AND_LEGAL.md).

### 4.2 Wirkung des Ausgabeformats

Das Dossier ist das Verkaufsargument, nicht der Befund. Ein Befund ist eine interne Notiz; ein
DIN-A4-Dokument mit Prüfsiegel und Behördenanschrift ist ein Arbeitsergebnis, das der Planer an
den Bauherrn weitergeben kann. Deshalb ist die Dossier-Ausgabe Teil jeder kostenpflichtigen Lizenz.

### 4.3 Akquise

Der Einstieg läuft über die Prüfung eines laufenden Vorhabens, nicht über eine Produktpräsentation.
Die Desktop-Anwendung läuft dafür 14 Tage ohne Schlüssel, und der Projekt-Pass für 49 € deckt ein
konkretes Vorhaben über 30 Tage ab — mit bis zu zehn Nachprüfungen bei Entwurfsänderungen.

**Betreff:** 90-Sekunden-Vorprüfung gegen die [Landesbauordnung] für Ihr nächstes Projekt

> jede Korrekturschleife beim Bauamt wegen übersehener Abstandsflächen, unklarer
> Vollgeschosszählung oder einer GRZ-Überschreitung kostet Ihr Büro Wochen Verzögerung und
> Nacharbeit.
>
> BauLens prüft Ihre Entwurfsparameter deterministisch gegen die aktuelle Landesbauordnung und die
> BauNVO und erzeugt einen Prüfbericht mit Paragraphennachweis — regelbasiert, nicht geschätzt.
> Gerne schalten wir Ihrem Büro drei kostenlose Vorprüfungen frei, ohne Installation.

---

## 5. Roadmap

### Umgesetzt (bis v0.9.0)

- Desktop-Workhorse: Multi-Projekt-Workspace, Ordner-Scan, Stapelprüfung und interaktives 3D/2D-Modell
- Download-Hub mit OS-Erkennung, Direktverweisen und Prüfsummenauslieferung über die Release-Pipeline
- Interaktive Showcase-Simulation auf der Produktseite (Abstandsflächen über drei Regelprofile)
- Preismodell in Euro: 49 € Projekt-Pass, 89 € Pro, 199 € Büro, Enterprise ab 1.490 €

### Umgesetzt (bis v0.8.1)

- Deterministischer Regelkern mit 15 Modulen und dokumentierten Formeln
- 34 DACH-Profile mit geschärften LBO-Faktoren (0,4 H in NRW, Hessen, Sachsen, Thüringen, Hamburg)
- 20 kommunale Satzungen (u. a. Hannover, Augsburg, Essen, Dortmund, Münster, Bonn, Mannheim, Karlsruhe)
- Reale Freistellungen: Grenzgaragen- und Nebengebäudeprivileg sowie Dachgeschoss-Vollgeschossprüfung
- Desktop-Shell-Support & JSON-Line IPC-Protokoll (`--rpc`) für native Apps (Tauri, Rust, Electron)
- Lokale Offline-Fonts (IBM Plex Sans / Mono) für vollständigen Offline-Betrieb
- Ingestion aus DXF, IFC, XPlanung, GeoJSON, Textteil und Baubeschreibung
- Geometrische Baugrenzenprüfung
- DIN-A4-Dossier mit SHA-256-Siegel und Vektor-Lageplan
- XBau-2.3-Antrags-XML nach OZG
- BCF-2.1-Export und lokaler REST-Dienst
- Compliance Repair Engine
- Engine-Ansicht mit 2D-Lageplan, 3D-Abstandsflächen-Hüllkurve und Paragraphen-Drawer

### Offen

| Vorhaben | Nutzen | Abhängigkeit |
|---|---|---|
| Automatisierte PDF-Textextraktion | Baubeschreibungen ohne Umweg über Textkopie einlesen | keine |
| Volltextprüfung gegen amtliche Fassungen | Zitate gegen das aktuelle Verkündungsblatt verifizieren | Zugang zu konsolidierten Fassungen |
| Schmalseitenprivileg-Modul (16 m) | 0,2 H vor maximal zwei Wänden bis 16 m Länge | Normrecherche je Land |
| Katalogprüfung im Batchbetrieb | Haustypenlisten über alle Profile in einem Lauf | keine |
| Ablöse- und Stellplatzrechner | Stellplatzdefizite bewerten statt nur melden | kommunale Satzungswerte |

Bewusst **nicht** geplant: eine generative Komponente im Prüfpfad. Der Wert des Werkzeugs liegt in
der Nachvollziehbarkeit; ein Modell, das Regelauslegung formuliert, würde genau diese Eigenschaft
aufgeben.
