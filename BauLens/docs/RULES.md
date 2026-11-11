# BauLens — Regelkatalog

Dieses Dokument beschreibt jedes implementierte Prüfmodul: Prüfgröße, Formel, Fundstelle,
Vereinfachungen und die Vertrauensstufe der hinterlegten Parameter. Es ist die verbindliche
Beschreibung dessen, was die Engine tatsächlich rechnet — nicht dessen, was sie rechnen sollte.

> **Einordnung:** Die Musterbauordnung ist ein *Muster*gesetz. Rechtlich verbindlich ist im
> Einzelfall ausschließlich die Landesbauordnung des jeweiligen Landes bzw. das kantonale Recht.
> BauLens bildet **bekannte Abweichungen über Profile** ab (Abschnitt 16). Jede Prüfung bleibt eine
> **Vorprüfung, keine Rechtsberatung**.

---

## Aufbau der Engine

Ein Prüflauf besteht aus vier Schritten:

1. **Validierung** (`src/schema.js`) — das Spec wird normalisiert. Fehlende Angaben werden zu
   Warnungen, falsche Typen zu Fehlern (Exit-Code 2).
2. **Regelauswahl** — neun Kernmodule (`ALL_RULES`) laufen immer; sechs Zusatzmodule
   (`EXTRA_RULES`) werden nur aktiviert, wenn das gewählte Profil sie vorsieht. `--rules id1,id2`
   schränkt den Lauf weiter ein.
3. **Bewertung** (`src/engine.js`) — jedes Modul gibt eine Liste von Befunden mit dem Status
   `ok`, `warn` oder `fail` zurück. Über Kontext (`ctx`) erhält es Profilparameter (`params`),
   den Ländercode (`land`) und die aktive Kommunalsatzung (`municipality`).
4. **Aggregation** (`src/report.js`) — das Gesamtverdikt ist das schlechteste Einzelergebnis:
   mindestens ein `fail` ergibt `nicht bestanden`, nur Warnungen ergeben `mit Auflagen`.

**Statuswerte.**

| Status | Bedeutung |
|---|---|
| `ok` | Prüfung bestanden oder Klassifizierung ohne Verstoß |
| `warn` | Abweichung, die im Verfahren regelbar ist; Nachweis oder Befreiung offen |
| `fail` | Verstoß gegen eine harte Anforderung; Antrag so nicht einreichungsfähig |

---

## Kernmodule

### 1. `gebaeudeklasse` — Gebäudeklassen

**Fundstelle:** §2 MBO · **Datei:** `src/rules/gebaeudeklasse.js`

**Entscheidungstabelle, in dieser Reihenfolge geprüft:**

| Bedingung | Klasse |
|---|---|
| Höhe > 22 m | GK 5 (Hochhaus) |
| Höhe > 13 m **oder** Vollgeschosse ≥ 5 | GK 4 |
| Höhe > 11 m **oder** Wohnungen ≥ 3 **oder** Vollgeschosse ≥ 4 | GK 3 |
| Höhe > 7 m **oder** Wohnungen = 2 | GK 2 |
| sonst (Höhe ≤ 7 m, ≤ 1 Wohnung) | GK 1 |

Ohne Höhenangabe erfolgt keine Klassifizierung (`warn`); die Folgemodule arbeiten dann
eingeschränkt. Das Ergebnis ist immer `ok` — eine Klassifizierung ist kein Verstoß.

**Vereinfachungen.** Die MBO-Tabelle kombiniert Höhe, Nutzungseinheiten und Grundflächen
differenzierter (GK 2 real: ≤ 7 m mit mehreren Nutzungseinheiten; GK 4: Fußboden des obersten
Geschosses über 13 m). Hier läuft eine dokumentierte, **monotone Höhenleiter mit 11-m-Stufe** als
Produktentscheid, um konservativ zu klassifizieren. „Höhe" ist `gebaeude.hoehe_m` (Gesamthöhe),
nicht die mittlere Wandhöhe.

### 2. `abstandsflaeche` — Abstandsflächen

**Fundstelle:** §6 Abs. 2 MBO · **Datei:** `src/rules/abstandsflaeche.js`

