# Architecture Decision Log

Decisions made during Event Forecast development.
Format: title, date, context, decision, rationale, alternatives, tradeoffs.
Rationale flagged `TBD: capture rationale` where it could not be confirmed from
code or docs.

---

## ADR-001: Rust + Rocket over Python / Go / Node.js

**Date:** 2026-06-03 (initial commit)

**Context:** The service needs to run a probabilistic forecasting loop — sorting
events, building transition count maps, computing medians, rolling forward a
multi-step prediction — on every API request. A Node.js prototype existed before
the first commit but was replaced.

**Decision:** Rust with Rocket 0.5 as the HTTP layer.

**Rationale (from docs):** The product brief calls for a model whose "failure
modes are obvious" and which "can be replaced by a stronger model later without
changing the event contract." Rust's type system makes the data contract explicit
at compile time; the borrow checker prevents whole classes of state bugs in the
mutable `HashMap` accumulation loops inside `fit_model`. The README explicitly
states the output must be debuggable "before the project adds heavier forecasting
or spatiotemporal models."

**Alternatives:** Node.js (replaced), Python/FastAPI, Go.

**Tradeoffs:**
- Build times are slower than Go or Node.
- Fewer data-science libraries than Python (no scikit-learn, pandas).
- `serde` + `rocket::serde::json` give zero-copy JSON with no runtime surprises.
- `thiserror` + `anyhow` make every error path named and composable.

---

## ADR-002: TimescaleDB over plain Postgres or ClickHouse

**Date:** 2026-06-03 (initial commit — migration `0001_timescale_events.sql`)

**Context:** Event streams are time-ordered. The primary query pattern is
`WHERE stream_id = $1 ORDER BY ts ASC`. The schema needs chunked storage and
fast time-range scans.

**Decision:** TimescaleDB extension on Postgres 16 (`timescale/timescaledb:latest-pg16`).
The `events` table is promoted to a hypertable on `ts` with four secondary indexes:
`(stream_id, ts desc)`, `(event_type, ts desc)`, `(entity_id, ts desc)`, and a GIN
index on `properties`.

**Rationale (from docs):** The OSS evaluation explicitly notes "TimescaleDB is
enough for now" and "consider query-level aggregation before adding a second
storage engine." The `DATABASE_URL` is optional — the service runs fully in-memory
with inline events — so the database is not on the hot path for development or testing.

**Alternatives:**
- Plain Postgres: loses automatic time partitioning and hypertable chunk pruning.
- ClickHouse: stronger analytics queries but a different operational model; flagged
  as "High for current TimescaleDB-backed service" in the OSS evaluation.
- Apache DataFusion: explicitly parked.

**Tradeoffs:**
- TimescaleDB adds an extension dependency but reuses the Postgres driver (`sqlx`).
- A `properties JSONB` column with a GIN index means arbitrary event schemas with
  no migrations for new property types.
- TBD: capture rationale for choosing `(id, ts)` as the composite primary key
  rather than a standalone UUID primary key.

---

## ADR-003: Statistical baseline before ML

**Date:** 2026-06-03 (product-brief.md, README.md)

**Context:** The team needs predictable, inspectable output and a measurable
evaluation loop before model complexity is justified.

**Decision:** The forecasting engine (`fit_model` / `predict_next_stream` in
`src/lib.rs`) uses only:
1. First-order Markov transition counts on event types.
2. Median inter-arrival time per event type (with global median fallback).
3. A four-source weighted merge for categorical property prediction:
   event-transition-conditioned distribution (0.55), per-event-type distribution
   (0.25), per-property-value transition (0.15), global marginal (0.05).

No embeddings, no gradient descent, no external ML library.

**Rationale (verbatim from product-brief.md):** "This is not meant to be final.
It is meant to make the data contract and failure modes obvious." The OSS
evaluation adds: "The product still needs a larger real event stream before
model sophistication matters."

