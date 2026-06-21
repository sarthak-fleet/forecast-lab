# Lessons — 2-axis demand (time × place), NYC taxi

Grounded in `run.py` + `run_ml.py`, 2024-01 taxi, 62 zones, hourly, last-7-days
temporal backtest. 2026-06-21.

## The bar, and the ML attempt

| Model | MAE | wMAPE | note |
|---|---|---|---|
| **LastWeek (lag 168h)** | **11.26** | **0.170** | naive — repeat same hour last week |
| GBT (HistGradientBoosting) | 12.23 | 0.184 | calendar + lag168 + seasonal-mean feature |
| SeasonalNaive (zone×dow×hod) | 12.73 | 0.192 | naive — historical average |
| ZoneMean | 41.0 | 0.617 | ignores time |
| GlobalMean | 62.7 | 0.944 | one number everywhere |

## What we learned

1. **Seasonality is the dominant signal.** Adding time structure to a zone average
   cuts error ~3× (wMAPE 0.62 → 0.17). The dumb seasonal/lag baselines land within
   ~17%. Strong bar — the demand-forecasting mirror of "beat popularity."

2. **Gradient boosting could NOT beat "repeat last week"** (0.184 vs 0.170). Same
   shape as recsys (Markov beat ALS/BPR): the simple baseline wins.

3. **Why it's not a model problem — it's a signal problem.** The GBT only had
   calendar + lag features, which the naive methods *already encode*. ML beats
   naive only when fed signal the baseline can't see — **weather, holidays,
   events, promotions, cross-zone spillover** — or on **anomalous periods** where
   "same as last week" breaks. Our test week was a normal week → no room for ML.

4. **All baselines under-predict the test week** (negative bias) — late Jan ran
   hotter than the training weeks. For inventory that = systematic understock; a
   trend/level term or more-recent lag helps.

## Not a final verdict (the SASRec caution)

A fair-shot ML rematch would add: `lag336`/trend, a **holiday flag** (Jan 1, MLK),
and a test window that contains an anomaly. But the structural point stands: on
stable, strongly-seasonal demand with only calendar features, the naive lag is
near-optimal.

## Where the value actually is

Not model complexity. Two levers:
- **Exogenous features** — weather, holidays, events, promos (the deviations from
  "same as last week").
- **Granularity / bucketing** — dayparts (time), region-groups (place), product-
  types (item). This is where the real judgment lives, and it's mostly a 3-axis
  concern. Taxi at zone×hour is already dense; the item axis forces it.

## 3-axis result (Olist: region-group × product-type × week)

Built `olist.py` + `run_olist.py`. 6 regions × 11 product-types × 87 weeks,
**94% non-zero** (mean 17/cell) — bucketing made it dense/tractable, exactly as
intended. 1-step-ahead weekly backtest, last 8 weeks held out.

| Model | MAE | wMAPE | bias |
|---|---|---|---|
| **LastWeek (lag1)** | **8.94** | **0.415** | +2.0 |
| GBT (Poisson, trend+lags+seasonality) | 9.68 | 0.450 | −2.9 |
| MA4 | 10.27 | 0.477 | +1.9 |
| GroupMean | 10.91 | 0.507 | −4.8 |

- **Bucketing works** — no sparsity collapse; the 3-axis problem is solvable.
- **Naive wins a 3rd time.** GBT (even with Poisson loss + trend + cross-series
  pooling) lost to "repeat last week." Across recsys + demand, 2 & 3 axes, 2
  domains, naive last-value beats the fancy model without exogenous signal — the
  M-competition / Makridakis consensus.
- **Predictability is domain-bound:** e-commerce wMAPE 0.42 vs taxi 0.17. Best
  *possible* forecast here is only ~58% accurate; that's the data, not the model.
- Caveat: GBT lost modestly and had NO exogenous features (weather/promos/holidays)
  — the one thing that could flip it. Burden of proof now on ML.

## Quantile / service-level layer (the inventory objective) — built

`run_quantile.py`: forecast a quantile of demand per cell, "stock" it, measure
fill rate + overstock + calibration. Olist 3-axis, last 8 weeks (528 test cells).

**Calibration — achieved fill rate (want ≈ target τ):**
| method | τ=0.5 | τ=0.8 | τ=0.9 | τ=0.95 | pinball |
|---|---|---|---|---|---|
| **Naive (lag1 + residual spread)** | 0.58 | 0.81 | 0.88 | 0.92 | **3.06** |
| Poisson (around lag1) | 0.58 | 0.74 | 0.80 | 0.83 | 3.10 |
| QuantileGBT (ML) | 0.44 | 0.58 | 0.66 | 0.76 | 3.32 |

