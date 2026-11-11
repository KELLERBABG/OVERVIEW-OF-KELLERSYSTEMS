# FieldSense Irrigation — SRE Operations Runbook & Maintenance Manual

## 1. Routine System Monitoring & Health Checks

### 1.1 Automated Probes:
- **Liveness Probe**: `GET /healthz` — returns HTTP 200 with service metadata.
- **Readiness Probe**: `GET /readyz` — verifies database read/write query execution (`SELECT 1`).

### 1.2 Inspecting Live Logs:
```bash
# Docker Compose logs
docker compose logs -f --tail=100 fieldsense-api

# Filter for WhatsApp dispatches
docker compose logs fieldsense-api | grep "SCHEDULED DISPATCH"

# Filter for security warnings
docker compose logs fieldsense-api | grep "SECURITY"
```

---

## 2. Database Backup & Disaster Recovery

### 2.1 SQLite Production Backup (Zero-Lock Snapshot):
SQLite in WAL mode allows live backups while active write transactions are occurring:
```bash
# Execute live backup via sqlite3 CLI
sqlite3 /app/data/fieldsense.db ".backup '/app/data/backups/fieldsense_backup_$(date +%Y%m%d_%H%M%S).db'"
```

### 2.2 PostgreSQL Backup:
```bash
pg_dump -U fieldsense -h localhost -d fieldsense | gzip > backup_$(date +%Y%m%d).sql.gz
```

---

## 3. Incident Response & Troubleshooting

### Scenario A: Sensor Battery Warning (< 3.3V)
- **Symptom**: Probe telemetry reports battery voltage dropping below 3.3V.
- **Root Cause**: Solar panel dirty, shaded by vine canopy, or damaged LiFePO4 cell.
- **Action**:
  1. Inspect physical node on trellis post.
  2. Clear overgrown leaves from the 0.6W solar panel surface.
  3. Verify South-facing 45° tilt.

### Scenario B: Weather API (Open-Meteo) Outage
- **Symptom**: Logs display `[WEATHER NETWORK ERROR] Failed to reach Open-Meteo`.
- **System Behavior**: FieldSense automatically switches to local 4-hour TTL cache. If cache expires, it activates a safe conservative baseline (3.5mm ET0) and logs a warning.
- **Action**: System recovers automatically when Open-Meteo connectivity is restored.

### Scenario C: Twilio Webhook Failures
- **Symptom**: Farmer replies in WhatsApp receive no automated confirmation.
- **Action**:
  1. Check Twilio Console Debugger for HTTP 403 or timeout responses.
  2. Verify `TWILIO_AUTH_TOKEN` in `.env` matches the Twilio Console secret.
  3. Ensure reverse proxy passes the `X-Twilio-Signature` header to FastAPI without stripping headers.
