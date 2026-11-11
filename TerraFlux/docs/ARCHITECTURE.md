# TerraFlux (FieldSense Engine) — System Architecture & Technical Specification

## 1. System Overview

TerraFlux is an autonomous IoT agronomy platform designed to calculate real-time soil
water depletion and deliver daily actionable recommendations via Telegram (primary
channel, zero Meta fees) and WhatsApp (fallback), without requiring farmers to log
into a dashboard. Every block, probe, advisory, delivery attempt and valve command is
recorded, so the operator can see what was advised, what was delivered and what was
actually applied.

---

## 2. Component Topology

```
+--------------------------------------------------------------------------------------------------+
|                                    Physical Layer / Edge IoT                                     |
|  [Soil Moisture & Temp Probe (RS485/ADC)] ---> [LoRaWAN Node (EU868)] ---> [LoRaWAN Gateway]     |
+--------------------------------------------------------------------------------------------------+
                                                   |
                                            (HTTP Webhook)
                                                   |
+--------------------------------------------------v-----------------------------------------------+
| TerraFlux Cloud Application (FastAPI + SQLModel)                                                |
|                                                                                                  |
|  +--------------------------------------------------------------------------------------------+  |
|  |  Security & Ingestion Layer                                                                |  |
|  |  - Request-ID & Structured Logging Middleware                                              |  |
|  |  - TTN Secret Token Ingestion: POST /api/v1/telemetry/ttn                                 |  |
|  |  - Twilio HMAC-SHA1 Webhook Ingestion: POST /api/v1/whatsapp/webhook                      |  |
|  |  - Weather Ingestion with 4h TTL Cache (Open-Meteo Agro Client)                            |  |
|  +--------------------------------------------------------------------------------------------+  |
|                                                |                                                 |
|  +---------------------------------------------v----------------------------------------------+  |
|  |  Agronomic Decision & Physics Engine                                                       |  |
|  |  - FAO-56 Penman-Monteith Reference ET0 & Crop Etc (Kc Stages)                            |  |
|  |  - Soil Water Reservoir Model (TAW, RAW, Depletion Dr, Critical Threshold)                 |  |
|  |  - 48h Rain Forecast Suppression & Irrigation Run-Time Calculator                          |  |
|  +--------------------------------------------------------------------------------------------+  |
|                                                |                                                 |
|  +---------------------------------------------v----------------------------------------------+  |
|  |  Automation & User Channel                                                                 |  |
|  |  - Asynchronous Background Scheduler (Daily 19:00 UTC hourly dispatcher)                   |  |
|  |  - WhatsApp Message Formatter & Two-Way Interactive Command State Machine                   |  |
|  |  - Twilio WhatsApp Client (Dispatches alerts to farmer mobile)                            |  |
|  +--------------------------------------------------------------------------------------------+  |
|                                                |                                                 |
|  +---------------------------------------------v----------------------------------------------+  |
|  |  Data Persistence & Storage Layer                                                          |  |
|  |  - SQLite (WAL mode, busy timeout 5s) / PostgreSQL Connection Pool                         |  |
|  |  - Tables: fields, sensors, telemetry_readings, irrigation_events, advisory_alerts          |  |
|  +--------------------------------------------------------------------------------------------+  |
+--------------------------------------------------------------------------------------------------+
```

---

## 3. Sequence Workflows

### 3.1 LoRaWAN Telemetry Ingestion Flow
```mermaid
sequenceDiagram
    autonumber
    participant Sensor as LoRa Soil Probe
    participant TTN as The Things Network (TTN)
    participant API as FastAPI Ingestion (/telemetry/ttn)
    participant DB as Database (SQLite/Postgres)

    Sensor->>TTN: LoRa Uplink (Moisture, Temp, Battery)
    TTN->>API: POST /api/v1/telemetry/ttn (X-TTN-Webhook-Secret)
    API->>API: Verify Token & Decode Payload (Dragino / LPP)
    API->>DB: Upsert Sensor (last_moisture, last_seen, battery)
    API->>DB: Insert TelemetryReading record
    API-->>TTN: HTTP 200 OK {"status": "processed"}
```

