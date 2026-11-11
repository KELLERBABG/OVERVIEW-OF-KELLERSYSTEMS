# TerraFlux — Gap Analysis & Completion Verification

**Repo:** `C:\Users\lukas\Downloads\SYSTEMS & CREATIONS\TerraFlux`
**Date:** 2026-11-10
**Scope:** Independent audit of the "≈85% complete" claim, verification of the previously stated gap list, and a list of *additional* findings.
**Method:** Full read of `src/`, `tests/`, `config/`, `docs/`, `Dockerfile`, `docker-compose.yml`, `.github/workflows/ci.yml`; plus live execution against a throwaway SQLite DB (`DATABASE_URL` override) using `fastapi.testclient` to confirm runtime behaviour rather than inferring it.

**Legend:** ✅ Verified by execution · 🔍 Verified by code read · ⚠️ Partially correct / needs restating

> **P0, P1, P2 and P3 remediation are complete** (2026-11-10). Every numbered item of this
> document is now **fixed** — see the change logs in §7.1 (P0), §7.2 (P1), §7.3 (P2) and
> §7.4 (P3). The findings below are kept as the audit record of the pre-remediation state.

---

## 0. Headline verdict

| Dimension | Assessment |
| :--- | :--- |
| Feature completeness (demo/MVP) | **~85%** — the vertical slice (ingest → balance → advise → chat → simulate → economics → dashboard) is real and works end-to-end. |
| Production readiness | **~55–60%** — undermined by two verified functional bugs and four verified unauthenticated side-effect endpoints (see §2, §3). |
| Test signal quality | **Misleading** — 33 tests pass, yet the `DELAY`/`PAUSE` feature is broken and half the API is unguarded. Green CI does not mean the product works. |

The gap between "85% feature-complete" and "not deployable" is **not missing features — it is unguarded surface area and one broken state machine.**

---

## 1. Verification of the previously stated answers

| Previous claim | Verdict | Evidence / correction |
| :--- | :--- | :--- |
| "No clear CRUD for fields/sensors/users" | ✅ Confirmed | `src/api/main.py` exposes only `GET /fields`, `POST /fields/{id}/invite-token`, `GET /fields/{id}/telemetry`. No create/update/delete for fields, probes, or users. |
| "Onboarding is partly manual" | ✅ Confirmed + **worse than stated** | Probe→field binding does not exist at all; TTN auto-registration creates probes with `field_id=None` (`main.py:419-432`) → orphan probes. |
| "No alert/event history APIs or UI" | ✅ Confirmed | `AdvisoryAlertModel` / `IrrigationEventModel` are write-only from the API's perspective. |
| "Multi-depth sensors not realized" | ✅ Confirmed | Single moisture column on `SensorModel` / `TelemetryReadingModel`; decoded EC is discarded (`lorawan_decoder.py:41`). |
| "Actuation is advisory-only" | ✅ Confirmed + **marketing risk** | No valve/zone entity exists at all, while root `index.html` claims "drives the valves directly" and shows V1–V4 solenoids. |
| "Background scheduling fragile under multiple workers" | ✅ Confirmed + **narrowed to a race** | The 20h idempotency guard (`scheduler.py:104-114`) is read-then-write, so two uvicorn workers can both pass it and double-send. No leader election. |
| "DB/migrations ad hoc" | ✅ Confirmed | SQLite-only hand-rolled `ALTER TABLE` loop (`database.py:42-75`); no Alembic; no Postgres path. |
| "Security defaults need tightening" | ✅ Confirmed + **understated** | Found a **privilege-escalation bug** and **four unauthenticated side-effect endpoints** — see §3.1/§3.2. |
| "Observability basic" | ✅ Confirmed | Plain stdout logging; no metrics/tracing; **and the logger itself crashes on emoji under Windows cp1252** (§2.5). |
| "Dashboard ↔ backend contract not fully aligned" | ✅ Confirmed | ET₀/rain/temp KPI cards are only refreshed by `triggerAdvisory()`, not on field switch; economics panel shows hardcoded placeholder numbers (`2,380%`, `38% margin`). |
| "Testing lighter on integration/E2E, concurrency, contract" | ✅ Confirmed + **new finding** | Worse than "light": tests run against the developer's **real `fieldsense.db`**, mutate it, and make **live network calls to Open-Meteo** with no HTTP mocking. |
| "CI/CD configuration isn't prominent in the repo root" | ⚠️ Correction | **CI exists**: `.github/workflows/ci.yml` runs `pytest -v` on Python 3.11/3.12. It is *minimal* (no coverage gate, no lint/type-check, no image build, no service matrix) — not *absent*. |
| "Savings claims inconsistent across docs" | ✅ Confirmed | Four different numbers live simultaneously: **18%** (`bom_economics.py:130`), **18–28%** (`index.html:159`), **−22.4%** (`index.html:113` hero), **15–22%** (`PLAN.md:24`, README). |
| "≈80–90% of an MVP/demo" | ⚠️ Restated | Correct for *demo* scope. For *field-deployable* scope it is materially lower because the `DELAY`/`PAUSE` control path is dead (§2.1) and advisory dispatch is unauthenticated (§3.2). |
| "PLAN.md checklist is complete / 24 of 24 tests passing" | ⚠️ Stale | Suite is now **33 tests, all passing** (`pytest -q` → `33 passed in 3.86s`). `PLAN.md:145` is out of date, and Steps 2–3 are marked complete while §2.1 shows Step 3 is not actually working. |

---

## 2. Verified functional bugs (NEW — not in the previous list)

### 2.1 🔴 `DELAY 1D` / `PAUSE 3D` are misrouted to the `WATER` branch and never pause anything
**File:** `src/bot/handler.py:171-174` (WATER branch) executes **before** `src/bot/handler.py:197` (DELAY/PAUSE branch).

