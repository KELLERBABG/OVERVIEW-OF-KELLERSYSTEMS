# TerraFlux — Development Roadmap & Action Tasks

> This file is the authoritative internal development roadmap and task sequencing plan.
> **Language:** `Python`
> **Summary:** A Python precision viticulture and micro-irrigation platform (`src/core/advisor.py`, `water_balance.py`, `models.py`, `api/main.py`, `scheduler.py`, `bot/handler.py`, `bot/telegram_client.py`, `bot/twilio_client.py`, `ingestion/lorawan_decoder.py`, `weather.py`, `actuation/controller.py`) with 21 passing test suites in `tests/`.

## Task Queue

- [ ] **TASK-01: Enforce FAO-56 RAW depletion bounds and deep percolation prevention in water_balance.py**
  - **Target Files:** `src/core/water_balance.py`, `tests/test_advisor.py`
  - **Context Files:** `src/core/models.py`
  - **Verify:** `pytest tests/test_advisor.py -q`
  - **Spec:** Harden `src/core/water_balance.py` to enforce two strict agronomic invariants:

1. **Depletion Bound:** Recommended irrigation depth $I_{\text{rec}}$ must bring root-zone depletion $D_r$ back into the readily available water range ($D_r \le \text{RAW} = p \cdot \text{TAW}$), ensuring transpiration is not reduced.
2. **Leaching Prevention:** Recommended irrigation depth must never exceed remaining root-zone storage capacity ($I_{\text{rec}} \le D_r$), preventing deep drainage loss and fertilizer runoff into groundwater.
3. Add unit tests in `tests/test_advisor.py` verifying compliant irrigation recommendations under synthetic drought and heavy rain scenarios.

- [ ] **TASK-02: Implement phenological growth stage Kc curve and Regulated Deficit Irrigation (RDI)**
  - **Target Files:** `src/core/advisor.py`, `tests/test_advisor.py`
  - **Context Files:** `src/core/water_balance.py`
  - **Verify:** `pytest tests/test_advisor.py -q`
  - **Spec:** Update `src/core/advisor.py` to compute dynamic $K_c$ from the vineyard's phenological stage:

1. Budburst to Flowering ($K_c \approx 0.3$), Berry Set to Veraison (RDI stress phase: clamp $K_c$ to $0.4\text{--}0.5$), Veraison to Harvest ($K_c \approx 0.6\text{--}0.7$), Post-Harvest ($K_c \approx 0.4$).
2. Prove via unit tests that during the RDI phase, the advisor targets controlled moderate water stress (depletion maintained between $0.4 \text{ TAW}$ and $0.6 \text{ TAW}$) without triggering permanent wilting.

- [ ] **TASK-03: Harden two-way bot command state machine and audit log in bot/handler.py**
  - **Target Files:** `src/bot/handler.py`, `tests/test_bot_handler.py`
  - **Context Files:** `src/api/main.py`, `src/core/models.py`
  - **Verify:** `pytest tests/test_bot_handler.py -q`
  - **Spec:** Ensure `src/bot/handler.py` handles farmer text commands (`DONE`, `DELAY 1D`, `WATER 15mm`, `STATUS`) with idempotent database updates and rate-limiting.

1. Double-sending `DONE` on the same day must not apply duplicate irrigation depths.
2. Add unit tests in `tests/test_bot_handler.py` for command parsing, multi-depth soil balance updates, and localized language formatting.
