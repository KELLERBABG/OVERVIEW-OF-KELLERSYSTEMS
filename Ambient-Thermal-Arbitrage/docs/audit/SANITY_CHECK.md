# ATA – vollständiger Projekt-Sanity-Check

**Erstprüfung: 11.10.2026; Auditkorrektur: 12.10.2026; Gegenprüfung der externen Auditliste: 12.10.2026.** Quellenabruf grundsätzlich 11.10.2026, Ausnahmen im Quellenregister. Prüfobjekt: Ambient Thermal Arbitrage, Branch `main`, Ausgangs-HEAD `4ac48390a0204b73985062168322a2367759efe4`. Keine Rechtsberatung, HACCP-Freigabe, Herstellerfreigabe oder Investitionsgarantie.

## 0. Urteil in einem Satz

**Die Idee „Kälte als zeitlich begrenzter Flexibilitätsspeicher“ ist real; die im Projekt beworbenen Einsparungen, Sicherheit und Erlöse sind in der jetzigen Form nicht belastbar nachgewiesen und teilweise durch Modell-/Abrechnungsfehler stark überhöht.** Nicht blind ausrollen. Zunächst Messdaten und einen herstellerseitig freigegebenen, begrenzten Piloten finanzieren – keine 407-Filialen-Umsatzplanung auf den bisherigen Ergebnissen.

### Die wichtigsten Zahlen vorweg

| Ebene | Ergebnis | Was es bedeutet |
|---|---:|---|
| Historische Projektbehauptung, Typ `aldi` | 24.809 €/a; 66,9 % | Alte Modellrechnung, kein Feldresultat |
| Aktueller unveränderter CLI-Lauf, `aldi` | **25.121,95 €/a; 67,76 %** | Reproduziert; weiterhin unfairer Baseline-Vergleich |
| Gleicher Physiksimulator, fester Sollwert versus Controller-Sollwertfolge | **460,39 €/a; 3,71 % der Modell-Beschaffungskosten** | Diagnostischer Kostenunterschied **ohne Endzustandsausgleich**, kein bereinigter Arbitragewert oder Feldresultat; Endzustände in 4.2 |
| Aktueller Modelllauf, 400 `aldi` + 5 `edeka` + 2 `marktkauf` | **10.651.202,21 €/a** | Lineare Hochskalierung der unfairen Baseline |
| Gleiche 407 Typinstanzen, gleicher-Physik-Gegenlauf | **186.840,19 €/a**, rechnerische Hälfte 93.420,10 € | Diagnostischer Kostenunterschied **ohne Endzustandsausgleich**, vor Kosten; weder abrechenbare Ersparnis noch Fee-Nachweis |
| Konservatives bedingtes Supermarktbeispiel, 10 kW × 2 h × 150 Events | **97,50 €/a vor O&M**, −902,50 €/a nach angenommenen 1.000 € O&M | Reine Arbitrage nach 5 % Mehrenergie; keine gemessene Prognose |
| Separates Upgrade-Paket aus Arbitrage, 5 % echter Energieeffizienz und 15 kW gesicherter Spitzenreduktion | **4.585,30 €/a vor O&M**, 3.085,30 €/a danach | Nur bei allen einzeln nachgewiesenen Voraussetzungen; nicht automatisch addierbar |
| Regelreserve/Redispatch heute gesichert | **0 € nachgewiesener Vertrags-/Markterlös** | Bedeutet fehlender Nachweis, nicht prinzipielle Unmöglichkeit |
| Zahl heute tatsächlich EPEX-wechselfähiger deutscher LEH-Standorte | **Öffentlich nicht quantifizierbar** | Verkaufsstellenzahlen sind keine Zähler-/Vertrags-/Freigabezählung |

**Zur Angst vor „zu greedy“:** Sie ist bei der bisherigen Rechnung begründet, aber sie beweist nicht, dass das Geschäftsmodell unmöglich ist. Besonders problematisch ist die Verwechslung von Preisverschiebung, Hardwareeffizienz, Modell-Baseline und realem Kundenüberschuss. Große Kühlhäuser, niedrige Grenzintegrationskosten über vorhandene OEM-Technik und echte Leistungspreisreduktion können ein anderes Ergebnis liefern als ein einzelner kleiner Markt.

**Ergänzung 12.10.2026:** Die zehn Positionen der externen Auditliste [independent Audit.txt](independent%20Audit.txt) sind in **Abschnitt 14** einzeln gegen den unveränderten Code, den echten Preiscache und ausgeführte Simulationsläufe geprüft — Verdikt je Position getrennt nach bestätigt / teilweise bestätigt / widerlegt / offen. Kein Produktcode, Commit oder Push in diesem Durchgang.

## 1. Auftrag, Methode und Evidenzklassen

Akzeptanzkriterien dieser Prüfung: deutschsprachige Datei **`SANITY_CHECK.md` im Projektroot**; Inventar und aktuelle Zahlen; deutsche Kälteanbieter, dokumentierte Schnittstellen/Rechte/Grenzen; Physik, Kühlkette, Dynamik und Sicherheit; reproduzierbare Rechnungen aktuell/konservativ/Upgrade getrennt; Quellen mit Abrufdatum; offene Daten ausdrücklich; kein Produktcode, Commit oder Push. Zusatzauftrag: rechtliche, technische und vertragliche EPEX-Wechselmöglichkeiten sowie belastbar zählbarer Markt.

- **[F] Fakt:** aus gelesenem Quelltext, ausgeführtem Test oder gelesener externer Quelle. Herstellerquelle belegt eine Herstellerbeschreibung, nicht automatisch unabhängige Leistung oder konkrete Standortfreigabe.
- **[A] Annahme:** expliziter Input einer Beispiel-/Sensitivitätsrechnung, kein gemessener Branchenwert.
- **[U] Unbekannt:** ohne Nachweis nicht als Zahl, Zugang oder Umsatz zugesagt.
- **[D] Ableitung:** transparenter Rechenschritt aus Fakten/Annahmen; trägt deren Grenzen weiter.

Sieben Web-Researcher-Aufträge wurden verwendet: fünf parallel zu Danfoss, Eckelmann/Wurm, weiteren Herstellern, Recht/Physik und Wirtschaft; anschließend Tarifwechsel/TAM und API-/EHI-/Wettbewerbsprüfung. Zentrale Befunde wurden zusätzlich direkt an Primärseiten überprüft. Onlinezugang und Subagents waren verfügbar. Es wurden **keine Accounts eingerichtet, Anbieter ausgewählt/integriert, Kaufanfragen versendet oder Anlagen angesprochen**.

**Arbeitsbaum respektiert:** Bereits zu Beginn waren [OPERATOR_FIELD_GUIDE.txt](OPERATOR_FIELD_GUIDE.txt) und [OPERATOR_FIELD_GUIDE_DE.txt](OPERATOR_FIELD_GUIDE_DE.txt) gelöscht (` D`); sie wurden weder wiederhergestellt noch geändert. Diagnoseprogramme, CSVs und Logs liegen lokal im temporären Arbeitsbereich; die wesentlichen Resultate und Reproduktionsanweisungen stehen hier. Das einzige neue nicht ignorierte Arbeitsergebnis dieser Prüfung ist dieser Bericht.

### Grenzen des Wortes „komplett“

Dies ist eine breite Desk-/Code-/Simulationsprüfung, **keine vollständige unabhängige Sicherheitszertifizierung oder Penetrationsprüfung**. Keine reale deutsche Anlage, kein Warenbestand, keine Stromrechnung, kein Liefervertrag, kein Aggregatorvertrag und kein Herstellerangebot wurden bereitgestellt. Einige Hersteller-PDFs waren im Web-Leser nicht extrahierbar, manche Detaildokumente erfordern Login. Unlesbare Dokumente und Inhaltsverzeichnisse wurden nicht als Register- oder Freigabenachweis verwendet. Kosten sind deshalb überwiegend Sensitivitätsannahmen, nicht Angebote. Es gibt keinen vollständigen 2026-Jahresdatensatz am Prüfdatum. Eine Rückrechnung mit echten vollständigen 2025-Preisintervallen plus realer Filiallast wurde nicht durchgeführt; der vorhandene Cache wurde untersucht und reproduziert.

## 2. Bestandsaufnahme des tatsächlichen Projekts

| Artefakt | Tatsächliche Rolle | Einschränkung |
|---|---|---|
| [Root-README](README.md), [Workspace](Cargo.toml) | Rust-Workspace mit Gateway v0.2.0 und Fleet v2.0.0 | README selbst sagt: noch nicht am realen PLC feldgetestet |
| [Fleet-CLI](crates/fleet/src/main.rs) | 8.760 Stunden; eine Repräsentativanlage je Typ, Resultat × Anzahl | Keine Simulation individueller Filialen, Stromverträge oder Flottenkorrelation |
| [Typdefinitionen](crates/fleet/src/store_types.rs) | **21** Typen inkl. `facility` und HVAC | Namen `aldi`/`edeka` sind Parameteretiketten, keine vermessenen oder freigegebenen Handelsanlagen |
| [Fleet-Physik](crates/fleet/src/physics.rs) | Einzonen-RC, vereinfachtes COP, Cargo-Lag, Abtauung, TES-Ladezähler | Keine vollständige Kältekreis-/Produkt-/Speicherbilanz |
| [Fleet-Regelung](crates/fleet/src/control.rs) | Kaskade, MPC-Heuristik, Governor, synthetische FCR-Rechnung | Nicht identisch mit ausgeführter Physik oder Gateway |
| [Fleet-Finanzen](crates/fleet/src/financial.rs) | Spotkosten, rechnerische Baseline, Sparprozente | Nicht all-in; Baseline verwendet andere Effizienz |
| [Gateway](crates/gateway/src/main.rs), [Modbus](crates/gateway/src/modbus_client.rs) | Reale Temperaturregister lesen, Coil oder Geschwindigkeitsregister schreiben | **Keine** spezifischen produktiven Adapter für die recherchierten Hersteller |
| [Gateway-Konfiguration](crates/gateway/config.toml) | Simulation an, Modbus aus, Live-Daten an; VFD an | Verschiedene Betriebsebenen; keine fertige Standortkonfiguration |
| [E2E-Fixture](scripts/e2e/plc-config.toml), [Fake PLC](scripts/e2e/fake-plc.js) | Lokales Protokoll-/API-Testgerät mit konstanter Temperatur | Keine thermodynamisch zurückgekoppelte reale Anlage |
| [Whitepaper](docs/whitepaper-ATA.md), [Pitch](docs/pitch-deutsch.md), [Benchmarks](benchmarks/fleet_results_v2_20260720_114808.csv) | Behauptungen und historische Aggregatergebnisse | „Energy-neutral“, „kein Risiko“, 25–84 % und garantierte Sicherheit nicht belegt |
| [Deployment-Guide](crates/gateway/DEPLOYMENT.md), [CI](.github/workflows/ci.yml) | Betriebshinweise und Softwareprüfungen | Keine OEM-/HACCP-/Marktfreigabe; Dokumentation teils veraltet |
| [Analyzer-Ausgabe](docs/report.json), [Hardening-Plan](docs/hardening-plan.md) | Alte Analysehinweise | Ohne Toolversion, reproduzierbaren Exploit und aktuellen Bezug keine Zertifizierung; nicht alle gemeldeten Probleme als echte Fehler zählen |
| [Tinkercad](tinkercad/refrigeration_thermostat.ino), [Website](index.html) | Demo und Darstellung | Kein Nachweis für industrielle Kältesteuerung |

### Ausgeführte Checks

Windows/Git Bash, `rustc 1.97.1 (8bab26f4f 2026-07-14)`, vorhandene Abhängigkeiten, kein Lockfilewechsel:

| Prüfung | Beobachtung |
|---|---|
| `cargo check --locked --workspace` | Erfolgreich, beide Crates geprüft |
| `cargo test --locked --workspace` | **97 Gateway- + 30 Fleet-Tests bestanden**, 0 Fehler; übrige Targets/Doc-Tests ohne Tests |
| `cargo clippy --locked -p thermal-arbitrage-mvp --all-targets -- -D warnings` | Erfolgreich |
| Fleet-CLI 7 Typen, danach **alle 21 Typen**, je volles Jahr | Erfolgreich; CSV exportiert, CLI-Ergebnis geprüft |
| Fleet `--volatile`, drei Retailtypen | Erfolgreich als Modelllauf; **kein sicherer Pilotmodus** |
| Diagnostischer gleicher-Physik-Gegenlauf über alle 21 Typen | Ausgeführt; Kostenunterschiede ohne Endzustandsausgleich, gemessene Modell-Endzustände in 4.2 |
| VFD-/Grenzverletzungsproben direkt gegen unveränderte Fleet-Funktionen | Erfolgreich; reproduzierte Mängel in Abschnitt 3 |
| Bestehendes `bash scripts/e2e/e2e-test.sh` | Alle sichtbaren Verhaltenschecks meldeten OK, **Gesamtexit 1** durch Windows-Cleanup `Device or resource busy`; eigener Gatewayprozess blieb bestehen und wurde nach Identitätsprüfung beendet |
| Separater diagnostischer E2E über lokalen HTTP-/Modbus-Weg | **Lokaler, nicht mitgelieferter Nachweis, Exit 0**: PLC 20 °C → Emergency/ON, 49,98 Hz, FC05-Empfang, HTTP 401/400/200, PLC-Ausfall → Fail-safe-Log und `/health` 200; Prozesse beendet und abgewartet. Harness und Logs nur im temporären Arbeitsverzeichnis, aus diesem Bericht nicht vollständig reproduzierbar; kein physischer Kühlungsnachweis |

Der separate Harness wurde nach einem eigenen Python-Prädikatfehler vollständig erneut ausgeführt. Sein lokaler Erfolg ersetzt weder den mitgelieferten Repository-E2E noch dessen fehlgeschlagenen Gesamtexit 1. Dieser wurde **nicht** als Pass umetikettiert; das Repository-Skript bleibt unrepariert. Kein Release-Build, Pi-/Linux-/Seriell-Hardwaretest, kompletter Security-Audit oder echter Ladenversuch in diesem Durchlauf.

**Ein bestandener Test beweist den getesteten Softwarevertrag, nicht dessen physikalische Richtigkeit.** Beispielsweise bestätigt ein Test eine Temperaturbegrenzung mittels `clamp`; das ist gerade kein Nachweis, dass eine reale Ware ihre Grenze nicht überschreitet.

## 3. Kritische Modell- und Produktbefunde

### 3.1 Rot: ungleiche Baseline erzeugt den Großteil der „Ersparnis“

[F] [Baseline-Code](crates/fleet/src/financial.rs):

```text
G = U*A + V*ACH/3600 * rho_air * cp_air       [W/K]
Q = (Tamb - Tset)*G/1000 + Qinternal + Pdefrost*duration/interval [kWth]
duty_baseline = clamp(1,3 * Q / (Pelektrisch * COP_base), 0,05, 0,9)
Ebaseline = Pelektrisch * duty_baseline /1000 [MWh je Stunde]
```

[F] Der optimierte Betrieb verwendet dagegen [dynamisches COP](crates/fleet/src/physics.rs): ungefähr `0,6*(Troom+273,15)/abs(Tamb−Troom)`, mit eigener Abweichungsstrafe und Begrenzung 0,5–6. Bei `aldi` ist `COP_base=1,8`, die Simulation kommt z. B. bei −22/25 °C auf etwa **3,206** und bei −22/10 °C auf etwa **4,709**. Die Baseline wird zusätzlich mit **×1,3** belastet. Das ist kein isolierter Vergleich einer Preisregelung.

[U] Die pauschale 30-%-Taktnachteilsbehauptung im Whitepaper hat hier keine konkrete ASHRAE-Fundstelle, Prüfanlage oder Kalibrierung. Schon rein algebraisch macht ×1,3 bei sonst identischem Verbrauch `(1−1/1,3)=23,08 %` scheinbare Einsparung. Der COP-Wechsel erhöht den Effekt stark. Selbst eine reale VFD-Nachrüstung wäre ein **eigenes Hardwareeffizienzprojekt mit Investition**, kein kostenloser Beleg für Arbitrage.

[D] Beim aktuellen `aldi`-Lauf fällt der Verbrauch gegenüber der Projektbaseline von 644,321 auf 218,119 MWh (**−66,15 %**). Im gleichen-Physik-Gegenlauf dagegen nur von 218,575 auf 218,119 MWh (**−0,209 %, ohne Endzustandsausgleich**, siehe 4.2). Die Aussage „gleiche Energie, Ersparnis ausschließlich aus zeitlicher Verschiebung“ passt nicht zur ausgewiesenen Projektbaseline.

### 3.2 Rot: Temperaturen werden auf erlaubte Grenzen abgeschnitten

[F] [Fleet-Physik](crates/fleet/src/physics.rs) setzt nach dem Temperaturschritt `zone.temp_c = zone.temp_c.clamp(t.t_min, t.t_max)`. Überforderung verschwindet aus dem Zustand, statt als Grenzverletzung protokolliert zu werden.

Direkt ausgeführte Belastungsprobe, `aldi`-Parametertyp, **0 kW Kälte**, 10.000 kW interne Wärme, 40 °C Umgebung, Start −22 °C:

```text
Temperatur vor clamp nach implementierter Dynamik: -17,468550 °C
Zurückgegebene Temperatur:                         -18,000000 °C
Gezählte Verdichterenergie:                         0 MWh
```

[A] Das ist absichtlich eine adversariale Überlast, keine realistische Ladenlast. [F] Sie zeigt mechanisch, dass der Simulator eine physikalisch errechnete Überschreitung verbergen kann. „Zero compromise“/„Conservative = safe“ ist deshalb nicht aus den Outputtemperaturen beweisbar.

### 3.3 Rot: simulierte Aktuation entspricht nicht Controllerbefehlen