```python
water_match = re.search(r'(?:WATER|GEWÄSSERT)?\s*(\d+(?:\.\d+)?)\s*(?:MM)?', clean_text)
if "WATER" in clean_text or (water_match and float(water_match.group(1)) > 0 and len(clean_text) <= 8):
```

The keyword prefix is **optional** and the length guard is `<= 8`, which is exactly the length of `"DELAY 1D"` and `"PAUSE 3D"`. The regex therefore matches the digit *inside* the command.

**Executed proof** (throwaway DB, real handler, real webhook endpoint):
```
DELAY_1D_reply:                    ✅ *Logged 1.0mm Irrigation for Kaiserstuhl Vineyard (Block A - Spätburgunder)*
DELAY_1D_irrigation_events_created: 1
DELAY_1D_paused_until_set:          False
DELAY_1D_field_paused_until:        None
PAUSE_3D_reply:                    ✅ *Logged 3.0mm Irrigation for Kaiserstuhl Vineyard (Block A - Spätburgunder)*
PAUSE_3D_paused_until:             None
```
**Impact:** (a) the entire `DELAY`/`PAUSE` feature — listed as *done* in `PLAN.md` Step 3 — is dead for 1–9 day values; (b) it **writes false irrigation records** into the audit trail (`IrrigationEventModel` rows of 1mm / 3mm that never happened); (c) `paused_until` is never set, so the scheduler keeps messaging the grower.
**Fix:** move the DELAY/PAUSE branch above WATER; require the literal keyword for WATER (`re.fullmatch(r'(?:WATER|GEWÄSSERT)\s*(\d+(?:\.\d+)?)\s*MM?', clean_text)`); drop the `len <= 8` heuristic entirely.

### 2.2 🔴 Both seeded fields share the same Telegram `chat_id` (`123456789`)
**Files:** `src/core/models.py:50` (default `'123456789'`); `src/api/database.py:126-235` (seed never sets `farmer_chat_id`).

**Executed proof:**
```
field_chat_ids: [('Kaiserstuhl Vineyard (Bl', '123456789', '+4915123456789'),
                  ('Wachau Apple Orchard (Ga', '123456789', '+436601234567')]
```
**Impact:** both fields' advisories are delivered to one chat; `handle_incoming_message` (`handler.py:71-102`) then resolves the sender ambiguously (phone *or* chat match) and can log irrigation against the **wrong field**.
**Fix:** seed distinct or `None` chat ids; make resolution deterministic (exact phone → exact chat → pending-alert tiebreak → refuse if ambiguous).

### 2.3 🟠 Gross-vs-net depth inconsistency between advisor, duration and moisture update
- `FieldModel.application_rate_mm_h` (`models.py:64-70`) **already multiplies by `irrigation_efficiency`** → duration = depth / *net* rate (`advisor.py:97-101`) → the recommended depth is a **net** figure.
- `calculate_added_vwc` (`handler.py:126-131`) multiplies the applied depth by efficiency **again** → infiltration is under-counted (≈10% low at 0.90 efficiency).
- The farmer-facing number (`formatter.py:31`, *"Water X mm"*) is therefore ambiguous: it is net, but a grower sets the valve in gross.
**Fix:** pick one convention (recommendation = **gross valve mm**, net = gross × efficiency) and apply it identically in `advisor.py`, `water_balance.calculate_irrigation_duration_min`, `formatter.py`, and `handler.py`.

### 2.4 🟠 `STATUS` uses a hardcoded 20% instead of the FAO-56 threshold
`handler.py:120` — `status_text = "Soil moisture is optimal" if (sensor and sensor.last_moisture_pct > 20)`.
For the seeded vineyard the real RAW threshold is **16.6%**, so a reading of 17% is reported to the grower as *"Soil moisture is optimal"* while the advisory engine simultaneously flags it as below threshold. Use `WaterBalanceEngine.analyze_soil_status(...).threshold_moisture_pct`.

### 2.5 🟠 Logger crashes on emoji under a non-UTF-8 console (Windows)
`src/core/logger.py:13-14` writes to `sys.stdout` with the platform default encoding; every advisory message contains ✅/💧/🚨.

**Executed proof** (Windows, cp1252):
```
--- Logging error ---
UnicodeEncodeError: 'charmap' codec can't encode character '\u2705' in position 87
```
**Impact:** every dispatch logs a traceback to stderr; in Docker/Linux this is masked by the UTF-8 locale, so the bug hides in CI and appears only on operator machines.
**Fix:** `sys.stdout.reconfigure(encoding="utf-8", errors="replace")` in `setup_logger()`, plus `ENV PYTHONUTF8=1` / `PYTHONIOENCODING=utf-8` in the Dockerfile.

### 2.6 🟠 Scheduler blocks the event loop and races across workers
- `scheduler.py:57` calls the **synchronous** `run_hourly_dispatch` (DB writes + HTTP with 8s timeouts) directly inside the async loop → the whole event loop stalls for the duration of every weather call.
- `Dockerfile:46` runs `--workers 2`; `main.py:110-117` starts the scheduler in **every** worker → two workers can both pass the 20h idempotency check before either commits → duplicate WhatsApp/Telegram messages to growers.
**Fix:** `await asyncio.to_thread(cls.run_hourly_dispatch, ...)`; move dispatch to a single-worker deployment (or a dedicated scheduler container / advisory-lock row).

### 2.7 🟡 Startup seed race
`create_db_and_tables()` runs at **import time** (`main.py:108`) *and* in `lifespan` (`main.py:113`). With two workers both call `seed_initial_data()` concurrently → possible unique-constraint collision on `users.email` / `sensors.deveui`. Make seeding idempotent-at-the-DB level (INSERT OR IGNORE) or run it only once.

