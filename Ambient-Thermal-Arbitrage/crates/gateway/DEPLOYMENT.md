# ATA Gateway — Production Deployment Guide

This guide walks through bringing the **Ambient Thermal Arbitrage** production
pilot (v0.2.0) to life — from a local simulation to a supervised, always-on
service controlling a real refrigeration plant.

---

## 1. What the Gateway Does in Production

In production mode the gateway is a **closed-loop controller**:

- Every loop iteration (`loop_interval_sec`, default 1 s) the gateway decides
  **on/off** (or a VFD speed in the fleet engine) for the compressors.
- The decision is written to the PLC via Modbus TCP (coil) and mirrored to
  `/status`, `/metrics`, and the CSV log.
- Safety always wins: `Emergency` → `ThermalGuard` → `GridBalancing` →
  `MpcPreCool` → `Arbitrage` → `Idle`.

---

## 2. Prerequisites

| Item | Requirement |
|---|---|
| Host | A device that runs 24/7 (Raspberry Pi 3/4/5, mini-PC, industrial PC, VM) |
| Rust | `rustup` with the `stable` toolchain (build only; the binary is standalone) |
| Network | Outbound HTTPS (price/weather APIs), LAN access to the PLC (port 502) |
| PLC | Modbus TCP server exposing: internal temp (int16, °C×100), ambient temp (int16), compressor coil, alarm coil; optional VFD speed register (uint16, 0..1000 = 0.0..100.0 %) |
| Secrets | A random `ATA_SECRET`; optionally a free ENTSO-E API key |

---

## 3. Build

### 3.1 Local (development / test)

```bash
cd thermal-arbitrage-mvp
cargo build --release        # optimized, LTO (takes a few minutes on RPi)
cargo test --release         # 69 unit tests
```

### 3.2 Cross-compile for Raspberry Pi (aarch64) from a PC

```bash
rustup target add aarch64-unknown-linux-gnu
sudo apt install gcc-aarch64-linux-gnu   # cross linker (Linux host)

cargo build --release --target aarch64-unknown-linux-gnu
```

Copy `target/<triple>/release/thermal-arbitrage-mvp` (Linux binary) together
with `config.toml` onto the device. The `[profile.release]` block is already
tuned for Cortex-A53 (RPi 3): `opt-level = 3`, `lto = true`,
`codegen-units = 1`.

> **Windows note:** the binary built here is
> `target\release\thermal-arbitrage-mvp.exe`. It runs fine as a console app or
> scheduled/service task, but the supported production target is Linux
> (systemd) or Docker.

---

## 4. Production Configuration

Minimal `config.toml` for a real site (secrets via environment, not files):

```toml
[system]
unit_id = "store-berlin-001"
loop_interval_sec = 1.0

[building]
name = "Medium Cold Storage NRW"
volume_m3 = 1000.0
surface_area_m2 = 600.0
u_value_w_per_m2k = 0.25
product_mass_kg = 200000.0
product_specific_heat_j_per_kgk = 2000.0
cop = 2.5
installed_cooling_power_w = 50000.0
air_ach_per_hour = 0.5

[sensor]
simulate = false                    # IMPORTANT: use real PLC readings
initial_temp_c = -20.0

[modbus]
enabled = true
host = "192.168.0.10"               # PLC address
port = 502
poll_interval_sec = 2.0
[modbus.registers]
internal_temp_addr = 0              # int16 °C×100
compressor_cmd_addr = 1             # coil
ambient_temp_addr = 2
alarm_addr = 3
# grid_frequency_addr = 4           # optional: signed int16 Hz×100 (e.g. 5000 = 50.00 Hz)
# compressor_speed_addr = 5         # optional VFD speed (uint16 0..1000 = 0..100 %); when set,
                                    # the speed register replaces the compressor coil as the
                                    # authoritative command (combine with [control] vfd_enabled = true)

[security]
auth_enabled = true

---

## 5. Run as a Service

### 5.1 Linux / Raspberry Pi (systemd) — recommended

`/etc/systemd/system/ata-gateway.service`:

```ini
[Unit]
Description=Ambient Thermal Arbitrage Gateway
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
WorkingDirectory=/opt/ata
ExecStart=/opt/ata/thermal-arbitrage-mvp
Restart=always
RestartSec=5
Environment=ATA_SECRET=CHANGE_ME
Environment=ENTSOE_API_KEY=CHANGE_ME
Environment=RUST_LOG=info
# Hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ReadWritePaths=/opt/ata
# Non-root user
User=ata
Group=ata

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now ata-gateway
sudo systemctl status ata-gateway      # check it is running
journalctl -u ata-gateway -f           # live logs
```

### 5.2 Windows

Run it via Task Scheduler (at boot, "run whether user is logged on or not",
set `ATA_SECRET` in the environment) or as an NSSM service:

```powershell
nssm install ATA-Gateway "C:\ata\thermal-arbitrage-mvp.exe" "C:\ata"
nssm set ATA-Gateway AppEnvironmentExtra ATA_SECRET=CHANGE_ME
nssm start ATA-Gateway
```

> The control loop is **non-blocking** (Ctrl+C is watched by a detached task),
> so it runs correctly without an interactive console — it no longer stalls
> when started by a service/scheduler.

### 5.3 Docker

```dockerfile
FROM debian:bookworm-slim
WORKDIR /opt/ata
COPY target/release/thermal-arbitrage-mvp .
COPY config.toml .
RUN useradd -r -s /usr/sbin/nologin ata
USER ata
EXPOSE 8080
CMD ["./thermal-arbitrage-mvp"]
```

```yaml
# docker-compose.yml
services:
  ata:
    build: .
    restart: unless-stopped
    environment:
      ATA_SECRET: "${ATA_SECRET}"
      ENTSOE_API_KEY: "${ENTSOE_API_KEY}"
      RUST_LOG: info
    ports: ["8080:8080"]
    volumes:
      - ./data_cache:/opt/ata/data_cache
      - ./thermal_arbitrage_log.csv:/opt/ata/thermal_arbitrage_log.csv
    network_mode: host      # reach the PLC subnet easily