[F] `main` ruft `simulate_hour(..., decision.setpoint_c, decision.tes_charging, 0.0, false)` auf. Die Physik ignoriert `_vfd_speed` und `_can_shed`; `compressor_on` wird gar nicht als Physikinput übergeben. Sie hält selbst über kontinuierliche Lastanpassung plus kleiner Temperaturkorrektur das Gleichgewicht.

[F] Direkte VFD-Probe: identischer Ausgangszustand, einmal Speed 0, einmal Speed 1 → **in beiden Fällen 0,035865219988 MWh und −22 °C**. Im konservativen Jahreslauf `aldi` gab es **1.331 Stunden mit Controllerbefehl OFF und trotzdem positiver simulierter Verdichterenergie**; `datacenter`: 6.634. Somit gelten OFF-Governor, Shed-Dauer und Gridbefehle nicht so für die thermische Aktuation, wie das UI suggeriert. Das Gateway schreibt hingegen tatsächlich eine Coil oder ein Speed-Register. Die Fleet-Wirtschaftlichkeit lässt sich nicht als Nachweis für dieses Gateway übertragen.

### 3.4 Rot: Preisdatensatz nicht das behauptete reale EPEX-Jahr

[F] Eingecheckter [Fleet-Cache](crates/fleet/data_cache/fleet_2025.json), 8.760 Datensätze, SHA-256:

```text
ebb40f27710964507e2c6e279e1e07d22f93934bd5b1a70d837bc16cbdd222d9
Preis-Mittel 57,832704 €/MWh; min -6,577152; max 268,145284
126 negative Stunden; 0 Stunden >300 €/MWh
Temperatur-Mittel 11,625696 °C; min -5,1; max 37,2
```

[F] BNetzA für reale DE-Day-ahead-Preise 2025: **89,32 €/MWh**, **573 negative Stunden**, **40 Stunden >300 €/MWh** [M1]. Ein anderer Preisbegriff oder fehlende Intervalle müssten dokumentiert sein; der Cache trägt keine solche Provenienz. Frequenz ist explizit synthetisch. [F] Der [Gateway-Seed](crates/gateway/data_cache/2025_dataset.json) nennt als Quelle nur `Open-Meteo (weather)`, hat **0 nicht-leere Preiswerte**, Abruf-Metadatum 25.06.2026.

[F] [Fleet-Preisparser](crates/fleet/src/simulation.rs) erwartet in `price` Objekte mit `data` und anschließend `price`/`unix_seconds`. Die direkt gelesene offizielle V1-API liefert hingegen **parallele Arrays `unix_seconds[]` und `price[]`** [M2]. Der geprüfte 01.10.2025 hatte 96 Intervalle à 900 s. Dieser Parser nimmt bei diesem Schema keine Preise auf, worauf synthetische Preise eingesetzt werden. Der Cache wird vor API-Abruf verwendet und enthält keine pro Intervall belegte Quellen-/Fallbackmarkierung.

Zusätzlich: dokumentierter Preisparameter `bzn=DE-LU`, nicht `country=de`; Wetter wird als lokale `Europe/Berlin`-Stunde indexiert, Preiscode als UTC-Ordinalstunde. DST, Jahresgrenzen und Viertelstundenaggregation sind damit nicht sauber einheitlich. Seit 01.10.2025 sind Viertelstunden-Day-ahead-Produkte relevant [M3]. Keine Aussage, dass jedes Gateway-Parsing denselben Fehler hat – Fleet- und Gateway-Pipelines sind getrennt.

### 3.5 Rot: thermische Masse, Wärmeeintrag und Produkte unkalibriert

[F] `aldi`: 4.000 m³, 200 kW elektrische Verdichter-Nennleistung, −22 °C Einzonensoll, 200.000 kg Ware, `thermal_mass_kj_k=8.000.000` → **8 Milliarden J/K**. Cargo-Wärmekapazität separat `200.000*2.000=400 Millionen J/K`: **Faktor 20**. Die Airzone wirkt damit so träge wie **4.000 t** mit cp=2 kJ/(kg·K); 1 K entspricht **2.222,22 kWhth**. Rein rechnerisch bei COP 2 sind das 1.111,11 kWhel/K, nicht eine bewiesene zugängliche Kurzzeitspeicherkapazität.

[U] Strukturmasse und thermische Ankopplung könnten beitragen, sind aber nicht belegt. 200 t TK-Bestand je Discounter sind ebenfalls nicht mit Inventar-/Lagerdaten bestätigt. Eine Einzelzone mit Gebäudewänden bei −22 °C bildet keinen typischen Markt mit warmem Verkaufsraum, Pluskühlung, TK-Möbeln, Türen und getrennten Sauggruppen ab. Außenlufttemperatur ist nicht automatisch die Umgebungs-/Verflüssigungstemperatur jedes Möbels.

[F] Die Cargo-Dynamik liest Lufttemperatur als passiven Lag, ist **nicht energetisch rückgekoppelt** in die Luft-/Verdichterbilanz; Temperaturstratifizierung wird berechnet, aber weder als maßgebliche Grenze noch als Verbraucher berücksichtigt. Der Diagnose-Jahreslauf liefert `aldi`-Deckentemperatur bis **+7,67 °C** bei formal „sicherer“ Einzonentemperatur. Das ist kein Beleg, dass Ware real so warm wird; es zeigt interne Widersprüche und fehlende Worst-Case-Prüfung. Interne Wärme, Türverkehr, latente Feuchtelast, Wareinlagerung und Besucher-/Öffnungsdynamik sind nicht standortkalibriert. Die Funktion `retail_hour_params` existiert und ist getestet, wird im Fleet-Hauptlauf aber nicht aufgerufen.

### 3.6 Rot/Gelb: Abtauung, Nebenverbrauch und COP-Bilanz

- [F] Eine 0,5-h-Abtauung wird im 1-h-Schritt als **eine volle Heizstunde** gerechnet. Bei `aldi`: 40 kW × 8.760/8 = **43,8 MWh/a** statt 21,9 MWh/a bei wirklich 30 Minuten. Baseline mittelt dagegen mit der eingestellten Dauer.
- [F] Physik verändert Temperatur schon im Abtauschritt und nimmt Abtauwärme dann nochmals in die Wärmelast auf. Lokale Gleichzeitigkeit Verdichter/Heizer, Zeitpunkt und Abtau-Ende sind nicht realistisch aufgelöst.
- [F] Die Fleet-Finanzrechnung nimmt **nur `compressor_energy_mwh`**, nicht `fan_energy_mwh` oder die separat gezählte elektrische Heizerenergie als tatsächliche Kosten. Verdichtermehrarbeit durch Heizer kann enthalten sein, elektrische Heizerarbeit dennoch fehlen. Baseline/Actual haben keinen vollständigen gleichen Bilanzumfang.
- [U] Das `v³`-Gesetz für Lüfter wird pauschal auf VFD-Verdichterlast bezogen; unabhängige Lüfterzustände, Pumpen, Anti-Sweat-Heizer, Steuerstrom und transkritische CO₂-Betriebskennfelder fehlen.
- [F] COP verwendet Raum-/Außentemperaturen statt Verdampfungs-/Verflüssigungstemperaturen, nimmt 60 % Carnot und Deckel 6 als universell. Kältekreisverluste, Ejektor-/Boosterzustände und Teillastkennfelder werden nicht belegt. Bei Cryo ist sogar die Untergrenze 0,5 größer als `cop_base=0,4`.

### 3.7 Rot: TES und FCR sind keine vermarktbare Batterie

[F] TES startet halb geladen und kann aus derselben surplus-Kälte zusätzlich geladen werden, die schon die Zone vorkühlt. **Keine Entladefunktion**, kein gekoppelter Wärmetauscher-/Pumpen-/Verlust-/Leistungs-/Temperaturbereich, keine terminale Gleichheit. Der Diagnose-Lauf endet z. B. Datacenter bei 32.000 statt initial 16.000 kWh. Ein bloßer Ladezähler beweist keine Reservebereitstellung.

[F] FCR wird aus Verdichter-Nennleistung × **angenommenen 15 €/MW/h** × Frequenzschwellen × Eventfaktor berechnet. Beide Outputfelder `fc_earnings_eur` und `droop_revenue_eur` erhalten **denselben Wert**; [Fleet-CLI](crates/fleet/src/main.rs) addiert sie. **Doppelzählung:** Datacenter 200.844 € enthält zweimal 100.422 €. Auch der halbierte Betrag bleibt hypothetisch: tatsächliche Zuschläge, verfügbare symmetrische Leistung, Messung, Abrufe, PQ und Kosten fehlen. FCR-Kapazitätsvergütung hängt nicht von diesem selbstgebauten Frequenz-Eventfaktor ab [R1].

[F] Die Steuerung sieht Stundenfrequenzen, nicht eine validierte schnelle Regelreserveantwort. Nennleistung ist nicht gleichzeitig nach oben/unten verfügbare Regelleistung. Die 1-MW-Mindestgebotsgröße kann durch Aggregation erreicht werden, ersetzt aber nicht Präqualifikation [R1–R2]. **Kein FCR-/aFRR-/mFRR-/Redispatch-Erlös in der konservativen Finanzierung.**

### 3.8 Rot: Gateway-Sicherheitsnachweis reicht nicht für einen realen Laden

Positiv [F]: Auth-startup fail-closed, einzelne finite/plausibility gates, Modbus-Timeout/Reconnect, Emergency-Latch, Beobachtbarkeit, Prüfungen zu FC06 und min-on/off. **Preisalterprüfung ist vorhanden, aber kein wirksamer Arbitrage-Schutz nachgewiesen:**