### 2.8 🟡 Empty WhatsApp webhook payload defaults to the seeded grower's number
`main.py:531` — `from_number = '+4915123456789'`. An empty POST therefore resolves to the first seeded field and returns live status for it. Default to empty and reject unknown/empty senders.

---

## 3. Verified security & tenancy gaps (NEW)

### 3.1 🔴 Privilege escalation via self-registration
`main.py:89-93` accepts `role` from the client; `main.py:196-198` — `role='admin' if payload.role == 'admin' else 'grower'`.

**Executed proof:**
```
register_admin_role_status:   200
register_admin_role_returned: admin
stored_role_in_db:            admin
```
**Fix:** drop `role` from `RegisterRequest` (or ignore it), always create `grower`, and expose admin promotion only behind an authenticated admin endpoint.

### 3.2 🔴 Four endpoints with no authentication that mutate data or leak tenant data
| Endpoint | File | Executed proof | Impact |
| :--- | :--- | :--- | :--- |
| `GET /api/v1/fields` | `main.py:269` | `200`, 2 fields, exposes `farmer_phone`, `farmer_chat_id`, `invite_token`, `telegram_deep_link` | Cross-tenant PII leak **and** invite-token leak → any Telegram user can run `/start <token>` and **take over a victim's advisory channel** (`handler.py:26-46`). |
| `POST /api/v1/advisory/evaluate/{id}` | `main.py:458` | `200` → simulated WhatsApp dispatch payload returned | Anyone can **send WhatsApp/Telegram messages to real growers** (spam/abuse/cost), and force advisory rows. |
| `POST /api/v1/simulation/run` | `main.py:597` | sensor `last_moisture_pct` **19.7 → 21.2**; `telemetry_readings` rows **21 → 45** | **Silent corruption of production telemetry**; synthetic rows are indistinguishable from real uplinks (see §5.3). |
| `GET /api/v1/fields/{id}/telemetry` | `main.py:360` | `45` readings returned without a token | Cross-tenant telemetry leak. |

**Fix:** require `get_current_user` + an ownership check on all four; make simulation an explicit `?dry_run=true` / admin-only operation; strip `farmer_phone`, `farmer_chat_id`, `invite_token`, `telegram_deep_link` from unauthenticated responses.

### 3.3 🟠 Token/session weaknesses (`src/core/security.py`)
- PBKDF2-HMAC-SHA256 × 120k with per-user salt and `hmac.compare_digest` — **good**, keep it.
- Tokens are HMAC-SHA256-signed with a 72h expiry (`:39-64`) but there is **no revocation list, no refresh, no server-side logout**; the UI "Sign out" only clears `localStorage` (`src/ui/templates/index.html` `handleLogout`), so a stolen token stays valid for 3 days.
- Password policy is `len >= 6` (`main.py:191`), there is **no rate limiting or lockout** on `/auth/login`, and no rate limiting anywhere in the app (no `slowapi`/`limiter` dependency).
- `SECRET_KEY` ships a default dev value (`config.py:17`, `.env.example:10`) and `CORS_ORIGINS` defaults to `['*']` (`config.py:21`).
- `ENFORCE_WEBHOOK_AUTH` defaults to `false` and `TTN_WEBHOOK_SECRET` is optional (`config.py:22-23`, `main.py:410`) → anyone can POST fake TTN uplinks and move the agronomic model.

### 3.4 🟡 No audit trail
Nothing records *who* changed a field, an advisory, or a sensor binding. `AdvisoryAlertModel`/`IrrigationEventModel` record the action but not the actor or the request id (the request-id middleware `main.py:136-150` never reaches the DB layer).

---

## 4. Verified product / feature gaps (NEW)

### 4.1 Field & probe lifecycle is not manageable
- No `POST/PATCH/DELETE /api/v1/fields` — a new tenant physically cannot onboard a farm through the product; fields exist only because `database.py` seeds two demo rows.
- No probe registration/binding API; no endpoint to list or retire probes; TTN uplinks auto-create orphan probes (`main.py:419-432`).
- Field detail reads only the **first** sensor of a field (`main.py:285`, `main.py:367`) → a 4-probe field shows one probe. There is **no zone ↔ probe ↔ valve mapping** anywhere in the schema.

### 4.2 No advisory / irrigation history surface
No endpoints or UI for: advisory history, irrigation event history, delivery status (sent / failed / snoozed), planned-vs-applied comparison, or export. `README.md` and the dashboard only ever show the *current* state.

### 4.3 No actuation layer
There is no valve, controller, or irrigation-window entity. `AdvisoryAlertModel` records a *recommendation*; nothing executes it, nothing confirms hardware action. The root landing page (`index.html:108,121,129-130`) advertises *"It drives the valves directly"* and renders V1–V4 solenoids, a 30 cm sensor and zone-level deficits — **none of which exist in the code**.

### 4.4 Multi-depth & EC telemetry not implemented
`PLAN.md` §3.1 asks for 15/30/60 cm columns and weighted root-zone moisture. The schema still has one moisture value per sensor, and the EC parsed by `lorawan_decoder.py:41` is silently dropped (no column, not persisted). No deep-percolation detection either.

### 4.5 Configuration contains agronomy that no code reads
Grep across `src/` for the YAML fields returns **zero** usages:
- `soils.yaml`: `infiltration_rate_mm_h`, `bulk_density_g_cm3` → the advisor never checks that a recommended depth can be applied without runoff (clay loam infiltrates at 10 mm/h).
- `crops.yaml`: `stress_strategy` (RDI!), `duration_days`, `desc` → the crop-specific **regulated deficit irrigation strategies are documentation only**.

