# Automated Mine Management System

[中文版本](./README.md)

A digital dispatch system for mine transportation operations. Digitizes the core business loop from dispatching, vehicle arrival, queue management, loading, weighing, to completion and data aggregation — replacing WeChat groups, phone calls, and Excel spreadsheets.

## Product Positioning

Solves three core problems in mine logistics:

- **Low dispatch efficiency**: jobs assigned through calls and chat groups with no synchronization
- **Poor field order**: pit queues are opaque, causing disputes and queue-jumping
- **Delayed business visibility**: trips, tonnage, exceptions, and efficiency are manually aggregated

Phase-one goals:

- Operable: dispatch-to-completion loop works end to end
- Controllable: key actions are traceable, manual overrides recorded, audit logs maintained
- Visible: dispatch room real-time dashboard for vehicles, pits, work orders, queues, and alerts

## Tech Stack

| Layer | Technology | Notes |
|-------|-----------|-------|
| Backend | Rust (axum 0.8 + sqlx 0.8) | High-performance async API |
| Cache | Redis 7 | Queue count cache, dashboard cache, rate limiting |
| Database | PostgreSQL 16 | Business database, 20+ tables |
| Frontend | Vue 3 + Vite 6 + TypeScript | Dispatch console web app |
| Mobile | Flutter | Driver + pit operation apps |
| Deployment | Docker Compose | One-command startup |
| Auth | JWT (HS256) | access token 24h + refresh token 30d |
| Realtime | WebSocket | 7 event types, JWT authenticated |
| Algorithm | WPMA + AI Enhanced | Pure algorithm / Pangu LLM dual mode, runtime switchable |

## Feature Modules

### Backend API (19 modules)

| Module | Function | Highlights |
|--------|----------|------------|
| **auth** | Login / token refresh | bcrypt verification, dual token |
| **driver** | CRUD + search + batch import | Keyword search, unique constraints |
| **pit** | CRUD | Real-time queue count |
| **waybill** | Full lifecycle | 9-state machine, 4 arrival methods |
| **queue** | Join / call-next / leave | Transaction + row lock, Redis write-through |
| **loading** | Start / finish | Transaction control, record linkage |
| **weighing** | Weigh and complete | Non-negative validation, auto-complete |
| **dashboard** | Operations overview | LATERAL JOIN queries, Redis 30s cache |
| **dispatch** | Smart dispatch recommendation | WPMA algorithm + AI enhancement |
| **ai** | Dispatch algorithm engine | WPMA weighted priority matching + Pangu LLM |
| **ws** | WebSocket push | 7 event types, JWT authenticated |
| **alert** | Alert management | QueryBuilder safe queries |
| **fence** | Geofence | Haversine distance, auto-arrival |
| **scale** | Weighbridge auto-collect | Serial / bluetooth, anti-cheat validation |
| **missions** | Autonomous mine truck tasks | claim / status / complete flow |
| **offline** | Offline sync | Idempotency keys + optimistic locking, transactional |
| **system_config** | Runtime config | Algorithm / AI mode switch (Arc\<RwLock\>) |
| **audit_log** | Operation audit | Fire-and-forget logging |
| **health** | Health check + OpenAPI | `GET /docs/openapi.json` |

### Waybill State Machine

```
pending_dispatch → dispatched → arrived → queueing → loading → loaded → weighing → completed
                  ↓
               cancelled (any non-terminal state, reason required)
```

**Four arrival methods**: manual check-in, plate scan, geofence auto-arrival, offline sync

### Middleware

- **JWT Auth**: protected routes auto-verify Bearer token
- **Redis Rate Limiting**: sliding window 100 requests / 60s / IP
- **CORS**: cross-origin support
- **Request Tracing**: TraceLayer

### Frontend Applications

- **admin-web**: Dispatch console (8 pages + 4 reusable components)
- **driver-miniapp**: Driver H5 (lightweight)
- **pit-h5**: Pit operation H5 (lightweight)
- **demo-hub**: Demo portal

### Flutter Applications

