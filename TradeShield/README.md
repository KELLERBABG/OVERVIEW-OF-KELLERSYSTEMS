# WashTrap Ledger

**Retrofit microfiber filter for commercial washing machines + capture ledger. Prove microplastic stopped, sell credits to textile brands.**

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Node.js](https://img.shields.io/badge/Node.js-20+-green.svg)](https://nodejs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-blue.svg)](https://www.typescriptlang.org/)

---

## What Is WashTrap?

WashTrap is a **hardware + software** solution for the microplastic crisis:

- **Hardware**: A retrofit polypropylene filter (80um mesh) installed on commercial washing machine drain lines, paired with an ESP32 microcontroller that counts wash cycles and measures water volume
- **Software**: A ledger system (SQLite, not blockchain) that receives telemetry from ESP32 devices, proves microplastic capture, and generates tradeable offset credits for textile brands

**The problem**: Washing machines release ~700mg of microplastic per 6kg wash. The EU Ecodesign for Sustainable Products Regulation (ESPR) will mandate filters on all new washing machines by 2027. Brands need auditable proof of capture for CSR reporting.

**The solution**: WashTrap provides that proof. Hardware costs 39 EUR, service is 9 EUR/mo. Credits are sold to brands at 1 EUR/gram.

---

## Quick Start

### Prerequisites

- [Node.js](https://nodejs.org/) 20+
- [pnpm](https://pnpm.io/) 8+

### 1. Install Dependencies

```bash
git clone <repository-url>
cd washtrap-ledger
pnpm install
```

### 2. Initialize Database

```bash
pnpm migrate    # Create tables
pnpm seed       # Populate with 6 months of demo data
```

### 3. Start Development Server

```bash
pnpm dev        # Starts API (port 4000) + Web (port 5173)
```

### 4. Open Dashboard

Navigate to [http://localhost:5173](http://localhost:5173)

**Demo credentials:**
- Email: `admin@berlin-laundry.de`
- Password: `password`

### 5. Run ESP32 Simulator (Optional)

In a separate terminal:

```bash
cd packages/api
pnpm simulate              # All devices, real-time
pnpm simulate:fast         # 120x speed
pnpm simulate:one          # Single device, 50 cycles
```

---

## Architecture

```
+-------------------------------------------------------------------+
|                        WashTrap Ledger                             |
+-------------------------------------------------------------------+
|                                                                    |
|  +--------------+    HTTPS     +-------------------------------+  |
|  |  ESP32        |------------>|  API Server (Hono)            |  |
|  |  Filter Unit  |<------------|  |- Event Ingest              |  |
|  |  (per washer) |  heartbeat  |  |- Device Management         |  |
|  +--------------+              |  |- Dashboard Queries         |  |
|                                |  |- Credit Issuance           |  |
|  +--------------+    REST     |  |- PDF Generation            |  |
|  |  Web Dashboard|------------>|                               |  |
|  |  (React)      |<------------|  +--------------------------+ |  |
|  +--------------+              |  | SQLite Database           | |  |
|                                |  | |- organizations          | |  |
|  +--------------+              |  | |- users                  | |  |
|  |  Credit       |   PDF      |  | |- devices                | |  |
|  |  Certificate  |<-----------|  | |- capture_events         | |  |
|  |  (PDF)        |             |  | |- credit_batches         | |  |
|  +--------------+              |  | |- credit_audit_log       | |  |
|                                |  +--------------------------+ |  |
|                                +-------------------------------+  |
|                                                                    |
+-------------------------------------------------------------------+
```

### Data Flow

1. **ESP32 -> API**: Device sends `POST /api/v1/events` with wash cycle data (water volume, estimated microplastic, cycle count)
2. **API -> SQLite**: Event is validated, deduplicated, and stored with timestamp
3. **Dashboard -> API**: Web app queries aggregated data for visualization
4. **Credits -> PDF**: Brand downloads certificate proving capture for a period

---

## Project Structure

```
washtrap-ledger/
+-- packages/
|   +-- shared/          # TypeScript types, Zod schemas, constants
|   +-- api/             # Hono REST API + SQLite
|   |   +-- src/
|   |   |   +-- routes/  # Event, device, dashboard, credit endpoints
|   |   |   +-- middleware/  # Auth, rate limiting, error handling
|   |   |   +-- db/      # Schema, migrations, seed data
|   |   +-- tests/       # 63 integration tests
|   |   +-- scripts/     # ESP32 simulator, batch generator
|   +-- web/             # React + Vite + Tailwind frontend
|       +-- src/
|           +-- pages/   # Dashboard, devices, events, credits
|           +-- components/charts/  # Recharts visualizations
+-- docs/                # Whitepaper, API docs, architecture
+-- data/                # SQLite database (gitignored)
```

---

## API Reference

### Authentication

Two authentication methods:

1. **User Session**: Cookie-based (`session=<user_id>`) for web dashboard
2. **Device API Key**: `X-API-Key: <device_id>` header for ESP32 devices

### Core Endpoints

| Method | Endpoint | Auth | Description |
|--------|----------|------|-------------|
| `POST` | `/api/v1/events` | Device | Submit wash cycle event |
| `POST` | `/api/v1/events/batch` | Device | Submit multiple events |
| `GET` | `/api/v1/events` | User | List events (paginated, filterable) |
| `POST` | `/api/v1/devices/:id/heartbeat` | Device | Device check-in |
| `GET` | `/api/v1/devices` | User | List devices |
| `GET` | `/api/v1/dashboard/summary` | User | KPI summary |
| `GET` | `/api/v1/dashboard/timeline` | User | Time-series data |
| `POST` | `/api/v1/credits/issue` | User | Generate credit batch |
| `GET` | `/api/v1/credits/:id/certificate` | User | Download PDF certificate |

Full API documentation: [docs/API.md](docs/API.md)

---

## Development

### Commands

```bash
pnpm dev              # Start API + Web dev servers
pnpm build            # Build all packages
pnpm test             # Run all tests
pnpm migrate          # Run database migrations
pnpm seed             # Seed demo data
```

### Testing

```bash
cd packages/api
pnpm test             # Run all 63 integration tests
pnpm test:watch       # Watch mode
```

### ESP32 Simulator

```bash
cd packages/api
pnpm simulate                      # All devices, real-time
pnpm simulate:fast                 # 120x speed
pnpm simulate:one                  # Single device
pnpm batch-generate --count 2000   # Bulk historical data
```

---

## Database

### Schema

8 tables with proper foreign keys and indexes:

- `organizations` -- Laundries, brands, admin
- `users` -- Human accounts with roles
- `devices` -- ESP32 filter units
- `capture_events` -- Individual wash cycles (idempotent)
- `credit_batches` -- Offset credits with lifecycle
- `credit_audit_log` -- Immutable audit trail
- `device_health_checks` -- Daily online/offline status
- `_migrations` -- Schema version tracking

### Key Design Decisions

- **Idempotency**: `UNIQUE(device_id, wash_cycle)` prevents duplicate events
- **Conservative estimates**: 0.08 mg/L (credibility > optimism)
- **Credit pricing**: 0.001 EUR/mg (1 EUR/gram)
- **Audit trail**: Every credit status change is logged

---

## Security

### What's Implemented

- Parameterized SQL queries (no injection)
- Rate limiting on sensitive endpoints
- Org-scoped data isolation
- Device authentication via API keys
- Input validation via Zod schemas
- Request ID tracking

### Known Limitations (MVP)

- Password auth uses hardcoded comparison (needs bcrypt)
- Session cookie security depends on NODE_ENV
- In-memory rate limiter (no persistence)

See [SanityCheck-Report.md](SanityCheck-Report.md) for full security audit.

---

## Deployment

### Production Setup

```bash
# 1. Set environment variables
export API_PORT=4000
export DB_PATH=/var/lib/washtrap/washtrap.db
export NODE_ENV=production
export CORS_ORIGINS=https://app.washtrap.io

# 2. Run migrations
pnpm migrate

# 3. Start with process manager
pm2 start packages/api/dist/index.js --name washtrap-api
```

### Docker (Recommended)

```dockerfile
FROM node:20-alpine AS builder
WORKDIR /app
COPY . .
RUN corepack enable && pnpm install --frozen-lockfile
RUN pnpm build

FROM node:20-alpine
WORKDIR /app
COPY --from=builder /app/packages/api/dist ./dist
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/packages/api/package.json ./
RUN addgroup -g 1001 washtrap && adduser -D -u 1001 washtrap
USER washtrap
EXPOSE 4000
CMD ["node", "dist/index.js"]
```

Full deployment guide: [docs/DEPLOYMENT.md](docs/DEPLOYMENT.md)

---

## Pricing Model

| Component | Price |
|-----------|-------|
| Hardware (PP filter + ESP32) | 39 EUR one-time |
| Monthly service | 9 EUR/month |
| Credit price | 1 EUR/gram of microplastic |

**Example**: A hotel with 3 commercial washers captures ~15g/month. That's 15 EUR/mo in credits sold to a textile brand.

---

## Regulatory Context

### EU Ecodesign for Sustainable Products Regulation (ESPR)

- **Mandate**: All new washing machines must have microplastic filters
- **Timeline**: Expected enforcement 2027
- **Impact**: ~50 million washing machines in EU need retrofit solutions
- **WashTrap advantage**: Provides auditable proof of capture for brands

### Why Blockchain Isn't Needed

The whitepaper explicitly states: **"Simple ledger (SQLite, not blockchain)."**

Reasons:
1. **Simplicity**: SQLite is battle-tested, requires no infrastructure
2. **Auditability**: Append-only audit log provides full traceability
3. **Cost**: No gas fees, no node operators, no consensus overhead
4. **Speed**: Sub-millisecond queries vs. blockchain confirmation times
5. **Trust**: WashTrap is a registered company, not an anonymous protocol

---

## Roadmap

### Phase 1: MVP (Current)
- [x] ESP32 data pipeline
- [x] Web dashboard with Recharts
- [x] Credit system with PDF certificates
- [x] 63 integration tests
- [ ] Production authentication (bcrypt)
- [ ] Deploy to pilot customer

### Phase 2: Scale
- [ ] PostgreSQL migration (for concurrent users)
- [ ] Real-time alerts (WebSocket)
- [ ] Mobile app for on-site monitoring
- [ ] API key management dashboard
- [ ] Webhook integrations

### Phase 3: Ecosystem
- [ ] Multi-region support
- [ ] White-label dashboard for brands
- [ ] Integration with CSR reporting platforms
- [ ] Marketplace for credit trading
- [ ] Regulatory compliance API

---

## Contributing

1. Fork the repository
2. Create a feature branch (`git checkout -b feat/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feat/amazing-feature`)
5. Open a Pull Request

### Development Guidelines

- Write tests for new features
- Follow existing code style
- Update documentation for API changes
- Run `pnpm test` before committing

---

## License

MIT License - see [LICENSE](LICENSE) for details.

---

## Contact

- **Website**: [washtrap.io](https://washtrap.io)
- **Email**: hello@washtrap.io
- **Documentation**: [docs.washtrap.io](https://docs.washtrap.io)

---

## Acknowledgments

- Plymouth University for microplastic research data
- EU Commission for regulatory framework
- Open source community for Hono, React, SQLite, and Recharts