### 4.6 Water-balance simplifications
- Depletion is derived from the **instantaneous** probe reading vs. field capacity (`water_balance.py:100-102`); rain and ET are not integrated between uplinks, so a stale reading after a 20 mm storm still reads as "depleted".
- `WeatherService` exposes `rain_next_24h_mm` but populates it with **today's** precipitation (`weather.py:70`) — mislabeled.
- Single-point, 3-day-horizon forecast with a 4h TTL cache and a conservative fallback; no multi-source blending, no historical backfill, no per-zone microclimate.

### 4.7 Multi-field chat UX is ambiguous
Telegram `callback_data` is a bare `"DONE"` / `"STATUS"` / `"DELAY 1D"` (`formatter.py:121-145`) with **no field reference**, so a grower managing two blocks cannot tell which block a button confirms. The resolver falls back to heuristics (`handler.py:84-102`).

### 4.8 Dashboard polish gaps (`src/ui/templates/index.html`)
- `#et0Val`, `#rainVal`, `#tempVal` update only inside `triggerAdvisory()` — switching fields leaves the previous field's weather on screen.
- The economics panel ships hardcoded placeholders (`€36,400`, `5,040 m³`, `2,380%`, *"38% Gross Margin Buffer"*) that do not match `bom_economics.py` output.
- The field `<select>` is hardcoded to two seeded fields before the first fetch.
- The chat simulator posts `chat.id = farmer_phone` — real Telegram ids are numeric; the simulator is a fake, which is fine but should be labelled.
- Telegram HTML advisory bodies are injected through `escapeHtml()` → the grower sees literal `<b>` tags in the mockup (`escapeHtml` at `ui/templates/index.html:701`).

### 4.9 Missing channel features
No `/help` command, no `RESUME`/`PAUSE 0D`, no localization beyond a couple of German tokens (`handler.py:171,197`), no per-field language, no quiet-hours, no weekly digest, no escalation (e.g. alert a second contact if the primary does not acknowledge).

---

## 5. Verified data, ops & quality gaps (NEW)

### 5.1 Tests are not isolated and depend on the live internet
- `tests/*` import `src.api.main`, which executes `create_db_and_tables()` at import time (`main.py:108`) → **the suite runs against the developer's real `fieldsense.db`** and mutates it: `test_scheduler.py` creates and deletes a field, `test_api_endpoints.py::test_simulation_run` overwrites sensor moisture and inserts telemetry rows, webhook tests insert irrigation events. No temp-DB fixture, no teardown, no `monkeypatch` of `DATABASE_URL`.
- `test_api_endpoints.py::test_advisory_evaluation` and `test_scheduler.py` perform **real HTTPS calls to Open-Meteo** with no mocking (`respx`/`httpx-mock` absent) → CI is coupled to a third party and to today's weather.
- Recommended: `conftest.py` with a session-scoped temporary SQLite file, `httpx` transport mocking for Open-Meteo/Telegram/Twilio, and `TestClient` with `AUTO_SCHEDULER_ENABLED=false`.

### 5.2 CI is minimal
`.github/workflows/ci.yml`: checkout → install → `pytest -v` on 3.11/3.12. Missing: coverage threshold (`pytest-cov` is installed but never invoked), lint/format/type-check (no ruff/flake8/mypy config), dependency pinning (only floors, no lockfile), Docker image build, and a Postgres matrix.

### 5.3 Synthetic and real telemetry are indistinguishable
`TelemetryReadingModel` (`models.py:92-108`) has **no `source`/`is_simulated`/`quality` column**, yet `run_simulation` writes simulated readings into the same table (`main.py:623-631`). Any analytics, ML training or audit built on this table will silently mix synthetic and physical data. Add a `source` enum (`ttn` / `simulation` / `manual`).

### 5.4 No versioned migrations
Only the SQLite `ALTER TABLE` loop exists (`database.py:42-75`). There is no Alembic, no down-migration, no schema-version table, and the Postgres path advertised in `docs/ARCHITECTURE.md` is unsupported in practice.

### 5.5 Container/production configuration gaps
- `docker-compose.yml` does **not** pass `SECRET_KEY`, `CORS_ORIGINS`, `TELEGRAM_BOT_TOKEN`, `TELEGRAM_WEBHOOK_SECRET`, `TTN_WEBHOOK_SECRET`, `OPEN_METEO_API_KEY`, or `ENFORCE_WEBHOOK_AUTH` → the container runs with the **dev secret key** and **cannot send Telegram at all** (the primary channel).
- `Dockerfile:46` `--workers 2` (see §2.6); healthcheck probes `/healthz` only (liveness), not `/readyz` (DB).
- `GET /` resolves the template through a **relative** path (`main.py:696`) → the dashboard 404s/empties whenever the process cwd ≠ repo root (systemd, some PaaS runners).

### 5.6 Observability & resilience
- No metrics, no tracing, no SLO, no alerting on dispatch failure; scheduler failures are logged and swallowed (`scheduler.py:148-149`).
- No retry/dead-letter queue for failed WhatsApp/Telegram deliveries; `TelegramBotClient`/`TwilioWhatsAppClient` return error dicts that are **not** persisted anywhere.
- Structured logging is plain `logging` with a formatter string (`logger.py`) — no JSON mode, no correlation id in log lines (the request id is logged but never attached to the record).

---

## 6. Verified documentation / marketing ↔ implementation mismatches (NEW)

