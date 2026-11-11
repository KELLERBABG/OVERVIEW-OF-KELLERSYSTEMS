# BauLens - Strategischer Transformations- & Entwicklungsplan
**Dokumentversion:** 1.0.0
**Datum:** 05. Oktober 2026
**Ziel:** Transformation von BauLens in eine dedizierte Multi-Projekt-Desktop-Workstation (Architekten-Workhorse mit interaktivem 3D/2D-Modell) und Neuausrichtung der Webpraesenz (Interaktive Demo-Animation, nativer Downloads-Hub, Entfernung des unlimitierten Web-Studios).

---

## 1. Ausgangslage & strategische Neuausrichtung

### 1.1 Problemstellung der bisherigen Architektur
1. **Unbegrenztes Web-Studio (studio.html) im Browser:**
   - Die Website bot bisher das vollwertige Pruefstudio frei zugaenglich an. Das untergraebt das kommerzielle Lizenzmodell (Pro / Buero) und vermittelt den Eindruck einer simplen Web-App statt einer hochperformanten Profi-Software.
   - Architekturbueros arbeiten mit vertraulichen Plaenen (IFC, DXF, XPlanung) und zoegern bei browserbasierten Tools (Sorge vor Cloud-Uploads, DSGVO-Bedenken).
2. **Bruch beim Download & GitHub-Umweg:**
   - Nutzer wurden bisher auf GitHub Releases verwiesen. Architekten und Baupruefer nutzen kein GitHub, suchen muehsam nach Installationsdateien und laden versehentlich CLI-Tools (baulens-windows-x64.exe) herunter, die sich per Doppelklick sofort wieder schliessen.
3. **Fehlender Multi-Projekt-Workflow (VS Code-Prinzip):**
   - Ein Architekturbuero bearbeitet parallel 5 bis 20 Bauantraege gleichzeitig. Das bisherige Studio kann immer nur ein einziges Vorhaben betrachten. Es fehlt eine Projektuebersicht, ein Datei-Explorer und die Moeglichkeit, Ordner mit mehreren Projekten im Stapel zu ueberwachen und abzuarbeiten.
4. **Statisches 3D/2D-Modell:**
   - Die bisherige Isometrie ist ein statischer Canvas-Render ohne Mausinteraktion (Drehen, Zoomen, Pan). Architekten erwarten jedoch, ihr Gebaeudemodell raeumlich zu inspizieren und Grenzkollisionen aus jedem Blickwinkel zu analysieren.

### 1.2 Zielbild: Das zweisaeulige Produktmodell

- **Saeule 1 (Web / lens.kellersystems.dev):**
  - Kommerzieller Auftritt & Brand
  - Interaktive Produkt-Demo-Animation (Hero-Showcase)
  - Direkter Download-Hub (Windows, Mac, Linux) mit OS-Erkennung
  - Rechtssicherheit & Pricing
  - Kein offenes Web-Studio mehr
- **Saeule 2 (Workhorse Desktop-App):**
  - Lokale Hochleistungs-Workstation
  - VS Code Multi-Projekt-Workspace (Ordner oeffnen)
  - Interaktiver 3D/2D CAD-Viewer (Orbit / Pan / Zoom)
  - Zero-Cloud / 100 Prozent DSGVO-offline
  - XBau 2.3, BCF 2.1 & DIN-A4 PDF-Export

---

## 2. Saeule 1: Neuausrichtung der Webpraesenz

### 2.1 Entfernung des offenen Web-Studios & CTA-Restrukturierung
- **Entfernung aus der Navigation:** Alle Buttons Studio oeffnen, Studio starten, Kostenlos pruefen (Web) in index.html, pricing/index.html und den Navigationsleisten werden entfernt.
- **Neuer Haupt-Call-to-Action:** **BauLens herunterladen** bzw. **Downloads**.
- **Weiterleitung / Deprecation:** studio.html wird aus dem Web-Deployment entfernt bzw. auf downloads/index.html umgeleitet, um ein Ausbluten von IP und unlizenzierte Nutzung zu verhindern.

### 2.2 Der neue Download-Hub (downloads/index.html)
Ein dediziertes, plattformoptimiertes Download-Portal:
1. **Automatische Betriebssystem-Erkennung:**
   - Erkennt Windows, macOS oder Linux und hebt den passenden Installer sofort im Hero-Bereich hervor (z. B. Fuer Windows herunterladen (64-Bit .exe / NSIS)).
