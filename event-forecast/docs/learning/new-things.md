# New things to learn — event-forecast

A Rust/Rocket service that fits a statistical event-sequence forecaster on timestamped event streams and predicts the next N events with per-property confidence scores.

---

## TimescaleDB hypertables
- What: Postgres extension that auto-partitions a table by time into fixed "chunks," making time-range scans sub-linear.
- Why here: TBD
- Gotcha (from code): composite `(id, ts)` primary key is required for hypertable promotion — a standalone UUID PK is rejected by `create_hypertable`. See `migrations/0001_timescale_events.sql:11` for the `primary key (id, ts)` declaration and line 14 for the `create_hypertable` call.
- Source: https://docs.timescale.com/use-timescale/latest/hypertables/
- See also: `./external-references.md`

---

## Tokio async runtime
- What: Rust's most widely used async executor — schedules `async fn` tasks on a thread pool.
- Why here: TBD
- Gotcha (from code): Rocket's `#[launch]` macro owns the Tokio runtime; the `rt-multi-thread` feature in `Cargo.toml:13` is required or `#[launch]` panics at startup — `tokio = { features = ["macros", "rt-multi-thread"] }`.
- Source: https://tokio.rs/tokio/tutorial

---

## Rocket 0.5 (Rust web framework)
- What: Type-safe async HTTP framework for Rust; request guards, fairings, and `#[launch]` entry point.
- Why here: TBD
- Gotcha (from code): `State<AppState>` is a Rocket request guard — if `AppState` is not `.manage()`'d in the builder, every handler that injects it returns a 500 at runtime with no compile error. See `src/main.rs:574` (`.manage(build_state().await)`) and `src/main.rs:193` (first guard injection).
- Source: https://rocket.rs/guide/

---

## SQLx (runtime SQL — no compile-time checking here)
- What: Async Rust SQL toolkit; its `query!` macro verifies SQL at compile time, but plain `sqlx::query(...)` is runtime-only.
- Why here: TBD
- Gotcha (from code): this project uses `sqlx::query(...)` (not `query!`), so SQL errors surface only at runtime. The pool itself is optional — `build_state()` at `src/main.rs:492-504` wraps the pool in `Option<PgPool>` and skips it when `DATABASE_URL` is absent, so the full test suite runs without a DB.
- Source: https://github.com/launchbadge/sqlx

---

## First-order Markov chain + transition matrix
- What: Model where next state depends only on current state; stored as a count matrix over (current, next) pairs.
- Why here: TBD
- Gotcha (from code): `event_transitions` (`src/lib.rs:84`) is an unnormalized count map. `predict_event_type` at `src/lib.rs:1283-1299` normalizes on read via `top_choice` (divides winning count by row total) and falls back to the global `event_counts` frequency when the current event type has no observed successors — there is no Laplace smoothing.
- Source: https://en.wikipedia.org/wiki/Markov_chain
- See also: `./external-references.md`

---

## Weighted evidence ensemble for categorical prediction
- What: Merging multiple sparse frequency distributions with fixed weights to predict a categorical value.
- Why here: TBD
- Gotcha (from code): weights `0.55 / 0.25 / 0.15 / 0.05` are literal constants in `predict_property` at `src/lib.rs:1320-1325` — the order is (event-transition counts, event-type counts, property-transition counts, global counts). There is no record of whether they were tuned against data or set by intuition.
- Source: TBD

---

## Median vs. mean for robust time estimates
- What: Median ignores outliers for central-tendency estimation; mean is sensitive to them.
- Why here: TBD
- Gotcha (from code): `fit_model` stores `global_interval_ms: median(&global_intervals, 60_000)` at `src/lib.rs:222` (fallback is 60 s). `predict_next_stream` also calls `median(intervals_by_event_type.get(...))` per event type at `src/lib.rs:245-252`. The separate `median_ms` helper at `src/lib.rs:1343-1354` is used for even-length arrays in evaluation scoring.
- Source: https://en.wikipedia.org/wiki/Median
- See also: `./external-references.md`

---

## Z-score anomaly detection
- What: Flag observations more than N standard deviations from the mean as anomalies.
- Why here: TBD
- Gotcha (from code): `detect_anomalies` at `src/lib.rs:532-571` computes population variance (divides by `n`, not `n-1`) and returns early with an empty list when `std_dev == 0.0` (all intervals identical). The default threshold is `2.0` — set in `src/main.rs:537`.
- Source: https://en.wikipedia.org/wiki/Standard_score
- See also: `./external-references.md`

---

## Held-out evaluation / train-test split
- What: Fit a model on the first X% of data, score predictions against the remaining Y%.
- Why here: TBD
- Gotcha (from code): `evaluate_stream` at `src/lib.rs:998-1134` enforces a minimum of 4 events (`src/lib.rs:1004`) and clamps `history_ratio` to `[0.1, 0.9]` (`src/lib.rs:1007`). The HTTP layer adds a second validation in `validate_history_ratio` at `src/main.rs:525-531`; passing a ratio outside `[0.1, 0.9]` via the API returns 400, not a clamp.
- Source: https://en.wikipedia.org/wiki/Training,_validation,_and_test_data_sets
- See also: `./external-references.md`

---

## serde(flatten) for schema-tolerant ingestion
- What: A `#[serde(flatten)]` field absorbs all unrecognized JSON keys into a `Map<String, Value>` at deserialization time.
- Why here: TBD
- Gotcha (from code): `RawEvent.top_level` at `src/lib.rs:33-34` is annotated `#[serde(flatten)]`. `normalize_event` at `src/lib.rs:106-111` iterates `DEFAULT_PROPERTY_FIELDS` and promotes any matching top-level key into `properties` — so senders can put `location` at the top level without nesting it under `properties`.
- Source: https://serde.rs/attr-flatten.html
