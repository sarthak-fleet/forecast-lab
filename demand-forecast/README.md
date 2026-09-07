# demand-forecast

**Spatiotemporal demand forecasting** — predict order volume per (area, time-window)
so you can pre-position inventory / stock / riders. The Swiggy/Zomato supply-demand
problem: *which areas will the orders come from next, and how many?*

This is the eval-first rebuild of what `event-forecast` was *meant* to be — its
next-event model collapsed to a constant on real data; the real target was always
**demand volume per (zone, time-bucket)**, not next-event prediction.

## Proxy data

NYC yellow-taxi pickups (Jan 2024, ~3M rows, reused from `event-forecast/data/`).
**Pickups per zone per hour ≡ orders per area per window** — same problem shape.

## The eval-first ladder

The bar to beat is **seasonal-naive** (historical average by zone × hour-of-day ×
day-of-week) — demand is dominated by daily/weekly cycles, so this dumb average is
strong. Everything must beat it on a **temporal backtest** (train on early weeks,
predict the held-out last week), scored by MAE / RMSE / **wMAPE**.

| Tier | Method | Status |
|---|---|---|
| 0 | Global mean, Zone mean | baseline floor |
| 0 | **Seasonal-naive** (zone×dow×hod), Last-week (lag 168h) | the real bar |
| 1 | **LightGBM** on space/time features (lags, calendar) | next — the workhorse |
| 2 | Classical TS (Holt-Winters/Prophet) per zone | later |
| 3 | Spatiotemporal DL (ST-GNN, TFT) | only if 1–2 plateau |

Core question (same as always): **does the ML beat seasonal-naive?**

## This is now an explainer tool

It grew into a forecasting **explainer**: industry-standard **methods** ×
**datasets** of different character (taxi · Olist · bike-sharing) × industry-standard
**eval gates** — showing *who wins where and why*. The headline lesson: the best
forecaster depends on the data regime (naive wins dense/stationary; pooling wins
sparse; **exogenous signal wins decisively where it exists — bike +50%**; trends need
Holt/log; accuracy is a granularity dial).

The historical matrix and headline percentages above are not a fresh forecast
accuracy guarantee. The corrected, reproducible bike entry point uses exact
hourly lags and labels target-hour observed weather as an oracle. Its
[protocol and measured receipt](docs/bike-reproducibility.md) supersede older
bike headline comparisons.

See [`docs/EXPLAINER.md`](docs/EXPLAINER.md) for the historical matrix + run commands;
[`docs/lessons.md`](docs/lessons.md) is the detailed findings log.

## Run

```bash
cd demand-forecast
uv sync --frozen --extra dev
uv run --frozen python run_bike.py
uv run --frozen pytest -q
```