- **driver-app**: Driver App (task display, check-in, plate recognition, offline sync)
- **pit-app**: Pit App (vehicle verification, queue management, loading confirmation)
- **shared**: Shared library (offline engine + plate OCR + geofence)

### Database

- 5 migrations, 20+ tables
- PostgreSQL enum types (6 status enums)
- Optimistic locking (waybills.version)
- Conditional unique index (single active waybill per driver)
- Check constraints (non-negative weight, cancel requires reason)

## Project Structure

```text
apps/
  api/              Rust API service (19 business modules)
  admin-web/        Vue 3 dispatch console
  driver-app/       Flutter driver app
  driver-miniapp/   Driver H5
  pit-app/          Flutter pit app
  pit-h5/           Pit H5
  demo-hub/         Demo portal
  shared/           Flutter shared library (offline + plate + geofence)
db/
  init.sql          Database initialization script
  migrations/       5 migration files
docs/               40+ technical docs + OpenAPI spec
scripts/            Scraper and email scripts
```

## Local Run

### Start Backend

```bash
docker compose up --build
```

PostgreSQL and Redis run inside the compose network. API exposed on local port `3000`.

### Start Frontend

```bash
# Install dependencies (first time)
npm install

# Dispatch console (port 5173)
npm run dev:admin

# Driver app (port 5174)
npm run dev:driver

# Pit app (port 5175)
npm run dev:pit

# Demo portal (port 5180)
npm run dev:demo
```

All frontend dev servers auto-proxy `/api` requests to `localhost:3000`.

### Build Frontend

```bash
npm run build:all
# Or individually
npm run build:admin
npm run build:driver
npm run build:pit
npm run build:demo
```

### Run Tests

```bash
# In container (recommended)
docker compose run --rm api-test

# On host
DATABASE_URL=postgres://postgres:postgres@localhost:5432/postgres cargo test -p api
```

### Run Migrations

```bash
cargo run -p api --bin migrate
```

## API Documentation

- OpenAPI spec: `GET /docs/openapi.json`
- Full API reference: `docs/api-reference.md`
- Deployment guide: `docs/deployment-guide.md`

## Algorithm

Dispatch algorithm based on **generic framework + terrain adaptation**:

- **WPMA Algorithm**: Weighted Priority Matching (idle 0.3 + workload 0.2 + distance 0.3 + pit priority 0.2)
- **AI Enhanced Mode**: Pangu LLM optimization
- **Dual Mode Switch**: `POST /api/v1/system/dispatch-mode` runtime switchable
- **Congestion Prediction**: threshold-based (>10 high, >5 medium, others low)
- **Anomaly Detection**: timeout rules (>30min anomaly, >15min warning)

See `docs/dispatch-algorithm.md` for details.

## Documentation

| Document | Description |
|----------|-------------|
| `docs/architecture.md` | System architecture |
| `docs/api-reference.md` | API reference |
| `docs/deployment-guide.md` | Deployment guide |
| `docs/database-schema.md` | Database schema |
| `docs/dispatch-algorithm.md` | Dispatch algorithm |
| `docs/development-guide.md` | Development guide |
| `docs/user-manual.md` | User manual |
| `docs/phase-plan.md` | Phased implementation plan |
| `docs/openapi.json` | OpenAPI 3.0 spec |

## User Roles

- **Dispatcher**: global view, assign tasks, handle exceptions, view AI recommendations
- **Driver**: receive tasks, check in, view queue status, view history
- **Pit Operator**: verify vehicles, manage queue, confirm loading
- **Weighbridge Operator**: record weights, complete trips
- **Finance / Operations**: review throughput, trips, tonnage, settlement data

## Test Coverage

- 22 unit tests (config, error, pagination, auth, ai modules)
- 2 integration tests (full waybill flow, status validation)
- All passing

## Future Expansion

- Report export (CSV/Excel)
- Multi-tenant isolation
- Financial settlement module
- Notifications (SMS / WeChat)
- App store publication (iOS / Android)
- PostGIS spatial index optimization
