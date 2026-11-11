# Ambient Thermal Arbitrage (ATA) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `Ambient-Thermal-Arbitrage/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**Ambient Thermal Arbitrage (ATA)** — An industrial edge gateway (Modbus RTU/TCP) that converts refrigerated warehouses and cold-chain facilities into virtual batteries for grid frequency regulation (FCR/Droop) and energy market arbitrage.

### Current Engineering Baseline
A 2-crate Rust workspace (`crates/gateway` and `crates/fleet`) implementing closed-loop Modbus RTU/TCP industrial refrigeration control, thermodynamic RC building envelopes, EPEX Spot / FCR droop frequency regulation, and multi-site fleet coordination. The edge gateway is mature, while fleet-level N-1 compressor dispatch, defrost collision avoidance, and cryptographic savings ledger proofs need hardening.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Thermal Inertia Utilization:** Strategische Nutzung der Wärmespeicherkapazität von Kühlhäusern, Supermärkten und Klimaanlagen als passive Energiespeicher.
    
- **Dynamic Price Anchoring:** Kopplung der Kühlleistung an Echtzeit-Strompreise, um energieintensive Phasen in Zeitfenster mit hoher Solar- oder Windeinspeisung zu verschieben.
    
- **Pre-Cooling Strategy:** Aggressives Herunterkühlen der Lagereinheiten bei Überangebot an günstigem Strom, um thermische Reserven für teure Lastspitzen aufzubauen.
    
- **Curtailed Load Shedding:** Gezielte Deaktivierung der Kompressoren für bis zu zwei Stunden während hoher Preisphasen, ohne die kritische Kühlkette zu unterbrechen.
    
- **Arbitrage Logic:** Systematische Ausnutzung der Preisdifferenz zwischen Erzeugungsspitzen und Verbrauchsspitzen zur massiven Reduktion von Betriebskosten.
    
- **Grid Balancing Response:** Aktive Unterstützung der Netzstabilität durch fungible Lastanpassung, was zusätzliche Erlöse durch Systemdienstleistungen generiert.
    
- **Automated Thermal Guarding:** Softwarebasierte Überwachung der Kerntemperatur, die bei Erreichen kritischer Schwellenwerte sofort autonom gegensteuert.
- **Predictive Solar Forecasting:** Integration von Wetterdaten-Modellen, um Kühlzyklen exakt auf die lokalen Produktionsspitzen von Photovoltaik-Anlagen abzustimmen.
    
- **Thermal Capacity Fingerprinting:** Mathematische Analyse der spezifischen Isolationsqualität jedes Gebäudes, um die maximale "Ausschaltzeit" individuell und sicher zu berechnen.
    
- **Smart-Contract Settlement:** Automatisierte Abrechnung der gesparten Energiekosten und CO2-Zertifikate über eine Blockchain, um Transparenz für Betreiber und Energieversorger zu schaffen.
    
- **Emergency Override Protocol:** Priorisierte Sicherheitslogik, die bei Überschreitung physikalischer Grenzwerte die Wirtschaftlichkeit ignoriert, um die Sicherheit der gelagerten Güter zu garantieren.
    
- **Micro-Grid Integration:** Fähigkeit des Systems, im Inselbetrieb mit lokalen Batteriespeichern zu interagieren, um die thermische Trägheit als primären Puffer zu nutzen.
    
- **Latency-Optimized Market Access:** Direkte API-Anbindung an Strombörsen (z.B. EPEX SPOT), um auf Preisschwankungen im Minutentakt reagieren zu können.
    
- **Aggregated Virtual Battery:** Zusammenschluss tausender kleiner Kühlstellen zu einem virtuellen Großspeicher, der wie ein Kraftwerk am Regelenergiemarkt agiert.

---

## 3. Theoretical Foundations & Protocol Mechanics (`Life Work.canvas`)

Extracted architectural derivations, mathematical formulations, and state-machine specifications from the master canvas:

### [Ambient Thermal Arbitrage] Specification Node
> Advanced Algorithmic Refinements
> 
> Your Model Predictive Control (MPC) and MILP engines can be made significantly more resilient.
> 
> - **Mixed-Integer Non-Linear Programming (MINLP):** Since your COP formula is inherently non-linear (fractional based on temperatures), standard Linear Programming (MILP) struggles unless you piecewise-linearize the COP curve.
>     
>     - _Node to add:_ **Piecewise COP Linearization** or a transition to a true **MINLP Solver Engine** (like Couenne or Bonmin) to handle the true non-linear thermodynamic curve efficiently.
>         
> - **Degradation Cost Function:** Every thermal swing stresses physical components (valves, pipes, compressors). The optimization function shouldn't just look at $P_{el}(t) \cdot \dot{W}_{comp}(t)$; it needs a penality term for mechanical wear-and-tear.
>     
>     - _Updated Objective Function:_
>         
>         $$J = \int_{t_0}^{t_1} \left[ P_{el}(t) \cdot \dot{W}_{comp}(t) + \Lambda \cdot \left| \frac{du}{dt} \right| \right] \, dt$$
>         
>         Where $\Lambda$ is the wear-and-tear financial penalty multiplier for cycling the hardware

### [Ambient Thermal Arbitrage] Specification Node
> Physical & Refrigeration Constraints
> 
> 
> While you have the basic $RC$ model and COP degradation mapped, refrigeration loops have mechanical limits that will break if optimized purely for financial gain.
> 
> - **Anti-Short Cycle Timer (ASCT) Constraint:** Compressors cannot cycle on and off rapidly without burning out the motors due to oil-return issues and high starting currents.
>     
>     - _Mathematical addition:_ Add a state constraint ensuring that if a compressor switches states ($u(t) \neq u(t-1)$), it must remain in that state for a minimum duration $\Delta t_{min\_run}$ or $\Delta t_{min\_off}$.
>         
> - **Defrost Cycle Interruption Logic:** Industrial freezers must run defrost cycles (typically electric resistive or hot-gas defrost) to clear ice from the evaporator. During defrost, the room actually _heats up_ intentionally. Your arbitrage model must predict and account for these mandatory energy spikes and prevent the system from flagging them as an anomaly or attempting to "cool through" them.
>     
> - **Thermal Stratification & Airflow Modeling:** A room's air temperature drops quickly, but the cargo (e.g., pallets of food) has massive thermal inertia. If you turn off fans to save energy, the air stratifies (hot air rises), risking local hot spots.
>     
>     - _Node to add:_ **Evaporator Fan VFD Optimization**. Balancing the trade-off between airflow energy consumption and cargo temperature homogeneity.

### [Ambient Thermal Arbitrage] Specification Node
> Thermal Inertia Utilization
> 
> Das Fundament des Systems, es setzt Kühlanlagen als thermische Batterien

### [Ambient Thermal Arbitrage] Specification Node
> Thermische Wiederstand ($R_{th}$):
> Das beschreibt die Isolationsgüte der Hülle, je höher $R$, desto langsamer verliert das System Energie
> 
> Zeitkonstante ($r$): Das ist die Größe der "Batter-Laufzeit"
> 
> $$\tau = R_{th} \cdot C_{th}$$
> 
> Dies gibt an, wie schnell das System auf Temperaturänderungen reagiert. Ein hohes $r$ sagt, wie lange du es ausgeschaltet haben kannst

### [Ambient Thermal Arbitrage] Specification Node
> Dynamic Price Anchoring
> 
> Der Mechanismus der Physikalische Kapazität mit dem Finanzmarkt verbindet

### [Ambient Thermal Arbitrage] Specification Node
> Die Constraints:
> Die Optimierung muss eingeschränkt sein:
> 
> 1. $T_{min} \leq T(t) \leq T_{max}$ 
> 	 (Die Temperatur muss innerhalb der Sicherheitsgrenze bleiben)
> 2. $\dot{T}(t) = f(T, T_{amb}, \dot{W}_{comp})$
> 	(Die Themodynamische Gleichung aus Punkt 1)