| Claim | Where | Reality |
| :--- | :--- | :--- |
| "Capacitance at three depths" | `index.html:4,108` | Single moisture value per sensor; no 15/30/60 cm model. |
| "sensor #4 at 30 cm" | `index.html:130` | One depth per probe (`install_depth_cm`), no per-depth readings. |
| "4 zones · 4 probes · 4 solenoid valves" / V1–V4 | `index.html:121` | No zone, valve or solenoid entity in the schema. |
| "drives the valves directly" | `index.html:108` | No actuation code whatsoever — advisory only. |
| "−22.4% water" (hero) | `index.html:113` | Model uses 18% (`bom_economics.py:130`); the page's own table says 18–28% (`:159`); `PLAN.md:24` says 15–22%. **Four numbers.** |
| "Zero-App WhatsApp Interface" (headline) | `README.md:4,16` | The product's primary channel is now **Telegram** (`README.md:78-89`, dashboard, `formatter.py`). |
| ER diagram without `users`/tenants | `docs/ARCHITECTURE.md:119-198` | Predates the multi-tenant auth work (`models.py:8-19`). |
| Architecture described as WhatsApp-only | `docs/ARCHITECTURE.md:96-115` | Telegram bot, inline keyboards, invite-token onboarding and webhook are undocumented there. |
| `docs/ARCHITECTURE.md` topology | whole file | No scheduler/worker model, no multi-worker warning, no rate limiting, no Alembic story. |
| README API table | `README.md:48-60` | Omits `/auth/register`, `/auth/login`, `/auth/me`, `/telegram/webhook`, `/fields/{id}/invite-token`, `/readyz`, `/healthz`. |
| "24 of 24 tests passing" | `PLAN.md:145` | 33 tests pass; and `PLAN.md` Step 3 ("persistent DELAY/PAUSE") is **not** actually working (§2.1). |

---

## 7. Remediation roadmap (suggested order)

> **Status update — P0 completed 2026-11-10.** All seven P0 items are implemented and covered by
> regression tests (`tests/test_p0_regressions.py`, 15 new tests; suite: **48 passed**).
> See §7.1 for the exact change list. P1–P3 remain open.

### P0 — before any grower sees it (≈1–2 days) — ✅ DONE
1. ✅ Fix the DELAY/PAUSE routing (§2.1) + add regression tests.
2. ✅ Add auth/ownership to `/fields`, `/fields/{id}/telemetry`, `/advisory/evaluate/{id}`, `/simulation/run`; strip tenant secrets from unauthenticated payloads (§3.2).
3. ✅ Remove `role` from self-registration (§3.1).
4. ✅ Stop `run_simulation` writing into production telemetry; add a `source` column (§5.3).
5. ✅ Give the two seeded fields distinct `farmer_chat_id`s (§2.2).
6. ✅ Force UTF-8 logging (§2.5).
7. ✅ Change `docker-compose.yml` to pass `SECRET_KEY`, `TELEGRAM_*`, `TTN_*`, `ENFORCE_WEBHOOK_AUTH` (§5.5).

### P1 — make it trustworthy (≈1–2 weeks) — ✅ DONE
8. ✅ `conftest.py` with isolated temp DB + mocked HTTP; add coverage gate to CI (§5.1, §5.2).
9. ✅ Single-worker scheduler or advisory-lock; `asyncio.to_thread` for the blocking dispatch (§2.6).
10. ✅ Alembic migration baseline + a real migration path for new columns (§5.4).
11. ✅ Rate limiting + login lockout + token revocation + password policy (§3.3).
12. ✅ Field/probe CRUD + probe→field binding + zone mapping (§4.1).
13. ✅ Advisory & irrigation history endpoints + dashboard views (§4.2).
14. ✅ Depth convention cleanup (gross vs net) + use the FAO threshold in `STATUS` (§2.3, §2.4).
15. ✅ Delivery-failure persistence, retry and dispatch audit log (§5.6).

### P2 — product depth (≈2–4 weeks) — ✅ DONE
16. ✅ Multi-depth probe support + weighted root-zone moisture + deep-percolation flag + EC column (§4.4).
17. ✅ Use `infiltration_rate_mm_h` and `stress_strategy` from the YAML (§4.5).
18. ✅ Rain/ET integration into the depletion model; correct `rain_next_24h_mm` labelling (§4.6).
19. ✅ Zone-level advisories and per-field callback payloads (§4.6, §4.7).
20. ✅ Actuation integration (valve controller) — or, if deferred, remove the valve claims from the landing page (§4.3).
21. ✅ i18n, weekly digest, quiet hours, escalation contacts (§4.9).

### P3 — honesty pass — ✅ DONE
22. ✅ Rewrite `index.html` hero metrics to match the model (single savings figure), and align `README.md` / `docs/ARCHITECTURE.md` with the Telegram-first, multi-tenant reality (§6).
23. ✅ Update `PLAN.md` (33 tests, Step 3 status) or archive it in favour of this file.

### 7.1 P0 change log (implemented 2026-11-10)