- **Calibration is the make-or-break metric for inventory.** QuantileGBT aiming
  for 95% delivers only **76%** fill — false confidence, the worst inventory
  failure. Naive residual method is best-calibrated + best pinball → **naive wins
  a 4th time**, now on the *distribution*.
- **Poisson under-covers at the top** (95%→83%) = textbook over-dispersion (demand
  variance > mean). A negative-binomial would fix the spread.
- **Tradeoff is now a dial:** naive 88% service = ~11.6 overstock units/cell; 92% =
  ~15.2. Fat-tailed demand makes high service expensive → the right target is the
  newsvendor optimum (marginal overstock cost = marginal stockout cost), not 100%.
- **ML caveat (discipline):** QuantileGBT's under-coverage is fixable with
  **conformal prediction** (recalibrate so 95% means 95%). Not "ML can't" — "ML
  quantiles are untrustworthy until conformalized." Naive is already calibrated +
  simpler → ship it.

## Newsvendor / cost-optimization layer — baked in

`demand/inventory.py` + `run_newsvendor.py`: cost ratio (Cu understock, Co overstock)
→ optimal service level **CR = Cu/(Cu+Co)** → per-cell stock quantity. Same demand
model auto-adjusts: perishable (1:3)→CR 0.25 (stock less); high-margin (4:1)→CR 0.80
(stock more).

**Validation caught the formula failing.** Sweeping τ for the perishable case
(formula CR=0.25), realized cost keeps *dropping* below τ=0.25 (min at ≤0.10):
the elegant newsvendor formula **over-stocks** because it assumes calibrated
quantiles, and the naive forecaster **over-covers at low τ** (aim 50%→get 58%).
Lesson (4th flavor): the textbook-optimal formula is only as good as the
calibration under it — pick τ from the **realized-cost curve** (data-driven), or
calibrate quantiles first, then apply CR. Don't trust the formula blindly.

## "Improves with time" — walk-forward + online feedback loop

`run_online.py`: each week, fit on all history, recalibrate stocking quantile τ
from realized cost on a trailing window, forecast + stock, observe, roll forward.
Perishable (Cu=1,Co=3, formula CR=0.25), 16 rolling weeks:

- **Adaptive (online τ) beats the static formula 7%** (13.63 vs 14.72 cost/cell),
  learning τ≈0.15 — it **auto-corrected the over-stocking** the formula caused.
  The decision layer self-heals its calibration by watching outcomes. ✓
- **Raw cost rose over the span** (first half 11.88 → second 15.38) — but that's
  demand non-stationarity (bigger/noisier later weeks), NOT the model degrading.
  → raw cost-over-time is a **confounded** "improvement" metric; the honest one is
  **regret** (cost above in-hindsight optimum) or the adaptive-vs-static gap.
- τ loop is **jittery** (8-wk window) — smooth it (EMA/Bayesian).
- Improvement is in **calibration/adaptation, not the point forecast** (naive-lag
  saturates) — the system gets the *decision* right over time, not the prediction.

