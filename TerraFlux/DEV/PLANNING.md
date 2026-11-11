# TerraFlux (FieldSense Precision Viticulture Engine) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `TerraFlux/`  
> **Primary Language / Toolchain:** `Python`  
> **Verification Command:** `pytest tests -q`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**TerraFlux (FieldSense Precision Viticulture Engine)** — A precision viticulture and micro-irrigation platform that pairs low-cost soil moisture sensor arrays with weather evapotranspiration models to deliver automated irrigation advisories directly to farmers via messaging.

### Current Engineering Baseline
A Python precision viticulture and micro-irrigation platform (`src/core/advisor.py`, `water_balance.py`, `models.py`, `api/main.py`, `scheduler.py`, `bot/handler.py`, `bot/telegram_client.py`, `bot/twilio_client.py`, `ingestion/lorawan_decoder.py`, `weather.py`, `actuation/controller.py`) with 21 passing test suites in `tests/`.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Scientific FAO-56 Penman-Monteith Evapotranspiration Algorithm:** Tägliche mathematische Berechnung der Referenz-Evapotranspiration ($ET_0$) auf Basis solarer Einstrahlung, Lufttemperatur, Windgeschwindigkeit und Dampfdruckdefizit.
    
- **Viticultural Phenological Growth Stage Crop Coefficient ($K_c$):** Dynamische Anpassung des Wasserbedarfs von Rebstöcken und Obstgehölzen entlang der phänologischen Vegetationskurve von Austrieb über Blüte bis zur Reife.
    
- **Root-Zone Soil Moisture Depletion & Readily Available Water (RAW):** Kontinuierliche Modellierung des Wassergehalts im durchwurzelten Bodenprofil, der Feldkapazität und des permanenten Welkepunkts zur präzisen Vermeidung von Trockenstress.
    
- **Low-Cost Solar LoRaWAN Multi-Depth Capacitive Probe Node (€55 BOM):** Extrem preisgünstige, solarbetriebene Hardware-Sonde mit kapazitiven Sensoren in mehreren Bodentiefen (z. B. 15, 30, 60 cm) und mehrjähriger autonomer Betriebszeit.
    
- **Multi-Source Satellite & ECMWF Meteorological Forecast Ingestion:** Automatische Fusion von ERA5-Reanalysedaten, Open-Meteo-Modellen und numerischen Wettervorhersagen zur vorausschauenden 7-Tage-Bedarfsplanung.
    
- **Zero-App WhatsApp Conversational UI:** Vollständiger Verzicht auf unübersichtliche Desktop-Dashboards; der Landwirt erhält jeden Abend eine präzise Handlungsempfehlung als Nachricht (*„Morgen ab 05:00 Uhr 12 mm / 45 min bewässern“*).
    
- **Closed-Loop Soil Hydration Feedback (`DONE`, `DELAY`, `WATER`):** Interaktive Chat-Kommandos des Landwirts fließen in Echtzeit zurück in das dynamische Bodenwasserbilanz-Modell und kalibrieren die nächste Empfehlung.
    
- **Energy Tariff & Solar Peak Irrigation Scheduling:** Intelligente zeitliche Verschiebung der Pumpintervalle in Phasen mit hohem Solarstromüberschuss oder günstigen dynamischen Stromtarifen zur Senkung der Betriebskosten.
    
- **Controlled Regulated Deficit Irrigation (RDI) Invariant:** Gezielte Drosselung der Wasserzufuhr während der Beerenreife, um die Konzentration von Zucker, Tanninen und Aromastoffen für Premium-Weinqualitäten wissenschaftlich zu optimieren.
    
- **Salinization & Deep Drainage Leaching Prevention:** Überwachung von Sickerwasserströmen verhindert das Auswaschen wertvoller Nährstoffe ins Grundwasser und schützt empfindliche Reblagen vor Bodenversalzung.
    
- **Mesh-Scale Agronomic Farm Map Topology:** Aggregation einzelner Sensorpunkte zu räumlichen Feuchtigkeitskarten unter Berücksichtigung von Hangneigung, Exposition und Bodentexturunterschieden im Weinberg.
    
