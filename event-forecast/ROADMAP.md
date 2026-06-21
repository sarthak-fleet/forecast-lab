# Roadmap

Event Forecast should become a stream prediction system for operational and
behavioral event data. The core loop is:

1. Ingest timestamped events.
2. Predict the next events in the stream.
3. Predict important properties on those future events.
4. Aggregate predictions into maps, timelines, and decision reports.

## Phase 1: Stream Forecasting Core

Goal: make the prediction contract real and measurable.

- Store timestamped events in TimescaleDB.
- Predict the next event sequence from recent history.
- Predict categorical properties such as location, service type, and product
  type.
- Return confidence and simple explanations with every forecast.
- Add held-out stream evaluation so model changes are judged by accuracy, not
  vibes.
- Add a loader command for sample and exported event files.

Success proof: a stored stream can be split into history and future, forecasted,
and scored automatically.

## Phase 2: Geographic Time Heatmap

Goal: turn event forecasts into a spatial-temporal surface.

- Normalize location properties into a stable location dimension.
- Support geohash, H3, or lat/lng-backed buckets for real map rendering.
- Aggregate observed and predicted events by time window, location, service
  type, and product type.
- Add heatmap endpoints for:
  - observed demand
  - predicted demand
  - forecast deltas
  - uncertainty
- Support replaying a stream over time so movement patterns are visible.

Success proof: for a chosen time range, the API can answer "where demand is
likely to move next" with location buckets and confidence.

## Phase 3: Explorer UI

Goal: make the system inspectable by operators and builders.

- Add a map plus timeline view.
- Toggle observed vs predicted heat.
- Filter by service type, product type, event type, and stream/entity.
- Show the likely next stream beside the map.
- Surface uncertainty instead of hiding it.
- Add saved report snapshots for "what changed since last window."

Success proof: a user can inspect a forecast without reading JSON.

## Phase 4: Operational Intelligence

Goal: move from prediction to useful decisions.

- Detect emerging hot zones and cooling zones.
- Explain service/product mix shifts by location.
- Forecast capacity pressure by neighborhood, route, queue, or cohort.
- Compare actuals against prior forecasts.
- Generate a short decision report:
  - what is likely next
  - where it is likely to happen
  - which service/product mix matters
  - where confidence is weak

Success proof: the system can produce an operator-readable daily or live report
from event streams.

## Phase 5: Stronger Models

Goal: upgrade prediction quality only after the baseline is measurable.

- Add per-entity and per-segment transition models.
- Add seasonal/time-of-day features.
- Add numeric property forecasting for quantities like price, duration,
  distance, or dwell time.
- Add anomaly and drift detection.
- Evaluate sequence models or gradient boosting if the baseline plateaus.
- Keep the API contract stable while swapping model internals.

Success proof: a stronger model beats the transparent baseline on held-out
streams without making the output impossible to explain.

## Parked Possibilities

These are promising follow-ons now that the durable PRDs below have shipped:

- [Configurable location source](docs/archive/prd-configurable-location-source.md) — shipped
- [Stream replay and playback](docs/archive/prd-stream-replay-and-playback.md) — shipped
- [Alerts and decision rules](docs/archive/prd-alerts-and-decision-rules.md) — shipped
- Simulated interventions such as "what if we add supply here."
- Route prediction for logistics.
- Product-category demand radar.
- User journey maps for apps and funnels.
- Embedding search over past stream shapes.
- Multi-tenant datasets and private workspaces.