Architecture for an improving system: walk-forward + online recalibration +
drift-adaptation (down-weight stale data — the non-stationarity confound proves
you'll need it). Measure with regret, not raw cost.

## Clustering (data-driven axis grouping) + hardened adaptive loop

**Clustering** (`demand/clustering.py`, `run_clustering.py`) — KMeans on normalized
demand profiles, replacing "top-N + other":
- **Products → 6 archetypes by shape.** Found a **holiday-gift cluster** (toys,
  perfumery, consoles, fashion-bags — peaks Nov) distinct from steady home/everyday
  categories. Interpretable + inventory-relevant (gifts need Nov pre-stocking).
- **Locations → 4 archetypes.** Separated the SP/RJ/MG/RS/PR/SC bloc (92k orders)
  from smaller northern states with different seasonal timing.
- **Time → 3 regimes.** "Peak" regime captured the Black Friday weeks (2017-11-20/27).

**Hardened online loop** (`run_online.py`): added regret tracking, EMA-smoothed τ,
drift down-weighting. Results (perishable Cu=1 Co=3, 16 wks):
- ✓ EMA smoothed τ (std 0.028, no jitter); ✓ feedback loop beats static formula on
  cost (13.99<14.50) AND regret (2.62<3.13).
- ✗ **Does NOT improve with time** — regret *rose* back-half (1.78→3.46) even after
  de-confounding demand size. The loop **adapts but doesn't compound**: naive
  forecast saturates (more data ≠ sharper), demand is non-stationary (+ Olist tail
  thins near cutoff), so trailing-learned τ goes stale.

**Meta-lesson:** "improves with time" is an *architecture* you can build, but whether
it actually improves is decided by the data, not the machinery. Genuine improvement
needs **new signal** (exogenous: weather/promos/holidays, the Nov-gift regime) or a
**more stationary/higher-volume domain** — not more of the same data.

## Accuracy is a DIAL, not a ceiling (`run_accuracy.py`)

Turning the two highest-ROI dials (time aggregation + clustering the place/product
axes) on naive last-period forecasting:

| place × product | time | mean/cell | wMAPE |
|---|---|---|---|
| state × category | weekly | 0.6 | **0.699** |
| state × category | monthly | 2.5 | 0.345 |
| clustered × clustered | weekly | 32 | 0.339 |
| **clustered × clustered** | **monthly** | 140 | **0.083** |

- **8× accuracy gain (0.70 → 0.08), zero new data, zero modeling.** Each dial ~halves
  error; together they compound. Mechanism is in mean/cell (0.6 → 140): coarser/denser
  cells average out noise. Ensemble (last+MA blend) was marginal — granularity and
  clustering are the real ROI.
- **Bought with aggregation = costs actionability.** 92%-accurate forecast is
  "region-bloc × product-archetype × month" — maybe too blunt to stock SKUs weekly.
  Finest/most-actionable grain (state×category×week) is least accurate (~30%).
- **Reframe: accuracy ↔ actionability tradeoff.** Pick the coarsest grain the
  inventory decision can use. ML + exogenous signal only earn their keep BELOW that —
  at fine actionable grain (the ~30% regime) where aggregation can't help.

## The query surface (`query.py`) — the product

Select (area, product, time) → **expected count + odds + stocking recommendation**,
read from each cell's own demand history (optionally a seasonal slot):

```
SP / health_beauty / any week
  expected ~43/wk (median 32); 80% of weeks in [15,83]; stock 83 to cover ~90%.
RS / perfumery / any week
  expected ~1.7/wk; 80% in [0,3]; stock 3 (covers 92%).
```

- Odds **self-scale** (dense cell wide & informative, thin cell narrow, dead cell ~0)
  because they're the cell's empirical distribution — calibrated by construction.
- **Self-reports calibration**: seasonal slots (Nov) only met their "90%" target ~75%
  of weeks — few sample weeks → noisy quantile. The surface flags when to distrust it.
- Thin seasonal cells are exactly where **clustering-pooling** (borrow the gift
  archetype's Novembers) and **exogenous signal** would earn their keep.

Ties the whole system together: clusters = the axes you select; query = count+odds;
realized coverage = honest confidence; stock rec = the decision.

## The best *defensible* predictor (`run_best.py`)

At the fine/actionable grain (state×category×week) where naive is weak, the
best-effort hybrid finally **beats naive**:

| Model | wMAPE |
|---|---|
| **Best hybrid (GBT + lag ensemble)** | **0.639** (−9% vs naive) |
| GBT (pooled, 16 features) | 0.675 |
| LastWeek (lag1) — naive | 0.699 |
| SeasonalNaive | 0.745 |

First model to beat naive on demand all session — because at the sparse fine grain
the **pooled GBT borrows strength across cells** (cluster ids + calendar + global
fit) where naive lag1-of-zeros can't, and the **ensemble with the naive anchor**
beats the raw GBT (M5 lesson).

**"Best defensible" = this hybrid**, because every claim survives scrutiny: (1) beats
baseline on held-out backtest by a measured margin; (2) interpretable (correction on
an explainable backbone + GBT importances); (3) ensemble-anchored → bounded downside
(can't do much worse than naive); (4) calibratable odds (NB/quantile, validated vs
realized coverage); (5) not over-engineered. A black-box deep net might gain a point
but you couldn't *defend* it. Recipe: naive backbone + pooled-GBT correction +
ensemble + calibrated NB odds, at the actionable grain. Ceiling is data-bound; only
exogenous signal moves it materially further (validated against this same baseline).

## Exogenous signal finally wins (`run_bike.py`)

Bike-sharing (hourly demand WITH weather + calendar) — the regime Olist/taxi lacked:

| method | wMAPE | vs naive |
|---|---|---|
| Naive (lag-1) | 0.342 | — |
| SeasonalNaive (lag-24/168) | 0.31–0.32 | +6–8% |
| GBT (calendar only) | 0.218 | +36% |
| **GBT (calendar + weather)** | **0.170** | **+50%** |

**First decisive ML win all session — because the signal exists here.** Isolation:
naive→seasonal +8% (periodicity), →calendar-GBT +36% (hour×weekday structure),
→+weather +50% (weather adds ~14 pts). Confirms the thesis with one number: **the
lever is signal, not model complexity.** Olist/taxi had none → naive won; bike has
weather+calendar → ML wins by half. Best wMAPE 0.17 ≈ taxi's 0.17 (predictable
domains) vs Olist 0.42 (noisy) — predictability is domain-bound.

> See [EXPLAINER.md](EXPLAINER.md) for the full methods × datasets × eval-gates matrix.

## Trend regime — Holt/ETS added (`run_trend.py`)

- **Trend-dominated (Olist 2017→2018 growth):** Holt (level+trend) **wins 3×** —
  wMAPE 0.144 (predicts ~1444) vs naive 0.458 (lags, flat ~873) vs GBT-time 0.255
  (**caps ~1219 — trees can't extrapolate** past max-observed; actual 1610).
- **Seasonal-reversal (bike winter tail):** Holt-Winters ≈ naive (weekly term can't
  see annual cycle); **GBT+weather wins (0.189)**. And **log Holt-Winters EXPLODED**
  (wMAPE 5.97) — multiplicative trend extrapolation compounds dangerously over long
  horizons (use damped/additive/short-horizon).
- Lesson: **match method to regime** — trend methods only shine when a trend
  actually dominates; otherwise exogenous+ML wins.

## Calibration gate — conformal (`run_conformal.py`)

- **Vanilla split conformal FAILED under drift** — pushed coverage the wrong way
  (target 0.9 → 0.80; worse at τ=0.8). Cause: cal window ≠ test window (Olist
  trended up) → exchangeability broken. A real, named limitation.
- **Adaptive/online conformal holds target** (0.89 vs target 0.90; fixed = 0.84) by
  re-estimating the correction from recent data — same mechanism as the walk-forward
  loop. Lesson (again): the elegant guarantee needs the online variant under
  non-stationarity.

## Retail + promo regime (`run_rossmann.py`) — the inventory domain

Rossmann (1115 stores × daily sales + promo/holiday/weather, no-auth via fast.ai),
6-week backtest: SeasonalNaive 0.177 → GBT(store+calendar) 0.164 → **GBT(+promo+holidays)
0.150 (+15%)**. Exogenous (promo) wins in retail too — the inventory-relevant edge
(know the promo calendar → forecast the spike). But +15%, not bike's +50%: retail's
seasonal baseline is already strong + promo is a fraction of days. **Refined lesson:
exogenous signal is always the lever; its *magnitude* = how much it drives demand ÷
how good the baseline already is.** (bike weak-baseline/weather-driven +50%; Rossmann
strong-baseline/promo +15%; Olist/taxi no-signal 0%.)

## Intermittency regime — M5 capstone (`run_m5.py`)

M5 Walmart (item × store × day + price/SNAP/events), CA_1 FOODS, 41% zero-days,
last 28 held out: SeasonalNaive **0.719** (wins) · GBT(item+calendar) 1.001 ·
GBT(+price+SNAP+events) 0.800. **Exogenous helped the GBT +20% relative** (lever
confirmed) but the smooth item×weekday rate (≈ Croston) still won — **intermittency
is its own regime**; a per-day count GBT over-reacts to 0/1/2 noise. Caveat: vanilla
GBT (no lags/tweedie/ensemble — the real M5-winning tricks); the point is *regime
matters as much as signal*, not "ML can't win M5."

## Session verdict

Complete vertical slice: forecast → stock a quantile → measured service level +
overstock. Across recsys + demand (2-axis, 3-axis, distribution), **the simple,
well-calibrated method beat the fancy one every time** without exogenous signal.
The value was in measuring the right thing (calibration, fill rate, the tradeoff),
not model complexity. Next real lever for accuracy = exogenous features
(weather/promos/holidays); next for inventory = newsvendor cost optimization +
negative-binomial spread.