**Formel:** `erforderlich = max(minM, hoehe_m × factor)` mit den Profilparametern
`factor` (Standard 0,5) und `minM` (Standard 3 m).

Geprüft wird jede angegebene Himmelsrichtung in `grundstueck.grenzabstaende`.
`ist + 1e-9 < erforderlich` → `fail` je Seite. Sind alle Seiten konform, fasst ein einzelner
`ok`-Befund sie zusammen.

**Vereinfachungen.** Real ist H die Höhe der dem Nachbargrundstück zugewandten Wand zzgl.
definierter Dachanteile; hier wird die Gesamthöhe angesetzt. Grenzanbau (Abstand 0) ist in vielen
Landesordnungen zulässig, wird aber schlicht als Verstoß gewertet — konservativ. Fehlen Höhe oder
Abstände, entsteht eine `warn`-Meldung, kein Fehler.

**Profilwirkung.** Bayern schaltet die Regel ab (dort gilt das Abstandsflächenregime des §6 MBO
nicht) und aktiviert stattdessen `grenzbebauung`. Baden-Württemberg setzt `minM` auf 5 m.

### 3. `grz` — Grundflächenzahl

**Fundstelle:** §19 BauNVO · **Datei:** `src/rules/grz.js`

**Formel:** Vergleich `gebaeude.grz_ist` gegen `einstellungen.grz_limit`
(Standard **0,40**, typisch allgemeines Wohngebiet). Überschreitung → `warn`.

**Begründung der Einstufung.** Die GRZ ist kein festes Gesetzessoll, sondern wird bauleitplanlich
festgesetzt; Ausnahmen sind im Verfahren regelbar. Es wird ausschließlich der angegebene Ist-Wert
geprüft, nicht aus Flächendaten berechnet.

### 4. `gfz` — Geschossflächenzahl

**Fundstelle:** §20 BauNVO · **Datei:** `src/rules/gfz.js`

**Formel:** Vergleich `gebaeude.gfz_ist` gegen `einstellungen.gfz_limit` (Standard **1,20**).
Überschreitung → **`fail`** mit dem Hinweis, dass eine Befreiung nach §31 BauGB vor Einreichung
erforderlich ist.

**Unterschied zur GRZ.** Die Geschossflächenzahl begrenzt das Maß der baulichen Nutzung härter;
eine Überschreitung ohne dokumentierte Befreiung ist ein Einreichungshindernis, kein
Nachweisdefizit. Sind weder Ist-Wert noch Grenzwert angegeben, entfällt die Prüfung
(§34 BauGB ohne Bebauungsplan).

### 5. `stellplaetze` — Kfz- und Fahrradstellplätze

**Fundstelle:** §48 MBO bzw. Landesbauordnung, kommunale Satzung ·
**Datei:** `src/rules/stellplaetze.js`

**Formel:**

```
pkwSoll = ceil(wohnungen_anzahl × pkwFaktor)
radSoll = ceil(wohnungen_anzahl × radFaktor)
```

**Faktor-Priorität:** Kommunalsatzung → `einstellungen.pkw_stellplatz_faktor` /
`einstellungen.rad_stellplatz_faktor` → Standard 1,0 (Kfz) bzw. 2,0 (Fahrrad).

| Situation | Status |
|---|---|
| `pkwFaktor = 0` (z. B. Berlin, keine Kfz-Pflicht für Wohnen) | `ok` |
| Kfz nicht deklariert | `warn` mit Soll-Wert |
| `stellplaetze_ist < pkwSoll` | `fail` — Ablösung oder Nachweis erforderlich |
| `fahrradstellplaetze_ist < radSoll` | `warn` |
| Fahrradplätze nicht deklariert | keine Meldung |

Die Prüfung läuft nur, wenn eine Wohnungszahl angegeben ist **und** entweder Stellplätze deklariert
sind, `einstellungen.stellplatznachweis_erforderlich` gesetzt ist oder eine Kommunalsatzung greift.
Ohne Wohnungszahl, aber mit Stellplatzangabe, entsteht eine `warn`-Meldung zur fehlenden
Verifikationsbasis.

### 6. `tageslicht` — Belichtung von Aufenthaltsräumen

