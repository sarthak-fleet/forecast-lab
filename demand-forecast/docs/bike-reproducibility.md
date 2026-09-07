# Reproducible bike demand evaluation

Run from `demand-forecast`:

```bash
uv sync --frozen --extra dev
uv run --frozen python run_bike.py
uv run --frozen pytest -q
```

The first run downloads 273 KB from the UCI repository and extracts only
`hour.csv`. Both the archive and CSV have pinned SHA-256 checksums; subsequent
runs verify and reuse the CSV. No account, API key, GPU, or paid service is used.
`threadpoolctl`, already present through scikit-learn, is declared directly to
bound the experiment to one CPU thread. Model selection and hyperparameters are
fixed before scoring; this command does not tune on the held-out tail.

## Data and attribution

Hadi Fanaee-T (2013), **Bike Sharing**, UCI Machine Learning Repository,
[doi:10.24432/C5W894](https://doi.org/10.24432/C5W894).
The [UCI dataset page](https://archive.ics.uci.edu/dataset/275/bike+sharing+dataset)
licenses the dataset under [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
It contains Capital Bikeshare rental counts and weather in 2011–2012. It is a
learning proxy, not region/product-level commerce demand.

The experiment derives timestamp-aligned historical demand features and excludes
rows whose required lag observations are missing; it does not fill gaps with zero.

## Forecast protocol

- Sort hourly records chronologically; freeze the first 80% as training data.
- Fit gradient boosting once on training rows. Each scored forecast is one hour
  ahead, with actual demand from previous hours available as time advances.
  Previous held-out observations can therefore feed later predictions. This is
  a rolling-origin evaluation, not a single 145-day-ahead batch forecast.
- Join lag-1, lag-24 and lag-168 by exact timestamp. The source has missing hours,
  so a shift by 24 rows is not necessarily yesterday at the same hour.
- Score every method on the same rows with available lag history.
- Compare persistence, daily and weekly seasonal naive baselines against GBT
  using calendar and past demand. A separate GBT includes **observed weather in
  the target hour**, labelled ORACLE. Those observations would not be available
  before the target hour; this does not measure production weather-forecast lift.
- MAE, RMSE and bias are rentals/hour. wMAPE is total absolute error divided by
  total actual rentals; lower is better. It is not a probability of correctness.

## Measured local receipt — 2026-09-07

Python 3.11.16; frozen lockfile; numpy 2.4.6, pandas 3.0.3,
scikit-learn 1.9.0. The CSV has 17,379 records. The cutoff is
2012-08-07 12:00:00. Training has 13,491 usable hours (412 excluded for lag
history); evaluation has 3,406 usable hours through 2012-12-31 23:00:00
(70 excluded). The models use 500 maximum iterations and random seed 0.

| Method | MAE | RMSE | wMAPE |
| --- | ---: | ---: | ---: |
| Previous-hour persistence | 85.496 | 129.912 | 0.3410 |
| Daily seasonal naive | 78.368 | 131.967 | 0.3126 |
| Weekly seasonal naive | 63.787 | 109.411 | 0.2544 |
| GBT: calendar + past demand | 52.273 | 83.431 | 0.2085 |
| GBT: observed-weather ORACLE | 41.659 | 65.317 | 0.1661 |

Two runs produced byte-identical output files locally:

- `metrics.json`: `193ec8181a3cf15cc3f181befc65f36f899f6945fa453ce248f78c1b5742f9b2`
- `predictions.csv`: `3a941ba66a1054d51f4c8b58135284c0df987c46c52bc9667f9503f540086d53`

Floating-point results can differ across supported Python/platform lockfile branches;
compare metrics with appropriate tolerance when reproducing elsewhere.

The calendar/past-demand model beats the strongest of these naive baselines on
this split. This is one dataset and one cutoff, with no uncertainty interval or
independent tuning holdout; it does not establish general superiority.

`data/bike-evaluation/metrics.json` contains the source, checksum, split, exclusions,
model parameters and scores. `predictions.csv` contains every scored timestamp,
actual count and method prediction so the metrics can be recomputed. Both files
are local outputs and gitignored. `--output PATH` selects another output directory.

## Fresh public setup — 2026-09-07

An unauthenticated download of public commit `15b7130f` was extracted into a
temporary directory without local data or an existing virtual environment.
The three documented commands installed 17 packages, downloaded the pinned
input, completed the full experiment and passed all seven tests. Both output
hashes matched the earlier receipt above. Independently recomputing all five
wMAPE values from 3,406 prediction rows matched their four-decimal reported
values within 0.00005. The terminal table and output provenance were inspected.
See [the fresh-setup receipt](fresh-public-setup-2026-09-07.json).

The public source repository currently has no source-code license; that owner
decision remains pending. The dataset's CC BY 4.0 license is separate. Fresh
setup is now verified, but the source usage-rights gate has not been resolved.
The temporary source, environment and downloaded dataset were cleaned after
retaining this receipt.

## What remains unqualified

Other datasets and historical tutorial percentages have not been rerun here.
Multi-step prediction, weather forecast availability, daylight-saving ambiguity,
multiple temporal folds, domain-specific customer data, and operational decision
costs need explicit evaluation before product claims. No website, deployment or
commercial forecasting service was qualified. The portfolio hold remains intact.
