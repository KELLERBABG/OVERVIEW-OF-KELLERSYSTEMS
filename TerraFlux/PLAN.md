# TerraFlux / FieldSense — Codebase Remediation & Productization Plan (`PLAN.md`)

> ## ⚠️ ARCHIVED — superseded by [`GAP_ANALYSIS.md`](GAP_ANALYSIS.md)
>
> This document is the original remediation plan (2026-11-10). It is kept as the
> historical record of what was wrong and why. **It is not the source of truth for the
> current status** — the audit, the verification and the completion status of every
> item live in [`GAP_ANALYSIS.md`](GAP_ANALYSIS.md), which also lists the two real bugs
> this plan did not find (the `DELAY`/`PAUSE` command routing and the naive/aware
> timestamp comparison).
>
> The execution checklist at the bottom of this file is therefore **not** a status
> report: it marked steps as done that were in fact broken (Step 3 claimed working
> `DELAY`/`PAUSE` persistence while the command was misrouted into the watering
> branch). Use `GAP_ANALYSIS.md` §7 for what is actually implemented.

> **Objective:** Fix all broken UI/SVG representations, JavaScript runtime crashes, identity contradictions, placeholder backend logic, and unrealistic economic/agronomic assumptions across the repository so it can stand up to real growers, engineers, and investors.

---

## Priority 0: Brand, Stack & Architectural Unification

Currently, the repository suffers from a split personality between **TerraFlux** (root [index.html](index.html) & [assets/banner.svg](assets/banner.svg)) and **FieldSense Irrigation** ([README.md](README.md), [src/](src/), [docs/](docs/)).

