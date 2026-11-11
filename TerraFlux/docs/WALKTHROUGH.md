# TerraFlux (FieldSense Engine) — Walkthrough

A deployable agronomy platform with real LoRaWAN IoT telemetry ingestion, live Open-Meteo weather integration, a FAO-56 soil water balance engine, a two-way Telegram bot (WhatsApp as fallback), zone level valve planning, multi-tenant accounts and an interactive ROI & simulation playground.

---

## 1. What Was Built

### 1.1 Core Agronomic Engine (`src/core/`)
* **`src/core/water_balance.py`**:
  * Implements standard **FAO-56 Soil Water Balance** equations.
  * Calculates **Total Available Water (TAW)**, **Readily Available Water (RAW)**, and critical moisture threshold ($\theta_{\text{threshold}}$).
  * Computes crop evapotranspiration ($ET_c = K_c \times ET_0$) across phenological stages (Grapevine, Maize, Apple, Olive).
  * Weights a multi-depth profile (15 / 30 / 60 cm) into one effective root-zone value and detects **deep percolation**.
  * Converts a **net** depth into the **gross** depth and the drip run time (one convention across the whole codebase).
* **`src/core/advisor.py`**:
  * Evaluates live root-zone soil moisture vs. depletion thresholds, per irrigation zone.
  * Corrects a stale probe reading for the crop demand and the rain since the uplink.
  * Implements **rain suppression** (48 h forecast and a 72 h forecast that covers the actual deficit).
  * Honours the crop **RDI strategy** and caps the recommendation at what the soil can infiltrate and the drip can deliver in one window.
  * Determines the optimal application start time (06:00 / 07:00) to minimize evaporative loss.

### 1.2 IoT Telemetry & Ingestion (`src/ingestion/`)
* **`src/ingestion/lorawan_decoder.py`**: Decodes commercial LoRaWAN payloads (**Dragino LSE01**, **Cayenne LPP**, the **TerraFlux multi-depth frame** and **The Things Network v3 / ChirpStack** webhooks), including soil EC.
* **`src/ingestion/weather.py`**: Real-time integration with the **Open-Meteo Agro Weather API** (fetches $ET_0$, precipitation forecasts, temperature, and solar radiation).

### 1.3 Message Channels & Two-Way Bot (`src/bot/`)
* **`src/bot/i18n.py`**: Message catalogue (English / German) shared by all channels.
* **`src/bot/formatter.py`**: Formats the daily advisory, status reports, weekly digest and escalation notice. Telegram buttons carry the scope (`ACTION|field_id|zone`).
* **`src/bot/handler.py`**: Handles two-way interactive commands (`DONE`, `YES`, `STATUS`, `WATER 15mm`, `DELAY 1D`, `PAUSE 3D`, `HELP`), resolves the block and zone from the button payload and updates the soil water model on confirmation.
* **`src/bot/dispatch.py`**: Delivers the advisory through every channel with retry and a persisted delivery audit trail.
* **`src/bot/telegram_client.py`** / **`src/bot/twilio_client.py`**: Telegram (primary) and Twilio WhatsApp (fallback) clients, each with a simulation mode.

### 1.4 Hardware BOM & Farmer ROI Economics Model (`src/simulation/`)
* **`src/simulation/bom_economics.py`**:
  * Detailed €55 manufacturing Bill of Materials breakdown for the €89 retail kit.
  * Farmer ROI calculator with the honest baseline: 450 m³/ha supplemental irrigation for vineyards, €0.45/m³ extraction and pumping, 18 % water savings (FAO RDI band 15–22 %), labour savings, hardware tiers (DIY / COTS) and an optional gateway.
* **`src/simulation/sensor_sim.py`**:
  * Physically accurate hourly simulation of root-zone infiltration, gravitational drainage, diurnal solar depletion curves, and solar battery charging.

### 1.5 Accounts, Actuation, Audit & Dashboard
* **`src/core/security.py`** / **`src/core/rate_limit.py`**: PBKDF2 password hashing, signed bearer tokens with server-side revocation, per-IP and per-account rate limits with lockout.
* **`src/actuation/`**: Turns an advisory into an executable valve plan (`ValvePlan`), hands it to a simulated or HTTP valve controller and records every state change in `actuation_events`.
* **`src/api/scheduler.py`**: Hourly dispatcher with a cross-process lease (only one worker notifies), quiet hours, weekly digest and escalation of unconfirmed critical advisories.
* **`src/api/main.py`**: REST API with tenant isolation, Webhooks for TTN / Telegram / WhatsApp, Alembic-managed schema and OpenAPI documentation.
* **`src/ui/templates/index.html`**:
  1. **Live Field Telemetry & Water Balance Chart**: Interactive Chart.js graph comparing sensor VWC % with the FAO-56 RAW threshold.
  2. **Telegram simulator**: Live chat mockup with the scoped inline buttons.
  3. **Advisory & Irrigation History**: What was advised, what was applied, and whether the message reached the grower.
  4. **Hardware BOM & ROI Calculator**: Sliders plus hardware tier and gateway options.
  5. **Simulation Playground**: 1-click execution of 14-day drought spells and heatwaves.

---

## 2. Test Verification

The automated test suite runs via `pytest` and verifies all components:

```bash
$ python -m pytest -q
........................................................................ [100%]
136 passed
```

The suite runs against a throwaway SQLite database and blocks all outbound network
access, so it is deterministic and offline-safe. It covers the agronomy rules
(`test_advisor.py`, `test_agronomy_rules.py`, `test_water_balance.py`), the API and
tenant isolation (`test_api_endpoints.py`, `test_field_crud.py`), authentication
(`test_auth_and_onboarding.py`, `test_auth_hardening.py`), the message channels
(`test_telegram_bot.py`, `test_i18n_and_notifications.py`), the ingestion path
(`test_lorawan_decoder.py`, `test_multidepth.py`, `test_weather_resilience.py`), the
scheduler (`test_scheduler.py`), actuation (`test_actuation.py`) and the audit trail
(`test_history_and_dispatch.py`).

---

## 3. How to Run & Experience the App

1. **Start the Server**:
   ```bash
   python -m src.api.main
   ```
2. **Open the Web Dashboard**:
   Navigate to `http://localhost:8000` in your web browser.
3. **Try the Key Flows**:
   * Sign in first (demo account `admin@terraflux.ag` / `kaiserstuhl2026`) - field data, telemetry and dispatch are tenant scoped.
   * Click **"Dispatch Advisory"** to trigger a real-time FAO-56 decision and watch the message appear in the virtual phone.
   * Click **"Confirm Done"** in the phone to simulate a grower confirming the watering of that zone, and watch the soil moisture graph replenish.
   * Adjust the sliders in the **BOM & ROI Calculator** (and toggle the COTS kit or the gateway) to see the payback for a 20 ha block.
   * Click **"14-Day Drought Spell"** under the simulation playground to observe multi-day soil depletion.
   * Check the **Advisory & Irrigation History** card to see what was advised, what was applied and whether the message was delivered.