[F] Bei `price_stale` leert der [Gateway-Hauptlauf](crates/gateway/src/main.rs#L276-L307) den MPC-Lookahead, übergibt aber weiterhin den Preis an den Controller. Die am 12.10.2026 ausgeführte Probe gegen die unveränderten öffentlichen Funktionen `ControlEngine::update_price`, `set_mpc_lookahead` und `decide` ergab: Standardkonfiguration mit `c_th=8.000.000.000 J/K`, Luft −22 °C, Umgebung 25 °C, Verdichter zuvor OFF, 50 Hz, Regelzeit 7.200 s, leerer Lookahead → bei **10 €/MWh `Arbitrage`, ON, Sollwert −24 °C**; bei **110 €/MWh `Arbitrage`, OFF, Sollwert −18 °C**. Die Probe zeigt den Controllerpfad, keinen vollständigen Netzwerkausfalltest; ein leerer Lookahead sperrt Arbitrage nicht.

[F] Zusätzlich ruft der [Preis-Cache-Fallback](crates/gateway/src/live_data.rs#L294-L302) erneut `update_price` auf und setzt damit `price_updated_at` auf jetzt; `load_last_price` berücksichtigt das gespeicherte `saved_at` nicht. Alter Cache-Bezug kann dadurch als frisch erscheinen. Die Logmeldung „arbitrage/MPC suppressed“ ist kein Nachweis dieser Schutzwirkung.

Weitere offene/konkrete Grenzen:

1. **Kommunikationsverlust:** Der Gateway setzt intern ON/Emergency. Bei fehlendem Kommunikationsweg kann er diesen Befehl aber nicht physisch übertragen. Ein zuletzt OFF geschalteter PLC braucht **lokalen Deadman/Timeout, autonome Normalregelung und unabhängige Schutzketten**. Fake-PLC `/health` beweist keine Kühlung.
2. **Ungültige frische Daten:** In [Hardwarezweig](crates/gateway/src/main.rs) wird bei frischen, unplausiblen Messungen die letzte gute Messung genommen, ohne dort `stale_fail_safe=true` zu setzen; Empfangszeit kann fortlaufend frisch sein. Startup ohne erste gute PLC-Messung verwendet Modellwerte. Nicht als fail-safe validiert.
3. **Zeitbasis:** Walltime hängt an `live_data.enabled`, nicht an Hardwaremodus. In der mitgelieferten E2E-Konfiguration ohne Live-Daten kann Hardwarebetrieb die beschleunigte Simulationszeit für Governor verwenden. API-Test in permanentem Emergency zeigt keinen korrekten Normalbetrieb mit realen 10-/15-Minuten-Locks.
4. **Bestätigung:** FC05-Echo ist Protokollantwort, kein Verdichterlauf-/Druck-/Leistungsfeedback; bei FC05 wird die echo-Adresse nicht so wie bei FC06 geprüft. Kein unabhängiger Messkanal für tatsächliche Kälteleistung.
5. **VFD:** Gateway-Modell und Finanztracking rechnen ON mit Nennleistung; Speed wird nicht als gemessene elektrische Arbeit integriert. Keine vollständige Frequenz-/Speed-/Ölrückführungsfreigabe des konkreten Verdichters. Beispiel-Ölpulsintervall ist 0, also deaktiviert.
6. **RTU:** Codec/CRC vorhanden; `run_modbus_loop` verbindet auch bei `protocol=rtu` weiterhin den TCP-Client. **Keine geprüfte serielle Transportimplementierung**, nicht „RS485 fertig“ behaupten.
7. **Temperatur:** Cargo-Zustand geschätzt, nicht echte Produkttemperaturmessung; Konfigurationsgrenze −17,5 °C ist kein allgemeiner HACCP-/TLMV-Nachweis.
8. **Datensicherheit:** Modbus TCP selbst ohne Auth/TLS; Mutations-Token ohne zeitgebundene Session sind bei Verlust wiederverwendbar. Netzwerksegmentierung, VPN, Zertifikate, Rechteentzug, Firmwarepatches und Audittrail erforderlich. Standard-API-Bind `0.0.0.0` ist kein Internet-Sicherheitsnachweis.
9. **Betrieb:** Alarmzustellung, OT-Change-Prozess, lokale Handbedienung, Wartungsmodus, Sensorfehlererkennung, Abtau-Sperren, Strom-/Prozessausfall, Wiederanlauf und Rebound sind noch standortbezogen nachzuweisen. Sicherheit darf nicht nur auf Pi/Gateway/App-Verfügbarkeit beruhen.

## 4. Aktuelle Rechnungen: reproduziert, aber nicht als Realität verkauft

### 4.1 Historischer Pitch und aktueller CLI-Output

Historische Datei [20.07.2026](benchmarks/fleet_results_v2_20260720_114808.csv): `aldi` 24.809,05 €, `edeka` 49.534,81 €, `marktkauf` 176.256,62 €. Daraus `400*24.809,05 + 5*49.534,81 + 2*176.256,62 = 10.523.807,29 €/a`. Kein Marktvolumenbeleg; andere historische CSVs skalieren sogar Hunderttausende Typinstanzen, die keine deutsche LEH-Grundgesamtheit darstellen.

**Neu ausgeführt: alle 21 Typen, je Anzahl 1, konservativer Modus, eingecheckter Cache.** Werte in MWh/a bzw. €/a; Dezimalpunkt entspricht CSV. „Reserve“ ist die fehlerhafte addierte FCR/Droop-Modellspalte, kein tatsächlicher Umsatz.

| Typ | Baseline MWh | Actual MWh | Baseline € | Actual € | „Ersparnis“ € | „Reserve“ € |
|---|---:|---:|---:|---:|---:|---:|
| aldi | 644.321 | 218.119 | 37076.30 | 11954.35 | 25121.95 | 0.00 |
| edeka | 1377.048 | 519.374 | 79143.45 | 29393.32 | 49750.13 | 0.00 |
| marktkauf | 4997.093 | 1941.618 | 286937.25 | 110101.48 | 176835.78 | 0.00 |
| freezer | 1868.561 | 585.168 | 107078.75 | 23565.56 | 83513.19 | 0.00 |
| cold | 896.476 | 354.130 | 52052.30 | 9245.35 | 42806.96 | 0.00 |
| frozen | 1066.722 | 375.722 | 61087.66 | 16245.50 | 44842.17 | 0.00 |
| pharma | 131.659 | 39.009 | 7617.07 | 1288.09 | 6328.98 | 10042.20 |
| cryo | 5322.064 | 3276.545 | 307651.07 | 177198.00 | 130453.07 | 117159.00 |
| immersion | 9317.455 | 7074.280 | 538853.60 | 396618.25 | 142235.34 | 167370.00 |
| mortuary | 60.767 | 25.560 | 3511.57 | 762.76 | 2748.81 | 0.00 |
| datacenter | 9495.998 | 5843.783 | 549233.56 | 325777.91 | 223455.65 | 200844.00 |
| chemistry | 16803.318 | 7325.567 | 972253.58 | 399889.72 | 572363.86 | 301266.00 |
| industry | 21149.118 | 10308.111 | 1224661.79 | 561379.00 | 663282.79 | 401688.00 |
| facility | 587.909 | 197.477 | 33704.17 | 8355.03 | 25349.14 | 0.00 |
| warehouse | 2067.222 | 834.754 | 116584.36 | 41035.13 | 75549.24 | 0.00 |
| blast_freezer | 2173.024 | 720.422 | 125238.81 | 23568.83 | 101669.98 | 0.00 |
| brewery | 888.093 | 323.116 | 51442.15 | 11602.99 | 39839.16 | 66948.00 |
| arena | 6271.522 | 2112.358 | 355591.94 | 110898.28 | 244693.65 | 0.00 |
| marine | 3150.663 | 1193.335 | 180547.08 | 48813.55 | 131733.53 | 0.00 |
| ripening | 194.422 | 90.177 | 11368.42 | 2832.42 | 8536.00 | 0.00 |
| hvac | 1003.999 | 452.454 | 58316.60 | 19323.53 | 38993.07 | 0.00 |

CSV rundet unabhängig; einzelne Differenzen um 0,01 € sind deshalb normal. Current Retailflotte aus gerundeten CSV-Werten: **10.651.202,21 €**, nicht empirisch verdient. `--volatile`: `aldi` 26.135,74 € statt 25.121,95 €; das ist kein Upgrade, sondern unterdrückt Teile der Shed-Schutzlogik und bleibt durch `clamp` künstlich begrenzt.

### 4.2 Gleicher-Physik-Gegenlauf: diagnostischer Kostenunterschied ohne Endzustandsausgleich

[D] Für jede Typinstanz wurden zwei identische Zonen erstellt. A hielt immer `t_setpoint`; B erhielt die unveränderte konservative `decide`-Sollwertfolge. Identischer Cache, COP-Funktion, Defrost und elektrische Bilanzgrenze (nur Verdichter) entfernen die große Baseline-Asymmetrie. **Die 460,39 €/a bei `aldi` sind ein diagnostischer Kostenunterschied ohne Endzustandsausgleich, kein bereinigter Arbitragewert.** Unterschiedliche Temperatur-/Speicherendzustände sowie die übrigen Physik-/Sensor-/Kostenmängel bleiben bestehen.

| Typ | A: ohne Preissteuerung MWh | B: geregelt MWh | A: Spotkosten € | A−B € ohne Endzustandsausgleich | ΔE B/A % |
|---|---:|---:|---:|---:|---:|
| aldi | 218.575 | 218.119 | 12414.74 | **460.39** | −0.209 |
| edeka | 519.458 | 519.374 | 29656.98 | **263.66** | −0.016 |
| marktkauf | 1941.725 | 1941.618 | 110784.98 | **683.50** | −0.006 |
| freezer | 594.497 | 585.168 | 33824.69 | 10259.12 | −1.569 |
| cold | 324.111 | 354.130 | 18228.64 | 8983.29 | **+9.262** |
| frozen | 379.927 | 375.722 | 21482.82 | 5237.33 | −1.107 |
| pharma | 38.659 | 39.009 | 2219.90 | 931.81 | +0.904 |
| cryo | 3275.116 | 3276.545 | 189323.73 | 12125.73 | +0.044 |
| immersion | 7083.990 | 7074.280 | 415071.68 | 18453.42 | −0.137 |
| mortuary | 25.163 | 25.560 | 1441.58 | 678.82 | +1.577 |
| datacenter | 5843.691 | 5843.783 | 337989.89 | 12211.98 | +0.002 |
| chemistry | 7324.523 | 7325.567 | 423802.84 | 23913.12 | +0.014 |
| industry | 10303.417 | 10308.111 | 596630.10 | 35251.10 | +0.046 |
| facility | 200.049 | 197.477 | 11396.32 | 3041.29 | −1.286 |
| warehouse | 830.848 | 834.754 | 46710.46 | 5675.33 | +0.470 |
| blast_freezer | 710.990 | 720.422 | 40716.03 | 17147.20 | +1.327 |
| brewery | 325.148 | 323.116 | 18693.38 | 7090.39 | −0.625 |
| arena | 2112.417 | 2112.358 | 120693.19 | 9794.91 | −0.003 |
| marine | 1205.783 | 1193.335 | 68194.95 | 19381.41 | −1.032 |
| ripening | 86.146 | 90.177 | 5056.03 | 2223.61 | +4.679 |
| hvac | 450.512 | 452.454 | 26167.70 | 6844.18 | +0.431 |

**Endzustände, im Audit am 12.10.2026 nach 8.760 Stunden gemessen (Modellzustände, keine Feldtemperaturen):**

| Typ | Luft A/B °C | Cargo A/B °C | TES A/B kWhth |
|---|---:|---:|---:|
| aldi | −21,910905 / −21,721879 | −21,919158 / −21,731097 | 0 / 0 |
| edeka | −21,884827 / −21,843153 | −21,891435 / −21,857897 | 0 / 0 |
| marktkauf | −21,885372 / −21,871973 | −21,890762 / −21,876169 | 0 / 0 |
| datacenter | 22,000000000 / 21,999999812 | 22,000000000 / 21,999985023 | 16.000 / 32.000 |

[D] Bei `aldi` entspricht `(T_B−T_A)*C_zone/3.600.000` mit **8 Milliarden J/K** rund **420,06 kWhth höherem thermischem Energieinhalt** in B (wärmer, weniger gespeicherte Kälte). Das ist keine elektrische Nachkühlenergie oder Eurokorrektur; Cargo wird wegen fehlender energetischer Rückkopplung nicht zusätzlich aufaddiert. Beim Datacenter bleibt A auf dem initialen TES-Stand, B endet voll geladen; der Ladezähler ist keine validierte Speicherbilanz. **Weder ein Ausgleichslauf noch eine monetäre Endzustandskorrektur wurde durchgeführt.** Abschnitt 12.2 gibt diese Endzustände für alle 21 Typen aus.

407-Retailinstanzen: `400*460,3872 + 5*263,6616 + 2*683,5028 = 186.840,19 €` **diagnostischer Kostenunterschied ohne Endzustandsausgleich**. Das entspricht etwa **1,75 % des bisher ausgewiesenen 10,65-Mio.-Vorteils**, nicht einem belegten Rest-Arbitragewert oder 98,25 % sicher widerlegter realer Flexibilität. Auch `cold` zeigt einen Verbrauchsanstieg; der Gegenlauf ist nicht pauschal energie-neutral.

## 5. Physikalische Machbarkeit außerhalb dieses Modells

### 5.1 Ja, aber mit räumlich und zeitlich begrenzter Flexibilität

[F] Ein realer US-Supermarkt wurde in neun DR-Tests untersucht [P1, ES-1/ES-2, S. v–vii; Messungen S. 20–33]. In den wirksamsten Tests rund **10 kW für über zwei Stunden**, bei beeinflusstem Anteil unter 20 % der Systemkapazität. Einzeltests stark unterschiedlich; Tabelle ES-2 enthält auch einen negativen mittleren Effekt. Kein deutscher CO₂-Feldnachweis.

**Nicht mit Messung verwechseln:** Die oft zitierten **15–20 kW für 1,5 h**, länger mit Vorkühlung, sind Ergebnisse kalibrierter Modelle für bestimmte TK-Möbel **ohne Speiseeis**, nicht universelle gemessene Filialflexibilität. ES-3 enthält Vorkühllast ca. 5,5–6,9 kW × 4 h und Rebound ca. 5,3–6,9 kW × 2,75–4 h. Eventabsenkung darf nicht ohne diese Phasen als gesparte Energie gerechnet werden.

[F] Fraunhofer FlexKälte untersuchte zehn heterogene Standorte einschließlich Supermarkt und Kühlhaus; genannte **35–54 % Strombeschaffungskosten** sind mathematische Optimierungsergebnisse mit Kälteerzeugung/-speichern, keine im Projekt erreichten Feldwerte [P2]. **Im selben Projekt steigt bei Kosten-/CO₂-Minimierung die bezogene Spitzenlast deutlich**, weil die Kältemaschine häufig mit Volllast läuft; in den Hauptszenarien wurde sie bewusst auf das 1,5-Fache des maximalen Kältebedarfs dimensioniert. Das ist ein Szenariobefund, kein universeller Anstieg jeder Anlage. Separates Peak-Shaving senkte die Spitzenlast, erzielte unter den dort angesetzten Day-ahead-Preisen aber keine Bezugskostenreduktion. Die 35–54 % belegen deshalb nicht gleichzeitig eine Leistungspreisersparnis.

[F] Flexality-Anbieterberichte nennen 15–20 % und mehrhundert-kW-Flexibilität in **Kühlhäusern** [P3]. Das stützt Markt-/Technikrelevanz, beweist aber weder unabhängige Rendite noch Übertragbarkeit auf kleine Supermärkte.

### 5.2 Energiebilanz, die der Pilot erfüllen muss

```text
C_air*dT_air/dt = Qwall + Qinfiltration_sensible + Qinfiltration_latent
                 + Qpeople/lights/fans + Qgoods_to_air + Qdefrost
                 - Qevaporator - Qstorage_exchange
C_product*dT_product/dt = Qair_to_product + Qnew_product
P_meter = Pcompressors + Pfans + Ppumps + Pdefrost + Panti_sweat + Pcontrol
Qstorage_end - Qstorage_start muss in der Vergleichsbilanz berücksichtigt werden.
```

Fakten müssen durch mindestens Zwei-/Mehrzonenidentifikation und Unterzähler ersetzt werden. Thermalmasse `m*cp*ΔT` ist nur ein oberer Energieinhalt; zugängliche Leistung wird durch Wärmeübergang, kritische kleine Packungen, Öffnungen und lokale Regelgrenzen begrenzt. Vorkühlung kann COP und Wärmeeintrag verschlechtern, kühlere Verflüssigungsumgebung kann helfen – Nettoeffekt messen. Keine Geschwindigkeit von FCR aus einem Stunden-RC-Lauf ableiten.

### 5.3 Energie-Größenordnungen statt Fantasieläden

[F] EHI für Food-Handel, Verbrauchsjahr 2024: **289 kWh Strom/(m² Verkaufsfläche·a)**, davon **52 % Kälte** [P4]. [D] Orientierung Kälte: **150,28 kWh/(m²·a)**; 1.000 m² → 150,28 MWh/a, Jahresmittel **17,16 kW** für gesamte Kältetechnik. 2.000 m² → 300,56 MWh/a, 34,31 kW. **Nicht Nenn-/Peak-/Shedleistung.** DACH-Befragung, kein ausschließlich deutscher Filialmessdatensatz; Einzelformate streuen.

[D] Die `aldi`-Baseline 644,321 MWh entspricht bei diesem Kälte-Orientierungswert etwa 4.287 m² Verkaufsfläche; Actual 218,119 MWh etwa 1.451 m². Dies ist nur ein Dimensionscheck, keine Flächenrekonstruktion. Gebäudevolumen und Verkaufsfläche dürfen nicht gleichgesetzt werden. Ein voller Markt, eine einzelne TK-Anlage und ein Kühlhaus sind unterschiedliche Gegenstände.

## 6. Temperaturrecht, Anlagen- und Betriebssicherheit

### 6.1 Keine „freie Temperaturspanne“ aus der gesetzlichen Ausnahme ableiten

- [F] **TLMV § 2 Abs. 4:** TK-Ware bis Abgabe an allen Punkten grundsätzlich **−18 °C oder kälter**; begrenzte Abweichungen bis 3 K unter ausdrücklich genannten Bedingungen [L1]. **Nicht als planmäßiges Arbitragebudget von −18 bis −15 °C ansetzen.** Standortspezifische Warenqualität/HACCP kann strenger sein.
- [F] **TLMV § 2a / VO 37/2005:** Lufttemperaturüberwachung/-aufzeichnung, bestimmte kleine Einzelhandels-Ausnahmen; Normbezüge EN 12830/13485/13486 [L2–L3]. Luftmessung und Temperatur aller Produktpunkte sind unterschiedliche Pflichten/Nachweise.
- [F] **VO 852/2004 Art. 5, Anhang II Kap. IX:** HACCP, Kühlketten-/Überwachungs-/Korrekturpflichten [L4]. **Kein universelles „+6 °C ist für gekühlte Ware sicher“.** Produktkennzeichnung, empfindlichste Ware, Haltbarkeit und Verfahren entscheiden.
- [F] VO 853/2004 hat z. B. im Kontext Hackfleischherstellung spezifische **2 °C Kern** bzw. **4 °C Fleischzubereitung** [L5]. Nicht ohne Anwendungsprüfung jede Verkaufsstätte diesem Einzelparagraphen gleichsetzen; ebenso wenig pauschal eine permissivere Grenze unterstellen.
- [U] Volle aktuelle Normtexte sind hier nicht geprüft. EN 12830 betrifft Messequipment; EN 378 Kälteanlagen-/Umweltsicherheit, nicht eine allgemeine Lebensmitteltemperaturfreigabe [L6].

### 6.2 Dynamik und lokale Regelhoheit

OEM-Regler behalten Verdichterstufung, Druckschutz, Expansionsventil-/Überhitzungsregelung, Ölmanagement, Mindestdrehzahl, min-on/off, Abtauung, Wiederanlauf und Schutzketten [D4]. Externe Befehle vorzugsweise nur als **freigegebene begrenzte Supervisory-Anforderung**, z. B. dokumentierter Lastabwurfeingang oder Sollwertoffset. Nicht blind Kompressorschütz, EEV, Sicherheitsgrenzen oder Ölalgorithmus ersetzen.

[U] Allgemeingültige „600 Sekunden“ oder „15 % Mindestdrehzahl“ sind nicht nachgewiesen. Die Werte des konkreten Verdichters/Racks sind erforderlich. Verteilte Möbel dürfen nach einem Event nicht synchron wieder voll anlaufen; Rebound kann neues Jahresmaximum schaffen. Bei CO₂ sind Druck-/Receiver-/Hochdruckzustände relevant, bei R290 Brand-/Installationsanforderungen; reine Software macht sie nicht irrelevant.

[F] BetrSichV § 3 verlangt Gefährdungsbeurteilung und Prüfung relevanter Änderungen [L7]. Welche Druckanlagen-/Prüfpflichten konkret greifen, hängt von Anlage/Einstufung ab. [F] F-Gas-Recht betrifft fluorierte Kreise; **R744 ist kein F-Gas**, gemischte Anlagen sind gesondert zu prüfen [L8]. Das ersetzt nicht Fachbetrieb, Betreiberfreigabe oder Druck-/Arbeitssicherheit.

**Pilot-Stop-Regeln:** kritische Produkt-/Luftgrenze einschließlich Messunsicherheit erreicht; Temperaturanstiegsprognose unzuverlässig; Sensor alt/unplausibel/stuck; Druck-/Öl-/Verdichteralarm; laufende Abtauung oder Wartung; Rückmeldung/Befehl widersprüchlich; Kommunikationsverlust; Rebound nähert sich Leistungsgrenze. Bei Stop: **lokal autonom freigegebener Normalbetrieb**, nicht lediglich „Gateway sagt ON“.

## 7. Wer deutsche Läden steuert – und was öffentlich tatsächlich erlaubt ist

**Keine belastbaren Marktanteile pro Controller gefunden.** Filialkette, Anlagenbauer und Reglerhersteller sind nicht dasselbe. Ein Markt kann mehrere Generationen/Protokolle und steckerfertige Einzelgeräte neben Verbundkälte betreiben. Protokollfähigkeit ≠ beschreibbarer Datenpunkt ≠ Integrationsvertrag ≠ Hersteller-/Betreiberfreigabe.

| Anbieter/System | Öffentlich dokumentiert [F] | Schreibzugriff/Grenze | Passung des jetzigen ATA |
|---|---|---|---|
| **Danfoss ADAP-KOOL; AK-SM 800A, AK-PC/AK-CC** | XML-Drittschnittstelle, EDx/EPK beschreiben Parameter und R/W; Modbus-Konfiguration; Systemmanager koordiniert, Packregler regelt Verdichter [D1–D4] | Strict XML verlangt HTTPS/Token; Parameter-/Rollen-/Firmwareabhängigkeit. **REST nicht belegt.** Modbus-Konfigurationsseite beweist keinen externen TCP-Write-Server mit ATA-Coilmap | Kein XML-Adapter; nur kundenspezifisches TCP-PLC-Mapping könnte nach Freigabe passen |
| **Eckelmann E*LDS CI 4x00, Virtus 5, UA-Regler, LDS Win** | CAN-Systembus; Modellunterschied CI4000/4100(+Modbus)/4400(+LAN); CI4x00 abgekündigt; LDS-Win-Fernparametrierung; Handbuch-TOCs enthalten Modbus/LSM [E1–E3] | TOC kein Write-Nachweis; Modbusrolle/Register/LSM-Trigger und Rechte offen. „UA“-Gerätename **nicht als OPC UA** interpretieren | Kein E*LDS/CAN/LDS/XML-Adapter; Herstellerbeschreibung der konkreten offenen Punkte erforderlich |
| **Wurm FRIGOLINK/FRIGOENTRY, FRIGODATA** | C-BUS; DCC-BL Lastabwurf/Fastreturn per Digitaleingang, Sollwertschiebung; ANI-1F3 dreistufiger Lastabwurf; SAC-Parametersperre [W1–W2] | Autorisierte FRIDA-/FRIGODATA-Parametrierung belegt, aber keine generische Drittanbieter-Cloud-API. Geeigneter Lastabwurfeingang ist **bedingt plausibler Integrationsweg**, erst nach Freigabe | ATA-FC05 kann über freigegebene PLC/Relais-Anforderung adaptiert werden; nicht direkt beliebige Reglercoil unterstellen |
| **Wurm MULTIGATE / C2C** | Älteres MULTIGATE nennt Modbus/TCP/BACnet/IP; C2C-Modbus RTU kann 16 Datenpunkte in externe Slaves schreiben; C2C-MOD-S GEN2 nennt Abfragen von Ist-/Soll-/Statuswerten [W3–W5] | Schreiben in angeschlossene Slaves ≠ Schreiben in Wurm-Regler. C2C-MOD-S-Webseite belegt externe Reads, **Writes nicht nachgewiesen**; nicht automatisch „nur Lesen technisch möglich“ behaupten | RTU-Transport fehlt; Versions-/Rollen-/Mappingprüfung statt pauschaler Kompatibilität |
| **CAREL boss/boss one, c.pCO/pCO, MPXPRO** | boss-one BMS Modbus/BACnet/SNMP; Supervisor zusätzlich XML/MQTT; Benutzerprofile/Variablenkonfiguration; programmierbare pCO- und Feldregler [C1–C3] | OEM-Anwendung und konkrete R/W-Liste bestimmen Punkte; MPX regelt auch EEV/Abtauung/Lüfter. Keine pauschale externe Write-/Lizenzfreigabe belegt | Nur dokumentiertes kundenspezifisches Modbus-TCP-Mapping wäre unmittelbar nutzbar; kein BACnet-/CAREL-Adapter |
| **Copeland/Emerson Dixell XWEB EVO/PRO** | Monitoring/Steuerungsplattform und Gerätebibliotheken; verlinkte Modellhandbücher [X1] | Detail-PDFs nicht auslesbar; Rollen/Register/Lizenz/Freigabe offen. **[U] EVO-Software-Supportende März 2026 nicht im Volltext bestätigt**: offizieller Bulletin-Link/Suchindex vorhanden, zitierte Product-News-HTML bestätigt den Termin beim Audit nicht; Modellumfang/Folgen offen [X2] | Kein geprüfter XWEB-/Dixell-Adapter |
| **AHT/Daikin, Epta/Viessmann, ehem. Carrier Commercial** | Reale deutsche Anlagen-/Möbelanbieter; Daikin Conveni-Pack nennt Modbus-BMS-Anbindung [I1–I3] | Anlagenmarke sagt nicht, welcher Controller exponiert ist; registerweise R/W offen. Carrier-Commercial-Kältegeschäft 2024 an Haier verkauft [I4] | Kein pauschales Anlagen-„Plug and play“ |
| **Siemens BMS/SCADA** | Supervisory-Integration etwa via SORIS in einem ukrainischen Supermarkt [I5] | **Kein deutscher Referenzbeleg aus dieser Quelle**, keine Write-Freigabe aus Monitoring ableiten | Eigenes BMS-/Kälte-Mapping erforderlich |

### Konkrete Deutschlandbelege und ihre Reichweite

- **Danfoss:** aktiv & irma, Oldenburg-Kreyenbrück, Kältetechnik Schön, CO₂/Multi-Ejektor und **AK-SM 800**, nicht 800A [D5]. Belegt historische reale Familie, keine 800A-Quote.
- **Wurm:** EDEKA-Center Wendorf/Voerde mit FRIGOLINK, Bericht 2010; EDEKA-Nordbayern-Interview 2025 zur Wurm-Nutzung [W6]. „900 betreute Objekte“ ist **nicht** „900 Wurm-Installationen“.
- **AHT/Daikin:** EDEKA Lüning/Paderborn, 1.300 m², 2024, 37 CO₂-Möbel und 24 R290-Plug-ins [I1]. Kein Register-/Schreibnachweis.
- **Epta:** offizielle deutsche EDEKA-Schneider-Referenz im Pressearchiv 2023 [I2]. Hersteller-Energiebehauptung nicht als unabhängig gemessene Kennzahl übernommen.
- **Eckelmann, CAREL, Dixell:** relevante Retail-Portfolios belegt; konkrete deutsche modellspezifische Filialzählung/Referenz in dieser Recherche nicht verifiziert. Das heißt **nicht**, dass sie nicht eingesetzt werden.

### Was Hersteller „erlauben“: fehlender Beweis ist kein Verbot

Die Recherche beweist öffentliche Funktionen, **keine allgemeine Vertrags-/Gewährleistungsfreigabe für ATA**. Benötigt je Standort: Modell/Seriennr/Firmware/OEM-Anwendung; Bus-/Netzplan; Server-/Clientrolle; R/W-Objektliste samt Skalierung; Servicekonto mit minimalen Rechten; Lizenzen/Cloudgebühren; OEM-/Kältebauer-/Betreiberfreigabe; zulässige Offsetgröße/-rate; Vorrang/Timeout/Quittierung; Rückfall; Testplan und Supportzuständigkeit. Nicht öffentlich bestätigte Writes sind für den Businesscase **nicht verfügbar gerechnet**, aber auch nicht grundsätzlich technisch unmöglich erklärt.

[D1/D6] Danfoss dokumentiert reale Sicherheitslücken vor Version 4.3.1 (u. a. post-auth RCE). Firmwareversion und neuere Advisories prüfen, Managementnetz nicht öffentlich exponieren. Fernzugriff gratis/Browser erreichbar heißt nicht „Integration lizenzfrei“: Alsense-Zugang ist abonnementgebunden; kein belastbarer Preis gefunden.

## 8. Wie viele Läden können von Fixpreis zu EPEX wechseln?

### 8.1 Ehrliche Antwort: nicht zählbar aus den verfügbaren Quellen

[F] EHI zählt **36.565 Lebensmittelgeschäfte 2024** [T1]: 15.976 Discounter, 10.700 Supermärkte, 1.300 große Supermärkte, 7.900 Kleinflächen; **689 SB-Warenhäuser nur rechnerisch als Rest**. Engere Groß-/Supermarkt-/Discounterbasis ohne Kleinflächen: **28.665**. [F] BMEL-Statistik nennt aktuell **rund 36.870 für 2025** [T2]. Nicht mit 25.840 Handelsunternehmen oder Fachhandelsunternehmen addieren. Unterschiedliche Jahre, keine exakte Inventarliste.

[U] Weder Zahl aktuell spotfähiger Standorte noch Zahl tatsächlich umgestiegener Standorte wurde öffentlich verifiziert. Dafür fehlen verknüpfte Daten zu Marktlokation, Vertragspartner, Messung, Kündigung, Konzernbeschaffung, Lieferantenannahme und marginaler Spotpreisweitergabe. **Nicht 36.565 oder 36.870 als verkaufbaren ATA-TAM ausgeben.**

### 8.2 Die sieben verschiedenen Hürden

1. **Recht:** § 41a Abs. 2 EnWG verpflichtet seit 01.01.2025 alle Stromlieferanten zu dynamischen Angeboten für Letztverbraucher mit **iMSys** [T3]. Das ist nicht nur für private Haushalte, aber auch keine Annahmepflicht für jeden Antragsteller [T4].
2. **Messung:** RLM liefert zeitaufgelösten Lastgang; **RLM ist nicht automatisch iMSys**. Gesetzlicher Angebotsanspruch und vertraglich freiwilliges Gewerbe-Spotangebot sind getrennt. Das Angebot coneva Dynamic nennt **RLM und mindestens 100.000 kWh/a**, ohne neue Hardware [T5]. Keine allgemeine iMSys-Pflicht für jeden individuellen RLM-Gewerbevertrag daraus ableiten.
3. **Vertrag:** Fixpreisvertrag kann feste Laufzeit/Kündigungsfristen haben. Dynamischer Wunsch schafft kein beliebiges Sonderkündigungsrecht. Technischer 24-h-Wechsel beendet keinen laufenden zivilrechtlichen Vertrag [T4/T6].
4. **Entscheider:** Stromkunde kann Kette, Regionalgesellschaft, selbstständiger Kaufmann, Vermieter oder Einkaufsverbund sein. Ladenleiter darf nicht zwingend kündigen.
5. **Beschaffung:** PPA, Forward-Hedge, Mischbeschaffung, zentrale Rahmenverträge und residualer Spotbezug unterscheiden sich. **Auch ohne Lieferantenwechsel** könnte bestehender Lieferant Spot-/Flexkomponente vereinbaren; Umstieg allein ist kein ATA-Beitrag.
6. **Lieferant:** Bonität, Jahresmenge, Prognose-/Bilanzierungsprofil, Region und MaLo müssen akzeptiert werden. Öffentliches Angebot ≠ unterschriebener Vertrag.
7. **Marginaler Preis:** Für Optimierung zählt die Kostenänderung einer zusätzlichen/verschobenen kWh. Spot im Konzernportfolio mit intern festem Filialpreis kann wirtschaftlich **keinen direkten Filialanreiz** bieten; Zuständigkeit und Settlement klären.

[F] EDEKA nennt 2025 rund **3.200 selbstständige Kaufleute, 5.629 Märkte im selbstständigen Einzelhandel, insgesamt 10.871 Lebensmittelmärkte**; EVG bündelt/organisiert Stromversorgung [T7]. Das sind weder 5.629 unabhängige sofort wechselnde Stromverträge noch 10.871 ATA-kompatible Regler.

[F] EHA als REWE-Tochter koordiniert Energie und beschreibt Versorgung deutscher REWE-Standorte seit 2008; Selbstständigenmodell existiert [T8]. ALDI SÜD beschreibt Regionalgesellschaften und rund 2.000 Filialen [T9]. Schwarz/RWE dokumentieren **250 GWh/a PPA ab 2028 für zehn Jahre** sowie gewisse Mietobjekt-Beschaffungsbindungen [T10]. PPA bedeutet nicht automatisch jede Filiale vollständig vom Spot abgekoppelt; weder 0 % noch 100 % freie Wechselquote ist daraus ableitbar.

[F] iMSys-Rolloutpflicht >6.000 kWh und tatsächlicher Rollout sind verschieden; für >100.000 kWh behandelt § 45 MsbG eine spätere Ausstattungsstaffel [T11]. **Keine Haushalts-/Gesamt-iMSys-Quote auf Supermärkte übertragen**.

### 8.3 Bedingte Sensitivität, ausdrücklich keine Marktstatistik

Besser als angenommene unabhängige Prozentfaktoren: eine **standortweise gemeinsame Eignungszählung**. Messung, Konzernentscheidung und Jahresverbrauch sind korreliert.

```text
eligible_i(H) = passende Messung UND akzeptierter Spotvertrag UND berechtigter Entscheider
                UND Vertragsende/Änderungsrecht im Horizont H
                UND freigegebener Kältezutritt UND positiver Netto-Businesscase
N_eligible(H) = Summe eligible_i(H)
```

Nur reine Größenordnungsarithmetik mit engerer EHI-Basis 28.665:

| Angenommene gemeinsam geeignete Quote [A] | Standorte [D], gerundet | Beweisstatus |
|---:|---:|---|
| 1 % | 287 | Nicht gemessene Sensitivität |
| 5 % | 1.433 | Nicht gemessene Sensitivität |
| 10 % | 2.867 | Nicht gemessene Sensitivität |

Ein Prozentpunkt entspricht 286,65 Standorten; **keine dieser Quoten ist eine Einschätzung der tatsächlichen Quote**. Selbst Zahl der zum Spot wechselbaren Standorte ist größer als Zahl der wirtschaftlich sinnvollen ATA-Anlagen. Direkt mit EVG/EHA, 2–3 Regionalgesellschaften und selbstständigen Kaufleuten gezielt Vertrags-/Zählerdaten erheben; nicht Ladenleiter-Selbstauskunft als Rechtsnachweis verwenden.

## 9. Reale wirtschaftliche Rechnung: Bilanzumfang und konservative Szenarien

### 9.1 Vier Vergleiche statt eines vermischten Sparprozents

A = heutige Anlage + heutiger Fixvertrag; B = heutige Anlage + Spotvertrag ohne ATA; C = identische Anlage + Spotvertrag mit ATA; D = nachgerüstete Anlage + ATA.

- **A−B:** Tarif-/Beschaffungswechsel, Risikoübergang; nicht automatisch ATA-Wert.
- **B−C:** echte Regelungsarbitrage und deren Zusatzverbrauch/-kosten.
- **C−D:** separate Nachrüstungseffizienz/-flexibilität minus Hardwareinvestition.
- Leistungspreis und Reserve erst ergänzen, wenn tatsächlich kompatibel, nachgewiesen und nicht schon enthalten.

Für intervalldiskrete Nettokosten:

```text
Kosten = Σ E_i * (p_spot,i + p_supplier,i + p_gridwork,i + p_tax/levies,i)
         + billed_peak_kW * demand_price_EUR_per_kW_year
         + fixed_meter/supplier_cost + O&M + real annualized_CAPEX
Nettonutzen = Kosten_referenz - Kosten_ATA + vertraglicher Netto-Marktumsatz
```

Bei gleicher Energiemenge und unveränderten konstanten Aufschlägen canceln diese für reine Verschiebung. Bei **Mehrverbrauch** fallen sie zusätzlich an; bei echter kWh-Einsparung können variable All-in-Bestandteile mitgespart werden. Negativer EPEX-Preis heißt deshalb nicht „für Stromverbrauch bezahlt werden“. Umsatzsteuer nur passend zur Vorsteuerabzugsfähigkeit behandeln.

[D] Wenn Spotanteil 90 €/MWh und variable übrige Bestandteile 110 €/MWh betragen [A], entsprechen 10 % Einsparung nur auf Spot bei gleicher Energie **4,5 %** des 200-€/MWh-Arbeitspreises, noch weniger der Gesamtrechnung inkl. Grund-/Leistungspreis. Reale Tarife und Entlastungen können stark abweichen. Branchenpreise nicht als konkretes Angebot einsetzen.

**Wechsel-Risiko:** [A] Bei 500 MWh/a Gesamtbezug bedeuten 20 €/MWh schlechterer beschaffungsgewichteter Preis **10.000 €/a Mehrkosten**. 97,50 € Kältearbitrage gleichen das nicht aus. Historischer einfacher Jahresmittelpreis ist kein eigener lastgewichteter Einkaufspreis oder Preisforecast.

### 9.2 Nicht ausgemessen: garantierter Planungswert bleibt null

Ohne Standortdaten/Freigaben/Vertrag ist **0 € gesichert erzielbarer Zusatzumsatz/-nutzen** der einzige belegbare Finanzierungsansatz. Das ist keine Behauptung „erwartete Ersparnis aller Anlagen = 0“, sondern eine Evidenzgrenze. Nachstehende Werte sind **bedingte konservative Designrechnungen**, keine aktuellen Messwerte oder statistischen Confidence-Intervals.

### 9.3 Beispielmodelle reiner Arbitrage nach Rebound/COP-Strafe

Alle Inputs [A]: absenkbare **elektrische** Leistung `P` (nicht Nennleistung), Eventdauer `h`, Events/a `n`, tatsächlich realisierter zeitnaher Spread `s`; zusätzliche Energiequote `r`; zusätzlicher Bezugskostenwert **150 €/MWh all-in variabel**; separate jährliche O&M. Es gilt:

```text
E_shift = P*h*n/1000                       [MWh/a]
E_extra = r*E_shift
G = E_shift*s - E_extra*150                [€/a vor O&M]
N = G - O&M                               [€/a vor CAPEX]
N_kunde_bei_50%-Fee = 0,5*G - O&M          [nur wenn Kunde O&M trägt]
```

| Bedingtes Szenario | P/h/n | s €/MWh | r | E_shift MWh | G €/a | O&M €/a | N €/a | Kunde nach 50 %-Fee und O&M |
|---|---|---:|---:|---:|---:|---:|---:|---:|
| Kleiner/schwacher Markt | 10 kW / 1 h / 100 | 20 | 10 % | 1 | 5,00 | 1.000 | −995,00 | −997,50 |
| Konservativer Markt | 10 kW / 2 h / 150 | 40 | 5 % | 3 | **97,50** | 1.000 | **−902,50** | −951,25 |
| Günstiger Markt, immer noch bedingt | 20 kW / 2 h / 200 | 60 | 3 % | 8 | 444,00 | 1.000 | −556,00 | −778,00 |
| Zusätzliche freigegebene Flexibilität | 30 kW / 2,5 h / 200 | 60 | 3 % | 15 | 832,50 | 1.500 | −667,50 | −1.083,75 |
| Größeres TK-Lager, nicht „normaler Supermarkt“ | 100 kW / 3 h / 200 | 60 | 5 % | 60 | 3.150,00 | 3.000 | 150,00 | −1.425,00 |

10 kW/2 h ist ein **nicht übertragbar garantiertes** Beispiel in Nähe eines Feldbefundes [P1]. 150/200 Events und Spreads sind **nicht empirisch ermittelt**. Freigegebene Fenster, Feiertage, Temperaturzustand und Preisverfügbarkeit können sie reduzieren. Kraftvolle TK-Lager haben andere Flexibilität als NT-Möbel. Das TK-Lager-Beispiel amortisiert angenommene 20.000 € CAPEX ohne 50 %-Fee erst nach **133,3 Jahren**, also in diesem Fall ebenfalls kein attraktiver Case.

[Sensitivität] Am konservativen Punkt: +10 €/MWh Spread = +30 €/a; +1 Prozentpunkt Mehrenergie = −4,50 €/a; 0 % statt 5 % Mehrenergie erhöht G lediglich auf 120 €/a. Bei Fixpreis ohne zeitvariablen Bezug ist `s=0`, G bei Mehrverbrauch negativ. Opportunitätskosten einer alternativen effizienteren Normalregelung separat prüfen.

### 9.4 Integration ist nicht kostenlos

[U] Öffentliche belastbare Preise für standortspezifische OEM-Lizenz, Write-Freigabe, Softwarefee oder Aggregatoranteil wurden nicht gefunden. Keine Preisangaben ausdenken. **Nur Budget-Sensitivitäten [A], netto ohne USt, keine Angebote:**

| Kostenblock | Beispielspanne | Nachweis vor Auftrag |
|---|---:|---|
| Industriegateway, Stromversorgung, OT-Netz/Schrankarbeit | 500–2.000 € einmalig | Stückliste/Einbauangebot, Ausfallschutz |
| Unterzähler, Sensoren/Produktsimulatoren, Kalibrierung | 500–3.000 € einmalig | Messplan/Genauigkeit/Messstellenvertrag |
| Mapping, OEM-/Kältebauerengineering, Abnahme/HACCP | 2.000–10.000 € einmalig | Stundensatz, Scope, Freigabe; Entwicklungskosten können deutlich höher sein |
| Sicherheits-/Produktvalidierung und Changeprozess | 1.000–5.000 € einmalig | Keine regulatorische Freigabe aus diesem Budget ableiten |
| Gesamter grober Individualpilot | **4.000–20.000 €** | Nicht plus eine zweite pauschale CAPEX-Zahl doppelt zählen |
| Betrieb/Netz/Monitoring/Service/Rezertifizierung | **500–3.000 €/a** | Vertrag; Testtabellen verwenden 1.000/1.500/3.000 € |
| OEM-/Cloud-/Schreiblizenz, Versicherung, Gewährleistung | **Unbekannt** | Noch zusätzlich, soweit nicht im Angebot enthalten |
| Flottenadapter-/Softwareentwicklung, Vertrieb, Rechtsberatung | **Unbekannt** | Nicht in Standortdeckungsbeitrag verstecken |

Warenverlust, Kälteserviceeinsatz und SLA-Verstöße nicht mit 0 budgetieren: `expected_loss = Ereigniswahrscheinlichkeit * Schaden`. Nur illustrative Sensitivität [A]: 1 % × 20.000 € = **200 €/a**, bereits größer als 97,50 € Arbitrage. Keine empirische Wahrscheinlichkeit und keine akzeptable Sicherheitsgrenze; die zulässige Lebensmittelgefährdung wird nicht durch erwarteten Gewinn kompensiert.

### 9.5 50 % Umsatz ist nicht 50 % Gewinn

[D] Die Hälfte der **460,39 €/a diagnostischen Kostendifferenz ohne Endzustandsausgleich** wäre rechnerisch 230,20 €; bei 400 Typinstanzen rund 92.077,44 € (aus ungerundeten 460,3872 €, siehe 4.2). **Das ist kein Fee-/Umsatznachweis:** Abrechenbar wäre nur separat nachgewiesene Ersparnis, vor Hosting, Service, Vertrieb, Entwicklung und Risiko. Falls ATA O&M trägt, verliert ATA im konservativen 97,50-€-Szenario bei 1.000 € O&M etwa **951,25 €/Standort/a**; falls Kunde O&M trägt, verliert Kunde diesen Betrag. Der Kostenträger muss im Vertrag ausdrücklich stehen.

[D] Für Kunde mit 5.000 € CAPEX, gewünschter einfacher 3-Jahres-Amortisation, 1.000 €/a O&M und 50 %-Fee benötigt man `G ≥ 2*(1.000 + 5.000/3) = **5.333,33 €/a**`. Bei netto realisiertem 40−0,05*150 = 32,50 €/MWh erfordert das **164,10 MWh/a verschobene Energie**. Das übersteigt sogar den gesamten EHI-orientierten Kältejahresverbrauch eines 1.000-m²-Marktes. Kein mathematisches Verbot für jede Großanlage, aber klarer Größenordnungsalarm für den kleinen Laden.

## 10. Was durch konkrete Änderungen/Upgrades bedingt erreichbar wäre

**Kein „Bonusprozentsatz“ auf alte 60-%-Sparwerte.** Softwaremodell reparieren erhöht vor allem Wahrheit/Verlässlichkeit, nicht zwangsläufig reale Einsparung. Hardware-/Betriebsmaßnahmen separat bilanzieren.

| Änderung | Bedingung | Quantifizierte bedingte Wirkung | Nicht zulässig zu behaupten |
|---|---|---|---|
| Gleiche Physik/Baseline; echte 15-min-Preise und vollständige Bilanz noch nachzuweisen | Identische Anlage/Servicequalität, Provenienz, Zeitstempel, Rebound, Endzustandsausgleich | Bisherige Retaildiagnose: 186.840 statt 10.651.202 € für 407 Typinstanzen, **Kostenunterschied ohne Endzustandsausgleich**, weiterhin alter Cache/nur Verdichter | „Reparatur erzeugt 186.840 € reale Ersparnis“ |
| OEM-Supervisory/Lastabwurfeingang, Messung, Produktsimulator, abgestimmte Sauggruppen | Schriftliche Freigabe, Feldtest, begrenzte Offsets, lokaler Timeout | Beispiel 10 → 30 kW, 2 → 2,5 h, 150 → 200 Events: **97,50 → 832,50 €/a** reine Arbitrage vor O&M | Diese Mehrflexibilität sei ohne Messung vorhanden |
| Optimierte Saugdruck-/Abtau-/Heizer-/Tür-/Nachtrollosteuerung | Gegen heutige gute Regelung gemessen, nicht gegen künstlich verschlechterte Baseline | **Angenommene 5 %** echter kWh-Gewinn auf 150,28 MWh × 200 €/MWh = **1.502,80 €/a** | 5 % als belegte allgemeine Einsparquote; doppelt als Arbitrage verrechnen |
| Rebound-aware Viertelstunden-Peak-Shaving | Lokaler Leistungstarif und tatsächlich niedrigeres Jahresmaximum inkl. aller Lasten | **Angenommene 15 kW × 150 €/kW·a = 2.250 €/a** | Jeden Event separat mit Leistungspreis vergüten |
| Physischer Zusatzspeicher/PCM | Garantierte Temperatur-/Wärmetauscherleistung, Lade-/Entladeeffizienz, Platz, CAPEX | **100 kWhth** bei COP 2 → ideal 50 kWhel; 200 Zyklen → 10 MWh/a; Spread 60, r=10 %, Bezug 150 → **450 €/a vor Speicherbetrieb** | 100 kWhth als 100 kWhel verkaufen; Kostenlose Speicherladung |
| VFD-/Rack-Nachrüstung | Eignung, Ölrückführung, Teillastkennfeld, heutiger Anlagenzustand | Nur derselbe ausdrücklich angenommene 5-%-Effizienzblock oben, wenn das der Nachweis ist | Nochmals 30 % on top addieren |
| FCR via Aggregator | PQ, symmetrische Leistung, Latenz/Messqualität, Liefervertrag, Preis-/Kostenrisiko | Formel `MW_awarded * hours_awarded * clearing_price`; **0 € finanziert bis Nachweis** | Modell-15-€/MW/h oder Coilreaktion als Zuschlag |

**Zusatzspeicher-Investitionsgrenze [D]:** Bei 450 €/a, angenommen 100 €/a Speicher-O&M bleiben 350 €/a. Für 5-Jahres-einfache-Amortisation max. **1.750 € investierbar**, vor Fee/Finanzierung/Wärmetauscherkosten. Kein Marktpreis für PCM und keine Zusage, dass so ein Speicher dafür erhältlich ist.

### Bedingtes kombiniertes Markt-Paket, nicht prognostiziert

[A] 1.000 m², Kältebasis 150,28 MWh/a; vollständiges OEM-/Mess-/Effizienz-/Peak-Paket, **15.000 € CAPEX**, **1.500 €/a O&M**, kein Reserveertrag. Angenommene, **nach Maßnahmen gemeinsam verifizierte** Ziele:

```text
Arbitrage nach Mehrenergie:                    832,50 €/a
5 % zusätzliche echte Energieeffizienz:      1.502,80 €/a
Gesicherte Jahrespeakreduktion:               2.250,00 €/a
G vor O&M:                                   4.585,30 €/a
N nach O&M, ohne Gewinnteilung:               3.085,30 €/a
Einfache Amortisation:                        4,86 Jahre
Kunde nach 50 % Fee auf G und O&M:              792,65 €/a
Kundenamortisation mit dieser Fee:            18,92 Jahre
```

Die drei Vorteile sind **nur addierbar**, wenn Effizienz nicht schon in Arbitrage enthalten ist, Flexleistung **nach** Effizienzmaßnahme verfügbar bleibt, Vorkühlung/Rebound keinen neuen abrechnungsrelevanten Peak erzeugen und keine Verträge dieselbe Flexibilität anderweitig binden. Der FlexKälte-Spitzenlastbefund in 5.1 unterstreicht diesen Zielkonflikt; er belegt nicht das gemeinsame 4.585,30-€-Paket. Bei nur Peak-Shaving wäre G=2.250; nach 1.500 O&M 750 €, nach 50 %-Fee und O&M −375 €. Diese Aufteilung offenbart, dass in diesem Szenario die Tarif-/Peakseite viel wichtiger ist als reine Preisarbitrage.

407 identische tatsächlich geeignete Märkte wären bei diesem **hypothetischen gemeinsamen** Paket G=1.866.217,10 €/a; Fee 933.108,55 €, Kundenüberschuss nach Fee/O&M 322.608,55 €, Kunden-CAPEX 6.105.000 €. Kein Beleg für 407 geeignete Läden, keine Standortsicherheits-/Portfolioausfallgarantie und keine Kosten der zentralen Entwicklung enthalten. Nachrüstungsbudget kann je Markt über 15.000 € liegen; dann verschlechtert sich Amortisation.

**Strategischer Schluss:** Nicht auf „universelle virtuelle Batterie“ als erstes Produkt wetten. Ein begrenztes Angebot „Messung + OEM-freigegebene Optimierung + nachgewiesene Peakvermeidung“ für **einen** Reglerstack oder größere TK-Lager hat nachvollziehbarere Hebel. Hersteller bieten bereits Saugdruck-/Abtau-/Fernoptimierung; ATA muss den **zusätzlichen** Wert gegenüber deren gutem Betrieb zeigen [D4/P3].

## 11. Nachweisplan mit echten Go/No-go-Schranken

### Gate 0 – Kunde und Tarif, vor Hardwarekauf

- Letzte 12–24 Monate Rechnungen und mindestens 15-min-Netzlastgang; Stromvertrag mit Arbeit-/Leistungs-/Grundpreisen, Laufzeit/Kündigung und Entscheidungsträger.
- Schriftliches Gewerbe-Spotangebot oder Änderungsvereinbarung, MaLo/Messsystem, all-in-Bestandteile, Bilanzierungs-/Managementfee und Risiko.
- Anlageninventar, Controller/Firmware, BMS-Netzplan, OEM-/Kältebauer-Writefreigabe und Lizenzangebot.
- Waren-/Sauggruppen, HACCP-Temperaturen, Sensorplatzierung/Messunsicherheit, Abtauung, realer Bestand.

**No-go:** Kein rechtlich/technisch zulässiger Eingriff oder kein positiver Businesscase nach realen Gesamtkosten. Monitoring kann als separat bepreistes Produkt trotzdem möglich sein; nicht gratis aus Projektpotenzial ableiten.

### Gate 1 – Read-only und Modellidentifikation

Mindestens 4–8 Wochen Shadow-Mode [A: geplante Pilotdauer, kein universeller Standard], inklusive Öffnung, Warenwechsel und Abtauung. Unterzähler/Tempdaten; keine Produktsteuerung. Raum-/Produktsimulator-Dynamik, Wärmelast und COP/Leistungskennfelder schätzen; Hold-out gegen andere Tage. Historische Preisreihen mit realen UTC-Zeitstempeln, DST und Liefervertrag zusammenführen. Sommer-/Winter-/Preisjahrvariation gesondert, keine perfekte Zukunftswetterkenntnis.

**No-go:** Modell unterschätzt kritische Aufwärmung oder Sensor-/Energieerfassung unzuverlässig. Niedrige RMSE allein reicht nicht; gefährliche Worst-Case-Unterschätzung prüfen.

### Gate 2 – begrenzter supervised Pilot

Nur freigegebene Gruppen, keine pauschale Pluskühlungsfreigabe. Kältebauer anwesend/erreichbar, lokale autonome Rückfallregelung, klare Stop-Kriterien. Kommunikationsunterbrechung, Gatewayausfall, ungültige frische/stuck Messung, Reboot, Alarme, Abtauung, Aktor-Quittierung und Rebound testen. Temperaturen nicht clampen, Überschreitung zählt als Fehler.

**Go-Kriterium:** Keine zulässige Produkt-/Anlagengrenze verletzt, zuverlässiger Rückfall, messbare elektrische Flexibilität inklusive Nachlauf. Nicht „0 Grenzverletzungen über 2 Wochen = garantiert sicher für immer“.

### Gate 3 – wirtschaftlicher Nachweis

Zeitlich vergleichbare/randomisierte A/B-Fenster, Baseline mit Wetter, Auslastung, Ware, Öffnung und Abtauung; gleicher Liefervertrag und gleiche Energie-Bilanzgrenze. Alle Vorkühl-/Reboundintervalle zählen, Speicher- und Produkt-Endzustände vergleichbar. Kosten aus echten Rechnungsregeln rekonstruieren. Konfidenz-/Unsicherheitsbereich statt nur Punktwert; Messunsicherheit und Preis-/Tarif-/Saisonsensitivität getrennt.

**Go-Kriterium:** Vorab vereinbarte Amortisation/Deckungsbeitrag nach Fee, O&M, CAPEX und Risiko erreicht; Kunde kann Nutzen nachvollziehen. Nicht 50 % eines ungemessenen Modellwerts fakturieren.

### Gate 4 – erst dann Flotte/Reserve

Eigene Filialvariabilität, homologe Adapter, Support/IT-Policy, Fleet-Rebound/gleichzeitige Ausfälle, Jahrespeak und zentrale Beschaffung berücksichtigen. Reserve nur mit Aggregator und tatsächlicher PQ. Vertragsstacking/Verfügbarkeit dokumentieren. Eine Installation erlaubt keine Hochrechnung auf jede Kette.

## 12. Reproduzierbarkeit ohne Produktcodeänderung

### 12.1 Workspace und CLI

Vom Projektroot in Bash:

```bash
cargo check --locked --workspace
cargo test --locked --workspace
cargo clippy --locked -p thermal-arbitrage-mvp --all-targets -- -D warnings
# Cache wird relativ zum CWD gelesen: ausdrücklich im Fleet-Verzeichnis starten.
(cd crates/fleet && cargo run --locked -p thermal-fleet -- \
  --out-dir tmp/sanity/reproduce \
  '1 aldi + 1 edeka + 1 marktkauf + 1 freezer + 1 cold + 1 datacenter + 1 industry')
# Lauf mit allen 21 Namen entspricht der Tabelle in 4.1.
```

`--out-dir` kann CSV-Dateinamen nach Zeitstempel verändern, nicht die deterministischen Werte dieses Cache-Laufs. Nicht aus Root mit beliebigem `data_cache` starten und behaupten, derselbe Datensatz sei benutzt. Unterschiedliche Compiler können letzte Rundungsstellen ändern. Für die originalen historischen Zahlen wäre auch der damalige Code-/Konfigurationsstand nötig; ihre CSV ist nicht derselbe Nachweis wie heutiger CLI-Lauf.

### 12.2 Eigenständige gleiche-Physik-Diagnose

Folgendes Pythonprogramm im Projektroot erzeugt im ignorierten Arbeitsbereich einen kleinen Rust-Harness, importiert **die bestehenden** Typ-/Physik-/Controlmodule und führt sie aus. Keine Bibliotheksänderung/Installation erforderlich. Es berechnet Tabelle 4.2 **als diagnostische Kostenunterschiede ohne Endzustandsausgleich**, ohne die unfaire Finanzbaseline und ohne FCR; auf stderr gibt es zusätzlich Luft-/Cargo-/TES-Endzustände aus. Standardlibrary, Python 3 und `rustc`; Format ist CSV mit Dezimalpunkt.

```python
import json, pathlib, subprocess
root = pathlib.Path.cwd()
out = root / 'tmp' / 'sanity-reproduce'
out.mkdir(parents=True, exist_ok=True)
hours = json.loads((root/'crates/fleet/data_cache/fleet_2025.json').read_text())['hours']
assert len(hours) == 8760
(out/'hours.tsv').write_text('\n'.join(
    f"{x['amb']}\t{x['price']}\t{x['freq']}" for x in hours), encoding='utf-8')
src = r'''
#[path="../../crates/fleet/src/store_types.rs"] mod store_types;
#[path="../../crates/fleet/src/physics.rs"] mod physics;
#[path="../../crates/fleet/src/control.rs"] mod control;
fn main() {
    let input = std::fs::read_to_string(std::env::args().nth(1).unwrap()).unwrap();
    let data: Vec<Vec<f64>> = input.lines().map(|s|
        s.split('\t').map(|x| x.parse().unwrap()).collect()).collect();
    println!("type,noopt_mwh,optimized_mwh,noopt_eur,optimized_eur,diagnostic_cost_difference_eur");
    for t in store_types::STORE_TYPES {
        let mut a = physics::create_zone(t, data[0][0]);
        let mut b = a.clone();
        let mut g = control::AsctGovernor::new();
        let mut integral = 0.0;
        let (mut ea,mut eb,mut ca,mut cb) = (0.0,0.0,0.0,0.0);
        for h in 0..8760 {
            a.amb_c = data[h][0]; b.amb_c = data[h][0];
            let look: Vec<(f64,f64)> = data[h..(h+12).min(8760)].iter()
                .map(|x| (x[1],x[0])).collect();
            let prices: Vec<f64> = look.iter().map(|x| x.0).collect();
            let d = control::decide(t,&b,h,data[h][1],data[h][0],data[h][2],
                &look,&mut g,integral,&prices,0.0,15.0,false);
            integral = control::compute_thermal_integral(
                b.temp_c,t.t_setpoint,integral,1.0,0.99);
            let ra = physics::simulate_hour(t,&mut a,h,t.t_setpoint,false,0.0,false);
            let rb = physics::simulate_hour(t,&mut b,h,d.setpoint_c,
                d.tes_charging,d.vfd_speed,true);
            g.update_shed_timer(d.compressor_on);
            ea += ra.compressor_energy_mwh; eb += rb.compressor_energy_mwh;
            ca += ra.compressor_energy_mwh*data[h][1];
            cb += rb.compressor_energy_mwh*data[h][1];
        }
        println!("{},{:.6},{:.6},{:.4},{:.4},{:.4}",t.name,ea,eb,ca,cb,ca-cb);
        eprintln!("END {} airA={:.9} airB={:.9} cargoA={:.9} cargoB={:.9} TES_A_kWhth={:.6} TES_B_kWhth={:.6}",
            t.name,a.temp_c,b.temp_c,a.cargo_temp_c,b.cargo_temp_c,
            a.charge_j/3_600_000.0,b.charge_j/3_600_000.0);
    }
    let t = &store_types::STORE_TYPES[0];
    let mut a = physics::create_zone(t,25.0); let mut b = a.clone();
    let x = physics::simulate_hour(t,&mut a,0,t.t_setpoint,false,0.0,false);
    let y = physics::simulate_hour(t,&mut b,0,t.t_setpoint,false,1.0,true);
    eprintln!("VFD energy0={} energy1={}",x.compressor_energy_mwh,y.compressor_energy_mwh);
    let mut s = t.clone(); s.cooling_power_kw=0.0; s.internal_heat_kw=10000.0;
    let mut z = physics::create_zone(&s,40.0); let before=z.temp_c;
    physics::simulate_hour(&s,&mut z,0,s.t_setpoint,false,0.0,false);
    let q = s.internal_heat_kw*1000.0 + (40.0-before)/z.r_th;
    let unclamped = before + q*z.r_th*(1.0-(-3600.0/z.tau).exp());
    eprintln!("CLAMP unclamped={} returned={}",unclamped,z.temp_c);
}
'''
(out/'audit.rs').write_text(src, encoding='utf-8')
exe = out/'audit.exe'  # Dateiendung funktioniert hier auch als expliziter Ausgabename.
subprocess.run(['rustc','--edition','2021',str(out/'audit.rs'),'-o',str(exe)], check=True)
r = subprocess.run([str(exe),str(out/'hours.tsv')], check=True,
                   capture_output=True, text=True, encoding='utf-8')
(out/'audit.csv').write_text(r.stdout, encoding='utf-8')
print(r.stdout)
print(r.stderr)
```

### 12.3 Bedingte wirtschaftliche Szenarien

```python
cases = [
 ('schwach',10,1,100,20,.10,1000),
 ('konservativ',10,2,150,40,.05,1000),
 ('guenstig',20,2,200,60,.03,1000),
 ('upgrade',30,2.5,200,60,.03,1500),
 ('tk_lager',100,3,200,60,.05,3000),
]
for name,p,h,n,spread,extra,om in cases:
    e = p*h*n/1000
    gross = e*spread - e*extra*150
    print(name, 'MWh',e,'vor_OM',gross,'nach_OM',gross-om,
          'Kunde_50pct_Fee',gross*.5-om)
# Kombiniertes Paket: keine Reserve, alle drei Ziele gleichzeitig vorausgesetzt.
g = 832.5 + (1000*289*.52/1000)*.05*200 + 15*150
assert abs(g-4585.3) < 1e-8
print('Paket',g,'netto',g-1500,'Jahre',15000/(g-1500))
```

Diese Formeln liefern reproduzierbare bedingte Zahlen. Sie ersetzen keine Ermittlung der tatsächlichen Inputs. Bei einer zukünftigen Überarbeitung des Produktcodes ändern sich Diagnosewerte; HEAD und Datensatzhash mitführen.

## 13. Quellenregister – URL, Fundstelle und Beweisgrenze

**Quellenabruf grundsätzlich 11.10.2026**, durch Web-Researcher und/oder direkten Web-Leser; Audit-Nachprüfungen vom 12.10.2026 bei P2/X2, Rechtsgegenprüfungen vom 12.10.2026 (N1–N4) ausdrücklich vermerkt. Datum im Titel ist Veröffentlichungs-/Dokumentdatum, nicht Abrufdatum. Herstellertexte sind Primärquellen zu Produktbeschreibung, keine unabhängige Leistungsgarantie. Wo Text nicht zugänglich war, ausdrücklich als Verzeichnis/TOC markiert. Die Rechtswiedergaben auf legislation.gov.uk sind zugängliche EU-Textkopien; aktuelle konsolidierte deutsche/EUR-Lex-Fassung vor rechtlicher Freigabe prüfen.

### Markt/Preise/Regelreserve

- **M1:** [BNetzA – Strommarktdaten 2025, 05.01.2026](https://www.bundesnetzagentur.de/SharedDocs/Pressemitteilungen/EN/2026/20260104_SMARD.html), Abschnitt „Wholesale electricity prices“: 89,32 €/MWh, 573 Negativ-/40 >300-Stunden. Volltext gelesen; keine Prognose.
- **M2:** [Energy-Charts V1 echter Tagesdatensatz 01.10.2025](https://api.energy-charts.info/price?bzn=DE-LU&start=2025-10-01&end=2025-10-01), Arrays/Einheit/Lizenz direkt geprüft; [offizielle API-Referenz](https://api.energy-charts.info/llms-full.txt), [OpenAPI](https://api.energy-charts.info/openapi.json). V1 parallel arrays; V2 anderes Format; keine vollständige Standortjahresrechnung damit durchgeführt.
- **M3:** [SMARD – 15-minute wholesale prices](https://www.smard.de/en/15-minute-wholesale-prices-available-218078), Umstellung ab 01.10.2025; [EU-Kommission](https://energy.ec.europa.eu/news/eu-electricity-trading-day-ahead-markets-becomes-more-dynamic-2025-10-01_en).
- **M4:** [BNetzA – Preisbestandteile](https://www.bundesnetzagentur.de/DE/Vportal/Energie/PreiseAbschlaege/Tarife-table.html), [Netzentgelte](https://www.bundesnetzagentur.de/EN/RulingChambers/Chamber8/RC8_06_Network%20charges/RC8_06_Network%20charges.html), Arbeits-/Leistungs-/sonstige Bestandteile; kein lokaler Tarifpreis daraus übernommen.
- **M5:** [BDEW-Strompreisanalyse](https://www.bdew.de/service/daten-und-grafiken/bdew-strompreisanalyse/), [SMARD Industriepreise](https://www.smard.de/page/en/topic-article/5892/216044/industrial-electricity-price-trends), Verbrauchs-/Entlastungsabhängigkeit; nicht als Supermarkt-Angebot eingesetzt.
- **R1:** [Amprion – Balancing Capacity in Germany](https://www.amprion.net/Energy-Market/Market-Platform/Control-Energy/), „Prequalification“, „Procurement“, „Activation“: 1 MW Mindestgebot, Vergütungsunterscheidung. Volltext direkt gelesen.
- **R2:** [Regelleistung – Präqualifikation](https://www.regelleistung.net/en-us/Become-a-balancing-service-provider/Prequalification), [50Hertz – technische Anforderungen](https://www.50hertz.com/en/Market/Balancingreserve/Technicalrequirements); Portalseiten, kein PQ-Dossier dieses Projekts geprüft. [SMARD Redispatch](https://www.smard.de/page/en/topic-article/212250/214108/redispatch), nicht pauschaler Zusatzmarkt für den Laden.

### Physik/Feldstudien/Benchmarks/Wettbewerb

- **P1:** [Hirsch et al., NREL/TP-5500-65009, Oktober 2015, vollständiger Bericht](https://www.osti.gov/servlets/purl/1226469), ES-1/2/3, S. v–ix; Messungen S. 20–33, Modellierung S. 38–41, Grenzen S. 42–48. Text zugänglich; reale Tests und Simulation klar getrennt.
- **P2:** [Fraunhofer UMSICHT – FlexKälte-Projektabschluss, 08.02.2024](https://www.umsicht.fraunhofer.de/en/press-media/press-releases/2024/cooling-supply-energy-balancing.html), „Cooling sector ...“, zehn Standorte, Modelloptimierung und Heterogenität. Volltext direkt gelesen. [FlexKälte – Ergebnisse Szenarienrechnungen](https://flexkaelte.de/flx-kaelte-wiki/zentrale-ergebnisse/), „Ergebnisse Hauptszenarien“, Volltext im Audit am **12.10.2026** gelesen: Kosten-/CO₂-Minimierung erhöht Spitzenlast bei 1,5-facher Dimensionierung; separates Peak-Shaving ohne Day-ahead-Bezugskostenreduktion in diesen Szenarien.
- **P3:** [EM-Power – Flexality, 23.06.2025](https://www.em-power.eu/news/start-up-story-flexality), [Anuga FoodTec – Interview 2026](https://www.anugafoodtec.com/magazine/ai-optimises-electricity-costs-in-cold-storage-facilities.php), [Anbieter-Produkt](https://flexality.de/en/solutions-alt/plant-schedules-alt/). Anbieter-/Interviewbefunde, keine öffentlich offengelegten unabhängigen M&V-Resultate/Preise.
- **P4:** [EHI – Investitionen in effiziente Technologien, 25.11.2025](https://www.ehi.org/presse/investitionen-in-effiziente-technologien/), „Energieverbrauch“ und „Datenbasis“: 289 kWh/m², 52 % Kälte, 46 Unternehmen/>42.000 DACH-Filialen. Volltext direkt gelesen; keine individuelle Anlagen-Shedleistung.
- **P5:** [EHI – Mehr gekühlte Waren, 16.11.2021](https://www.ehi.org/news/immer-mehr-gekuehlte-waren-im-supermarkt/), Flächenanteile Plus/Minus/Theken – **keine Stromaufteilung**. [LBNL industrielle Kühllager 2011](https://www.osti.gov/servlets/purl/1050706), standortabhängige DR; [RTF Kühllager](https://rtf.nwcouncil.org/demand-response-refrigeration-warehouse-controls/), grobe Literatur-/Expertenpotenziale, keine deutschen LEH-Messwerte.

### Temperatur-/Anlagensicherheit

- **L1:** [TLMV § 2](https://www.gesetze-im-internet.de/tlmv/__2.html), Abs. 4 Produktanforderung/Ausnahmen. Direkt gelesen.
- **L2:** [TLMV § 2a](https://www.gesetze-im-internet.de/tlmv/__2a.html), Luftmessung/Ausnahmen.
- **L3:** [VO 37/2005 Art. 2](https://www.legislation.gov.uk/eur/2005/37/article/2), Aufzeichnung/Normbezüge; keine vollständige Normprüfung.
- **L4:** [VO 852/2004 Art. 5](https://www.legislation.gov.uk/eur/2004/852/article/5), [Anhang II Kap. IX](https://www.legislation.gov.uk/eur/2004/852/annex/II/chapter/IX), HACCP und Kühlkette.
- **L5:** [VO 853/2004 Anhang III Abschnitt V](https://www.legislation.gov.uk/eur/2004/853/annex/III/section/V), Kapitel III, spezifischer Herstellungskontext.
- **L6:** [AREA Einführung EN 378](https://area-eur.be/publications/introduction-refrigeration-standard-en-378), [EN-12830-Scope](https://standards.iteh.ai/catalog/standards/cen/b6c1bd4b-fafb-41ee-9f9c-e729e4ca7c97/en-12830-2018), nur Einführung/Katalog, keine vollständigen aktuellen Normtexte.
- **L7:** [BetrSichV § 3](https://www.gesetze-im-internet.de/betrsichv_2015/__3.html), [Anhang 2](https://www.gesetze-im-internet.de/betrsichv_2015/anhang_2.html), Änderung/Gefährdungsbeurteilung; anlagenspezifische Einstufung offen.
- **L8:** [EU-Kommission F-Gas-Ausrüstungspflichten](https://climate.ec.europa.eu/areas-action/fluorinated-greenhouse-gases/stakeholder-obligations/f-gases-equipment-and-products_en), [UBA Verordnung](https://www.umweltbundesamt.de/en/topics/climate-energy/fluorinated-greenhouse-gases-fully-halogenated-cfcs/statutes-regulations/eu-regulation-concerning-fluorinated-greenhouse), VO 2024/573; kein standortspezifischer Prüfbescheid.

### Netz-/Energierecht (Gegenprüfung der externen Auditliste, 12.10.2026)

- **N1:** [§ 19 StromNEV](https://www.gesetze-im-internet.de/stromnev/__19.html), Normtext direkt gelesen am 12.10.2026: Abs. 2 Satz 1 atypische Netznutzung mit **Untergrenze 20 %** des veröffentlichten Netzentgelts (⇒ rechnerisch max. 80 % Minderung, kein Anspruch); Satz 2/3 stromintensive Netznutzung mit ≥ 7.000 h **und** > 10 GWh und 20/15/10 % Untergrenze. Ausdrücklich **nicht** verwandt: ein behaupteter Regelfall von 80 % Rabatt.
- **N2:** [BNetzA – Individuelle Netzentgelte Strom](https://www.bundesnetzagentur.de/DE/Beschlusskammern/BK04/BK4_71_NetzE/BK4_71_Ind_NetzE_Strom/BK4_Ind_NetzEntg_Strom.html), atypische Netznutzung/lastzeitbezogene Voraussetzung und Prüfweg; [50Hertz – Hochlastzeitfenster](https://www.50hertz.com/de/Transparenz/Kennzahlen/Netzdaten/Hochlastzeitfenster), VNB-spezifisch und prognostiziert. Kein bundesweit einheitlicher Zeitplan belegt; der Proxy in Abschnitt 14 ist deshalb ausdrücklich nur ein Prüfmaßstab.
- **N3:** [BNetzA – Integration steuerbarer Verbrauchseinrichtungen](https://www.bundesnetzagentur.de/DE/Vportal/Energie/SteuerbareVBE/start.html), direkt gelesen am 12.10.2026: Anwendungsbereich „mehr als 4,2 kW“ Netzanschlussleistung, 4,2 kW als Mindestleistung im Steuerungsfall, abweichende Berechnung für große Wärmepumpen/Klimaanlagen, Niederspannungsbezug. Die Einordnung **gewerblicher Kälteanlagen** war über die zugänglichen Seiten nicht abschließend entscheidbar.
- **N4:** [§ 14a EnWG](https://www.gesetze-im-internet.de/enwg_2005/__14a.html); [BNetzA-Pressemitteilung 27.11.2023](https://www.bundesnetzagentur.de/SharedDocs/Pressemitteilungen/DE/2023/20231127_14a.html): Festlegung vom 27.11.2023, Regelungen ab **01.01.2024**, Übergangsregeln für Bestandsanlagen.

### Hersteller und Zugänge

- **D1:** [Danfoss System Manager FAQ](https://www.danfoss.com/en/service-and-support/fix-and-troubleshooting/cooling-installer-hub/electronic-controllers-services-faqs/system-manager-faq/), EDx R/W, XML, Modbus-Konfiguration, IP, Ethernet-Lizenz. Direkt gelesen.
- **D2:** [AK-SM 800A Software/Security](https://www.danfoss.com/de-de/service-and-support/downloads/dcs/adap-kool-software/ak-sm-800a/), Remote XML, Strict HTTPS/Token; [StoreView/Alsense](https://www.danfoss.com/de-de/products/dcs/monitoring-and-services/storeview-web/storeview-web-full-remote-access-to-your-ak-sm800a-and-ak-sm800/), Abonnementabgrenzung.
- **D3:** [Offizielles AK-SM-Handbuch SW 5.1.x](https://assets.danfoss.com/documents/latest/590332/BC337629891709en-001101.pdf), [XML-Handbuch](https://assets.danfoss.com/documents/latest/376199/AU342138209420en-000501.pdf): **PDFs hier nicht auslesbar**, keine Detail-Registerbehauptungen daraus.
- **D4:** [Danfoss transkritische CO₂-Regelung](https://www.danfoss.com/en/service-and-support/articles/dcs/intelligent-and-adaptive-control-in-transcritical-co2-refrigeration/), Öl/MSS/ALC/Druck/Abtauung; [Packregler](https://www.danfoss.com/en/products/dcs/electronic-controls/products/pack-controllers/), [AK-PC782B IP, 30.09.2022](https://www.danfoss.com/en/about-danfoss/news/dcs/ip-communication-enabled-on-ak-pc-782b-towards-the-ak-sm-800a-series/). Keine universellen Grenzwerte/Freigabe für ATA.
- **D5:** [Danfoss Oldenburg aktiv & irma, 12.12.2019](https://www.danfoss.com/de-de/service-and-support/case-studies/cf/energy-efficient-supermarket-in-oldenburg-case-study/), ausdrücklich AK-SM800.
- **D6:** [Danfoss DSA-2025-08-01](https://www.danfoss.com/en/service-and-support/coordinated-vulnerability-disclosure/danfoss-security-advisories/dsa-2025-08-01/), betroffene Versionen <4.3.1, CVE-2025-41451/-41452, direkt gelesen. Nicht als vollständige aktuelle Schwachstellenliste ausgeben.
- **E1:** [Eckelmann CI4000-Familie](https://edp.eckelmann.de/edp/lds/object?id=35610), Modellabgrenzung/Abkündigung in Produktbeschreibung; [CI4000 Anleitung](https://edp.eckelmann.de/edp/lds/show?id=70648), [Virtus5 Anleitung](https://edp.eckelmann.de/edp/maa/show?id=74643), nur TOCs auslesbar. Keine externe Schreibsemantik bestätigt.
- **E2:** [Eckelmann LDS Win](https://www.eckelmann.de/produkte/kaelte-gebaeudeleittechnik/ldswin/), Fernwartung/Parametrierung, keine offene API-Freigabe.
- **E3:** [Eckelmann LEH-Lösungen](https://www.eckelmann.de/en/sectors-references/refrigeration-building-management-systems/), allgemeine Zielbranche, kein modellspezifischer deutscher Standortnachweis in dieser Prüfung.
- **W1:** [Wurm DCC-BL](https://infocenter.wurm.de/Product/Index/5960), Digitaleingangs-Lastabwurf, Fastreturn, externe Vorgaben C-BUS, SAC, direkt gelesen; [ANI-1F3](https://infocenter.wurm.de/Product/Index/5949), dreistufiger Lastabwurf.
- **W2:** [Wurm Datenmanagement](https://www.wurmaustria.com/en/products-solutions/data-management-software), FRIDA/FRIGODATA Nutzerrechte/VPN/Fernparametrierung; keine allgemeine Entwickler-API.
- **W3:** [MULTIGATE](https://infocenter.wurm.de/Product/Index/3386), ältere SCADA-Schnittstellen; [MULTIGATE3](https://infocenter.wurm.de/Product/Index/7076), anderes Modell/Dokumentation teils Login.
- **W4:** [C2C-Modbus](https://infocenter.wurm.de/Product/Index/3401), Modbus-RTU, 16 Write-/20 Read-Datenpunkte für externe Slaves; nicht Wurm-Regler-Writefreigabe.
- **W5:** [C2C-MOD-S GEN2](https://infocenter.wurm.de/Product/Index/7117), externe Abfragen, direkt gelesen; [C2C-REWE-MOD GEN2](https://infocenter.wurm.de/Product/Index/7116), spezifische Truhenanbindung, kein Universalinterface.
- **W6:** [TEKO/EDEKA-Wurm-Interview, 06.11.2025](https://teko-gmbh.com/en/insights/news/why-edeka-relies-on-wurm-systemy-a-behind-the-scenes-look), Kundeninterview; [KKA Voerde/Wendorf 2010](https://www.kka-online.info/artikel/kka_Energieeffizienz_ist_Pflicht-1036564.html), historische konkrete Referenz.
- **C1:** [CAREL boss one](https://www.carel.com/product/boss-one), Protokolle/Variablen/Nutzerprofile direkt gelesen; Redirect auf offizielle BMS-Seite. Keine vollständige R/W-Liste.
- **C2:** [CAREL c.pCO](https://www.carel.com/c.pco-sistema/-/journal_content/56_INSTANCE_i4q5KIMLInKK/10191/128788), programmierbarer Controller; [pCOWeb](https://www.carel.com/product/pcoweb-card), ältere Karte/Fernverwaltung.
- **C3:** [MPXPRO](https://www.carel.com/product/mpxpro-spm-), Feldaktoren/Handbuchliste; Detail-PDFs nicht auslesbar.
- **X1:** [Copeland Facility/System Controls](https://www.copeland.com/en-my/products/controls-monitoring-systems/facility-controls-electronics/facility-and-system-controls), [XWEB EVO](https://webapps.copeland.com/Dixell/Pages/XWEB_EVO), [XWEB PRO](https://webapps.copeland.com/Dixell/Pages/XWEB_PRO), Bibliotheken/Handbuchlinks, Detail-PDFs nicht auslesbar.
- **X2 [U]:** [Copeland Product News](https://webapps.copeland.com/Dixell/Pages/ProductNews) zeigte bei Audit-Abruf am **12.10.2026** nur ältere Meldungen, keinen sichtbaren März-2026-Beleg. [Offizieller XWEB-EVO-EoS-Bulletin-Link](https://webapps.copeland.com/dixell/Content/Pages/AdvertisingMaterials/ProductNews/XWEB%20EVO%20EoS%20-%20Product_Bulletin_March_2026.pdf) im Suchindex gefunden, **PDF-Volltext nicht geprüft**. Indextext deutet auf Software-Support-/Update-Ende am 16.03.2026 und separat End-of-Sales Oktober 2023 hin; Termin, vollständiger Modellumfang und Folgen bleiben unbestätigt. Nicht als verifizierte Produktabkündigung übernehmen.
- **I1:** [AHT EDEKA Lüning/Paderborn](https://www.aht.at/remote-co2-system-mit-waermerueckgewinnung/), konkrete Referenz; [Daikin Conveni-Pack](https://www.daikin.eu/en_us/product-group/refrigeration/conveni-pack.html), Modbus-BMS erwähnt, Registerrechte offen.
- **I2:** [Epta Pressearchiv](https://www.eptarefrigeration.com/en/newsroom/press-and-media/press-releases?page=2), EDEKA Schneider 13.06.2023, Referenzlisting, nicht unabhängige Effizienzmessung.
- **I3:** [Epta/Viessmann-Joint-Venture](https://www.epta-centralnortheurope.com/en/newsroom/latest-news/epta-viessmann-refrigeration-solutions-join-forces-create-new-market-leader), Unternehmens-/Markenkontext, keine Schnittstellenfreigabe.
- **I4:** [Carrier Commercial Refrigeration an Haier verkauft](https://www.carrier.com/refrigeration/en/worldwide/news/news-article/carrier-completes-775m-sale-of-its-commercial-refrigeration-business-to-haier.html), Transicold nicht gleich Commercial Refrigeration.
- **I5:** [Siemens Fozzy Group Kyiv](https://references.siemens.com/en/reference/fozzy-group?id=24721), SORIS-Supervisory, ausdrücklich kein deutscher Referenzstandort.

### Filialzahl und Tarifwechsel

- **T1:** [EHI – Preisbewusstes Einkaufen, 02.09.2025](https://www.ehi.org/presse/preisbewusstes-einkaufen-discounter-legen-zu/), „Expansion“/Definitionen, 2024 Filialbasis direkt gelesen.
- **T2:** [BMEL-Statistik LEH](https://www.bmel-statistik.de/ernaehrung/ernaehrungsgewerbe/lebensmitteleinzelhandel), 2025 ~36.870 und Unternehmens-/Filialunterschied, direkt gelesen. Keine Spot-Zählung.
- **T3:** [EnWG § 41a](https://www.gesetze-im-internet.de/enwg_2005/__41a.html), Abs. 2/3/6/7, direkt gelesen.
- **T4:** [BNetzA dynamische Stromtarife](https://www.bundesnetzagentur.de/DE/Vportal/Energie/Vertragsarten/DynStromtarife/start.html), Voraussetzungen, Vertragsfreiheit/Kündigung, Preisrisiko.
- **T5:** [coneva Dynamic Gewerbe](https://coneva.com/tarif/), FAQ RLM/100.000 kWh, Grund-/Managementfee/Umlagen, Kündigung/Lieferbeginn, direkt gelesen. Produktverfügbarkeit, **keine Empfehlung/Integration oder verbindliches Angebot**; dortige Werbeprozente nicht übernommen. [LUOX Gewerbetarif](https://www.luox-energy.de/en/gewerbe/dynamischer-stromtarif), Anbieteranforderungen unterscheiden sich.
- **T6:** [BNetzA Lieferantenwechsel](https://www.bundesnetzagentur.de/DE/Vportal/Energie/KuendigungLieferantenwechsel/Lieferantenwechsel/start.html), technischer vs. zivilrechtlicher Wechsel; [EnWG § 20a](https://www.gesetze-im-internet.de/enwg_2005/__20a.html). BNetzA benennt technischen Rollout 06.06.2025, Norm 01.01.2026; beide ändern keine Kündigungsfristen.
- **T7:** [EDEKA Geschäftsjahr 2025](https://verbund.edeka/presse/pressemeldungen/geschaeftsjahr-2025-edeka-verbund-mit-positiver-entwicklung-selbststaendige-kaufleute-bleiben-wachstumsmotor/), Kaufleute/Standorte; [EVG](https://evg.edeka/), Bündelung/Energieversorgung. Keine Filialvertragsliste.
- **T8:** [EHA Unternehmensbeschreibung](https://www.eha.net/unternehmen.html), REWE-Energieversorgung; [Selbstständig mit REWE](https://karriere.rewe.de/selbststaendig), Organisationsmodell, keine freie Spotvertragsquote.
- **T9:** [ALDI SÜD Unternehmensstruktur](https://www.aldi-sued.de/unternehmen/ueber-uns), Regionalgesellschaften/Filialen, keine Zählpunkt-/Vertragszählung.
- **T10:** [Schwarz/RWE PPA, 05.04.2023](https://gruppe.schwarz/en/press/archive/2023/rwe-green-energy-from-offshore-wind-farm), 250 GWh/a ab 2028, zehn Jahre, Zusatzquelle und Mietobjekt-Ausnahmen.
- **T11:** [MsbG § 29](https://www.gesetze-im-internet.de/messbg/__29.html), [§ 45](https://www.gesetze-im-internet.de/messbg/__45.html), [BNetzA iMSys-Rollout](https://www.bundesnetzagentur.de/DE/Fachthemen/ElektrizitaetundGas/NetzzugangMesswesen/Mess-undZaehlwesen/iMSys/artikel.html), Pflicht/Einbau/Verbrauchsgruppen. Keine LEH-spezifische Quote.

## 14. Gegenprüfung der unabhängigen Auditliste (`independent Audit.txt`)

**Quelle:** [independent Audit.txt](independent%20Audit.txt) — eine einzige Zeile (6.405 Zeichen, 6.433 Bytes, Ablage 12.10.2026 06:19), also ohne Zeilenumbrüche. Zitate daher als `independent Audit.txt#L1` plus Abschnittstitel; eine Zeilennummer ist nicht unterscheidbar. Diese Datei wurde in diesem Durchgang **erstmals gelesen**; keine der Zahlen der Abschnitte 0–13 bezog sich zuvor auf ihren Inhalt — alle Aussagen dort bleiben unabhängig gültig.

Isoliert wurden **zehn prüfbare Positionen**. Verdikte sind bewusst dreistufig geführt und in ihren Teilaussagen getrennt: **bestätigt** (Code/Daten stützt die Aussage), **widerlegt** (Code/Daten widerspricht), **offen** (im Repo nicht entscheidbar, mit Grund). „Teilweise bestätigt“ heißt: einzelne Teilaussagen sind bestätigt, andere widerlegt oder offen — aufgeschlüsselt in der jeweiligen Position.

### 14.0 Prüfstand, Datenbasis und Gegenprobe

[F] Ein temporärer Rust-Prüfstand **außerhalb** des Repos bindet die unveränderten Module `store_types.rs`, `physics.rs`, `control.rs`, `financial.rs` (Flotte) und `thermal/mod.rs` (Gateway) per `#[path]`-Import ein — dieselbe Technik wie in [Abschnitt 12.2](SANITY_CHECK.md#L548). Er führt **denselben Jahreslauf** aus wie [crates/fleet/src/main.rs](crates/fleet/src/main.rs): gleiche Aufrufreihenfolge, gleiche Argumente (`volatile = false`, FCR-Input 15,0, Lookahead 12 h). Produktdateien blieben unverändert; Prüfstand und Logs liegen im Betriebssystem-Temp-Verzeichnis, nicht im Repo.

[F] **Datenbasis** ist der echte Jahresdatensatz, den der unveränderte Fleet-Client in diesem Durchgang selbst beschafft hat (temporäres Arbeitsverzeichnis, `data_cache/fleet_2025.json`): 8.760 h, Preis-Mittel **57,83 €/MWh**, min −6,58, max +268,15, **126 Negativstunden**, keine Stunde > 300 €/MWh; Außentemperatur min −5,10 °C, max **37,20 °C**, 19 h > 34 °C an 5 Tagen. Das deckt sich mit dem Cachebefund aus [3.4](SANITY_CHECK.md#L121) und weicht von der BNetzA-Zählung ab — es ist echter Stundenhandel derselben Quelle, nicht das behauptete Preisfenster.

[F] **Gegenprobe der Codegleichheit:** Der Prüfstand reproduziert die Produktzahlen **exakt**: `aldi` **25.121,95 €** (67,76 %), `edeka` 49.750,13 €, `freezer` 83.513,19 €, `cold` 42.806,96 €, `datacenter` 223.481,73 € — identisch mit dem unveränderten CLI-Lauf desselben Tages (25.122 € / 49.750 € / 83.513 € / 42.807 € / 223.482 €). Die Messungen unten sind damit echte Codeläufe, keine Nachbauten.

[F] Gegenüber 12.2 zusätzlich gemessen, für **alle 21 Typen** je volles Jahr: maximaler **Sollwertsprung je Stunde**, maximaler **elektrischer Lastsprung** und dessen Anteil an der Nennleistung, Anzahl und Größe der **Coast→EIN-Rückkehrsprünge**, **Jahreslastspitze** mit Stunde/Wochentag/Modus/Außentemperatur, Coast- und Pre-Cool-Stunden (auch bei > 28 °C) sowie deren Anteil in einem bewusst groben **Hochlastfenster-Proxy** (Mo–Fr 06–09 und 16–20 Uhr; echte Fenster sind VNB-spezifisch, siehe [N2]), Deckentemperatur-Maximum und Abtauenergie. Rechenweg-Bausteine: `max|ΔSollwert|/h`, `max|ΔP_el|/h`, `argmax P_el`, `#Coast→ON`, `compute_cop(t, t_setpoint, T_amb)`, `ThermalState::step(5 s, true, 50 kW)`.

[F] **Gateway-Thermomodell direkt ausgeführt:** `ThermalState::new(-20,0 °C; 25,0 °C; R_th 0,05; C_th 1.000.000)` mit `max_electrical_power_w = 50.000` und wiederholtem `step(5,0 s, cooling = true, 50.000 W)` liefert nach 120 s **−35,00 °C** und bleibt dort stehen. Die ausgelieferte Konfiguration setzt `simulate = true` und überschreibt beim Start `sensor.cooling_power_w` mit `building.installed_cooling_power_w` (50.000 W, [main.rs](crates/gateway/src/main.rs#L55-L61)). Der historische Produktlog [thermal_arbitrage_log.csv](benchmarks/thermal_arbitrage_log.csv) zeigt denselben Pfad **vor** Commit `01c02b2` (11.10.2026, 18:20 Uhr): −19,22 °C → **−113,80 °C** in 43 Minuten, größter Einzelsprung 93,8 K in 9,9 s. Seit diesem Commit liegt im Modell das harte Band `[−35, +50]` ([thermal/mod.rs](crates/gateway/src/thermal/mod.rs#L218)) und im Regler eine Frostschutz-Abschaltung bei `t_min` ([control/mod.rs](crates/gateway/src/control/mod.rs#L431-L437)). **Beides sind Grenzwerte, keine Gradientenbegrenzungen** — der Befund der Position A1 bleibt davon unberührt; die −113 °C sind ein bereits behobener Fehler und dürfen **nicht** als aktueller Zustand gelesen werden.

### 14.1 A1 — Überhitzung, Flüssigkeitsschlag, Rampenbegrenzung

Behauptung: Vorkühlen flutet Verdampfer, ein abrupter Sollwertdrift bei Preis-Spikes riskiert Flüssigkeitsschlag; Rampenraten `dT/dt` und `dHz/dt` dürfen Herstellergrenzen nie überschreiten.

- [F] **Kein Kältekreis im Modell:** Suchmuster `R744|CO2|transkrit|gas_cooler|refrigerant|EEV|superheat|suction` → **0 Treffer** in `crates/`. Es gibt nur einen VFD-Lastfaktor `0,0…1,0`.
- [F] **Keine Rampen-/Slew-Begrenzung:** Suchmuster `ramp|slew|drift_rate|per_hour_limit` → **0 Treffer** in `crates/`.
- [F] **Gemessen (21 Typen):** maximaler **Sollwertsprung je Stunde 2,00–16,00 K** (`aldi` 4,00; `cold` 6,00; `frozen` 7,00; `freezer` 8,00; `cryo` 16,00), maximaler **elektrischer Lastsprung 10,4–20,0 % der Nennleistung je Stunde**, absolut bis **1.816,6 kW** (`industry`). Die Lufttemperatur selbst bewegt sich wegen der RC-Trägheit nur um 0,043–1,217 K/h — sie folgt dem Befehl, sie begrenzt ihn nicht.
- [F] **Gateway:** genau eine mechanische Schutzfunktion existiert — der Ölrückführungsimpuls ([control/mod.rs](crates/gateway/src/control/mod.rs#L313-L339)); er ist in der ausgelieferten [config.toml](crates/gateway/config.toml) mit `vfd_oil_return_pulse_every_sec = 0.0` **deaktiviert**. Daneben nur der ASCT-Governor (min. 900 s EIN / 600 s AUS) — eine Schaltfrequenz-, keine Gradientenbegrenzung.

**Verdikt:** Mechanik (Flüssigkeitsschlag, EEV-Flutung, Saugdruck) → **offen** (im Repo nicht abbildbar und ohne Anlagendaten/Herstellergrenzen nicht prüfbar; nicht widerlegt). Forderung „Rampenbegrenzung“ → **bestätigte Lücke**: sie ist nirgends implementiert.

### 14.2 A2 — Hochdruckdecke, COP < 1,4, Derating nach Außentemperatur

- [A] R744-Kritikpunkt „31 °C / 73,6 bar“ ist mit dem Lehrbuchwert (30,98 °C / 73,77 bar) konsistent, in diesem Durchgang aber **nicht primärquellenbelegt** → extern übernommen, offen. Druckbereich 90–100 bar: nicht prüfbar.
- [F] **Kein Kältemittel-, Gas-Cooler- oder Hochdruckmodell** (0 Treffer, siehe 14.1), also auch kein HD-Schalter.
- [F] **Kein Derating nach Außentemperatur:** `decide(...)` nimmt sie als **`_amb`** entgegen und verwendet sie nicht; die MPC-Kosten sind `cooling_power_kw · price / 1000` — ohne COP und ohne Temperatur ([control.rs](crates/fleet/src/control.rs)).
- [D] **Code-COP bei Hitze:** `compute_cop` liefert bei 34 °C Außentemperatur **2,69** (`aldi`, Sollwert −22 °C), **2,52** (`freezer`, −25 °C), 5,54 (`cold`), 6,00 (Deckel). Für ein transkritisches CO₂-System nennt der Audit dort < 1,4 — das Modell liegt **rund 1,8–1,9× darüber**; die Zahl < 1,4 ist extern und hier nicht belegt.
- [F] **Der Optimierer arbeitet bei Hitze:** 137 Stunden > 28 °C im Jahr; Coast-Stunden bei > 28 °C: `brewery` 79, `arena` 46, `freezer` 42, `facility` 39, `cold` 36, `mortuary` 27, `warehouse` 18 — Lastverschiebung ohne jede Temperaturdrosselung. Pre-Cool-Stunden > 28 °C bleiben mit 0–3 je Typ selten.
- [F] **Teilbehauptung widerlegt:** „EPEX-Preise brechen in sommerlichen Mittagsstunden ein“ bestätigt der verwendete echte Datensatz nicht — Juni–August 12–16 Uhr: Mittel **75,99 €/MWh**, nur 8 von 460 h < 30 €/MWh, keine Negativstunde; die billigste Stunde eines Werktags liegt **0 von 261** Werktagen im Proxy-Hochlastfenster. Vorbehalt: Stundenauflösung; die 15-Minuten-Vermarktung ab 10/2025 kann Dellen zeigen, die ein Stundenmittel verdeckt.

**Verdikt:** Derating fehlt → **bestätigt**. CO₂-Betriebspunkt nicht abbildbar / Modell optimistisch → **bestätigt**. „Mittagsdelle“ → für diesen Datensatz **widerlegt**. COP < 1,4 und Druckniveau → **offen** (extern).

### 14.3 A3 — Rebound-Synchronität, gestaffelte Erholung

- [F] [main.rs](crates/fleet/src/main.rs) simuliert **eine Zone je Typ** und multipliziert das Ergebnis mit `count`; es gibt **keinen Diversitätsparameter**. Alle N Anlagen eines Typs fahren damit deterministisch identisch — dieselbe Schwäche wie in [Abschnitt 2](SANITY_CHECK.md#L44) vermerkt, hier quantifiziert.
- [F] **Gemessen über 21 Typen: 22.837 Coast→EIN-Übergänge je Jahr** (`mortuary` 3.341, `marine` 2.531, `cold` 2.351, `warehouse` 1.878, `frozen` 1.739, `freezer` 1.469, `aldi` 472). Jeder davon ist bei mehreren Anlagen desselben Typs **gleichzeitig**.
- [F] Rückkehrsprung je Anlage: im Mittel 8,0 kW (`aldi`) bis 395,7 kW (`datacenter`); größter **Einzel**rücksprung 1.204,4 kW (`industry`), 960,8 kW (`datacenter`), 511,9 kW (`immersion`), 220,2 kW (`freezer`).
- [D] Hochskalierung im Modell: 400 identische `aldi` ⇒ **10,3 MW synchronisierter Rückkehrsprung** (400 × 25,8 kW) und — in der Abwurf- oder Rückkehrstunde — bis **14,7 MW Laständerung** (400 × 36,8 kW) innerhalb einer Stunde. Das Modell kann keine Staffelung erzeugen, weil es sie nicht kennt.
- [F] **Gateway:** Anti-Chatter-Governor vorhanden (min. 900 s EIN / 600 s AUS), **keine** Möbel-/Verbundstaffelung; Suchmuster `stagger|queue|rotate` → **0 Treffer**. Ein Gateway steuert eine Einheit.

**Verdikt:** **bestätigt** als Modell- und Produktlücke. Die behauptete Folge (Saugdruckeinbruch, Niederdruckstörung) ist Anlagenphysik und hier nicht messbar → offen.

### 14.4 A4 — Reifbildung durch kältere Verdampfer

- [F] **Keine Feuchte-, Reif- oder Verdampferverschmutzungszustände** (Suchmuster `humidity|frost|feuchte|tauwasser` → 0 Treffer). Abtauung ist ein **festes Zeitintervall** (`defrost_interval_h`), unabhängig von Temperaturniveau, Coast-Zyklen oder Feuchte; der Controller kennt Abtauung nicht (`defrost` in `control.rs` → 0 Treffer).
- [F] Re-verifiziert und in [3.6](SANITY_CHECK.md#L146) bereits belegt: `aldi` lädt **43,80 MWh/a** Heizenergie, obwohl die eigenen Parameter 0,5 h je 8 h bei 40 kW vorgeben (21,9 MWh/a) → **Faktor 2,0** durch den 1-h-Schritt.
- [F] Kein Notabtau- oder Abtau-Sperrpfad, der eine Arbitrage-Ersparnis aufheben könnte.

**Verdikt:** Reifmechanik und ihre Rückwirkung → **offen** (nicht modelliert, im Repo nicht messbar; weder bestätigt noch widerlegt). Bestehender Abtau-Energiefehler → **bestätigt** (unabhängig von dieser Auditliste bereits geführt).

### 14.5 A5 — § 19 Abs. 2 StromNEV (atypische Netznutzung)

**Recht (nachgeprüft am 12.10.2026):**

- [F] § 19 Abs. 2 Satz 1 StromNEV setzt für atypische Netznutzung eine **Untergrenze** des individuellen Netzentgelts von 20 % des veröffentlichten Netzentgelts; „bis zu 80 %“ ist damit **rechnerische Obergrenze, kein zugesagter Rabatt** [N1].
- [F] Die 7.000-Benutzungsstunden-/10-GWh-Schwellen gehören zu Satz 2/3 (stromintensive Netznutzung, dort 20/15/10 % Untergrenze) — **nicht** zur atypischen Netznutzung [N1].
- [F] Der Normtext verlangt die vorhersehbare erhebliche Abweichung von der zeitgleichen Jahreshöchstlast der Netz-/Umspannebene; die **Hochlastzeitfenster** konkretisieren diese Prüfung, sind aber VNB-spezifisch veröffentlicht und **nicht bundesweit feste Uhrzeiten** [N1][N2].

**Umsetzung:**

- [F] **Kein Hochlastzeitfenster-, Netzentgelt- oder Leistungspreis-Modell** (Suchmuster `Hochlast|Netzentgelt|19 Abs` → 0 Treffer in `crates/`). Die Fleet-Kosten sind reine Spotkosten ([Abschnitt 9](SANITY_CHECK.md#L382)).
- [F] **Gemessen:** Die **Jahreslastspitze** fällt bei **10 von 21 Typen** in ein Werktags-Proxy-Fenster 06–09/16–20 Uhr (`edeka` Do 17 h, `cold`/`frozen`/`marine` Do 07 h, `datacenter`/`chemistry`/`brewery`/`ripening`/`hvac` Do 08 h) und bei **12 von 21** in eine Stunde > 28 °C; Spitzenstunden liegen zwischen Tag 164 und 360 bei 26,5–37,2 °C. In **16 von 21 Typen** tritt die Spitze im Modus **`MpcPreCool`** auf — die Lastspitze entsteht also durch die eigene Lastverschiebung, nicht durch den Thermostaten (3 × `ThermalGuard`: `aldi`/`edeka`/`marktkauf` bei 35,2/36,4 °C; 2 × `IdleNormal`).
- [F] Pre-Cool-Stunden im Proxy-Fenster: **5.998 von 27.734 (21,6 %)**; Coast-Stunden dort: 4.769 von 14.482 (32,9 %).
- [D] **Größenordnung:** Mit der belastbaren Einzelanlagenzahl des gleicher-Physik-Gegenlaufs (**460,39 €/a**, [4.2](SANITY_CHECK.md#L216)) entspräche ein fünfstelliger Netzentgeltverlust **mehr als 21 Jahren** Modellspanne; mit der Modellzahl (42.807–83.513 €/a je Typ, [4.1](SANITY_CHECK.md#L184)) bliebe die Formulierung „wipes out all software arbitrage value for years“ plausibel — aber unbelegt.

**Verdikt:** Implementierungslücke und strukturelles Risiko → **bestätigt**; Rechtszahl „bis zu 80 %“ → **bestätigt als Obergrenze**; wirtschaftliche Folge → **offen** (kein Standorttarif, keine Netzentgelte im Modell; VNB-Fenster nicht bekannt).

### 14.6 A6 — § 14a EnWG, netzorientierte Steuerung

- [F] Recht: Die 4,2-kW-Größe gehört zur netzorientierten Steuerung im **Niederspannungsanschluss**; die Neuregelung gilt seit **01.01.2024**; für Mittelspannung/größere Anlagen ist sie nicht pauschal anwendbar, für große Klimaanlagen nennt die BNetzA eine abweichende Mindestleistungsberechnung [N3][N4]. Die Audit-Formulierung „often to a 4.2 kW baseline“ ist eine Vereinfachung.
- [F] **Kein §-14a-Eingang:** API-Routen sind ausschließlich `/ui`, `/status`, `/health`, `/metrics`, `/config` (GET/PUT) und `/price` (POST) ([api/mod.rs](crates/gateway/src/api/mod.rs#L139-L147)); Suchmuster `14a|curtail|dimming|SMGW|Steuerbox` → **0 Treffer**. Der Gateway **empfängt** eine netzseitige Drosselung nicht und führt seinen wirtschaftlichen Dispatch unverändert fort. Externe Eingriffe sind nur Netzfrequenz (GridBalancing-Logik) und PLC-Verlust (Fail-safe EIN).

**Verdikt:** „kann netzseitigem Dimming nicht entgegenwirken“ → **bestätigt**, sogar stärker als formuliert: er sieht das Signal nicht. Die 4,2-kW-Zahl → **teilweise bestätigt** (andere Kundengruppe/Spannungsebene).

### 14.7 A7 — Luft- vs. Kerntemperatur (HACCP)

- [F] **Das Gateway hat ein Kerntemperatur-Modell und nutzt es als Sicherheitsgrenze:** `update_cargo_temp`, `cargo_temp_c`, Notfallverriegelung auf `cargo_t_max` ([control/mod.rs](crates/gateway/src/control/mod.rs#L395-L403), [L697-L714](crates/gateway/src/control/mod.rs#L697-L714)); ausgeliefert mit `cargo_lag_enabled = true` und `cargo_t_max = −17,5 °C`. Die Aussage „ohne Kern-/Warenmodell“ ist damit **widerlegt** für das Gateway — es bleibt ein **Modell ohne Messung**, ohne Oberflächen-, Strahlungs- oder IR-Packungstemperatur (Suchmuster `surface_temp|radiant|ir_` → 0 Treffer).
- [F] **In der Flotte** ist `cargo_temp_c` ein passiver Lag **ohne Reglerwirkung**; gemessen bleibt die Ware stets **kälter** als die Luft (max. |Cargo − Luft| 0,12–1,01 K; `aldi` Cargo max −21,67 °C, `cold` +4,00 °C). Der Audit-Fall „Luft −20 °C, Ware > −18 °C“ kann im Modell nicht entstehen. Die Deckentemperatur überschreitet `t_max` dagegen ganzjährig (`aldi` +7,67 °C über 8.747 h) und wird **nicht** als Grenze verwendet ([3.5](SANITY_CHECK.md#L138)).

**Verdikt:** „nur Luftmessung, keine Waren-/Oberflächenmodellierung“ → **teilweise bestätigt** (trifft die Flotte und die fehlende Oberflächenphysik, **nicht** das Gateway); die behauptete Vorzeichenrichtung (Ware wärmer als Luft) → **widerlegt** im Modell. HACCP-Relevanz im Einzelfall → **offen**, nicht aus dem Repo entscheidbar.

### 14.8 A8 — Versicherung und Drittänderung an zertifizierten Regelungen

- [U] **Nicht aus dem Repo prüfbar:** kein Versicherungs-, Zertifizierungs-, Abnahme- oder Betreiber-Freigabedokument im Projekt (keine Police, kein VDKF-/BWP-Nachweis). Vorhanden sind nur Betriebshinweise ([DEPLOYMENT.md](crates/gateway/DEPLOYMENT.md)) und die dokumentierte Feststellung fehlender OEM-Freigaben ([3.8](SANITY_CHECK.md#L162), [7](SANITY_CHECK.md#L308)).

**Verdikt:** **offen** — organisatorisch/vertraglich, ohne Vertrags- oder Policenkenntnis weder bestätigt noch widerlegt; als Risiko plausibel, nicht als Beleg übernehmen.

### 14.9 A9 — EEPROM-/Flash-Verschleiß durch Schreiblast

- [F] **Schreibpfad gemessen:** Der Modbus-Loop schreibt **jeden Poll-Zyklus unbedingt** — Verdichter-Coil (`write_coil`) bzw. Geschwindigkeitsregister (`write_register`) **und** zusätzlich die Alarm-Coil; **keine Änderungserkennung** ([modbus_client.rs](crates/gateway/src/modbus_client.rs#L380-L420)). Standardintervall `poll_interval_sec = 2.0` ([config.toml](crates/gateway/config.toml)), E2E-Fixture 1,0 s.
- [D] **Rechnung:** 43.200 Zyklen/Tag × 2 Schreibvorgänge = **86.400 Schreibvorgänge/Tag** auf die Alarm-Coil; 100.000 Zyklen wären bei nichtflüchtigem Zielspeicher nach **1,16 Tagen** erreicht (bei nur einem Schreibvorgang je Zyklus 2,31 Tage). Die Audit-Rechnung (15-Minuten-Raster, ~2,85 Jahre) ist intern korrekt, unterschätzt die Kadenz dieses Produkts aber um **Faktor 900**.
- [F] **Präzisierung:** Das Gateway schreibt **keinen Temperatur-Sollwert** in die SPS (`setpoint` im Modbus-Pfad → 0 Treffer). Der beschriebene Weg „setpoint adjustments every 15 minutes“ trifft einen anderen Integrationsweg als den implementierten.
- [U] Ob die adressierten Coils/Register im Zielgerät flüchtig oder nichtflüchtig sind, steht **nirgends** im Projekt (0 Treffer zum Speicherverhalten) — genau diese Prüfung fordert der Audit zu Recht; sie ist nicht dokumentiert.

**Verdikt:** **teilweise bestätigt** — mit verschärfendem Zusatzbefund (Kadenz statt 15 Minuten), einer Korrektur (kein Sollwert-Schreibweg) und offener Folge (Zielgerät unbekannt).

### 14.10 A10 — Kältebauer/SLA, Isolation im Servicefall

- [F] **Kein Wartungs-/Isolationsmodus im Repo:** Suchmuster `service_mode|bypass_switch|isolation|maintenance` → 0 Treffer; die Treffer für „bypass“ betreffen ausschließlich „Emergency umgeht den Governor“. Vorhanden sind Fail-safe-EIN bei PLC-Verlust und ein token-geschütztes `PUT /config`.

**Verdikt:** Als **Produktlücke bestätigt** (kein dokumentierter physischer oder software-seitiger Isolationsweg für Servicetechniker); die Vertrags-/SLA-Folgen sind **offen** (keine Wartungsverträge, keine Betreiberfreigaben im Prüfgegenstand).

### 14.11 Verdiktübersicht

| Nr. | Position (`independent Audit.txt#L1`) | Verdikt | Kernzahl aus diesem Repo |
|---|---|---|---|
| A1 | Überhitzung/Flüssigkeitsschlag, Rampengrenzen | offen / **Lücke bestätigt** | Sollwertsprung bis 16 K/h; Lastsprung bis 20 % der Nennleistung; `ramp\|slew` 0 Treffer |
| A2 | Hochdruckdecke, COP < 1,4, Derating | teilweise bestätigt | `_amb` unbenutzt; Code-COP 2,52–2,69 bei 34 °C; 137 h > 28 °C |
| A3 | Rebound-Synchronität | **bestätigt** | 22.837 Rückkehrereignisse; 400 `aldi` ⇒ 10,3 MW Rücksprung / bis 14,7 MW Laständerung gleichzeitig |
| A4 | Reifbildung | offen | keine Feuchte-/Reifzustände; Abtauung fest getaktet |
| A5 | § 19 Abs. 2 StromNEV | teilweise bestätigt | Lastspitze bei 10/21 Typen im Proxy-Fenster, 16/21 im Modus `MpcPreCool` |
| A6 | § 14a EnWG Dimming | teilweise bestätigt | 0 Treffer `14a\|curtail\|SMGW`; 6 API-Routen ohne Drosselungseingang |
| A7 | Luft- vs. Kerntemperatur | teilweise bestätigt | Kernmodell vorhanden (Gateway), in der Flotte passiv; Ware nie wärmer als Luft |
| A8 | Versicherung/Drittänderung | offen | kein Policen-/Freigabedokument im Repo |
| A9 | EEPROM-/Flash-Verschleiß | teilweise bestätigt (verschärft) | 86.400 Schreibvorgänge/Tag ⇒ 100 k Zyklen in 1,16 Tagen |
| A10 | Kältebauer/SLA, Bypass | offen / **Lücke bestätigt** | 0 Treffer Isolations-/Servicemodus |

**Summe (10 Positionen):** **1 × bestätigt** (A3); **5 × teilweise bestätigt** (A2, A5, A6, A7, A9 — Teilwiderlegungen ausdrücklich in A2 „Mittagsdelle“ und A7 „ohne Kernmodell“); **4 × offen** (A1, A4, A8, A10), davon **2 mit ausdrücklich bestätigter Lücke** (A1 Rampengrenzen, A10 Servicemodus); **0 × vollständig widerlegt**. Keine Position der externen Liste ließ sich als falsch abtun; keine wurde ungeprüft übernommen.

### 14.12 Die drei belastbarsten neuen Befunde

1. **Die Lastspitze entsteht durch den Optimierer selbst.** In 16 von 21 Typen liegt die Jahreslastspitze im Modus `MpcPreCool`, zehn davon in einem Werktags-Proxy-Fenster 06–09/16–20 Uhr und zwölf bei > 28 °C. Das ist die maschinell erzeugte Fassung des §-19-Konflikts — vor jeder Aussage über Netzentgeteiltarife und vor jeder Standortentscheidung zu klären.
2. **Das Modell kann keine Staffelung und keine Rampenbegrenzung.** Eine Zone je Typ, linear skaliert, ergibt 100 % Gleichzeitigkeit (400 `aldi` ⇒ 14,7 MW); der Sollwertsprung beträgt bis 16 K in einer Stunde, der Lastsprung bis 20 % der Nennleistung, und weder Flotte noch Gateway besitzen eine Gradientenbegrenzung.
3. **Temperatur- und Speicherverschleiß-Fragen sind offen statt erledigt.** Das Gateway drosselt nicht nach Außentemperatur (kein Derating, kein Kältemittelmodell), und seine Schreibkadenz ist 900× höher als in der externen Kritik angenommen — bei unbekannter Speicherart des Zielgeräts.

## 15. Abschließende Entscheidung

**Heute NO-GO für autonome universelle Ladensteuerung, garantierte 60–80-%-Einsparung, automatische FCR-Erlöse oder 407-Filialen-Hochrechnung als Umsatzplan. GO für begrenzte, betreiber- und OEM-abgestimmte Machbarkeitsprüfung mit echten Daten.**

Priorität: **(1)** faire Mess-/Baseline-/Preisbilanz, **(2)** ein konkreter freigegebener Regler-/Lastabwurfweg mit lokalem Fallback, **(3)** Stromvertrag/Leistungspreis und vollständig kalkulierte Standortkosten, **(4)** Produkt-/Worst-Case-Validierung. Erst danach größere Modelle und Marktintegration. Kein Produktcode wurde für diesen Bericht repariert; die roten Befunde bleiben bewusst offen und dürfen nicht durch diesen Bericht als erledigt gelten.