### 0.1 Files to Edit
- **[index.html](index.html#L136-L138)**:
  - **Bug:** Shows Rust CLI commands (`cargo test`, `cargo run --bin terraflux -- field --probe 15,30,60 --et0 open-meteo`) in a 100% Python/FastAPI repository.
  - **Fix:** Replace lines 136–138 with the real Python/Docker commands:
    ```html
    <pre>git clone https://github.com/KELLERBABG/TerraFlux.git
    cd TerraFlux &amp;&amp; pip install -r requirements.txt &amp;&amp; pytest -v
    uvicorn src.api.main:app --host 0.0.0.0 --port 8000</pre>
    ```
- **Brand Alignment Across Repo ([README.md](README.md), [src/api/main.py](src/api/main.py), [src/ui/templates/index.html](src/ui/templates/index.html), [docker-compose.yml](docker-compose.yml))**:
  - Pick **one canonical name** (e.g., **TerraFlux** by Keller Systems, or **TerraFlux (FieldSense Engine)**) and unify page titles, WhatsApp bot headers, FastAPI title, and README headers.
- **Reconcile Water Savings Claim**:
  - [index.html](index.html#L85) claims `-38.4% water` and `35–45%` in the comparison table ([index.html](index.html#L131)), while [WHITEPAPER.md](docs/WHITEPAPER.md#L16) and [`bom_economics.py`](src/simulation/bom_economics.py#L125) claim `18% (15–22%)`.
  - **Fix:** Align both to **18–28% verified against FAO-56** (which is scientifically defensible against empirical Regulated Deficit Irrigation literature; >38% only occurs when replacing a grossly misconfigured daily clock timer).

---

## Priority 1: HTML, SVG & Frontend Bug Fixes

### 1.1 Fix SVG Overflows & Layout Defects in Root [index.html](index.html)

1. **Water Balance SVG Pill Overflow ([index.html](index.html#L115))**:
   - **Bug:** `<rect class="app" x="346.4" y="96" width="52" height="13" rx="2"/>` and `<rect class="app" x="678.1" y="96" width="52" height="13" rx="2"/>` are `52px` wide, while `<text class="av">12 mm applied</text>` in `8.5px IBM Plex Mono` is `~68px` wide, overflowing both edges onto the light background.
   - **Fix:** Widen both `<rect class="app">` elements from `width="52"` to `width="78"` and shift their `x` coordinates left by `13px` (`x="333.4"` and `x="665.1"` so they remain centered around `372.4` and `704.1`), and increase `height="15"` (`y="95"`).
   - **Scale Fix:** Represent applied irrigation bars proportionally or add a clear two-track label (`Demand (mm/day)` above axis, `Irrigation Event` pill below axis) so a `12 mm` event does not visually look smaller than a `5.2 mm` demand bar.

2. **Field Plan SVG Legend Collision ([index.html](index.html#L93))**:
   - **Bug:** `<text class="lg" x="52" y="511">zone shade = volumetric water content</text>` extends to `x ≈ 256`, colliding with `<rect class="sw2" x="246" y="502"/>`.
   - **Fix:** Shift the 2nd legend group (`probe`) from `x="246"` to `x="290"` (text at `x="308"`).

3. **Field Plan SVG Main Line & Probe Count Mismatch ([index.html](index.html#L93))**:
   - **Bug 1:** Caption states `4 zones · 6 probes · 4 solenoid valves`, but only 4 probes (`P1`–`P4`) exist.
   - **Bug 2:** `<line class="main" x1="22" y1="270" x2="882" y2="270"/>` is drawn at `y="270"`, cutting right across the top of Zone B1 and B2 (`y="265"`) and colliding with the `B1`, `20.3%`, `B2`, `16.8%` text (`y="283"`).
   - **Bug 3:** Zones A1/A2/B1/B2 only span `x="60..580"`, leaving `x="580..882"` (`302px`) empty.
   - **Fix:**
     - Widen each zone from `width="250"` to `width="340"` (Col 1: `x="50..390"`, Col 2: `x="420..760"`) so the 4 zones properly fill the plot, leaving a clean right margin (`x="770..882"`) for the North arrow and scale bar.
     - Move Row B down slightly (`y="275"`) and place `<line class="main">` at `y="260"` (centered cleanly in the corridor between Row A `y_end=245` and Row B `y_start=275`), with its label at `y="254"`.
     - Either update the caption to `4 zones · 4 probes · 4 solenoid valves` or add `P5`/`P6` in the larger zones.

---

### 1.2 Fix JavaScript Crashes & UI Bugs in Dashboard ([src/ui/templates/index.html](src/ui/templates/index.html))

1. **Fix Simulation Crash on Timestamp Parsing ([src/ui/templates/index.html](src/ui/templates/index.html#L454) & [src/api/main.py](src/api/main.py#L427))**:
   - **Bug:** [`run_simulation`](src/api/main.py#L427) formats timestamps as `'%Y-%m-%d %H:%M'` (space separator), while [`renderChart`](src/ui/templates/index.html#L454) calls `r.timestamp.split('T')[1].substring(0, 5)`, throwing `TypeError: Cannot read properties of undefined (reading 'substring')`.
   - **Fix:**
     - In [`src/api/main.py`](src/api/main.py#L427), return ISO-8601 `s.timestamp.isoformat()` (or `'%Y-%m-%dT%H:%M'`).
     - In [`src/ui/templates/index.html`](src/ui/templates/index.html#L454), parse resiliently:
       ```javascript
       const labels = readings.map(r => {
         const parts = r.timestamp.split(/[T ]/);
         return parts[1] ? parts[1].substring(0, 5) : r.timestamp;
       });
       ```

2. **Fix Broken `NaN%` Moisture Progress Bar ([src/ui/templates/index.html](src/ui/templates/index.html#L433) & [src/api/main.py](src/api/main.py#L149-L159))**:
   - **Bug:** `field.soil_status.saturation_pct` is missing from the `/api/v1/fields` payload in [`src/api/main.py`](src/api/main.py#L149-L159), causing `moistureBar` width to evaluate to `NaN%`.
   - **Fix:**
     - Add `'saturation_pct': status.saturation_pct` to `soil_status` in [`src/api/main.py`](src/api/main.py#L150).
     - In [`src/ui/templates/index.html`](src/ui/templates/index.html#L433), scale the progress bar across `[0 .. field_capacity_pct * 1.15]` (so `WP`, `RAW`, and `FC` markers align visually with the bar) and update the bar color dynamically (blue when healthy, amber when near RAW, red when below RAW).

3. **Bind All 4 Top KPI Cards Dynamically ([src/ui/templates/index.html](src/ui/templates/index.html#L83-L125))**:
   - **Bug:** `#et0Val`, `#rainVal`, `#tempVal`, `#actionRequiredBadge`, and `#suggestedWatering` are hardcoded to Field 1's initial values and never update when switching fields or running simulations.
   - **Fix:** Have `/api/v1/fields` include the latest `advisory` preview (or call `/api/v1/advisory/preview/{field_id}` inside `loadFieldData()`) and update all 4 KPI cards dynamically.

4. **Bind WhatsApp Simulator to Selected Field's Phone Number ([src/ui/templates/index.html](src/ui/templates/index.html#L594))**:
   - **Bug:** [`handleUserChatSubmit`](src/ui/templates/index.html#L594) hardcodes `formData.append('From', '+4915123456789')`. When Field 2 (Wachau Apple Orchard, `+436601234567`) is selected, WhatsApp commands still hit Field 1.
   - **Fix:** Store `currentFieldPhone` in JS state when `loadFieldData()` runs and pass `formData.append('From', currentFieldPhone)`.

5. **Fix Header Mobile Overflow & Windows `<select>` Contrast ([src/ui/templates/index.html](src/ui/templates/index.html#L25-L54))**:
   - **Bug:** Header has fixed `h-16` without `flex-wrap`, overflowing on `<768px` viewports; `<option>` elements inside `#fieldSelect` have light text (`text-slate-200`) on a white native dropdown background on Windows.
   - **Fix:** Change `h-16` to `min-h-16 py-2 flex-wrap gap-2` and add `class="bg-slate-800 text-slate-100"` to the `<option>` tags inside `#fieldSelect`.

---

## Priority 2: Backend Logic, State Machine & Concurrency Fixes

### 2.1 Make `DELAY` and `PAUSE` Actually Persist in the Database
- **Files:** [`src/core/models.py`](src/core/models.py#L30-L36), [`src/bot/handler.py`](src/bot/handler.py#L110-L114), [`src/api/scheduler.py`](src/api/scheduler.py#L73-L80)
- **Bug:** Replying `DELAY 1D` or `PAUSE 3D` in [`WhatsAppCommandHandler`](src/bot/handler.py#L111) only logs a message and returns a confirmation string without modifying the database. The scheduler still sends an alert at the next tick.
- **Fix:**
  1. Add `paused_until: Optional[datetime] = Field(default=None)` to [`FieldModel`](src/core/models.py#L34).
  2. In [`WhatsAppCommandHandler`](src/bot/handler.py#L111), parse the number of days (`DELAY 1D` $\rightarrow$ 1 day, `PAUSE 3D` $\rightarrow$ 3 days), set `field.paused_until = datetime.now(timezone.utc) + timedelta(days=days)`, update the latest `AdvisoryAlertModel.status = 'SNOOZED'`, and commit to the DB.
  3. In [`DailyAdvisoryScheduler.run_hourly_dispatch`](src/api/scheduler.py#L73), skip fields where `field.paused_until and field.paused_until > now`.

### 2.2 Support Multiple Fields/Zones per Farmer Phone Number
- **Files:** [`src/bot/handler.py`](src/bot/handler.py#L25-L35)
- **Bug:** `select(FieldModel).where(FieldModel.farmer_phone == from_number).first()` only ever matches the first block if a grower manages multiple vineyard blocks with one phone number.
- **Fix:**
  1. Query `.all()` fields matching `from_number`.
  2. Allow block targeting in commands (e.g., `DONE B2`, `STATUS A1`, `WATER B2 15mm`), or default `DONE` to the field that most recently had a `PENDING` `AdvisoryAlertModel`.
  3. On `STATUS`, return a compact multi-block summary if the grower owns multiple fields.

### 2.3 Stop Artificial Moisture Injection on `DONE` Reply
- **Files:** [`src/bot/handler.py`](src/bot/handler.py#L74-L79)
- **Bug:** When a farmer replies `DONE`, line 77 immediately mutates `sensor.last_moisture_pct = min(fc, sensor.last_moisture_pct + (applied_depth * 0.8))`. If a real LoRaWAN sensor is attached, its next uplink 30 minutes later will overwrite `last_moisture_pct` with the raw probe reading; conversely, if the probe is simulated, `0.8 % VWC per mm` ignores `root_depth_m` ($1\text{ mm}$ across a $1.2\text{ m}$ grapevine root zone is $0.083\%\text{ VWC}$, not $0.8\%\text{ VWC}$—a $10\times$ physics error!).
- **Fix:**
  - Convert applied water depth to VWC using the actual root depth formula from [`SoilSensorSimulator`](src/simulation/sensor_sim.py#L68):
    `added_vwc = (applied_depth * field.irrigation_efficiency) * (100.0 / (1000.0 * root_depth_m))`
  - Only mutate `sensor.last_moisture_pct` directly when `sensor.is_simulated == True`; for physical sensors, track `pending_infiltration_mm` on the field water balance so real sensor telemetry is never corrupted.

### 2.4 Prevent Duplicate Daily WhatsApp Dispatches Under Multiple Workers
- **Files:** [`Dockerfile`](Dockerfile#L46), [`src/api/scheduler.py`](src/api/scheduler.py#L66-L108)
- **Bug:** [`Dockerfile`](Dockerfile#L46) starts `uvicorn` with `--workers 2`. Both worker processes start `DailyAdvisoryScheduler` and both dispatch identical WhatsApp messages at `19:00 UTC`.
- **Fix:**
  - In [`run_hourly_dispatch`](src/api/scheduler.py#L79), before evaluating a field, check if an `AdvisoryAlertModel` already exists for `field.id` within the last 20 hours (idempotency guard).

### 2.5 WhatsApp Business API 24-Hour Window Template Support
- **Files:** [`src/bot/twilio_client.py`](src/bot/twilio_client.py#L24-L44), [`src/core/config.py`](src/core/config.py#L25-L29)
- **Bug:** Outbound daily advisories use free-form `Body`, which Twilio/Meta rejects with Error `63016` if the farmer hasn't messaged the bot in the past 24 hours.
- **Fix:** Add optional `TWILIO_ADVISORY_CONTENT_SID` support to `TwilioWhatsAppClient.send_message` so production deployments can pass `ContentSid` + `ContentVariables` for pre-approved WhatsApp Utility Templates, falling back to `Body` in sandbox/dev mode.

---

## Priority 3: Agronomic & Economic Realism Upgrades

### 3.1 Multi-Depth Sensor Support (`15cm / 30cm / 60cm`)
- **Files:** [`src/core/models.py`](src/core/models.py#L51-L87), [`src/ingestion/lorawan_decoder.py`](src/ingestion/lorawan_decoder.py)
- **Upgrade:** Add optional `moisture_15cm_pct`, `moisture_30cm_pct`, `moisture_60cm_pct`, and `soil_ec_us_cm` columns to `SensorModel` and `TelemetryReadingModel`. Compute effective root-zone moisture as a weighted profile ($25\% \times \theta_{15} + 50\% \times \theta_{30} + 25\% \times \theta_{60}$) and flag deep percolation when $\theta_{60}$ rises by $>2\%$ after irrigation.

### 3.2 Realistic Farm Economics & Labor/Yield Value Model
- **Files:** [`src/simulation/bom_economics.py`](src/simulation/bom_economics.py), [`src/ui/templates/index.html`](src/ui/templates/index.html#L195-L317)
- **Upgrade:**
  1. Include the **LoRaWAN 4G Gateway (€185)** in the hardware capex calculation instead of pretending 3 probes operate without a gateway.
  2. Adjust default DACH vineyard supplemental irrigation baseline from an unrealistic `1,400 m³/ha/yr` to `450 m³/ha/yr` (and `€0.45/m³` extraction/pumping cost), while adding **Labor Saved (hours/season not driving to check blocks/valves)** and **Yield/Quality Protection & EU Compliance Logging** so the ROI remains compelling without using inflated water numbers that a real vintner will immediately distrust.
  3. Add a **COTS Commercial Starter Kit** toggle alongside the DIY Reference BOM in the calculator.

---

## Execution Checklist (historical — see `GAP_ANALYSIS.md` for the truth)

- [x] **Step 1:** Fix root [index.html](index.html) (SVG text overflows, legend collision, main line position, zone widths, and Rust $\rightarrow$ Python CLI commands).
- [x] **Step 2:** Fix dashboard [src/ui/templates/index.html](src/ui/templates/index.html) & [src/api/main.py](src/api/main.py) (timestamp crash in `runScenario`, `NaN%` progress bar, frozen KPI cards, hardcoded phone number, mobile header wrap, Windows `<select>` styling).
- [~] **Step 3:** Upgrade backend ([`models.py`](src/core/models.py), [`handler.py`](src/bot/handler.py), [`scheduler.py`](src/api/scheduler.py)) for persistent `DELAY`/`PAUSE`, multi-field phone routing, idempotent worker dispatch, and physically accurate root-zone moisture updates. — *marked done, but the `DELAY`/`PAUSE` routing was still broken; fixed later in `GAP_ANALYSIS.md` §2.1 / P0.*
- [x] **Step 4:** Verified with full automated test suite ([`tests/`](tests/)) — the suite grew from 24 to **136 passing tests** (see `GAP_ANALYSIS.md` §7).