2. **Direkte Download-Links (Kein GitHub-Zwang):**
   - Direkte Links zu den signierten Release-Dateien.
   - Matrix-Uebersicht fuer alle Systeme:
     - Windows 10/11: .exe Installer (NSIS) & Portable .exe
     - macOS: Apple Silicon & Intel .dmg
     - Linux: .deb & .AppImage
     - CLI Workhorse: Standalone-Binaries fuer Server/CI/CD-Integration
3. **Vertrauens- & Sicherheitsmerkmale:**
   - SHA-256 Pruefsummen fuer jede Datei direkt einsehbar.
   - Zertifikat- und Signaturhinweise (SmartScreen-Transparenz).
   - Hervorhebung: 100 Prozent Lokale Datenverarbeitung - Kein Byte verlaesst Ihr Buero.

### 2.3 Interaktive Produkt-Demo-Animation (Hero-Bereich)
Anstelle des statischen Screenshots oder des offenen Web-Studios erhaelt die Landingpage eine leichtgewichtige, interaktive **Showcase-Animation**:
- **Visuelle Simulation einer echten Vorpruefung:**
  - Ein 3D-Baukoerper wird gezeichnet.
  - Ein Abstandsflaechen-Laser scannt die Fassaden (Nord, Ost, Sued, West).
  - Bei einer Grenzueberschreitung blinkt die betroffene Abstandsflaeche rot auf (Art. 6 BayBO: 0,4m Huellkurven-Kollision).
  - Ein virtueller Klick auf Heilungsvorschlag verschiebt das Gebaeude sanft um 45 cm, die Ampel springt auf Gruen (OK / Genehmigungsfaehig).
  - Ein amtlicher Pruefbericht-Stempel mit Hash-Pruefziffer erscheint.
- **Interaktionsmoeglichkeiten fuer den Web-Besucher:**
  - Besucher kann per Schieberegler Bundesland waehlen (z. B. Bayern vs. NRW vs. Wien) und sieht live, wie sich die Abstandsflaechen-Formel aendert.
  - Voellig autark in Vanilla JS/Canvas/CSS umgesetzt (keine externen WebGL-Monster-Bibliotheken, blitzschnelle Ladezeit).

---

## 3. Saeule 2: Die Desktop-Workstation (The Workhorse)

### 3.1 Das VS Code-Paradigma fuer Architekturprojekte
Architekten arbeiten in zwei typischen Szenarien:

#### Szenario A: Die Ad-hoc-Schnellpruefung (Single-File-Mode)
- **Use Case:** Ein Kollege schickt per Mail eine entwurf_v3.ifc oder lageplan.dxf.
- **Workflow:** Datei per Drag & Drop ins Anwendungsfenster ziehen oder Doppelklick im Explorer.
- **Verhalten:** Die App startet direkt in der Detailansicht, prueft in 80 ms, visualisiert das Gebaeude und zeigt sofort die Maengelliste mit Paragraphen. Kein Anlegen von Projekten, kein Zwang zur Ordnerstruktur.

#### Szenario B: Der Multi-Projekt-Workspace (Open Folder Mode)
- **Use Case:** Das Buero betreut 5 bis 15 aktive Bauantraege parallel (z. B. Neubau MFH, Sanierung Villa, Gewerbehalle).
- **Workflow:** Klick auf Projektordner oeffnen (oder Menue Datei -> Ordner oeffnen...).
- **Verhalten:**
  - BauLens indiziert den ausgewaehlten Bueroordner rekursiv nach CAD- und Plandaten (.ifc, .dxf, .gml, .json, .bcf).
  - Jedes Unterverzeichnis wird als eigenstaendiges Projekt erkannt.
  - Die linke Explorer-Sidebar listet alle Bauvorhaben mit kompakter Status-Ampel auf.

### 3.2 Interaktives 3D- & 2D-Modell (Orbit, Pan & Zoom)
Das aktuelle Modell ist auf eine feste 30-Grad-Isometrie fixiert. Architekten muessen jedoch die Abstaende zu Grundstuecksgrenzen aus der Vogelperspektive, von der Grundstuecksecke und im Fassadenschnitt betrachten koennen.

