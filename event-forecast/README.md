# Event Forecast

Event Forecast predicts the next events in a stream and the likely properties on
those events: location, service type, product type, or any other categorical
field supplied in `properties`.

The first version is a Rust + Rocket web service backed by TimescaleDB. The
prediction model is still a transparent baseline, not a black-box ML model. It
learns from historical event sequences using:

- event-type transitions
- median time between events
- property transitions between neighboring events
- property distributions conditioned on event-type transitions

This makes the output debuggable before the project adds heavier forecasting or
spatiotemporal models.

## Current Input Shape

```json
{
  "id": "evt_001",
  "ts": "2026-06-03T09:00:00.000Z",
  "event_type": "order_created",
  "entity_id": "order_1",
  "properties": {
    "location": "koramangala",
    "service_type": "delivery",
    "product_type": "grocery"
  }
}
```

Top-level aliases like `location`, `service_type`, and `product_type` are also
accepted and normalized into `properties`.

## Run

```bash
cargo test
cargo run
```

Rocket starts on `http://127.0.0.1:8088`.

### Predict From Inline Events

The service can predict from request-local events without a database:

```bash
curl -s http://127.0.0.1:8088/predict \
  -H 'content-type: application/json' \
  -d '{"events":[...],"horizon":5,"summary":true}'
```

### Use TimescaleDB

Start local TimescaleDB:

```bash
docker compose up -d
export DATABASE_URL=postgres://event_forecast:event_forecast@localhost:54329/event_forecast
psql "$DATABASE_URL" -f migrations/0001_timescale_events.sql
cargo run
```

Ingest events:

```bash
curl -s http://127.0.0.1:8088/events \
  -H 'content-type: application/json' \
  -d '{"stream_id":"orders-demo","events":[...]}'
```

Predict from stored stream:

```bash
curl -s http://127.0.0.1:8088/predict \
  -H 'content-type: application/json' \
  -d '{"stream_id":"orders-demo","horizon":10,"summary":true}'
```

### Aggregate Report

`POST /report` returns a digest of the next forecasted window: top event
types, mix of locations, service types, and product types, mean
confidence, and any low-confidence steps.

```bash
curl -s http://127.0.0.1:8088/report \
  -H 'content-type: application/json' \
  -d '{"events":[...],"horizon":6}'
```

### Heatmap

`POST /heatmap` aggregates observed and predicted events into
`(location bucket, time window)` rows, including deltas and the
hottest/cooling buckets. Default bucket field is `location`, default
window is 30 minutes.

```bash
curl -s http://127.0.0.1:8088/heatmap \
  -H 'content-type: application/json' \
  -d '{"events":[...],"horizon":6,"window_minutes":30}'
```

### Anomaly Detection

`POST /anomalies` flags events whose inter-arrival interval deviates
beyond `z_threshold` standard deviations from the stream's mean.

```bash
curl -s http://127.0.0.1:8088/anomalies \
  -H 'content-type: application/json' \
  -d '{"events":[...],"z_threshold":2.0}'
```

### Numeric Property Forecasting

`POST /numeric` forecasts numeric properties (price, dwell time,
distance, etc.) using the median observed value per event type.

```bash
curl -s http://127.0.0.1:8088/numeric \
  -H 'content-type: application/json' \
  -d '{"events":[...],"horizon":5,"numeric_fields":["amount"]}'
```

### Decision Report

`POST /decision-report` compares the previous half of the stream
against the current half, flags hot/cooling zones and mix shifts,
writes a short narrative, and attaches held-out forecast quality when
the stream has at least four events.

```bash
curl -s http://127.0.0.1:8088/decision-report \
  -H 'content-type: application/json' \
  -d '{"events":[...],"horizon":6,"history_ratio":0.6}'
```

### Action Report (decision surface)

`POST /action-report` returns the map heatmap rows, next-stream
predictions, decision narrative, and held-out forecast quality in one
payload. The explorer UI uses this endpoint.

```bash
curl -s http://127.0.0.1:8088/action-report \
  -H 'content-type: application/json' \
  -d '{"events":[...],"horizon":6,"history_ratio":0.6}'
```

Fixture-backed artifact (written by `cargo test action_report_fixture`):

`artifacts/action-report-sample.json`

### Per-Entity Predictions

`POST /predict` and `POST /numeric` accept an optional `entity_id`.
When set, the service fits a model on just that entity's events
(falling back to the global model when sparse), so a single
order/session/user can have its own transition profile.

### Explorer UI

`GET /explorer` serves a single-file Leaflet map that calls
`/action-report` for heatmap, predictions, decision narrative, and
held-out forecast quality. Click **Load sample stream** or POST your
own events into the textarea. Add `?demo=1` to the URL to autoload the
sample on page load.

### Held-Out Evaluation

`POST /evaluate` splits a stream by `history_ratio` (default 0.6), fits
the model on the history segment, predicts forward, and scores the
prediction against the held-out suffix. Metrics include event-type
accuracy, per-field property accuracy (`location`, `service_type`,
`product_type`), timestamp error (mean and median ms), uncertainty
(mean event and property confidence), and per-step scores.

```bash
curl -s http://127.0.0.1:8088/evaluate \
  -H 'content-type: application/json' \
  -d '{"events":[...],"history_ratio":0.6}'
```

Run the same evaluation offline against the fixture-backed sample
stream (no database or running server required):

```bash
cargo run --bin evaluate -- tests/fixtures/sample-stream.json
```

Write a metrics JSON report to a file:

```bash
cargo run --bin evaluate -- \
  tests/fixtures/sample-stream.json \
  --history-ratio 0.6 \
  --output /tmp/event-forecast-metrics.json
```

The default input path is `tests/fixtures/sample-stream.json`. You can
also pass `data/sample-events.json` or any JSON array in the same event
shape.

### Load Sample Data Into TimescaleDB

The `load_events` binary reads a JSON event file and inserts it via
`DATABASE_URL`. Useful for seeding `data/sample-events.json` without
juggling curl payloads.

```bash
cargo run --bin load_events -- data/sample-events.json --stream-id orders-demo
```

## What It Predicts

The prediction output is a sequence:

```json
{
  "event_type": "driver_assigned",
  "expected_ts": "2026-06-03T09:04:00.000Z",
  "confidence": 0.83,
  "properties": {
    "location": {
      "value": "koramangala",
      "confidence": 0.71
    }
  },
  "why": {
    "event_type": "transition from order_created"
  }
}
```

## Product Direction

This should become a stream-to-decision engine:

1. Ingest an event stream.
2. Predict the next stream.
3. Predict important properties on each future event.
4. Show where the stream is likely to move geographically or behaviorally.
5. Produce a short "what changed / what to do next" report.

Examples:

- delivery demand by neighborhood and service type
- user journeys through a product funnel
- short-link or link-in-bio traffic paths
- product/category demand shifts
- operational queue movement

See [ROADMAP.md](ROADMAP.md) for the staged plan, including the geographic
time-based heatmap direction.

## Non-Goals

- No production deployment yet.
- No paid APIs or external services.
- No PII ingestion.
- No opaque model until the baseline is measurable.

## Verification

```bash
cargo fmt --check
cargo test
cargo check
```
