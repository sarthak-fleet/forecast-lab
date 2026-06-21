# OSS Integration Evaluation

Last updated: 2026-06-09

## Scope

Evaluate OSS libraries that could move Event Forecast beyond the transparent
transition/median baseline while keeping the Rust + Rocket + TimescaleDB service
explainable and testable.

## Shortlist

| Candidate | Source | Fit | Cost | Decision |
| --- | --- | --- | --- | --- |
| augurs | https://docs.rs/augurs and https://docs.augu.rs/ | Best fit. Rust-native time-series toolkit with forecasting, outlier detection, clustering, seasonality, and changepoint detection. MIT/Apache on docs.rs. | Medium: needs adapter from timestamped event streams into numeric per-series inputs. | Recommended first spike once a larger stream exists. |
| smartcore | https://github.com/smartcorelib/smartcore | Rust ML library for classical models. Useful for categorical property prediction and anomaly baselines. | Medium: model choice and feature engineering are on this repo. | Watchlist after augurs. |
| linfa | https://github.com/rust-ml/linfa | Rust ML framework with clustering/classification primitives. | Medium/high: broader ML framework than current baseline needs. | Watchlist for property/anomaly experiments. |
| Polars | https://github.com/pola-rs/polars | Strong Rust dataframe/query engine for feature extraction and offline evaluation reports. | Medium: useful only if data prep becomes the bottleneck. | Park until larger evaluation datasets exist. |
| Apache DataFusion | https://github.com/apache/datafusion | SQL/query engine for analytics over event exports. | High for current TimescaleDB-backed service. | Park; TimescaleDB is enough for now. |
| TimescaleDB toolkit path | https://github.com/timescale/timescaledb | Database-adjacent time-series features and continuous aggregates fit the current storage choice. | Low/medium but production DB semantics matter. | Consider query-level aggregation before adding a second storage engine. |
| Lag-Llama | https://github.com/time-series-foundation-models/lag-llama | Strong probabilistic forecasting research direction. | High: Python/deep-learning stack and rewrite risk. | Research only; not a near-term dependency. |

## Decision

Do not add a forecasting dependency in this pass. The product still needs a
larger real event stream before model sophistication matters. When that stream
exists, start with a fixture-backed `augurs` spike because it is Rust-native and
can be evaluated beside the existing transparent baseline.

## Suggested Implementation Slice

1. Add a benchmark fixture with at least a few hundred events across multiple
   entities, locations, and event types.
2. Convert event counts or numeric properties into one or more `augurs` series.
3. Compare timestamp error, event-type accuracy, anomaly recall, and
   explanation readability against the current baseline.
4. Keep the existing transparent baseline as the fallback until the new model
   wins on held-out reports.

## Verification

Docs-only evaluation in this pass. Run:

```bash
cargo test
```