- **Trustless Precision Irrigation Settlement Gate:** Lückenlose digitale Protokollierung der Wassereinsparung gegenüber pauschaler Bewässerung als Nachweis für Behörden und Umweltzertifizierungen.

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `TerraFlux/`: **56**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `pyproject.toml` | 50 | Build / Package Manifest |
| `migrations/env.py` | 68 | Core Implementation Module |
| `migrations/versions/46c73bee87d9_initial_schema.py` | 211 | Core Implementation Module |
| `migrations/versions/4e1fb54be86f_add_dispatch_audit_log.py` | 63 | Core Implementation Module |
| `migrations/versions/b1cd8787e1d0_add_irrigation_zone_to_sensors.py` | 46 | Core Implementation Module |
| `migrations/versions/ccc09e66f702_multi_depth_probes_zones_actuation_and_.py` | 160 | Core Implementation Module |
| `src/__init__.py` | 1 | Core Implementation Module |
| `src/actuation/__init__.py` | 21 | Core Implementation Module |
| `src/actuation/controller.py` | 117 | Core Implementation Module |
| `src/actuation/planner.py` | 89 | Core Implementation Module |
| `src/api/__init__.py` | 1 | Core Implementation Module |
| `src/api/database.py` | 413 | Core Implementation Module |
| `src/api/main.py` | 1808 | Core Implementation Module |
| `src/api/scheduler.py` | 476 | Core Implementation Module |
| `src/bot/__init__.py` | 1 | Core Implementation Module |
| `src/bot/dispatch.py` | 156 | Core Implementation Module |
| `src/bot/formatter.py` | 217 | Core Implementation Module |
| `src/bot/handler.py` | 388 | Core Implementation Module |
| `src/bot/i18n.py` | 183 | Core Implementation Module |
| `src/bot/telegram_client.py` | 92 | Core Implementation Module |
| `src/bot/twilio_client.py` | 96 | Core Implementation Module |
| `src/core/__init__.py` | 1 | Core Implementation Module |
| `src/core/advisor.py` | 256 | Core Implementation Module |
| `src/core/config.py` | 69 | Core Implementation Module |
| `src/core/logger.py` | 39 | Core Implementation Module |
| `src/core/models.py` | 255 | Core Implementation Module |
| `src/core/rate_limit.py` | 103 | Core Implementation Module |
| `src/core/security.py` | 102 | Core Implementation Module |
| `src/core/time_utils.py` | 30 | Core Implementation Module |
| `src/core/water_balance.py` | 253 | Core Implementation Module |
| `src/ingestion/__init__.py` | 1 | Core Implementation Module |
| `src/ingestion/lorawan_decoder.py` | 174 | Core Implementation Module |
| `src/ingestion/weather.py` | 109 | Core Implementation Module |
| `src/simulation/__init__.py` | 1 | Core Implementation Module |
| `src/simulation/bom_economics.py` | 202 | Core Implementation Module |
| `src/simulation/sensor_sim.py` | 170 | Core Implementation Module |
| `tests/conftest.py` | 164 | Verification & Test Suite |
| `tests/test_actuation.py` | 107 | Verification & Test Suite |
| `tests/test_advisor.py` | 67 | Verification & Test Suite |
| `tests/test_agronomy_rules.py` | 164 | Verification & Test Suite |
| `tests/test_api_endpoints.py` | 50 | Verification & Test Suite |
| `tests/test_auth_and_onboarding.py` | 166 | Verification & Test Suite |
| `tests/test_auth_hardening.py` | 155 | Verification & Test Suite |
| `tests/test_bom_economics.py` | 46 | Verification & Test Suite |
| `tests/test_field_crud.py` | 175 | Verification & Test Suite |
| `tests/test_history_and_dispatch.py` | 176 | Verification & Test Suite |
| `tests/test_i18n_and_notifications.py` | 243 | Verification & Test Suite |
| `tests/test_lorawan_decoder.py` | 38 | Verification & Test Suite |
| `tests/test_multidepth.py` | 174 | Verification & Test Suite |
| `tests/test_p0_regressions.py` | 236 | Verification & Test Suite |
| `tests/test_scheduler.py` | 81 | Verification & Test Suite |
| `tests/test_security_webhooks.py` | 79 | Verification & Test Suite |
| `tests/test_telegram_bot.py` | 100 | Verification & Test Suite |
| `tests/test_water_balance.py` | 49 | Verification & Test Suite |
| `tests/test_weather_resilience.py` | 17 | Verification & Test Suite |
| `tests/test_zones_and_callbacks.py` | 129 | Verification & Test Suite |

---

## 5. Gap Analysis & Production-Readiness Audit

- **FAO-56 Soil Water Balance & Readily Available Water (RAW) Invariant (`src/core/water_balance.py`):** Must guarantee that daily irrigation recommendations prevent root-zone soil water depletion $D_r$ from exceeding the management allowed depletion threshold ($D_r > \text{RAW} = p \cdot \text{TAW}$), avoiding water stress while barring over-irrigation that causes deep percolation and nutrient leaching.
- **Regulated Deficit Irrigation (RDI) Phenological Curve (`src/core/advisor.py`):** Crop coefficient $K_c$ must adjust dynamically across grapevine phenology (budburst, flowering, veraison, post-harvest) to intentionally apply controlled water stress between fruit set and veraison for phenolic maturity and sugar concentration.
- **Two-Way Messaging Closed Loop & Rate Limiting (`src/bot/handler.py`):** User replies (`DONE`, `DELAY 1D`, `WATER 15mm`) must update the live soil balance model atomically and prevent duplicate water credit applications.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Enforce FAO-56 RAW depletion bounds and deep percolation prevention in water_balance.py | `src/core/water_balance.py`, `tests/test_advisor.py` | `pytest tests/test_advisor.py -q` |
| **TASK-02** | Implement phenological growth stage Kc curve and Regulated Deficit Irrigation (RDI) | `src/core/advisor.py`, `tests/test_advisor.py` | `pytest tests/test_advisor.py -q` |
| **TASK-03** | Harden two-way bot command state machine and audit log in bot/handler.py | `src/bot/handler.py`, `tests/test_bot_handler.py` | `pytest tests/test_bot_handler.py -q` |
