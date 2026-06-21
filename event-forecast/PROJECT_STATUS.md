# event-forecast — PROJECT STATUS
Last updated: 2026-06-20

## Why / What

**Event Forecast** is a time-series forecasting product for event, order, and operations streams. Product thesis: a transparent, debuggable baseline (transition/median models) that proves the data shape and evaluation loop before heavier ML — stream-to-decision engine for ops teams.

**Users:** Researchers and fleet operators evaluating forecasting on timestamped event streams; future ops teams needing demand-by-neighborhood reports.

**Constraints:** Local/research only — no production deployment. No PII ingestion. No opaque ML until baseline is measurable on real volume. Sample stream is tiny (14 events, 4 entities).

**IN scope:** Rust + Rocket API, TimescaleDB ingest, explorer UI, held-out evaluation, alert/decision/replay slices.

**OUT of scope:** Production API/UI deploy, ML frameworks, geocoding services, persisted alert subscriptions, custom alert profile editor.

## Dependencies

### External

- **TimescaleDB/PostgreSQL:** local via Docker Compose on :54329; schema `migrations/0001_timescale_events.sql`.
- **Leaflet:** single-file explorer UI (`src/explorer.html`).
- **Rust crates:** `chrono`, `serde`, `serde_json`, `sqlx`, `tokio`, `uuid`, `anyhow`, `thiserror`; Rocket 0.5.

### Internal (fleet)

- **SaaS Maker:** fleet registry entry.
- **Future spike:** `augurs` OSS forecasting evaluation documented in `docs/oss-integration-evaluation.md` — keep baseline until larger stream.

### Stack & commands

**Stack:** Rust + Rocket 0.5 + TimescaleDB/PostgreSQL (sqlx) + Leaflet explorer UI (single-file HTML in `src/explorer.html`).

| Command | Purpose |
|---------|---------|
| `docker compose up -d` | Start TimescaleDB on :54329 |
| `export DATABASE_URL=postgres://event_forecast:event_forecast@localhost:54329/event_forecast` | DB connection |
| `psql "$DATABASE_URL" -f migrations/0001_timescale_events.sql` | Apply schema |
| `cargo run` | Rocket server → http://127.0.0.1:8088 |
| `cargo test` | Unit + integration tests |
| `cargo fmt --check` | Format check |
| `cargo check` | Compile check |
| `cargo run --bin evaluate -- tests/fixtures/sample-stream.json` | Offline held-out eval |
| `cargo run --bin load_events -- data/sample-events.json --stream-id orders-demo` | Seed DB from JSON |

**Env vars:** `DATABASE_URL` (optional — inline events work without DB).

## Timeline

- **Shipped (Phases 1–5)** — Core API, transparent baseline model, configurable location source, alerts/decision rules, stream replay/playback, explorer UI, TimescaleDB schema, fixture tests.
- **Next gate** — Real data volume validation per `ROADMAP.md` before Phase 6+ or `augurs` spike.

## Products

- **Local API:** http://127.0.0.1:8088 — Rocket server; no auth; no production deploy.
- **Explorer UI:** `GET /explorer` — Leaflet map with heatmap, replay controls, decision narrative.
- **CLI binaries:** `evaluate` (offline metrics JSON), `load_events` (JSON file ingest).
- **Sample data:** `data/sample-events.json`, `tests/fixtures/sample-stream.json` (14 events, 4 entities).
- **Artifacts:** `artifacts/action-report-sample.json` from `action_report_fixture` test.

## Features (shipped)

### Core API routes (`src/main.rs`, port 8088)

- `GET /` — HTML index listing all endpoints.
- `GET /health` — `{ok, database: configured|not_configured}`.
- `GET /explorer` — Leaflet map UI (`src/explorer.html`); `?demo=1` autoloads sample.
- `GET /sample-events.json` — bundled sample data.
- `POST /events` — ingest events into TimescaleDB (`stream_id`, events array).
- `POST /predict` — next-stream predictions from inline events or `stream_id`; optional `entity_id`, `horizon`, `summary`.
- `POST /report` — aggregate forecast digest: top event types, location/service/product mix, mean confidence, low-confidence steps.
- `POST /evaluate` — held-out split by `history_ratio` (default 0.6); event-type + property accuracy, timestamp error, uncertainty.
- `POST /heatmap` — location-by-time buckets; hot/cooling summaries; default 30min windows.
- `POST /anomalies` — z-score inter-arrival anomaly detection (`z_threshold`).
- `POST /numeric` — numeric property medians per event type (`numeric_fields`).
- `POST /decision-report` — window-half comparison, hot/cooling zones, mix shifts, narrative.
- `POST /action-report` — bundles heatmap + forecast + decision + evaluation metrics (explorer driver).
- `POST /replay` — windowed replay with per-step heatmap, forecast, drift markers, alerts.