#### Technische Architektur des neuen 3D-Viewers:
1. **Kamera- und Transformationssteuerung (Maus & Touchpad):**
   - **Linke Maustaste gedrueckt halten + Ziehen:** Orbit-Rotation (Azimut um Z-Achse, Elevation 10 bis 85 Grad).
   - **Rechte Maustaste / Mittlere Maustaste + Ziehen:** Pan (Verschieben des Baufelds in X/Y).
   - **Mausrad:** Stufenloser Zoom auf die kritische Grenze oder das Dachgeschoss.
   - **View-Presets per Shortcut/Button:**
     - Taste 1: Isometrie Sued-West
     - Taste 2: Draufsicht (2D-Lageplan orthogonal von oben)
     - Taste 3: Nord-Ansicht (Fassadenschnitt)
     - Taste R: Kamera zuruecksetzen (Reset View)
2. **Interaktive Selektion & Hervorhebung:**
   - Klick auf einen Mangel in der rechten Liste fokussiert und rotiert die Kamera automatisch auf die betroffene Fassade.
   - Die verletzte Abstandsflaeche pulsiert rot, waehrend unkritische Flaechen transparent blau bleiben.
3. **Erweiterter Modell-Import (IFC & DXF Geometrie):**
   - Architekten exportieren aus Revit, Archicad, Allplan oder Vectorworks standardmaessig IFC (IFC2x3 / IFC4).
   - Der Parser extrahiert Geschosshoehen, Bounding-Polygone, Dachformen und Grundstuecksgrenzen.
   - Die echte Gebaeudegeometrie wird als Wireframe / Shaded Massing direkt im Viewer gerendert.

---

## 4. Technische Implementierungs-Roadmap

### Phase 1: Desktop-Workhorse UI & Projektverwaltung
- Eigenstaendiges Workhorse-Interface mit professionellem 3-Spalten-Layout (Sidebar, Center-Stage, Inspector).
- Implementierung von Workspace-Scanning (rekursives Auffinden von Bauvorhaben in Ordnern).
- Neue IPC-Methoden fuer Workspace-Events und Batch-Pruefungen.

### Phase 2: Interaktiver 3D/2D-Viewer
- Transformationsmatrizen mit kontinuierlichen Winkeln (Orbit/Pan/Zoom).
- Mouse-Event-Handling fuer fluessige 60fps-Interaktion.
- Dynamische Bemassungslinien und Klick-Selektion von Grenzkanten.

### Phase 3: Website, Downloads-Hub & Hero-Demo
- Erstellung von downloads/index.html mit automatischer Betriebssystem-Erkennung und direkten Download-Links.
- Umbau von index.html: Entfernung aller Studio-Links, Implementierung der interaktiven Hero-Demo-Animation.
- Anpassung von scripts/build-site.mjs, scripts/check-links.mjs und site.config.json.

### Phase 4: Release-Automatisierung & Qualitaetssicherung
- Integration des Desktop-Installers in GitHub Actions CI/CD.
- Sicherstellung aller 8 Release-Gates und 157 Testfaelle.
- Veroeffentlichung des Meilensteins als v0.9.0.

---

## 5. Konkrete Datei- & Aufgabenstruktur

| Datei | Status | Beschreibung der geplanten Aenderungen |
| :--- | :--- | :--- |
| UPDATE_PLAN.md | [NEU] | Dieser umfassende Master-Plan. |
| downloads/index.html | [NEU] | Zentrale Download-Seite fuer alle Plattformen mit OS-Detection. |
| scripts/build-site.mjs | [MODIFIZIERT] | Generator um downloads/-Generierung und Verlinkungen erweitern. |
| site.config.json | [MODIFIZIERT] | Tiers anpassen (Kein unlimitiertes Web-Studio mehr, Download-Links pflegen). |
| index.html | [MODIFIZIERT] | Header/Hero auf Downloads umstellen, Hero-Demo-Animation einbauen. |
| desktop/app/index.html | [NEU] | Dedizierte Workhorse-Benutzeroberflaeche (VS-Code 3-Spalten-Layout). |
| desktop/inject/desktop.js | [MODIFIZIERT] | Integration von Workspace-Ordnerauswahl und File-Tree-Events. |
| desktop/src-tauri/src/lib.rs | [MODIFIZIERT] | Native Ordner-Dialoge (tauri-plugin-dialog) und Workspace-Management. |
| .github/workflows/release.yml | [MODIFIZIERT] | Desktop-Installer-Kompilierung im Release-Gate. |---