### [Ambient Thermal Arbitrage] Specification Node
> Pre-Cooling Strategy
> 
> Hier wird der Aufbau von thermischen Reserven angepeilt. Hier wird das Überangebot an günstigem Strom genutzt umd Proaktiv geld zu sparen

### [Ambient Thermal Arbitrage] Specification Node
> Diminishing Returns Effekt
> 
> Ich muss den COP berücksichtigen. Je tiefer die Zieltemperatur ($T_{target}$), desto mehr muss der Kompressor arbeiten - Ergo, schlechterer Wirkungsrgrad
> $$\text{COP}_{ist} = \eta_C \cdot \frac{T_{kalt}}{T_{warm} - T_{kalt}}$$
> Wobei $T$ in Kelvin gemessen wird und $\eta_C$ der Carnot-Gütegrad ist.

### [Ambient Thermal Arbitrage] Specification Node
> Set-Point Rampen: 
> 	es wird nicht ein/aus, kontinuierlich runter oder rauf.
> Hysterese-Management:
> 	Die Hysterese wird asymmetrisch verschoben, ich erlaube dem System weit nach unten zu Überschwingen solange der Preis niedrig ist
> Feuchtigkeitmanagement:
> 	Ich muss berechnen ob Feuchtigkeit ausfriert. Ich muss die Luftfeuchtigkeit einbeziehen

### [Ambient Thermal Arbitrage] Specification Node
> Curtailed Load Shedding
> 
> Das System schaltet nicht ab, wenn es fertig ist, sondern wenn es wirtschaftlich Sinnvoll ist

### [Ambient Thermal Arbitrage] Specification Node
> Shedding Potenzial
> 
> Ich muss die maximale Zeitdauer $t_{max}$ berechnen, die ich ohne Strom auskomme bevor $T_{limit}$ eintrifft

### [Ambient Thermal Arbitrage] Specification Node
> Die Kosten-Optimierungsfunktion
> 
> Ich muss die Steuerung der Anlage als mathematisches Optimierungsproblem formulieren. Ich suche nach den minimalsten Betriebskosten innerhalb sicherheit

### [Ambient Thermal Arbitrage] Specification Node
> Mathematik:
> Um die Trägheit zu messen und zu steuern, muss ich die Wärmekapazität und Wärmedurchgang verstehen. Hier kommt das RC-Modell aus der Gebäudephysik, welches die Analogie zwischen Strom und Wärme nutzt

### [Ambient Thermal Arbitrage] Specification Node
> Die Zerfallsfunktion der Temperatur
> 
> Sobald die Kompressoren abgeschaltet sind, steigen die Temperaturen. Das folgt der exponentiellen Annäherungsfunktion an die Umgebungstemperatur ($T_{amb}$)

### [Ambient Thermal Arbitrage] Specification Node
> Formel:
> $$t_{max} = -\tau \cdot \ln\left(\frac{T_{limit} - T_{amb}}{T_{start} - T_{amb}}\right)$$

### [Ambient Thermal Arbitrage] Specification Node
> Die Enthalpie-Steigerung und der "Vorkühl-Offset"
> 
> Hier verschieben wir den Arbeitspunkt des Systems. Ich muss berechnen, wie viel zusätzliche Energie  $\Delta Q$ Ich im System speichern kann, bevor ich $T_{min}$ erreiche

### [Ambient Thermal Arbitrage] Specification Node
> Verdampfer-Management:
> - Wenn Kompressoren stoppen, laufen die Lüfter nach. Ich muss berechnen wieviel Energie die Lüfter verbrauchen versus wie viel thermische Homogenität sie bringen
> Weidereinschalt-Synchro.
> - Einschaltung muss zeitlich versetzt sein, um Lastspitzen zu vermeiden
> Kritische Kühlkette
> - Virtuelle Seonsoren die die Kerntemperatur der Ware schätzen

