# TerraFlux

> **Precision Viticulture & Micro-Irrigation Platform (Powered by FieldSense Agronomic Engine)**  
> **One-liner:** Cheap soil probe array + Open-Meteo ET₀ → one daily Telegram advisory: *"Run the drip for 12 mm tomorrow at 06:00 — 40 minutes"*. WhatsApp stays available as a fallback channel. No dashboard for the farmer.

---

## 🌾 Overview & Value Proposition

Small-to-midscale European vineyards and orchards (20–100ha in Germany, Austria, Italy, France) face **+30% surging water tariffs and energy pumping costs**, yet existing precision agriculture platforms (John Deere Ops Center, Climate FieldView) require complex desktop dashboards and costly multi-thousand-euro sensor grids.

**TerraFlux (FieldSense Engine)** eliminates software friction entirely:
1. **Low-Cost Hardware BOM (€89 retail / €55 BOM)**: Solar-powered LoRaWAN soil moisture & temperature probe.
2. **Open Satellite & Weather Model**: Live reference evapotranspiration ($ET_0$) from Open-Meteo Agro API + Copernicus/ECMWF forecasts.
3. **FAO-56 Soil Water Balance Engine**: Scientific calculation of Readily Available Water (RAW), root-zone depletion, and crop coefficients ($K_c$).
4. **Zero-App Message Interface (Telegram first)**: Farmers receive a single actionable message every evening with the exact depth to run (gross mm), the start time and the drip runtime. The message names the irrigation zone and valve, and the inline buttons carry the block, so a grower managing several blocks confirms the right one. Replies (`DONE`, `STATUS`, `WATER 15mm`, `DELAY 1D`) update the soil model in real time. WhatsApp (Twilio / Meta) is supported as a fallback channel with the same commands.

---

## 🚀 Quickstart & Running the App

### 1. Installation
```bash
# Clone and install dependencies
pip install -r requirements.txt
```

### 2. Launch the Application Server
```bash
python -m src.api.main
# Or run with uvicorn:
uvicorn src.api.main:app --host 0.0.0.0 --port 8000 --reload
```

Open your browser at **`http://localhost:8000`** to access:
* **Live Field Monitor & Soil Water Balance Timeline**.
* **Virtual WhatsApp Smartphone Mockup** (test two-way interactive messaging in real time).
* **Hardware BOM & Farmer ROI Economics Calculator**.
* **Fast-Forward Physical Simulation Playground** (drought, storm, and heatwave scenarios).

### 3. Run Automated Test Suite
```bash
pip install -r requirements-dev.txt
python -m pytest -v                      # full suite (isolated DB, no network needed)
ruff check src tests                     # lint gate (also enforced in CI)
```

### 4. Database Migrations
Schema changes are managed with **Alembic** (`migrations/`). The application also
applies backwards-compatible column backfills on startup for older SQLite databases.

```bash
alembic upgrade head                     # apply all pending migrations
alembic revision --autogenerate -m "..." # create a revision after a model change
alembic check                            # fail if the models drifted from the migrations
```

---

## 📡 API Endpoints (Production & Ingestion)

> **Authentication:** every field-scoped endpoint requires a bearer token
> (`Authorization: Bearer <token>`) obtained from `/api/v1/auth/login`. Growers only
> see the fields they own; administrators see all of them. Webhook endpoints use their
> own shared secrets (`TTN_WEBHOOK_SECRET`, `X-Twilio-Signature`,
> `X-Telegram-Bot-Api-Secret-Token`) and fail closed in production.