**Fundstelle:** §49 MBO · **Datei:** `src/rules/tageslicht.js`

**Formel:** je Aufenthaltsraum `erforderlich = flaeche_m2 × ratio` mit dem Profilparameter
`ratio` (Standard **1/8**). `fensterflaeche_m2 + 1e-9 < erforderlich` → `fail`.
Alternativ wird `fenster_flaeche_m2` akzeptiert.

**Vereinfachungen.** §49 Abs. 2 lässt in Sonderfällen 1/10 zu; Dachflächenfenster und Lichtbänder
werden wie Fensterflächen behandelt. Räume ohne vollständige Angaben werden gesammelt als `warn`
gemeldet und nicht geprüft.

### 7. `raumhoehen` — Raumhöhen

**Fundstelle:** §46 MBO · **Datei:** `src/rules/raumhoehen.js`

**Schwellen** (Profilparameter `minHoehe` = 2,50 m, `reduzierteMindesthoehe` = 2,20 m):

| Mittlere Raumhöhe | Status |
|---|---|
| ≥ 2,50 m | `ok` |
| 2,20 m bis unter 2,50 m | `warn` — Reduzierung nur im Rahmen landesrechtlicher Ausnahmen |
| < 2,20 m | `fail` |

**Vereinfachungen.** Die angegebene Höhe wird als **mittlere Raumhöhe** interpretiert.
Differenzierungen nach §46 Abs. 2/3 (Nebenräume, kleine Räume) sind nicht modelliert.

### 8. `rettungswege` — Zweiter Rettungsweg und Treppenraum

**Fundstellen:** §11 MBO (Rettungswege), §35 Abs. 4–5 MBO (Treppenraum ab GK 4) ·
**Datei:** `src/rules/rettungswege.js`

**Formel:**

- Kritisch, wenn Gebäudeklasse ≥ 4 **oder** Vollgeschosse ≥ 5.
- Nicht kritisch → `ok`.
- Kritisch **und** Treppenraum **und** zweiter Rettungsweg vorhanden → `ok`.
- Treppenraum fehlt → `fail` (die notwendige Treppe muss ein Treppenraum sein).
- Nur Treppenraum vorhanden → `warn` (Nachweis des zweiten Rettungswegs offen).

**Vereinfachungen.** Stark reduziert. Die realen Anforderungen unterscheiden nach
Gebäudeklasse und Nutzung (Sicherheitsleitern, Wandhydranten, freiliegende Treppenlauftüren,
Rettungsweglängen). Geprüft werden ausschließlich die deklarativen Spec-Werte.

### 9. `vollgeschosse` — Konsistenz von Höhe und Geschosszahl

**Fundstelle:** §16 BauNVO · **Datei:** `src/rules/vollgeschosse.js`

**Formel:** `mittel = hoehe_m / geschosse_ueber_grund`. `mittel > 3,5 m` → `warn`
(Widerspruch: vermutlich Keller- oder Staffelgeschosse falsch erfasst oder die Höhenangabe ist
unplausibel). Fehlen Höhe oder eine ganzzahlige Geschosszahl, entsteht kein Befund.

---

## Zusatzmodule

Diese Module laufen nur, wenn das aktive Profil sie aktiviert oder die nötigen Daten vorliegen.

### 10. `grenzbebauung` — Bauen an der Grundstücksgrenze

**Fundstelle:** §22 BayBO, Grenzbebauungsvorschriften der BauO Bln ·
**Datei:** `src/rules/grenzbebauung.js`

Aktiv in den Profilen **Bayern** und **Berlin**. Das Modul gibt ausschließlich `warn`-Befunde aus:
Ob eine Grenzwand oder ein Anbau an die Grenze zulässig ist, hängt von Bebauungsplan bzw.
Innenbereichslage (§34 BauGB) und den Grenzwand-Anforderungen ab. BauLens weist darauf hin, statt
eine Zulässigkeit zu behaupten.

### 11. `gruendach` — Dachbegrünung

**Fundstelle:** kommunale Gründachsatzung · **Datei:** `src/rules/gruendach.js`