| Item | Files changed | What was done |
| :--- | :--- | :--- |
| #1 | `src/bot/handler.py` | The DELAY/PAUSE branch now runs **before** the WATER branch and matches on `\b(DELAY\|PAUSE\|SNOOZE\|SPÄTER)\b`. The WATER branch requires an explicit keyword (`WATER`/`WASSER`/`GEWÄSSERT`) or an explicit `MM` unit, so a bare number can never be logged as a watering depth. |
| #2 | `src/api/main.py`, `src/ui/templates/index.html` | `GET /api/v1/fields`, `GET /api/v1/fields/{id}/telemetry`, `POST /api/v1/advisory/evaluate/{id}` and `POST /api/v1/simulation/run` now require a bearer token and pass `require_field_access()` (owner or admin; 403 otherwise). Contact data and Telegram invite tokens are only disclosed to the owner/admin. The dashboard was updated to send the token, to show a "sign in required" notice instead of an infinite 401 retry loop, and to ignore stale 401 responses that arrive after a successful sign-in. |
| #3 | `src/api/main.py` | `RegisterRequest.role` was removed; public registration always creates a `grower`. Role changes moved behind the admin-only endpoint `POST /api/v1/admin/users/{user_id}/role` (self-demotion blocked). |
| #4 | `src/core/models.py`, `src/api/database.py`, `src/api/main.py` | `TelemetryReadingModel.source` added (`ttn` / `simulation` / `manual`) with an SQLite migration. `/simulation/run` no longer writes to the database or touches probe state unless `persist=true` is passed explicitly, and persisted rows are tagged `source='simulation'`. |
| #5 | `src/core/models.py`, `src/api/database.py`, `src/api/scheduler.py`, `src/api/main.py` | `farmer_chat_id` now defaults to `None` instead of the shared `'123456789'`; the two demo fields are seeded with distinct ids (`900000001` / `900000002`); a migration repairs the legacy placeholder only (never a grower-linked chat id); the hardcoded `"123456789"` dispatch fallback was removed from both the scheduler and the advisory endpoint (Telegram dispatch is skipped with a warning instead). |
| #6 | `src/core/logger.py`, `Dockerfile` | stdout/stderr are reconfigured to UTF-8 (`errors='replace'`) at logger setup; the image sets `PYTHONUTF8=1` and `PYTHONIOENCODING=utf-8`. |
| #7 | `docker-compose.yml`, `Dockerfile`, `src/api/main.py` | The compose file now passes `SECRET_KEY`, `CORS_ORIGINS`, `TTN_WEBHOOK_SECRET`, `TELEGRAM_BOT_TOKEN`, `TELEGRAM_WEBHOOK_SECRET`, `TWILIO_ADVISORY_CONTENT_SID`, `OPEN_METEO_API_KEY` and `ENFORCE_WEBHOOK_AUTH=true`; `SECRET_KEY`, `TTN_WEBHOOK_SECRET` and `TELEGRAM_WEBHOOK_SECRET` are **required** (`:?` — the stack refuses to start without them). Production now fails closed: TTN/Twilio/Telegram webhooks reject unauthenticated calls (503 when the secret is missing), the app refuses to boot with the default dev `SECRET_KEY`, and the dashboard template is resolved by absolute path with `Cache-Control: no-store`. |

**New test coverage:** `tests/test_p0_regressions.py` (15 tests) + `tests/conftest.py` (isolated throwaway SQLite database, `admin_headers` / `grower_headers` fixtures) + updates to `tests/test_api_endpoints.py` and `tests/test_telegram_bot.py`. Suite result: **48 passed**.

**Side effect:** the local `fieldsense.db` no longer carries the test rows that the pre-P0 suite had written into it (8 leftover "Timezone Test Vineyard" / "Farmer One Private Vineyard" / "Deep Link Test Orchard" rows were removed); the database now contains only the two demo fields.

### 7.2 P1 change log (implemented 2026-11-10)

| Item | Files changed | What was done |
| :--- | :--- | :--- |
| #8 | `tests/conftest.py`, `.github/workflows/ci.yml`, `requirements-dev.txt` | The suite now runs against a throwaway SQLite database and all outbound HTTP is intercepted with `httpx.MockTransport` (Open-Meteo is served from a canned payload, every other host raises `ConnectError`), so the tests are deterministic, fast and offline-safe. CI gained a **lint gate** (`ruff check src tests`), a **coverage gate** (`--cov-fail-under=75`) and a **migration drift check** (`alembic check`). |
| #9 | `src/api/scheduler.py`, `src/core/models.py` | New `SchedulerLockModel` implements a cross-process **dispatch lease**: only the holder of a live lease may send notifications, so several uvicorn workers can no longer double-message a grower. The blocking dispatch moved to `asyncio.to_thread`, so the event loop is no longer blocked by weather HTTP calls. |
| #10 | `alembic.ini`, `migrations/`, `Dockerfile`, `src/api/database.py` | Alembic baseline: `migrations/env.py` reads the URL from the app settings (SQLite **and** PostgreSQL), `render_as_batch` for SQLite, three revisions (initial schema, sensor zone, dispatch log) that are idempotent. The Docker image runs `alembic upgrade head` before uvicorn. |
| #11 | `src/core/rate_limit.py`, `src/core/config.py`, `src/core/security.py`, `src/core/models.py`, `src/api/main.py`, `src/ui/templates/index.html` | Dependency-free rate limiter with per-key failure lockout. Login (per IP **and** per account), registration, API calls (per user) and the three webhooks (per IP) are limited; repeated failed logins lock the account. Password policy raised to 8+ chars with a letter and a digit. Tokens now carry a `token_version`; `POST /api/v1/auth/logout` bumps it and invalidates every issued token (server-side logout), and the dashboard calls it on sign-out. |
| #12 | `src/api/main.py`, `src/core/models.py`, `src/ui/templates/index.html` | Full field/probe lifecycle: `POST/GET/PATCH/DELETE /api/v1/fields`, `GET/POST /api/v1/fields/{id}/sensors`, `PATCH/DELETE /api/v1/sensors/{id}`, `POST /api/v1/sensors/{id}/bind` for orphaned probes. New `sensors.zone` column maps probes to irrigation zones, surfaced as a per-zone status map. Unknown crops/stages/soils are rejected with a helpful message. |
| #13 | `src/api/main.py`, `src/ui/templates/index.html` | New history endpoints `…/advisories`, `…/irrigations`, `…/dispatches` and a combined `…/history`. The dashboard gained an *Advisory & Irrigation History* card with summary tiles and a merged timeline. |
| #14 | `src/core/water_balance.py`, `src/core/advisor.py`, `src/core/models.py`, `src/bot/formatter.py`, `src/bot/handler.py` | One depth convention across the codebase: **net** = water at the root zone, **gross** = what the grower runs. New `calculate_gross_depth_mm` / `calculate_net_depth_mm`; the advisory now reports both and the message says *"Run the drip for X mm"* (gross) + *"Root zone refill: Y mm net"*. `STATUS` uses the real FAO-56 threshold instead of a hardcoded 20 %. |
| #15 | `src/bot/dispatch.py`, `src/core/models.py`, `src/api/main.py`, `src/api/scheduler.py` | New `AdvisoryDispatcher` centralises delivery: it retries transient failures (3 attempts, exponential backoff), persists **every** attempt in the new `dispatch_logs` table and reports skipped channels (e.g. a block without a Telegram chat id). Both the scheduler and the manual dispatch button use it. |