**Alternatives:** `augurs` (Rust-native time-series toolkit) is the documented
next step. `smartcore`, `linfa`, and Lag-Llama are on the watchlist.
See `docs/oss-integration-evaluation.md`.

**Tradeoffs:**
- Predictions are fully explainable (`why` field on every `EventPrediction`).
- `evaluate_stream` gives a held-out accuracy loop so any future model upgrade
  can be compared objectively.
- A first-order Markov chain cannot model longer-range patterns (e.g., a five-step
  cycle). That limitation is explicitly accepted.

---

## ADR-004: Flat `events` table with JSONB properties

**Date:** 2026-06-03 (migration `0001_timescale_events.sql`)

**Context:** Events carry heterogeneous properties (`location`, `service_type`,
`product_type` are the defaults but callers can supply any string key). A
normalized properties table would require schema migrations for each new field.

**Decision:** Single `events` table: `(id uuid, stream_id text, entity_id text,
ts timestamptz, event_type text, properties jsonb)`. Property fields are read
from `properties` at query time; the forecasting engine works purely in-memory
over the deserialized structs.

**Rationale:** Confirmed by the `DEFAULT_PROPERTY_FIELDS` constant in `lib.rs`
and the `normalize_event` function, which lifts top-level aliases (`location`,
`service_type`, `product_type`) into `properties` for callers that do not nest
them. This lets the schema be stable while the property vocabulary grows.

**Alternatives:** Separate `event_properties` rows (more normalized, harder to
query in bulk). Typed columns per property (requires a migration per new property
type).

**Tradeoffs:**
- GIN index on `properties` supports containment queries but is not as fast as a
  btree index on a dedicated column.
- The forecasting engine never queries individual properties from SQL; it loads
  the full event list and processes in Rust, so query-time property access cost
  is not on the critical path today.
- TBD: capture whether the `properties` GIN index was intentional for future
  filter queries or added defensively.

---

## ADR-005: Optional database — stateless request mode

**Date:** 2026-06-03 (initial commit — `AppState { db: Option<PgPool> }`)

**Context:** Most development and testing workflows don't need a running
TimescaleDB instance. Spinning up Docker just to test the transition logic adds
friction.

**Decision:** `DATABASE_URL` is optional. When absent, `AppState.db` is `None`.
All endpoints accept either `events: [...]` (inline) or `stream_id` (database
lookup). The entire test suite runs without a database.

**Rationale:** Confirmed by `build_state()` in `main.rs` which branches on
`env::var("DATABASE_URL")`. The product brief's non-goals include "do not build
a dashboard yet," implying the database-backed path is secondary to proving
the prediction contract.

**Tradeoffs:**
- Simplifies CI: `cargo test` needs no external services.
- Dual code path (inline vs. DB) must be maintained in `resolve_events`.
- TBD: capture the original decision to keep the database optional rather than
  always requiring it.

---

## ADR-006: `entity_id` as optional event grouping dimension

**Date:** 2026-06-09 (Phase 5 — `fit_per_entity_models`)

**Context:** A single stream can contain events from multiple orders, sessions,
or users. A global model averages over all entities; a per-entity model captures
entity-specific transition patterns.

**Decision:** `entity_id` is an optional field on every event. `fit_per_entity_models`
splits events by `entity_id` and fits a `ForecastModel` per group. The `/predict`
and `/numeric` endpoints accept an optional `entity_id`; when set they use the
per-entity model, falling back to the global model when the entity is sparse.

**Rationale:** Confirmed by `pick_model` in `main.rs` and the
`per_entity_models_split_by_entity_id` test in `lib.rs`. The test asserts that
`order_1`, `order_2`, `order_3` each produce a distinct model from the sample
data.

**Tradeoffs:**
- Per-entity models can be underfitted when an entity has only a few events.
  The fallback to the global model mitigates this.
- Memory grows linearly with the number of entities in the request payload.