Läuft nur, wenn eine Kommunalsatzung mit `satzungen.gruendach` aktiv ist oder
`einstellungen.gruendach_pflicht` gesetzt wurde.

Als Flachdach gilt: `dachneigung_grad ≤ satzung.maxGrad` (Standard 15°) oder Dachform enthält
„flach" bzw. „pult".

| Situation | Status |
|---|---|
| Steildach über dem Grenzwinkel | `ok` — von der Pflicht befreit |
| Flachdach, `gruendach_vorhanden = false` | `fail` — Begrünungsnachweis fehlt |
| Flachdach begrünt, Substrat unter `minSubstratStaerkeCm` | `warn` |
| Flachdach begrünt, Substrat ausreichend | `ok` |
| Flachdach ohne Deklaration | `warn` |
| Dachform und Neigung unbekannt | `warn` — Angaben nachreichen |

### 12. `brandwand` — Brandwände bei Grenznähe

**Fundstelle:** §30 Abs. 2 MBO, OIB RL 2 · **Datei:** `src/rules/brandwand.js`

**Formel:** Jede Grenzseite mit Abstand `< thresholdM` (Standard **2,50 m**) löst eine
`warn`-Meldung aus: Die Abschlusswand ist als feuerbeständige Brandwand (F90-A bzw. REI 90-M) ohne
ungeschützte Öffnungen auszuführen. Sind alle Abstände ≥ 2,50 m, entsteht eine `ok`-Meldung, dass
keine gebäudeabschlussseitige Brandwand erforderlich ist.

### 13. `barrierefreiheit` — Aufzug und stufenlose Erschließung

**Fundstellen:** §50 MBO, DIN 18040-2, OIB RL 4, SIA 500 ·
**Datei:** `src/rules/barrierefreiheit.js`

**Aufzugspflicht:** `geschosse_ueber_grund ≥ 4` **oder** `hoehe_m > 13,0 m`.

| `aufzug_vorhanden` | Status |
|---|---|
| `true` | `ok` |
| `false` | `fail` — Personenaufzug zwingend erforderlich |
| nicht angegeben | `warn` — Nachweis deklarieren |

**Stufenlose Erschließung:** ab **3 Wohnungen**. `eingang_stufenlos = true` und
`rampen_neigung_prozent > 6,0 %` → `warn` (Grenzwert nach DIN 18040-2 / SIA 500).
`eingang_stufenlos = false` → `warn`. Unterhalb beider Schwellen meldet das Modul `ok`.

### 14. `schallschutz` — Außenbauteile und Haustechnik

**Fundstellen:** TA Lärm, DIN 4109 · **Datei:** `src/rules/schallschutz.js`

| Prüfung | Bedingung | Status |
|---|---|---|
| Außen-Wärmepumpe | `abstand_waermepumpe_grenze_m < 3,0 m` | `warn` — Schallschutzhaube oder Standortanpassung |
| Außen-Wärmepumpe | Abstand ≥ 3,0 m | `ok` |
| Außen-Wärmepumpe ohne Abstandsangabe | — | `warn` |
| Lärmpegelbereich | `schallschutz_laermpegelbereich ≥ 4` | `warn` — erhöhter Schalldämm-Nachweis R'w,res |
| Lärmpegelbereich | < 4 | `ok` |

Ohne deklarierte Außenaggregate und ohne Lärmpegelbereich meldet das Modul `ok`.

### 15. `bplan_festsetzungen` — Bebauungsplan-Festsetzungen

**Fundstellen:** §9 und §30 BauGB, §22/§23 BauNVO, §16/§18 BauNVO ·
**Datei:** `src/rules/bplan_festsetzungen.js`

Läuft nur, wenn `spec.bplan` vorhanden ist. Geprüft werden fünf Gruppen:

| Festsetzung | Vergleich | Status bei Überschreitung |
|---|---|---|
| Firsthöhe | `gebaeude.firsthoehe_m` (oder `hoehe_m`) vs. `bplan.firsthoehe_max_m` | `fail` — Befreiung nach §31 Abs. 2 BauGB |
| Traufhöhe | `gebaeude.traufhoehe_m` vs. `bplan.traufhoehe_max_m` | `fail` |
| Zahl der Vollgeschosse | `geschosse_ueber_grund` vs. `bplan.vollgeschosse_max` | `fail` |
| Dachform | `gebaeude.dachform` gegen `bplan.zulaessige_dachformen` | `fail` |
| Dachneigung | `dachneigung_grad` gegen `dachneigung_min_grad` / `_max_grad` | `fail` |