### 3.2 Daily Advisory Background Evaluation & WhatsApp Dispatch (19:00 UTC)
```mermaid
sequenceDiagram
    autonumber
    participant Scheduler as Background Scheduler (Hourly)
    participant DB as Database
    participant Weather as Open-Meteo Weather API
    participant Advisor as FAO-56 Decision Engine
    participant Twilio as Twilio WhatsApp API
    participant Farmer as Farmer WhatsApp

    Scheduler->>DB: Query active fields where daily_alert_hour == current_hour
    loop For each active field
        Scheduler->>Weather: Fetch forecast (ET0, Rain 48h, Temp)
        Weather-->>Scheduler: Return weather summary (Cache Hit/Miss)
        Scheduler->>Advisor: Evaluate soil moisture vs RAW threshold
        Advisor-->>Scheduler: Return AdvisoryDecision (Water mm, Start Time, Drip Duration)
        Scheduler->>DB: Store AdvisoryAlertModel (Status: PENDING)
        Scheduler->>Twilio: POST message to farmer_phone
        Twilio->>Farmer: WhatsApp Alert: "💧 Water 12mm tomorrow at 06:00"
    end
```

### 3.3 Farmer Two-Way Reply Flow
```mermaid
sequenceDiagram
    autonumber
    participant Farmer as Farmer WhatsApp
    participant Twilio as Twilio Webhook
    participant API as FastAPI Webhook (/whatsapp/webhook)
    participant Bot as WhatsAppCommandHandler
    participant DB as Database

    Farmer->>Twilio: Replies "DONE" or "WATER 15mm"
    Twilio->>API: POST /api/v1/whatsapp/webhook (X-Twilio-Signature)
    API->>API: Verify HMAC-SHA1 signature
    API->>Bot: Parse command & lookup field by from_number
    Bot->>DB: Insert IrrigationEventModel (depth, duration, timestamp)
    Bot->>DB: Update Sensor last_moisture_pct towards Field Capacity
    Bot-->>API: Formatted Confirmation Message
    API-->>Twilio: TwiML / JSON Response
    Twilio->>Farmer: "✅ Logged 12.0mm Irrigation for Kaiserstuhl Vineyard. Reservoir updated!"
```

---

## 4. Entity-Relationship Data Model

```mermaid
erDiagram
    USER ||--o{ FIELD : owns
    FIELD ||--o{ SENSOR : contains
    FIELD ||--o{ IRRIGATION_EVENT : logs
    FIELD ||--o{ ADVISORY_ALERT : generates
    FIELD ||--o{ DISPATCH_LOG : "delivery audit"
    FIELD ||--o{ ACTUATION_EVENT : "valve audit"
    ADVISORY_ALERT ||--o{ DISPATCH_LOG : "attempted delivery"
    ADVISORY_ALERT ||--o{ ACTUATION_EVENT : "planned action"
    SENSOR ||--o{ TELEMETRY_READING : records

    USER {
        int id PK
        string email UK
        string hashed_password
        string full_name
        string role "admin | grower"
        boolean is_active
        int token_version "bumped on logout to revoke tokens"
        datetime created_at
    }

    FIELD {
        int id PK
        int user_id FK
        string invite_token
        string name
        string location_name
        float latitude
        float longitude
        float area_ha
        string crop_key
        string growth_stage
        string soil_key
        float drip_emitter_spacing_m
        float drip_emitter_lph
        float drip_line_spacing_m
        float irrigation_efficiency
        string farmer_name
        string farmer_phone
        string farmer_chat_id
        string timezone
        int daily_alert_hour
        string language "en | de"
        string quiet_hours_start
        string quiet_hours_end
        boolean digest_enabled
        int digest_day
        int digest_hour
        string escalation_phone
        boolean active
        datetime paused_until
        datetime created_at
    }

    SENSOR {
        int id PK
        string deveui UK
        string name
        int field_id FK
        string model_type
        int install_depth_cm
        string zone "irrigation zone (A1, B2, ...)"
        string valve_id "physical valve (V1, V2, ...)"
        float moisture_15cm_pct
        float moisture_30cm_pct
        float moisture_60cm_pct
        float soil_ec_us_cm
        boolean deep_percolation
        float battery_volts
        float last_moisture_pct
        float last_temp_c
        datetime last_seen
        boolean is_simulated
    }

    TELEMETRY_READING {
        int id PK
        int sensor_id FK
        datetime timestamp
        float soil_moisture_pct
        float soil_temp_c
        float battery_volts
        int rssi
        float snr
        string source "ttn | simulation | manual"
        float moisture_15cm_pct
        float moisture_30cm_pct
        float moisture_60cm_pct
        float soil_ec_us_cm
        boolean deep_percolation
    }

    IRRIGATION_EVENT {
        int id PK
        int field_id FK
        datetime timestamp
        float depth_applied_mm "NET depth at the root zone"
        int duration_minutes
        string zone
        string source "whatsapp_reply | custom | manual"
        string notes "incl. gross depth and efficiency"
    }

    ADVISORY_ALERT {
        int id PK
        int field_id FK
        datetime timestamp
        float soil_moisture_pct
        float threshold_pct
        float forecast_et0_mm
        float forecast_rain_mm
        float recommended_depth_mm "NET depth"
        float recommended_gross_mm "GROSS depth to run"
        int recommended_duration_min
        string recommended_start_time
        string zone
        string urgency "NONE | LOW | OPTIMAL | HIGH | CRITICAL"
        string status "PENDING | ACCEPTED | SNOOZED | REJECTED | AUTO_LOGGED | ESCALATED"
        string message_text
    }

    ACTUATION_EVENT {
        int id PK
        int field_id FK
        int alert_id FK
        string zone
        string valve_id
        string status "planned | commanded | simulated | confirmed | failed"
        string start_time
        int duration_minutes
        float gross_depth_mm
        float net_depth_mm
        string detail
        datetime created_at
    }

    DISPATCH_LOG {
        int id PK
        int field_id FK
        int alert_id FK
        string channel "telegram | whatsapp"
        string target
        string status "sent | simulated | skipped | error | exception"
        string detail
        int attempts
        datetime created_at
    }

    SCHEDULER_LOCK {
        int id PK
        string name UK
        string holder
        datetime acquired_at
        datetime expires_at
    }
```

