# Automated Mine Management System

[中文版本](./README.md)

This repository is the product foundation for a mine transportation dispatch system.

The goal is not to build just another admin panel. The goal is to digitize the most important operational loop inside a mine transportation business: dispatching, vehicle arrival, queue management, loading, weighing, completion, and operational visibility.

## Product Positioning

The system is designed to solve three practical problems in mine logistics:

- Low dispatch efficiency: jobs are assigned through calls and chat groups
- Poor field order: pit queues are opaque and easy to manipulate
- Delayed business visibility: trips, tonnage, and exceptions are still aggregated manually

The first release focuses on three outcomes:

- Operable: the dispatch-to-completion loop works end to end
- Controllable: key actions are traceable and manual overrides are recorded
- Visible: dispatch rooms can monitor vehicles, pits, queues, work orders, and alerts in real time

## Core Users

- Dispatcher: assigns jobs and handles exceptions
- Driver: receives tasks, checks in, tracks queue status, views history
- Pit operator: verifies vehicles, manages the line, confirms loading
- Weighbridge operator: records weights and completes the trip
- Finance / operations: reviews tonnage, trips, throughput, and settlement data

## MVP Scope

The first version targets the shortest business loop instead of a feature-heavy platform.

- Dispatch console: Web application
- Driver mobile app: Flutter
- Pit operation app: Flutter app or lightweight H5 client
- Real-time queue state: Redis
- Core business database: PostgreSQL
- Real-time notifications: WebSocket
- Operational dashboards: Vite-based front-end views

## Technical Direction

- Backend: Rust
- Frontend: Vite
- Mobile: Flutter
- Database: PostgreSQL
- Cache and live queue state: Redis
- Deployment: Docker + Nginx

Recommended Rust stack:

- `axum` for HTTP APIs and WebSocket
- `sqlx` for database access and migrations
- `tokio` as the async runtime
- `redis` for queue state and short-lived data
- `serde` for serialization

## Repository Contents

- `docs/product-overview.md`: product-facing project introduction
- `docs/requirements-baseline.md`: MVP requirements baseline
- `docs/architecture.md`: architecture and implementation guidance
- `docs/scenario-coverage-analysis.md`: field-scenario coverage review
- `db/init.sql`: initial PostgreSQL schema

## Monorepo Layout

```text
apps/
  admin-web/   Vite admin console
  api/         Rust API service
  driver-app/  Flutter driver app
  pit-app/     Flutter pit operation app
db/
docs/
```

## Phase-One Goal

Phase one is about operational standardization before optimization.

1. Every waybill has a clear state, owner, and timeline
2. Every pit queue is visible, controlled, and auditable
3. Daily trips, tonnage, exceptions, and efficiency metrics are captured automatically

## Future Expansion

- License plate recognition and camera integration
- Automated weighbridge integration
- Dispatch recommendation and pit balancing
- Exception detection and congestion prediction
- Production, sales, and settlement integration