**Geometrische Baugrenzenprüfung.** Liegt `bplan.baugrenze_polygon` mit mindestens drei Punkten
vor und ist ein Gebäudeumriss angegeben (`gebaeude.umriss_polygon` oder
`grundstueck.gebaeude_polygon`), prüft `checkBuildingWithinBaugrenze` aus `src/bplanparser.js`, ob
der Umriss vollständig innerhalb des Baufensters liegt. Das Verfahren ist 2D-Ray-Casting
(Punkt-in-Polygon) plus Segment-Intersection (Kantenkreuzung). Ausgetretene Eckpunkte werden
gezählt und als Verstoß gegen §23 Abs. 3 BauNVO gemeldet. Alternativ wird die deklarative Angabe
`bplan.baugrenze_eingehalten` ausgewertet.

---

## Exit-Codes des CLI

| Code | Bedeutung |
|---|---|
| 0 | bestanden **oder** mit Auflagen (nur Warnungen) |
| 1 | nicht bestanden (mindestens ein `fail`) |
| 2 | Bedienungs- oder Eingabefehler (Usage, Datei/JSON/Schema ungültig) |

## Konfigurierbarkeit

Über `spec.einstellungen`:

| Schlüssel | Wirkung | Standard |
|---|---|---|
| `grz_limit` | GRZ-Richtwert überschreiben (0 < x ≤ 1) | 0,40 |
| `gfz_limit` | GFZ-Höchstmaß überschreiben | 1,20 |
| `pkw_stellplatz_faktor` | Kfz-Stellplätze je Wohnung | 1,0 |
| `rad_stellplatz_faktor` | Fahrradstellplätze je Wohnung | 2,0 |
| `stellplatznachweis_erforderlich` | Stellplatzprüfung erzwingen | aus |
| `gruendach_pflicht` | Gründachprüfung erzwingen | aus |

Über die Kommandozeile: `--rules id1,id2` schränkt den Lauf auf einzelne Module ein,
`--land` und `--stadt` erzwingen ein Profil bzw. eine Satzung.

---

## Regelprofile

### Vertrauensstufen

| Stufe | Bedeutung |
|---|---|
| `reference` | MBO als reine Referenzbasis |
| `hoch` | Strukturabweichung gut dokumentiert (Modul an/aus, Parameter gesichert) |
| `pruefen` | Parameterwert vereinfacht oder aus Sekundärliteratur — gegen den Gesetzestext prüfen |
| `default` | Keine gesicherte Abweichung hinterlegt; MBO-Defaults laufen weiter |

Ein Profil kann vier Dinge tun: Parameter setzen (`params`), Paragraphenzitate umstellen (`para`),
Module deaktivieren (`enabled: false`) oder Zusatzmodule aktivieren (`extraRules`).

### Deutschland — 17 Profile

`MBO` (Referenz), `BW`, `BY`, `BE`, `BB`, `HB`, `HH`, `HE`, `MV`, `NI`, `NW`, `RP`, `SL`, `SN`,
`ST`, `SH`, `TH`.

| Code | Zitatbasis | `abstandsflaeche` | `grenzbebauung` | Status |
|---|---|---|---|---|
| MBO | MBO | 0,5 H, mind. 3 m (Referenz) | aus | `reference` |
| BW | LBO BW | 0,5 H, **mind. 5 m**, §6 LBO BW | aus | `pruefen` |
| BY | BayBO | **deaktiviert** — §6-MBO-Regime gilt in Bayern nicht | **an** (Warnstufe), §22 BayBO | `hoch` |
| BE | BauO Bln | 0,5 H, mind. 3 m | **an** (Warnstufe) | `hoch` |
| NW | BauO NRW | **0,4 H**, mind. 3 m, §6 BauO NRW | Grenzgaragenprivileg | `hoch` |
| HE | HBO | **0,4 H**, mind. 3 m, §6 HBO | aus | `hoch` |
| SN | SächsBO | **0,4 H**, mind. 3 m, §6 SächsBO | aus | `hoch` |
| TH | ThürBO | **0,4 H**, mind. 3 m, §6 ThürBO | aus | `hoch` |
| HH | HBauO | **0,4 H**, mind. 3 m, §6 HBauO | aus | `hoch` |
| NI | NBauO | 0,5 H, mind. 3 m, §5 NBauO | aus | `hoch` |
| übrige 7 | jeweilige LBO | MBO-Default (0,5 H, mind. 3 m) | aus | `default` |