```

---

## 6. Live Data Sources (current status, Aug 2026)

The price pipeline uses a **multi-source failover chain**:

| Priority | Source | Status | Notes |
|---|---|---|---|
| 1 | **ENTSO-E Transparency Platform** | ✅ reliable, official | Requires free API key (`ENTSOE_API_KEY`). Best choice. |
| 2 | energy-charts.info | ⚠️ intermittent (503 in tests) | No key. May be down/maintenance. |
| 3 | SMARD | ❌ 404 in tests | The `chart_data` JSON endpoint pattern appears discontinued. |
| 4 | `data_cache/last_price.json` | ✅ always available | Last-known-good price persisted on every successful fetch. |

Weather (Open-Meteo) worked in tests (real ambient temp returned). When no
source is reachable the gateway keeps the last price and logs
`All price sources failed (failure #N)`.

**Price staleness:** a day-ahead price older than `live_data.price_max_age_hours`
(default 6 h) is treated as STALE — arbitrage and MPC pre-cooling are
suppressed (thermostat + safety modes keep running), a `price_stale` alert
fires, and the last-known price is frozen. This prevents the gateway from
trading on a dead market feed.

**MPC lookahead:** in live mode the MPC horizon is built from the real
day-ahead hourly price series (ENTSO-E / energy-charts) paired with the
Open-Meteo hourly temperature forecast — not a flat repeat of the current
price, so `MpcPreCool` can actually engage when a cheap window exists.

**Grid frequency — three sources, in priority order:**

1. **Modbus meter register (recommended):** set `modbus.registers.grid_frequency_addr`
   (signed int16, Hz × 100, e.g. `5000` = 50.00 Hz). The gateway then responds
   to the meter's ACTUAL frequency — required for real FCR/grid-balancing.
   If the PLC/meter does not expose it, any Modbus-capable grid meter can be
   added to the register map.
2. **Live URL source:** `live_data.frequency_url` → any JSON endpoint returning
   `{"frequency_hz": 50.01}` (contract: a single number, refreshed on the price
   cadence, ignored unless fresh < 5 min). Useful when an operator already
   publishes frequency (e.g. their own gateway or a VPP platform).
3. **Dataset (fallback):** the NRW 2025 simulated frequency — simulation only;
   never acceptable for real grid services. If neither 1 nor 2 is configured,
   the gateway logs that grid balancing is running on simulated frequency.

**Recommended production setup:** register a free ENTSO-E API key
(https://transparency.entsoe.eu → “Data via API”), set `ENTSOE_API_KEY`,
keep energy-charts/SMARD/cache as fallbacks, and wire a real frequency input
(Modbus register preferred) before relying on grid-balancing revenue.

**Validate the real data path before go-live:** `cargo run --release -- --validate-live`
probes every source (ENTSO-E, energy-charts, SMARD, cache, Open-Meteo,
frequency URL) through the real fetch/parse code and prints a JSON report —
run it with `ENTSOE_API_KEY` set to confirm the official price path works
end-to-end on the target host.


---

## 8. Monitoring (Prometheus + Grafana)

`/metrics` exposes Prometheus text metrics (see README). Minimal scrape config:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: ata
    static_configs:
      - targets: ["localhost:8080"]
```

A ready-made Grafana dashboard for these metrics ships at
`grafana/ata-dashboard.json` (provisioning instructions in
`grafana/README.md`).

Useful alerts (example Prometheus rules):

```yaml
groups:
  - name: ata
    rules:
      - alert: ATAEmergency
        expr: ata_control_mode_total{mode="Emergency"} > 0
        for: 1m
        labels: { severity: critical }
      - alert: ATAPriceSourceDown
        expr: ata_alerts_total{severity="warning"} > 0
        for: 10m
```

---

## 9. Alerts

With `alerts.enabled = true` and a webhook URL (Slack/Discord/Teams), the
gateway posts structured JSON alerts with exponential retry. Test it:

```bash
curl -X POST localhost:8080/alerts/test -H "Authorization: Bearer $TOKEN"
```

Emergency/ThermalGuard alerts are throttled (max one every 300 s) to avoid
notification storms.

---

## 10. OTA Updates

The gateway can self-update via a signed manifest:

```json
{
  "version": "0.3.0",
  "url": "https://cdn.example.com/ata/firmware-0.3.0.bin",
  "sha256": "<hex sha256 of bundle>",
  "signature_hex": "<ed25519 signature over the bundle, hex>"
}
```

- Point `ota.manifest_url` at it, set `ota.verify_key_hex`, and `ota.enabled = true`.
- Verification is fail-closed (Ed25519 + SHA-256). Bundles stage to
  `ota_staging/`; the bootloader applies them on next restart.
- If you don't have a bootloader integration yet, keep `ota.enabled = false`
  and ship new binaries the normal way.

---

## 11. Go-Live Checklist

- [ ] PLC reachable from the gateway host (`ping`, Modbus test)
- [ ] `sensor.simulate = false`, register addresses match the PLC map
- [ ] `ATA_SECRET` set; `auth_enabled = true`; verify `/price` returns 401 without a token
- [ ] `ENTSOE_API_KEY` set; confirm `/status` shows a real `price_eur_per_mwh` and no `All price sources failed`
- [ ] `/status` shows real ambient temp from the weather API (or PLC)
- [ ] Compressor coil flips correctly when you `POST /price` a cheap (5 €/MWh) and an expensive (150 €/MWh) value
- [ ] Emergency behavior: with a warm ambient, watch TTF decrease and Emergency mode force cooling
- [ ] Alerts reach your Slack/Teams channel (`POST /alerts/test`)
- [ ] Prometheus scrapes `/metrics`; Grafana shows temp/price/mode
- [ ] CSV logging file is writable and growing
- [ ] Service starts on boot (systemd enabled / scheduler configured)
- [ ] Backups: `config.toml` + `data_cache/` copied off-device daily

---

## 12. Troubleshooting

| Symptom | Likely cause | Fix |
|---|---|---|
| `Failed to bind 0.0.0.0:8080` (os error 10048) | Port already in use | Another instance running; stop it or change `api.bind_address` |
| `All price sources failed` | No internet / APIs down / no ENTSO-E key | Check `ENTSOE_API_KEY`; gateway falls back to cached price |
| `Modbus connect failed` | PLC unreachable | Check `modbus.host`/`port`/firewall; gateway retries with backoff |
| API returns 401 with correct token | Wrong `ATA_SECRET` used by server vs. client | Both must use the same secret; restart after changing env |
| Log shows `ALERT [critical]` | Cold chain at risk | Emergency mode active — verify cooling is actually running |
| Metrics all zero on first scrape | Old build (metrics init bug) | Rebuild from current sources (self-initializing registry) |

---

## 13. Security

- **Auth:** with `security.auth_enabled = true`, every mutating route
  (`PUT /config`, `POST /price`, `POST /alerts/test`, `POST /ota/check`)
  requires a per-route bearer token:

  ```
  token = base64url( HMAC-SHA256( ATA_SECRET, route_path ) )
  ```

  Example (PowerShell):

  ```powershell
  $s='YOUR_ATA_SECRET'
  $h=New-Object System.Security.Cryptography.HMACSHA256
  $h.Key=[Text.Encoding]::UTF8.GetBytes($s)
  ([Convert]::ToBase64String($h.ComputeHash([Text.Encoding]::UTF8.GetBytes('/price')))
     ).Replace('+','-').Replace('/','_').TrimEnd('=')
  ```

  Use it:
  `curl -X POST localhost:8080/price -H "Authorization: Bearer $TOKEN" -d '{"price_eur_per_mwh":70}'`

- **TLS:** set `tls_cert_path`/`tls_key_path` (PEM) to serve HTTPS; or
  terminate TLS at a reverse proxy (nginx/Caddy).
- **Network:** bind the API to a private interface or `127.0.0.1` behind a
  proxy. Never expose the Modbus port publicly.

**Sample `config.toml`** (placeholders only — set `webhook_url`, `ENTSOE_API_KEY` and `ATA_SECRET` before go-live):

```toml
[api]
bind_address = "0.0.0.0:8080"
# tls_cert_path = "/etc/ata/tls/fullchain.pem"
# tls_key_path  = "/etc/ata/tls/privkey.pem"

[alerts]
enabled = true
webhook_url = "https://hooks.slack.com/services/XXXX"
max_retries = 5
retry_backoff_sec = 2

[live_data]
enabled = true
price_refresh_sec = 900
weather_refresh_sec = 3600
latitude = 52.52
longitude = 13.40

[ota]
enabled = false                     # enable once a manifest server exists
```

**Secrets (never in config files):**

```bash
export ATA_SECRET="$(openssl rand -base64 32)"     # API bearer-token secret
export ENTSOE_API_KEY="your-free-key-here"          # official price source
```

The gateway overrides `security.api_secret` from `ATA_SECRET` and reads the
ENTSO-E key from `ENTSOE_API_KEY` at startup.