| Method | Endpoint | Auth | Description |
| :--- | :--- | :--- | :--- |
| `GET` | `/` | – | Interactive Web Dashboard, Virtual Telegram Phone & BOM Evaluator |
| `GET` | `/api/v1/health`, `/healthz` | – | Service health status (liveness) |
| `GET` | `/readyz` | – | Readiness probe including database connectivity |
| `POST` | `/api/v1/auth/register` | – | Create a grower account (always role `grower`) |
| `POST` | `/api/v1/auth/login` | – | Exchange credentials for a bearer token |
| `POST` | `/api/v1/auth/logout` | token | Revoke every token issued to the caller |
| `GET` | `/api/v1/auth/me` | token | Profile, owned fields and Telegram invite links |
| `POST` | `/api/v1/admin/users/{id}/role` | admin | Promote / demote a user account |
| `GET` | `/api/v1/fields` | token | List owned fields, soil profiles & live soil status |
| `POST` | `/api/v1/fields` | token | Create a new farm block |
| `GET` | `/api/v1/fields/{id}` | token | Block detail incl. probes and per-zone status |
| `PATCH` | `/api/v1/fields/{id}` | token | Update crop, soil, drip system, alert hour, … |
| `DELETE` | `/api/v1/fields/{id}` | token | Deactivate a block (history is preserved) |
| `GET` | `/api/v1/fields/{id}/sensors` | token | Probes of a block incl. irrigation zones |
| `POST` | `/api/v1/fields/{id}/sensors` | token | Register a probe (DevEUI) against a block |
| `PATCH` | `/api/v1/sensors/{id}` | token | Update a probe or move it to another block |
| `POST` | `/api/v1/sensors/{id}/bind` | token | Bind an orphaned probe to a block |
| `DELETE` | `/api/v1/sensors/{id}` | token | Remove a probe and its telemetry history |
| `POST` | `/api/v1/fields/{id}/invite-token` | token | Regenerate the private Telegram onboarding link |
| `GET` | `/api/v1/fields/{id}/telemetry` | token | Historical sensor telemetry timeline (VWC %, temp, battery) |
| `GET` | `/api/v1/fields/{id}/advisories` | token | Advisory history (net + gross depth, status) |
| `GET` | `/api/v1/fields/{id}/irrigations` | token | Irrigation event history |
| `GET` | `/api/v1/fields/{id}/dispatches` | token | Delivery audit trail (per channel, per attempt) |
| `GET` | `/api/v1/fields/{id}/history` | token | Combined operational history + summary counters |
| `GET` | `/api/v1/fields/{id}/actuation` | token | Valve plans and valve commands (audit trail) |
| `POST` | `/api/v1/fields/{id}/actuation/plan` | token | Compute the valve plan for the current advice (no hardware) |
| `POST` | `/api/v1/fields/{id}/actuation/execute` | token | Hand the valve plan to the controller (simulated by default) |
| `POST` | `/api/v1/advisory/evaluate/{id}` | token | Run FAO-56 decision engine & dispatch the advisory |
| `POST` | `/api/v1/telemetry/ttn` | secret | Production webhook for **The Things Network (TTN v3) / ChirpStack** |
| `POST` | `/api/v1/whatsapp/webhook` | signature | Production webhook for **Twilio / Meta WhatsApp API** |
| `POST` | `/api/v1/telegram/webhook` | secret | Production webhook for **Telegram Bot API** (messages + inline buttons) |
| `POST` | `/api/v1/simulation/run` | token | Fast-forward multi-day physical drying curve simulation |
| `POST` | `/api/v1/economics/evaluate` | – | Compute BOM cost, water bill savings, and payback period |

### Depth convention (net vs. gross)

| Term | Meaning |
| :--- | :--- |
| **Net depth** | Water that actually reaches the root zone. This is the agronomic target the engine calculates (`decision.target_depth_mm`, `AdvisoryAlertModel.recommended_depth_mm`). |
| **Gross depth** | What the grower must run through the valve (`decision.gross_depth_mm` = net ÷ irrigation efficiency). This is the number in the message. |
| **Run time** | `net depth ÷ net application rate` (`FieldModel.application_rate_mm_h`). |

Irrigation events store the **net** depth; the gross value and the efficiency are kept
in the event notes so the audit trail stays complete.

---

## 🌍 Agronomic Capabilities