## 6. Kommerzielle Strategie & Preisstruktur (DACH B2B)

### 6.1 Pain-Driven Messaging
Architekturbueros und Generaluebernehmer kaufen Software nicht aus Neugier, sondern zur Risikovermeidung und Zeiteinsparung:
> **"Eine einzige bauaufsichtliche Nachforderung kostet Sie 3 bis 6 Wochen Verzoegerung und tausende Euro Nacharbeit. BauLens prueft Ihren Entwurf deterministisch vorab - ab 49 Euro."**

### 6.2 Die 3 Saeulen des neuen Preismodells (100 Prozent in Euro)
Die bisherige Bindung an 1.499 USD jaehrlich war eine unueberwindbare Vertrauenshuerde fuer Neukunden. Das neue Modell senkt das Einstiegsrisiko radikal und setzt auf Product-Led Growth:

| Tier | Preis | Abrechnung | Zielgruppe & Leistungsumfang |
| :--- | :--- | :--- | :--- |
| **Starter (Projekt-Pass)** | **49 EUR** | Einmalig (Pay-per-Project) | **Der risikolose Einstieg fuer Skeptiker:** 1 konkretes Bauvorhaben 30 Tage lang pruefen (inkl. bis zu 10 Re-Runs/Nachpruefungen fuer Entwurfsaenderungen). Inklusive vollstaendigem DIN-A4 Dossier mit Pruefsiegel und XBau 2.3 XML. *(Hinweis auf Website: "Ab dem 2. Vorhaben rechnet sich Pro.")* |
| **Pro Architekt** | **89 EUR / Monat** *(oder 890 EUR / Jahr)* | Monatlich kuendbar *(bzw. 2 Monate geschenkt bei Jahreszahlung)* | **Fuer freie Architekten & Einzelplaner:** Unbegrenzte Projektpruefungen, unbegrenzte Dossier-Exporte, Compliance-Repair Engine (Heilungsvorschlaege), 1 Arbeitsplatz. Jahrespreis unter 1.000 EUR (steuerlich sofort absetzbar, keine interne Freigabehuerde). |
| **Buero / Kanzlei** | **199 EUR / Monat** *(oder 1.990 EUR / Jahr)* | Monatlich kuendbar *(bzw. 1.990 EUR bei Jahreszahlung)* | **Fuer Planungsbueros & Teams:** 3 Arbeitsplaetze inklusive (~66 EUR je Platz). Jeder weitere Arbeitsplatz: +49 EUR / Monat (+490 EUR / Jahr). Individuelles Buerologo/Kopfzeile im Dossier, DACH-Quervergleich ueber alle 34 Profile. |
| **Enterprise** | **ab 1.490 EUR / Monat** | Individuell | Fuer Serien-, Typen- und Fertighaushersteller mit Massenpruefung, API-Zugang und BCF-Pipeline in die BIM-Umgebung. |

### 6.3 Testphasen & Demo-Regelung
1. **Web-Demo (Eingeschraenkt):**
   - Kein vollwertiges Studio mehr im Web.
   - Stattdessen eine kostenlose Browser-Demo mit 3 Pruefmodulen (Abstandsflaechen, Vollgeschosse, GRZ) zur Sofort-Einschaetzung ohne Installation.
   - Kein PDF-Dossier-Export im Web (Upsell auf Starter 49 EUR oder Desktop-App).
2. **Desktop-Trial (14 Tage):**
   - Voller Funktionsumfang der Desktop-Workstation fuer 14 Tage ab Erststart (lokaler Timestamp, kein frustrierendes DRM).

### 6.4 Transparenz-Garantie auf allen Tiers
Auf allen Preis- und Checkout-Seiten wird klar deklariert:
- *Vollstaendig adaptierte Landesbauordnungen fuer Bayern (BayBO), NRW, Baden-Wuerttemberg, Berlin u. a.*
- *Alle weiteren Bundeslaender und Kantone auf gesicherter MBO-/OIB-Referenzbasis.*