### Architecture

- Rocket serves JSON API + static explorer HTML; no auth.
- Events normalized to `{id, ts, event_type, entity_id, properties}` shape; top-level aliases accepted.
- Baseline model: event-type transitions, median inter-arrival, property transitions, property distributions conditioned on transitions.
- Inline mode: predict from request body events without DB. Stored mode: `POST /events` ingest → `stream_id` predict.
- Per-entity models on `/predict` and `/numeric` with global fallback when sparse.
- Explorer UI (`GET /explorer`) calls `/action-report` for map + narrative + metrics.

### Transparent baseline model (`src/lib.rs` and modules)

- Event-type transition probabilities; median inter-arrival times.
- Property transitions between neighboring events; distributions conditioned on event-type transitions.
- Per-entity transition models (`entity_id` scoping with global fallback).
- Z-score anomaly detection on inter-arrival intervals.
- Numeric forecasting via per-event-type medians.
- `why` explanations on predictions (e.g. "transition from order_created").
- Confidence scores on event types and properties.

### Configurable location source (`src/location.rs`, PRD shipped)

- `LocationResolver` from event lat/lng, injected catalog, or deterministic fallback.
- Heatmaps return `locations` metadata.
- `/action-report` accepts optional `location_catalog`.
- Explorer renders arbitrary streams from API coordinates.

### Alerts and decision rules (`src/alerts.rs`, PRD shipped)

- Quiet default profile for hot/cooling zones, confidence floors, anomaly spikes, mix shifts.
- `/action-report` includes `alerts` summary.
- Explorer shows actionable alerts.

### Stream replay and playback (`src/replay.rs`, PRD shipped)

- Windowed replay recomputes heatmap, forecast, decision, drift markers, alerts per step.
- Explorer play/pause/scrub/step controls.
- Fixture-backed tests.

### Data & storage

- TimescaleDB schema: `migrations/0001_timescale_events.sql`.
- `docker-compose.yml` for local Postgres/Timescale on :54329.
- Sample data: `data/sample-events.json`, `tests/fixtures/sample-stream.json` (14 events, 4 entities).
- `load_events` binary for JSON file ingest.
- `evaluate` CLI binary for offline metrics JSON output.

### Explorer UI features

- Single-file Leaflet map with heatmap overlay.
- Load sample stream button; textarea for custom events.
- Play/pause/scrub/step replay controls.
- Decision narrative panel; alert display; held-out forecast quality when ≥4 events.

### Tests & artifacts

- `cargo test` including `action_report_fixture` → `artifacts/action-report-sample.json`.
- Entity-scoped numeric forecast tests in `main.rs` tests module.
- `cargo fmt --check`, `cargo check` as standard verification.

## Todo / Planned / Deferred / Blocked

### Planned

1. Run model on larger real stream to validate Phase 5 accuracy (`ROADMAP.md` gate).
2. Add fixture-backed `augurs` benchmark only after larger stream exists (`docs/oss-integration-evaluation.md`).
3. Decide first real event source to ingest (`POST /events` pipeline).
4. Custom alert profile editor in explorer UI (`src/explorer.html`).

### Deferred

- Production API or UI deployment.
- ML frameworks or model training beyond transparent baseline.
- PII-heavy data sources.
- Persisted alert subscriptions and delivery integrations (email/paging).
- Geocoding service integration for name-only locations.
- Staged product plan phases 6+ in `ROADMAP.md` until real data volume validates baseline.

### Blocked

- Sample stream too small for meaningful accuracy validation — all metrics are fixture/demo quality only.
- No production deployment path or real ingest pipeline yet.
- Phases 1–5 code shipped locally; next gate is real data volume per `ROADMAP.md`.
- No auth, rate limiting, or multi-tenant isolation (acceptable for local research).