**Test coverage:** 81 tests (up from 48) — `tests/test_auth_hardening.py` (10),
`tests/test_field_crud.py` (10), `tests/test_history_and_dispatch.py` (9) plus the
existing P0 suites. Coverage is **80 %** overall (CI gate at 75 %).

### 7.3 P2 change log (implemented 2026-11-10)

| Item | Files changed | What was done |
| :--- | :--- | :--- |
| #16 | `src/core/models.py`, `src/core/water_balance.py`, `src/ingestion/lorawan_decoder.py`, `src/api/main.py` | Probes and readings gained the optional 15 / 30 / 60 cm profile plus soil EC. `compute_root_zone_moisture()` returns the weighted effective root-zone value (25 / 50 / 25 %, renormalised over the reported depths) and `detect_deep_percolation()` flags a 60 cm rise of more than 2 %. The decoder understands a documented 12-byte TerraFlux multi-depth frame and the TTN `decoded_payload` depth fields; the EC that was decoded but thrown away is now persisted. |
| #17 | `src/core/water_balance.py`, `src/core/advisor.py` | `stress_strategy` from `config/crops.yaml` is now honoured: in the late stages the depletion fraction is raised (RDI) and the advice says so. `infiltration_rate_mm_h` caps the recommendation at what the soil can take in the irrigation window ("split over two cycles"), and a runoff warning appears when the drip applies faster than the soil absorbs. |
| #18 | `src/core/advisor.py`, `src/core/water_balance.py`, `src/ingestion/weather.py` | A stale probe reading is corrected for the crop demand and the rain since the uplink (`adjust_moisture_for_weather`), the rain suppression now also compares the 72 h forecast with the actual deficit, and `rain_next_24h_mm` is documented correctly (Open-Meteo day 0 = today) with a new `rain_next_72h_mm`. |
| #19 | `src/core/advisor.py`, `src/core/models.py`, `src/bot/formatter.py`, `src/bot/handler.py`, `src/api/main.py` | Probes can be assigned to a zone and a valve. The driest zone of a block decides the recommendation (`select_critical_zone`), the message names zone and valve, and the Telegram buttons carry the scope (`ACTION|field_id|zone`). A grower can only confirm a block that belongs to them. |
| #20 | `src/actuation/*`, `src/core/models.py`, `src/api/main.py`, `src/api/scheduler.py` | New actuation package: `build_plan()` turns an advisory into an executable `ValvePlan`, `SimulatedValveController` records the command, `HttpValveController` posts it to a real gateway (`VALVE_CONTROLLER_URL`). Every state change lands in the new `actuation_events` table. Automatic actuation is off by default (`AUTO_ACTUATION_ENABLED`). |
| #21 | `src/bot/i18n.py`, `src/bot/formatter.py`, `src/bot/handler.py`, `src/api/scheduler.py`, `src/core/time_utils.py` | All grower facing wording moved into a message catalogue with English and German; the Telegram channel renders the same text as HTML. Blocks gained `language`, `quiet_hours_start/end`, `digest_enabled/day/hour` and `escalation_phone`. The scheduler honours quiet hours, sends the weekly digest (once per week, guarded by the dispatch log) and escalates unconfirmed critical recommendations to the second contact. |
| — | `src/core/time_utils.py` | **Bug found while implementing #21:** SQLite drops `tzinfo`, so comparing a stored timestamp with `datetime.now(timezone.utc)` raised `TypeError`. The paused-field check in the scheduler silently failed (it skipped the field with an exception instead of honouring the pause). All comparisons now go through `as_utc()` / `hours_between()`. |
| — | `src/core/advisor.py` | **Bug found while implementing #20:** a large deficit produced a 32-hour valve run, which no grower can act on. The recommendation is now capped by what the drip can deliver in one irrigation window (and by what the soil can take), and the advice says how many cycles the refill has to be split into. Verified live: the same block now yields 360 min / 12 mm gross with *"Split over 6 cycles"*. |

**Test coverage:** 132 tests (up from 81) — `tests/test_multidepth.py` (11),
`tests/test_agronomy_rules.py` (13), `tests/test_zones_and_callbacks.py` (7),
`tests/test_actuation.py` (7), `tests/test_i18n_and_notifications.py` (13).
Final count after the last refinement: **134 tests, 82 % coverage, ruff clean, `alembic check` without drift.**

### 7.4 P3 change log (implemented 2026-11-10)

| Item | Files changed | What was done |
| :--- | :--- | :--- |
| #22 | `index.html`, `README.md`, `docs/ARCHITECTURE.md`, `docs/WALKTHROUGH.md`, `src/simulation/bom_economics.py`, `src/ui/templates/index.html` | **One savings figure instead of four:** the landing page now says **18 %** (the value the model actually uses) in both the hero and the comparison table, with the literature band (15–22 %) named. The valve claim was qualified ("a valve controller is driven only when one is connected"), the sample message was rewritten in the real format, the telemetry block now shows the real log lines, and the field plan is labelled *example* instead of *live*. **The economics model itself was corrected**: the vineyard baseline is now 450 m³/ha supplemental irrigation (not the 1,400 m³/ha gross demand) and the extraction/pumping cost is €0.45/m³ (not €0.10), with labour savings counted separately and never folded into the 18 % water figure. The dashboard calculator exposes the hardware tier and the gateway, and shows the labour line; the static placeholders are now computed values instead of invented ones. README and the docs describe the product as Telegram-first with WhatsApp as fallback, with tenant isolation and the actuation boundary described. |
| #23 | `PLAN.md` | The plan is **archived** with a header pointing at this document, and its execution checklist no longer claims that the broken `DELAY`/`PAUSE` step was done (it is marked as such and cross-referenced to §2.1). |

