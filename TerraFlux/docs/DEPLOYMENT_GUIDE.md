# FieldSense Irrigation — Production Cloud Deployment Guide

## 1. Prerequisites

- Python 3.12+ (or Docker & Docker Compose).
- A domain name with SSL certificate (e.g. `https://api.fieldsense.ag`).
- A free [The Things Network (TTN)](https://www.thethingsnetwork.org/) account.
- A [Telegram Bot](https://core.telegram.org/bots) token from @BotFather (primary channel).
- A [Twilio](https://www.twilio.com/) account with WhatsApp Sandbox or WhatsApp Business profile enabled (fallback channel).

---

## 2. Environment Configuration (`.env`)

Create a `.env` file in the project root:

```bash
# Core Environment
ENVIRONMENT=production
LOG_LEVEL=INFO
DATABASE_URL=sqlite:////app/data/fieldsense.db # or postgresql://user:pass@postgres:5432/fieldsense
SECRET_KEY=generate-a-strong-random-64-character-hex-secret

# Security & CORS
CORS_ORIGINS=["https://fieldsense.ag"]
TTN_WEBHOOK_SECRET=your-ttn-preshared-secret-token-here
TELEGRAM_WEBHOOK_SECRET=your-telegram-secret-token-here
ENFORCE_WEBHOOK_AUTH=true

# Telegram Bot (primary advisory channel)
TELEGRAM_BOT_TOKEN=123456789:ABCdefGHIjklMNOpqrSTUvwxYZ

# Twilio WhatsApp Credentials (fallback channel)
TWILIO_ACCOUNT_SID=ACXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXXX
TWILIO_AUTH_TOKEN=your_twilio_auth_token_here
TWILIO_WHATSAPP_FROM=whatsapp:+14155238886
TWILIO_ADVISORY_CONTENT_SID=   # optional pre-approved template for the 24h window

# Agronomy & Weather
WEATHER_CACHE_TTL_SECONDS=14400
AUTO_SCHEDULER_ENABLED=true

# Authentication hardening (optional, defaults shown)
PASSWORD_MIN_LENGTH=8
LOGIN_MAX_FAILURES=5
LOGIN_LOCK_SECONDS=900
LOGIN_RATE_LIMIT_PER_MINUTE=20
REGISTER_RATE_LIMIT_PER_HOUR=10
API_RATE_LIMIT_PER_MINUTE=120
WEBHOOK_RATE_LIMIT_PER_MINUTE=600
```

> **Fail-closed in production.** With `ENVIRONMENT=production` the service refuses to
> start with the default `SECRET_KEY`, and the TTN / Twilio / Telegram webhooks reject
> unauthenticated calls (HTTP 503 while their secret is missing).

---

## 3. Database Migrations

The schema is managed with **Alembic**. Apply the migrations before starting the API:

```bash
pip install -r requirements-dev.txt        # includes alembic
alembic upgrade head                       # apply all pending migrations
alembic check                              # verify the models match the migrations
```

The Docker image runs `alembic upgrade head` automatically before starting uvicorn.
After changing a model in `src/core/models.py` create a revision:

```bash
alembic revision --autogenerate -m "describe the change"
alembic upgrade head
```

---

## 4. Deployment Options

### Option A: Docker Compose (Recommended for VPS / Cloud Server)

```bash
# 1. Clone repository
git clone https://github.com/your-org/fieldsense-irrigation.git
cd fieldsense-irrigation

# 2. Configure .env (SECRET_KEY, TTN_WEBHOOK_SECRET and TELEGRAM_WEBHOOK_SECRET are required)
cp .env.example .env
nano .env

# 3. Build and launch container
docker compose up -d --build

# 4. Verify health and readiness probes
curl http://localhost:8000/healthz
curl http://localhost:8000/readyz
```

> The compose file starts **one** API container. `AUTO_SCHEDULER_ENABLED=true` runs the
> daily advisory dispatcher inside it. If you scale to several replicas, keep the
> scheduler on exactly one of them (or rely on the built-in dispatch lease, which lets
> only one worker send per tick).

### Option B: Cloud Hosting (Fly.io / Railway / Render)

To deploy to Fly.io:
```bash
fly launch --image python:3.12-slim
fly secrets set TWILIO_ACCOUNT_SID=ACxxx TWILIO_AUTH_TOKEN=xxx TTN_WEBHOOK_SECRET=xxx \
  TELEGRAM_BOT_TOKEN=xxx TELEGRAM_WEBHOOK_SECRET=xxx SECRET_KEY=xxx
fly deploy
```

Run `alembic upgrade head` as a release command before starting the server.

---

## 5. Configuring The Things Network (TTN) Webhook

1. Log into your **The Things Stack Console**.
2. Navigate to **Applications** $\rightarrow$ **Integrations** $\rightarrow$ **Webhooks** $\rightarrow$ **Custom Webhook**.
3. Configure the integration:
   - **Webhook ID**: `fieldsense-telemetry`
   - **Webhook format**: `JSON`
   - **Base URL**: `https://<your-domain>/api/v1/telemetry/ttn`
   - **Additional headers**:
     - Key: `X-TTN-Webhook-Secret`
     - Value: *(Must match `TTN_WEBHOOK_SECRET` in `.env`)*
   - **Enabled event types**: Check `Uplink message`.
4. Save integration. When your LoRa probe transmits, TTN will forward the telemetry payload securely to FieldSense.

> A probe that uplinks before it is registered is auto-registered **without a field**.
> Bind it afterwards with `POST /api/v1/sensors/{id}/bind` (or `PATCH /api/v1/sensors/{id}`
> with a `field_id`), and assign an irrigation `zone` so the block map stays correct.

---

## 6. Configuring the Telegram Bot (primary channel)

1. Create the bot with [@BotFather](https://core.telegram.org/bots) and copy the token
   into `TELEGRAM_BOT_TOKEN`.
2. Pick a random secret and put it into `TELEGRAM_WEBHOOK_SECRET`:

   ```bash
   python -c "import secrets; print(secrets.token_hex(16))"
   ```

3. Register the webhook (the secret must match `TELEGRAM_WEBHOOK_SECRET`):

   ```bash
   curl -X POST "https://api.telegram.org/bot$TELEGRAM_BOT_TOKEN/setWebhook" \
        -d url="https://<your-domain>/api/v1/telegram/webhook" \
        -d secret_token="$TELEGRAM_WEBHOOK_SECRET"
   ```

4. In the dashboard, copy the **Telegram /start invite link** of a block and send it to
   the grower. The first `/start <invite_token>` binds the grower's chat id to that
   block; every advisory is then delivered with inline buttons
   (`Confirm`, `Delay 24h`, `Live Status`).

---

## 7. Configuring Twilio WhatsApp Webhook

1. Log into your **Twilio Console** $\rightarrow$ **Messaging** $\rightarrow$ **Senders** $\rightarrow$ **WhatsApp Senders** (or WhatsApp Sandbox).
2. Under **Endpoint Configuration**:
   - **WHEN A MESSAGE COMES IN**: Set URL to `https://<your-domain>/api/v1/whatsapp/webhook`
   - **HTTP METHOD**: `POST`
3. Save settings. Incoming messages from growers (`DONE`, `STATUS`, `WATER 15mm`) will now be processed automatically by the FieldSense bot.

---

## 8. Verifying a deployment

```bash
# 1. Liveness / readiness
curl -fsS https://<your-domain>/healthz
curl -fsS https://<your-domain>/readyz

# 2. Sign in and read the fields
TOKEN=$(curl -fsS -X POST https://<your-domain>/api/v1/auth/login \
        -H 'Content-Type: application/json' \
        -d '{"email":"admin@terraflux.ag","password":"<password>"}' | python -c 'import json,sys; print(json.load(sys.stdin)["access_token"])')
curl -fsS https://<your-domain>/api/v1/fields -H "Authorization: Bearer $TOKEN"

# 3. Dispatch an advisory and check the delivery audit trail
curl -fsS -X POST https://<your-domain>/api/v1/advisory/evaluate/1 -H "Authorization: Bearer $TOKEN"
curl -fsS https://<your-domain>/api/v1/fields/1/dispatches -H "Authorization: Bearer $TOKEN"
```

Every delivery attempt is persisted in `dispatch_logs`; `GET /api/v1/fields/{id}/dispatches`
shows the channel, the status (`sent` / `simulated` / `skipped` / `error`) and the number
of attempts, so a grower who never received the message is visible to the operator.

