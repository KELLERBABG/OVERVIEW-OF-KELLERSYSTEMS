# TerraFlux — Telegram Bot Setup & Production Onboarding Guide (`docs/TELEGRAM_BOT_SETUP.md`)

This guide walks you through setting up and deploying the **TerraFlux Telegram Bot** (`@terraflux_bot`) for zero-fee, interactive agronomic advisories.

---

## 1. Create Your Telegram Bot (via `@BotFather`)

1. Open Telegram and search for **`@BotFather`** (the official bot management tool with the blue checkmark).
2. Send `/newbot`.
3. Enter a display name:
   ```text
   TerraFlux Agronomist
   ```
4. Enter a unique bot username ending in `bot`:
   ```text
   terraflux_agro_bot
   ```
5. `@BotFather` will reply with your **HTTP API Token**:
   ```text
   Use this token to access the HTTP API:
   7123456789:AAHxyzABC-123456789_EXAMPLE_TOKEN
   ```
   *(Keep this token secret — it gives full control of your bot!)*

---

## 2. Set Up Bot Profile & Commands in Telegram

To give farmers a clean onboarding experience with autocompleting commands:

1. In `@BotFather`, send `/setdescription` and paste:
   ```text
   Autonomous viticulture and precision micro-irrigation controller. Daily soil moisture balance, ET₀ forecasts, and 1-click drip cycle confirmations.
   ```
2. Send `/setcommands` and paste:
   ```text
   status - View live soil moisture, temp and probe battery
   done - Confirm completed irrigation cycle
   water - Record custom depth (e.g. /water 15mm)
   delay - Postpone advisory for 24h
   pause - Silence alerts for spraying or harvest
   help - Show interactive command reference
   ```

---

## 3. Configure Your Environment (`.env`)

Add the token and generate a secure random secret token for webhook verification:

```bash
# In your .env file:
TELEGRAM_BOT_TOKEN="7123456789:AAHxyzABC-123456789_EXAMPLE_TOKEN"
TELEGRAM_WEBHOOK_SECRET="generate-a-random-32-char-hex-secret"
ENFORCE_WEBHOOK_AUTH=true
```

---

## 4. Register the Webhook with Telegram

Once your server is accessible via HTTPS (e.g. via Cloudflare Tunnel, Caddy, or your domain):

### Using cURL:
```bash
curl -X POST "https://api.telegram.org/bot<YOUR_TELEGRAM_BOT_TOKEN>/setWebhook" \
     -H "Content-Type: application/json" \
     -d '{
       "url": "https://terra.kellersystems.dev/api/v1/telegram/webhook",
       "secret_token": "<YOUR_TELEGRAM_WEBHOOK_SECRET>",
       "allowed_updates": ["message", "callback_query"]
     }'
```

### Response from Telegram:
```json
{
  "ok": true,
  "result": true,
  "description": "Webhook was set"
}
```

---

## 5. Farmer Onboarding & Chat ID Resolution (`/start <token>`)

Farmers onboard with zero friction using Telegram deep links:

1. **Private Invite Deep Links:**
   Each field generated in TerraFlux has a unique invite token (e.g. `kaiser_alpha`). The farm manager shares the deep link:
   ```text
   https://t.me/terraflux_agro_bot?start=kaiser_alpha
   ```
   *(This link is prominently displayed with a 1-click copy button on the web dashboard)*.

2. **Self-Serve Binding:**
   When the grower taps the link and presses **Start**, Telegram automatically sends `/start kaiser_alpha`.
   The backend:
   - Validates the token against `FieldModel.invite_token`.
   - Binds the grower's Telegram `chat_id` directly to that field in the database.
   - Replies with a welcoming confirmation card showing crop type, location, and quick commands (`STATUS`, `HELP`).

3. **Daily Dispatches:**
   From that point on, daily advisories at the farm's scheduled alert hour arrive directly in their Telegram chat with interactive inline buttons (`[✅ Confirm Done]`, `[⏳ Delay 24h]`, `[📊 Live Status]`).

---

## 6. Local Development & Testing

When testing locally without public HTTPS:
- TerraFlux automatically runs `TelegramBotClient` in **simulated mock mode** if `TELEGRAM_BOT_TOKEN` is unset.
- You can test full interactive conversations inside the web dashboard at `http://localhost:8000` using the virtual smartphone simulator.
- To receive real Telegram webhooks on `localhost`, use Cloudflare Tunnel or ngrok:
  ```bash
  cloudflared tunnel --url http://localhost:8000
  ```

---

## 7. Weather API Strategy & Zero-Cost Open-Meteo Setup

### Why Open-Meteo?
Reference evapotranspiration ($ET_0$) is the critical physical input for the FAO-56 daily soil water balance. Calculating $ET_0$ locally requires complex solar radiation integrals, psychrometric vapor pressure curves, and wind speed vectors. Open-Meteo computes FAO-56 Penman-Monteith $ET_0$ directly on their meteorological cluster.

### Why You Don't Need to Pay $30/Month Pre-Revenue:
- **Free Tier Allowance:** Open-Meteo provides up to **10,000 API requests per day** for evaluation and pilot deployments.
- **Internal 4-Hour Cache:** TerraFlux includes an automated in-memory cache in `src/ingestion/weather.py` with a 4-hour TTL (`WEATHER_CACHE_TTL_SECONDS=14400`).
- **Real-World Consumption:** A farm only queries the weather service at most 6 times per day. Even running **50 active farm blocks** consumes only ~300 calls/day (less than 3% of the free quota limit).

### The Scalable Solution Implemented:
The engine is architected to scale seamlessly from free pilot to commercial enterprise without code changes:

1. **Default Mode (Free / Pre-Revenue):**
   When `OPEN_METEO_API_KEY` is omitted or unset, TerraFlux requests weather data from:
   ```text
   https://api.open-meteo.com/v1/forecast
   ```
2. **Commercial Enterprise Mode (Post-Revenue):**
   Once you have paying customers and require a commercial license and dedicated SLA, simply set your key in `.env`:
   ```bash
   OPEN_METEO_API_KEY="your-commercial-api-key-here"
   ```
   TerraFlux automatically detects this key and transparently switches to Open-Meteo's dedicated commercial endpoint (`customer-api.open-meteo.com`) with the required `apikey` parameter. Zero refactoring needed.