**Final state:** 136 tests passing, ruff clean, 82 % coverage, `alembic check` reports no drift
(4 idempotent revisions), and every public claim in `index.html`, `README.md` and `docs/`
now matches what the code does.


---

## 8. How to add new functionality in this codebase (conventions to follow)

**Layering (keep the existing boundaries):**
| Change | Where |
| :--- | :--- |
| Agronomy rules, thresholds, ET/ETc maths | `src/core/water_balance.py`, `src/core/advisor.py` |
| Persistence & schema | `src/core/models.py` (+ migration in `src/api/database.py`) |
| HTTP endpoints | `src/api/main.py` — **split into `src/api/routers/<domain>.py`** as it grows (the file is already 700+ lines and mixes auth, fields, ingest, webhooks, simulation, economics, UI) |
| Background jobs | `src/api/scheduler.py` (or a dedicated `src/api/jobs.py` if the task count grows) |
| Device/cloud I/O | `src/ingestion/*` (weather, payload decoders) |
| Messaging channels & commands | `src/bot/*` (`formatter` = presentation, `handler` = command state machine, `*_client` = transport) |
| Reference data | `config/*.yaml` (loaded via `load_yaml` in `water_balance.py`) — add a value **and** consume it, or it is dead config |
| UI | `src/ui/templates/index.html` (split into partials/static assets when it grows past ~1k lines) |

**Definition of Done for any new feature**
1. Model + index changes in `models.py`; extend the SQLite migration **and** plan the Alembic revision.
2. Core logic in `src/core/*` as a pure function/class with no HTTP or DB import where possible (the existing `water_balance`/`advisor` style makes this trivial to unit-test).
3. Transport wiring in `src/api/*` or `src/bot/*`; **require `get_current_user` + ownership checks** on any endpoint that touches a specific field (see §3.2).
4. Idempotency + audit: writes that a scheduler can repeat must carry a guard (see `scheduler.py:104-114`) and record actor + request id.
5. Config surface: new env vars → `src/core/config.py` **and** `.env.example` **and** `docker-compose.yml`.
6. Tests: unit test in `tests/` mirroring the existing naming style; any external call must be mocked; DB access must go through the new shared `conftest.py` fixture.
7. Docs: `README.md` API table, `docs/ARCHITECTURE.md` ER/topology, and the marketing claims in `index.html` if the feature changes what the product can honestly promise.
8. Validate: `python -m pytest -v` → `uvicorn src.api.main:app --reload` → exercise `/docs`, the dashboard, and the simulated Telegram/WhatsApp paths.

**Naming/format conventions in use:** dataclasses for value objects (`WeatherForecastSummary`, `AdvisoryDecision`, `SoilWaterStatus`), SQLModel for persistence, `logger = setup_logger()` for logs, mm for depth, %VWC for moisture, UTC-aware datetimes with explicit `ZoneInfo` conversion for farm-local time, and a `simulated`/`mock` flag on every outbound integration.

---

## Appendix A — reproduction script

`verify_gaps.py` (throwaway DB, no side effects on the real one) used for this audit:

```python
import os, pathlib, tempfile, sys
sys.stdout.reconfigure(encoding="utf-8", errors="replace")
tmpdb = pathlib.Path(tempfile.gettempdir()) / "terraflux_verify.db"
os.environ["DATABASE_URL"] = f"sqlite:///{tmpdb}"
os.environ["AUTO_SCHEDULER_ENABLED"] = "false"

from fastapi.testclient import TestClient
from src.api.main import app
from src.api.database import engine, create_db_and_tables
from src.core.models import FieldModel, IrrigationEventModel

create_db_and_tables()
client = TestClient(app)

# (1) DELAY 1D must pause, not irrigate
r = client.post("/api/v1/whatsapp/webhook", data={"From": "+4915123456789", "Body": "DELAY 1D"})
print("reply:", r.json()["reply"])          # currently: "✅ Logged 1.0mm Irrigation ..."
with Session(engine) as s:
    f = s.exec(select(FieldModel).where(FieldModel.name.like("Kaiserstuhl%"))).first()
    print("paused_until:", f.paused_until)   # currently: None

# (2) registration must not accept a role
r = client.post("/api/v1/auth/register",
                json={"email": "a@b.ag", "password": "secret123", "role": "admin"})
print("role granted:", r.json().get("role")) # currently: admin

# (3) field list must require a token
r = client.get("/api/v1/fields")
print("unauth fields:", len(r.json()))       # currently: 2, with farmer_phone + invite_token
```

## Appendix B — audit trail of this review
- Read: 100% of `src/` (18 modules), `tests/` (10 files), `config/*.yaml`, `docs/ARCHITECTURE.md`, `README.md`, `PLAN.md`, `HARDWARE.md`, root `index.html`, `Dockerfile`, `docker-compose.yml`, `.github/workflows/ci.yml`, `.env.example`.
- Executed: `pytest -q` → **33 passed**; `verify_gaps.py` against an isolated DB → all §2/§3 findings reproduced.
- Not audited (out of scope this pass): `HARDWARE.md`, `docs/FIRMWARE_SPEC.md`, `docs/DEPLOYMENT_GUIDE.md`, `docs/OPERATIONS_RUNBOOK.md`, `docs/TELEGRAM_BOT_SETUP.md`, `docs/WALKTHROUGH.md`, `docs/WHITEPAPER.md`, `docs/HARDWARE_GUIDE.md`, `assets/*`.