### [Ambient Thermal Arbitrage] Specification Node
> Mixed-Integer Linear Programming
> 
> Das ist die "Engine" die besagt, ob der Generator jetzt angeht oder erst in 15 Minuten.

### [Ambient Thermal Arbitrage] Specification Node
> So sollte die Zielfunktion $J$ aussehen:
> 
> $$J = \int_{t_0}^{t_1} [P_{el}(t) \cdot \dot{W}_{comp}(t)] \, dt$$
> Wo:
> $P_{el}(t)$ Der Zeitabhängige Strompreis ist, und 
> $\dot{W}_{comp}(t)$ Die elektrische Leistungsaufnahme der Kompressoren zum Zeitpunkt $t$ zeigt

### [Ambient Thermal Arbitrage] Specification Node
> Die Herausforderung:
> Ich muss berechnen, ob der günstige Strompreis, den Effizienzverlust rechtfertigt. Wenn der Strompreis 50% sinkt, aber der COP um 20% einbricht, ist es Netto Gewinn

### [Ambient Thermal Arbitrage] Specification Node
> Formel:
> $$T(t) = T_{amb} + (T_{start} - T_{amb}) \cdot e^{-\frac{t}{\tau}}$$
> $T_{start}:$ Die Temperatur am Ende des Pre-Cooling-Zyklus
> $\tau:$ Die Zeitkonstante $R_{th} \cdot C_{th}$

### [Ambient Thermal Arbitrage] Specification Node
> Thermische Kapazität ($C_{th}$):
> Das gibt an wie viel Energie ($Q$) ein Körper speichern kann, um die Temperatur um ein Kelvin zu erhören

### [Ambient Thermal Arbitrage] Specification Node
> Preiselastizität
> Es muss wissen, wie aggressiv es auf die Preise reagieren soll.
> 
> Niedriger Preis: 
> 	Das System setzt den Sollwert auf $T_{min}$
> Extremer Preis-Peak:
> 	Das System setzt den Sollwert auf $T_{max}$

### [Ambient Thermal Arbitrage] Specification Node
> Wenn Strompreis billig, auf -24 kühlen, dann wenn es teuer wird, abschalten, so lange wie das gerät braucht bis zu -18 ist gesparter Strom