### Österreich — 11 Profile

`AT_OIB` (harmonisierte OIB-Richtlinien 1–6, Ausgabe 2023) sowie die neun Bundesländer `AT_W`
(BO für Wien), `AT_NOE`, `AT_OOE`, `AT_SBG`, `AT_T`, `AT_STMK`, `AT_KTN`, `AT_VBG`, `AT_BGLD`.
Wien führt Bauwich und Bauklassen I–VI; die übrigen Länder setzen landesspezifische
Abstandsflächenfaktoren.

### Schweiz — 6 Profile

`CH_IVHB` (Interkantonale Vereinbarung über die Harmonisierung der Baubegriffe) sowie die
Leitkantone `CH_ZH` (PBG ZH), `CH_BE`, `CH_BS`, `CH_LU`, `CH_SG`, `CH_GE`. Ergänzend werden
SIA 416 (Flächen und Volumen) und SIA 500 (Hindernisfreie Bauten) referenziert.

### Kommunale Satzungen — 20 Städte

`MUENCHEN`, `NUERNBERG`, `AUGSBURG`, `BERLIN`, `HAMBURG`, `KOELN`, `DUESSELDORF`, `ESSEN`,
`DORTMUND`, `BONN`, `MUENSTER`, `FRANKFURT`, `WIESBADEN`, `STUTTGART`, `MANNHEIM`, `KARLSRUHE`,
`LEIPZIG`, `DRESDEN`, `HANNOVER`, `BREMEN` (`src/municipalities.js`).
Hinterlegt sind Stellplatz-, Gründach- und Freiflächenvorgaben; Berlin setzt den Kfz-Faktor für Wohnungen auf 0, Münster den Fahrradfaktor auf 3,0 je WE.

### Ehrliche Einschränkungen

1. Der 5-m-Wert für Baden-Württemberg ist die häufig zitierte Auslegung, aber nicht gegen den
   aktuellen Wortlaut verifiziert → `[prüfen]`.
2. Für die 7 übrigen deutschen Länder (BB, HB, MV, RP, SL, ST, SH) liegen noch keine spezifischen Abweichungen im Tool; dort laufen
   MBO-Defaults weiter.
3. Österreichische und schweizerische Profile bilden die jeweils zitierten Normen mit den
   hinterlegten Faktoren ab; eine kantons- oder landesspezifische Sonderprüfung ersetzt das nicht.

---

## Geometrie und Parser

| Funktion | Datei | Verfahren |
|---|---|---|
| `isPointInPolygon` | `src/bplanparser.js` | Ray-Casting |
| `checkBuildingWithinBaugrenze` | `src/bplanparser.js` | Ray-Casting plus Segment-Intersection |
| `calculatePolygonArea` | `src/cadparser.js` | Gauß'sche Trapezformel |
| `parseXPlanGml` | `src/bplanparser.js` | XPlanung GML 5.x / 6.0 |
| `parseBPlanGeoJson` | `src/bplanparser.js` | GeoJSON-Baugrenzen und Attribute |
| `parseBPlanText` | `src/bplanparser.js` | Heuristik auf amtlichen Textfestsetzungen |
| `parseDxf` | `src/cadparser.js` | Layer- und Entitätsauswertung |
| `parseIfc` | `src/cadparser.js` | IFC 2x3 / IFC4, LOD 200 |
| `parseBaubeschreibung` | `src/textparser.js` | Regex-Extraktion deutscher Exposés, DIN 277 |