| Capability | What it does |
| :--- | :--- |
| **Multi-depth probes** | Optional 15 / 30 / 60 cm profile. The effective root-zone moisture is the weighted mean (25 / 50 / 25 %, renormalised over the depths that are reported). Soil EC is stored as well. |
| **Deep percolation** | When the 60 cm layer gains more than 2 % of VWC, the reading is flagged: the applied water left the root zone instead of being taken up. |
| **Regulated Deficit Irrigation** | The crop configuration (`config/crops.yaml` → `stress_strategy`) is honoured: in the late stages the depletion fraction is raised, so the block is not watered back to field capacity too early. |
| **Infiltration limit** | The recommendation is capped at what the soil can take during the irrigation window (`infiltration_rate_mm_h`). The advice then says to split the cycle. Runoff is flagged when the drip applies faster than the soil absorbs. |
| **Rain / ET integration** | A stale probe reading is corrected with the crop demand and the rainfall since the uplink, and irrigation is suppressed when the forecast rain covers most of the deficit. |
| **Zone level advice** | Probes can be assigned to irrigation zones (`A1`, `B2`, …) and to a valve (`V1`). The driest zone of a block decides the recommendation, and the message names zone and valve. |
| **Scoped one-tap buttons** | Telegram buttons carry the scope of the advice (`ACTION|field_id|zone`), so a grower managing several blocks confirms the right block. |
| **Valve actuation** | An advisory can be turned into an executable valve plan and handed to a controller (`VALVE_CONTROLLER_URL`). Without a controller the command is simulated and recorded in `actuation_events`. Automatic actuation is off by default (`AUTO_ACTUATION_ENABLED`). |
| **Language** | Grower facing messages come from a message catalogue (`src/bot/i18n.py`) with English and German; the language is a property of the block. |
| **Quiet hours, weekly digest, escalation** | No advisories inside the quiet hours of a block, an optional weekly summary, and a second contact that is notified when a critical recommendation stays unconfirmed. |


---

## 🛠️ Hardware Bill of Materials (€89 Kit)

> **ROI assumptions (see `src/simulation/bom_economics.py`):** supplemental irrigation baseline
> 450 m³/ha per year for vineyards, extraction and pumping cost €0.45/m³ on top of the water
> tariff, and water savings of **18 %** — the conservative end of the 15–22 % band reported by
> FAO regulated-deficit-irrigation field studies. Labour savings (block walks avoided during the
> irrigation season) are counted separately and are *not* part of the 18 % water figure.

| Category | Component Description | Ref Part | BOM Cost (€) |
| :--- | :--- | :--- | :---: |
| **Sensor Probe** | Capacitive Soil Moisture + Temp (RS485 / Analog) | RS-WS-N01-TR | €16.50 |
| **MCU & Radio** | LoRaWAN Transceiver Node (EU868 / US915) | Seeed Wio-E5 / ESP32 | €12.20 |
| **Power** | LiFePO4 18650 Battery (3.2V 1500mAh) + BMS | IFR18650 / TP5000 | €4.80 |
| **Solar** | 0.6W Monocrystalline ETFE Mini Panel | 5V 120mA Laminated | €3.40 |
| **Enclosure** | IP67 Enclosure + PG7 Gland + 60cm Ground Stake | ABS Waterproof Box | €8.50 |
| **PCBA & Box** | PCB Assembly, Conformal Coating, Calibration Box | Custom PCBA | €9.60 |
| **Total BOM** | **Manufacturing Cost** | *(Target Retail: €89)* | **€55.00** |

---

## 🤖 Telegram Bot & Interactive Zero-Fee Commands

Farmers interact seamlessly via the **TerraFlux Telegram Bot** (zero Meta fees, zero templates, zero subscription overhead):
* **One-Click Inline Buttons:** Every daily advisory includes interactive action buttons (`[✅ Confirm 12mm Done]`, `[⏳ Delay 24h]`, `[📊 Live Status]`).
* `DONE` or `YES` — Confirms irrigation completed; logs event and replenishes soil moisture model.
* `STATUS` — Returns live soil moisture (% VWC), soil temperature, probe battery, and last uplink.
* `WATER 15mm` — Records custom irrigation depth.
* `DELAY 1D` — Postpones irrigation advisory by 24 hours.
* `PAUSE 3D` — Silences alerts for 3 days (e.g. during canopy spraying or harvest).
* `HELP` — Shows command reference.

*(Note: Legacy WhatsApp / Twilio webhooks are also supported via `/api/v1/whatsapp/webhook`).*