### [Ambient Thermal Arbitrage] Specification Node
> Die gespeicherte Energiemenge wird hieraus berechnet:
> 
> $$Q_{storage} = \int_{T_{target}}^{T_{set, normal}} C_{th}(T) \, dT$$

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `Ambient-Thermal-Arbitrage/`: **36**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 14 | Build / Package Manifest |
| `.freebuff/sanity/audit.rs` | 47 | Core Implementation Module |
| `.freebuff/sanity/e2e_verify.py` | 57 | Core Implementation Module |
| `.freebuff/sanity/report-check-0.py` | 63 | Core Implementation Module |
| `.freebuff/sanity/report-check-1.py` | 16 | Core Implementation Module |
| `.freebuff/sanity-reproduce/audit.rs` | 46 | Core Implementation Module |
| `crates/fleet/Cargo.toml` | 16 | Build / Package Manifest |
| `crates/fleet/src/bundesland.rs` | 304 | Core Implementation Module |
| `crates/fleet/src/control.rs` | 542 | Core Implementation Module |
| `crates/fleet/src/financial.rs` | 178 | Core Implementation Module |
| `crates/fleet/src/main.rs` | 420 | Core Implementation Module |
| `crates/fleet/src/physics.rs` | 345 | Core Implementation Module |
| `crates/fleet/src/simulation.rs` | 284 | Core Implementation Module |
| `crates/fleet/src/store_types.rs` | 192 | Core Implementation Module |
| `crates/gateway/Cargo.toml` | 59 | Build / Package Manifest |
| `crates/gateway/src/alerts.rs` | 199 | Core Implementation Module |
| `crates/gateway/src/config.rs` | 496 | Core Implementation Module |
| `crates/gateway/src/lib.rs` | 17 | Core Implementation Module |
| `crates/gateway/src/live_data.rs` | 787 | Core Implementation Module |
| `crates/gateway/src/main.rs` | 460 | Core Implementation Module |
| `crates/gateway/src/metrics.rs` | 197 | Core Implementation Module |
| `crates/gateway/src/modbus_client.rs` | 617 | Core Implementation Module |
| `crates/gateway/src/ota.rs` | 363 | Core Implementation Module |
| `crates/gateway/src/security.rs` | 115 | Core Implementation Module |
| `crates/gateway/src/watchdog.rs` | 185 | Core Implementation Module |
| `crates/gateway/src/api/mod.rs` | 859 | Core Implementation Module |
| `crates/gateway/src/bin/modbus_mock.rs` | 159 | Core Implementation Module |
| `crates/gateway/src/control/mod.rs` | 1324 | Core Implementation Module |
| `crates/gateway/src/data_ingestion/mod.rs` | 364 | Core Implementation Module |
| `crates/gateway/src/financial/mod.rs` | 258 | Core Implementation Module |
| `crates/gateway/src/logging/mod.rs` | 217 | Core Implementation Module |
| `crates/gateway/src/sensor/mod.rs` | 300 | Core Implementation Module |
| `crates/gateway/src/simulation_data/mod.rs` | 617 | Core Implementation Module |
| `crates/gateway/src/simulator/mod.rs` | 298 | Core Implementation Module |
| `crates/gateway/src/thermal/mod.rs` | 397 | Core Implementation Module |
| `scripts/e2e/fake-plc.js` | 27 | Core Implementation Module |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Thermodynamic RC Fingerprinting & Defrost Coordination (`crates/gateway/src/thermal/mod.rs`):** Needs online recursive least-squares (RLS) identification of thermal resistance $R_{\text{th}}$ and capacitance $C_{\text{th}}$ to detect door-open anomalies and schedule defrost cycles outside high-price grid windows.
- **HACCP Food-Safety Hard Interlock (`crates/gateway/src/control/mod.rs`):** Must formally prove that any external arbitrage or FCR droop command is immediately overridden if core product temperature $T_{\text{core}} > T_{\text{max}} - \Delta T_{\text{margin}}$ or compressor short-cycle minimum off-time ($t_{\text{off}} < 180\text{ s}$) is violated.
- **Fleet Virtual Power Plant Aggregation (`crates/fleet/src/control.rs`):** Needs optimal allocation of a fleet-wide MW regulation target across heterogeneous cold stores ordered by thermal time constant $\tau = R_{\text{th}} C_{\text{th}}$.
- **Verifiable Baseline Savings Audit (`crates/gateway/src/financial/mod.rs`):** Needs counterfactual baseline simulation (standard hysteresis thermostat vs. ATA pre-cooling) with signed daily settlement records.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Enforce strict HACCP core-temperature and compressor short-cycle safety interlocks | `crates/gateway/src/control/mod.rs`, `crates/gateway/src/watchdog.rs` | `cargo test -p ata-gateway --quiet` |
| **TASK-02** | Implement online thermal capacity fingerprinting and door-open anomaly detection | `crates/gateway/src/thermal/mod.rs`, `crates/gateway/src/sensor/mod.rs` | `cargo test -p ata-gateway --quiet` |
| **TASK-03** | Implement counterfactual baseline simulator and auditable arbitrage savings ledger | `crates/gateway/src/financial/mod.rs` | `cargo test -p ata-gateway --quiet` |
| **TASK-04** | Implement priority-ranked multi-site VPP dispatch in the fleet engine | `crates/fleet/src/control.rs`, `crates/fleet/src/physics.rs` | `cargo test -p ata-fleet --quiet` |
| **TASK-05** | Add anti-synchronization defrost stagger scheduler across compressor banks | `crates/gateway/src/control/mod.rs` | `cargo test -p ata-gateway --quiet` |