**Depth convention:** `AdvisoryAlertModel.recommended_depth_mm` and
`IrrigationEventModel.depth_applied_mm` hold the **net** depth (water that reached the
root zone). `recommended_gross_mm` holds the **gross** depth the grower has to run
(``net ÷ irrigation efficiency``). Run time is ``net depth ÷ net application rate``.

**Provenance:** `TelemetryReadingModel.source` separates physical uplinks (`ttn`) from
fast-forward simulation output (`simulation`), so analytics never mix the two.

---

## 5. Security Architecture

1. **Authentication & Ingestion Authorization:**
   - Bearer tokens (HMAC-SHA256 signed, 72 h expiry) with a per-user `token_version`;
     bumping the version revokes every previously issued token (`POST /api/v1/auth/logout`).
   - Passwords are hashed with PBKDF2-HMAC-SHA256 (120 000 iterations, per-user salt).
   - Pre-shared secret validation (`X-TTN-Webhook-Secret`) on IoT endpoints.
   - Cryptographic HMAC-SHA1 validation on Twilio webhook endpoints.
   - Secret-token validation (`X-Telegram-Bot-Api-Secret-Token`) on the Telegram webhook.
   - Immediate rejection of unregistered phone numbers with zero cross-tenant contamination.
2. **Tenant Isolation:**
   - Every field-scoped endpoint requires a token and checks ownership (`require_field_access`).
   - Growers only ever see their own blocks; administrators see all.
   - Contact data and Telegram invite tokens are only disclosed to the owner (or an admin).
   - Public registration can never create an administrator; role changes require an admin token.
3. **Rate Limiting & Lockout:**
   - Per-IP and per-account limits on login/registration, per-user limits on the API and
     per-IP limits on the webhooks (`src/core/rate_limit.py`).
   - Repeated failed logins lock the account for `LOGIN_LOCK_SECONDS`.
4. **Data Sanitization & Injection Prevention:**
   - Parameterized SQL execution through SQLModel/SQLAlchemy ORM.
   - HTML entity escaping across web templates.
5. **Security Headers:**
   - `X-Content-Type-Options: nosniff`
   - `X-Frame-Options: SAMEORIGIN`
   - `Referrer-Policy: strict-origin-when-cross-origin`

---

## 6. Schema Migrations

The schema is managed with **Alembic** (`migrations/`, `alembic.ini`):

```bash
alembic upgrade head                       # apply migrations
alembic revision --autogenerate -m "..."   # new revision after a model change
alembic check                              # detect model/migration drift (CI gate)
```

The Docker image applies `alembic upgrade head` before starting the API. The service
additionally performs backwards-compatible column backfills on startup, so an older
SQLite file keeps working without a manual migration.

