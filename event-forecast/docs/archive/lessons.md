# Lessons Learned

Concrete lessons from building Event Forecast. Stubs mark areas not yet
documented in code or commits.

---

## Forecasting Engine

### Property prediction needs multiple evidence sources, not just the most recent transition

**What happened:** A single transition (previous property value → next property
value) is often sparse, especially for short streams or rare event-type pairs.
Relying only on it produces overconfident predictions when one value dominates a
thin sample.

**What was done:** `predict_property` merges four evidence sources with fixed
weights: event-transition-conditioned distribution (0.55), per-event-type
distribution (0.25), property-value transition (0.15), global marginal (0.05).
This was confirmed as correct in the test
`predicts_configured_categorical_properties` which asserts specific values for
`location`, `service_type`, and `product_type` against the sample stream.

**Lesson:** In a small-data regime, weighted ensemble of evidence sources beats
any single source. The weights (0.55 / 0.25 / 0.15 / 0.05) were chosen to
prioritize the most specific signal without completely ignoring the global prior.
TBD: capture whether these weights were tuned against a validation set or set by
intuition.

---

### Tie-breaking in `top_choice` must be deterministic

**What happened:** Equal transition counts made `predict_next_stream` return
different event types on different runs, making tests flaky and evaluation numbers
unstable.

**What was done:** `top_choice` in `lib.rs` breaks ties by preferring the
lexicographically smallest value (`then_with(|| right.0.cmp(left.0))`). The commit
message "deterministic ties" in `35ff208` confirms this was a deliberate fix.

**Lesson:** Any system that accumulates counts and picks a winner must specify a
stable tie-breaking rule. HashMap iteration order in Rust is not stable across
runs.

---

### Median interval is more robust than mean for sparse streams

**What happened:** A single outlier event interval (e.g., a 6-hour gap introduced
in `anomalies_flag_intervals_far_from_mean` test) would skew a mean-based
timestamp prediction significantly.

**What was done:** `predict_next_stream` uses `median` of per-event-type intervals
with a global median fallback (60,000 ms default). Mean is only used inside
`detect_anomalies` for the z-score baseline, where it is the right statistic.

**Lesson:** Median for central tendency in predicted timestamps; mean for anomaly
z-scores. The choice depends on whether you want robustness (median) or
sensitivity to extremes (mean).

---

## Schema and Data Model

### Normalizing top-level property aliases prevents silent missing data

**What happened:** Callers sending `location` at the top level of an event object
(not nested under `properties`) would have their location silently dropped,
producing zero-confidence property predictions.

**What was done:** `normalize_event` checks `DEFAULT_PROPERTY_FIELDS`
(`location`, `service_type`, `product_type`) and copies them from `top_level`
into `properties` if not already present. Test:
`normalizes_top_level_property_aliases`.

**Lesson:** Accept the most natural shape the caller would reach for, not just
the canonical shape. Silent data loss is worse than a validation error.

---

### Composite `(id, ts)` primary key enables TimescaleDB hypertable without a surrogate

**What happened:** TBD — the reason a composite primary key was chosen over a
standalone UUID primary key is not documented in commits or PRDs.

**Lesson stub:** TBD: document whether the `(id, ts)` composite PK was required
by TimescaleDB hypertable constraints, a deliberate idempotency choice (the `ON
CONFLICT (id, ts) DO NOTHING` in the insert), or both.

---

## API Design

### Horizon validation prevents allocation failure on adversarial inputs

**What happened:** An uncapped `horizon` parameter would let a caller request
10,000,000 prediction steps, growing the prediction vector until memory was
exhausted.

**What was done:** `MAX_HORIZON = 10_000` enforced in `validate_horizon`. The
comment in `main.rs` reads: "Prediction work and memory grow linearly with
`horizon`, so an unchecked request-supplied value can abort the process on
allocation failure."

**Lesson:** For any endpoint that drives proportional work, validate the size
parameter at the boundary, not deep in the library.

---

### `history_ratio` clamping must be enforced at both the library and API layers

**What happened:** Commit `415b3d6` ("reject out of range history ratio") and
`7acb487` ("reject invalid history ratios in api") appear close together, suggesting
the API-layer validation was added after the library-layer clamp was already present.
`evaluate_stream` clamps to `[0.1, 0.9]` internally; the API now returns a 400 if
the caller supplies a value outside that range.

**Lesson:** Library-level clamping is for correctness; API-level validation is for
giving callers a useful error message. Both are needed.

---

## Evaluation Loop

### Build the evaluation harness before building features

**What happened:** The held-out evaluation loop (`evaluate_stream`, `/evaluate`,
`cargo run --bin evaluate`) was added in the early phases (commit `789043f`). The
OSS evaluation doc credits this loop as the reason the team can defer adding `augurs`
until a larger stream exists.

**Lesson:** The evaluation contract (split by `history_ratio`, score event-type
accuracy + property accuracy + timestamp error) made every subsequent feature
checkable. Adding it early meant the Phase 5 work (per-entity models, anomaly
detection) could be evaluated rather than eyeballed.

---

### Small sample data limits what evaluation numbers can tell you

**What happened:** The sample data has 14 events across 4 entities
(noted in `PROJECT_STATUS.md`: "sample data only has 14 events across 4 entities").
Held-out accuracy numbers on 14 events are not statistically meaningful.

**Lesson stub:** TBD: document the first evaluation run on a real production
stream, including event count, event-type accuracy, and median timestamp error.
The OSS evaluation identified this gap explicitly.

---

## Build and Dev Workflow

### The service runs without a database — keep that working

**Lesson:** The optional-DB design (ADR-005) means `cargo test` and inline-event
API calls work without Docker. Every time a new endpoint is added, the
`resolve_events` dual-path must be exercised in both modes. Tests confirm the
inline path; integration tests with Docker confirm the DB path.
TBD: add a CI step that starts TimescaleDB and runs the DB-backed path.
