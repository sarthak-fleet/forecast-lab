# Forecasting Explainer — methods × datasets × eval gates

A teaching harness: run **industry-standard methods** across **datasets of different
character**, score them with **industry-standard eval gates**, and learn *who wins
where and why*. The punchline the whole thing teaches: **the best forecaster depends
on the data regime — measure it, don't assume it.**

## 1. Methods (industry-standard)

| Method | What it is | Wins when | script |
|---|---|---|---|
| **Naive (last value)** | repeat last period | dense + stationary; the bar everything must beat | all |
| **Seasonal-naive** | same hour/day last cycle | strong periodicity | `run.py`, `run_bike.py` |
| **Moving average** | mean of recent periods | smooth, low-trend | `run.py` |
| **Holt / ETS** | exp-smoothing **with trend** | linear/exponential growth *(GBT can't extrapolate — use this)* | `run_trend.py` |
| **Gradient boosting (LightGBM/HGB)** | trees on lags+calendar+**exogenous** | rich features / exogenous signal exist | `run_ml.py`, `run_bike.py` |
| **Pooled GBT + clusters** | global model, cluster ids | **sparse** cells (borrow strength) | `run_best.py` |
| **Ensemble** | blend naive + model | almost always (anchored downside) | `run_best.py`, `run_online.py` |
| *Probabilistic:* empirical / Poisson / **Negative-Binomial** | the **odds**, not just the point | inventory; NB for over-dispersed counts | `run_quantile.py`, `query.py` |
| *Calibration:* **conformal** (split → **adaptive/online**) | make "τ" actually mean τ coverage | any quantile model; *adaptive* needed under drift | `run_conformal.py` |
| *Decision:* newsvendor | cost ratio → stock quantile | turning forecast into inventory | `run_newsvendor.py` |

## 2. Datasets (regimes)

| Dataset | Character | What it exercises |
|---|---|---|
| **NYC taxi** | rhythmic, dense, **no exogenous** | does ML beat naive on clean periodic demand? (no) |
| **Olist e-commerce** | noisy, sparse, trending, **no exogenous**, 3-axis | sparsity/pooling, granularity dial, calibration |
| **Bike-sharing** | weather + calendar = **exogenous signal**, trend | does signal let ML win? (**yes, +50%**) |
| **Rossmann retail** | store × day + **promo/holiday/weather** | the inventory domain — promo is known-ahead signal (**+15%**) |
| **M5 (Walmart)** | item × store × day + **price/SNAP/events**, **intermittent** (41% zeros) | the full cube — exogenous helps (+20%) but intermittency keeps naive on top |

## 3. Eval gates (industry-standard)

- **Point:** MAE, RMSE, **wMAPE** (well-defined with zeros)
- **Probabilistic:** pinball loss, **coverage / calibration** (does P90 cover 90%?)
- **Decision:** service level / **fill rate**, overstock cost, **regret**
- **Protocol:** temporal split, **walk-forward (rolling-origin)**, no leakage

## 4. Results — who wins where (wMAPE)

| Dataset / grain | Naive | Best model | Winner & why |
|---|---|---|---|
| Taxi (zone×hour) | **0.17** | GBT 0.18 | **naive** — no signal to add |
| Olist 3-axis (fine) | 0.70 | hybrid **0.64** | **pooled GBT +9%** — pooling beats sparse-naive |
| Olist (clustered×monthly) | **0.08** | — | **granularity dial** — 8× by aggregating, no model |
| Bike (hourly) | 0.34 | GBT+weather **0.17** | **ML +50%** — exogenous signal is the lever |
| Olist weekly (trend) | 0.46 | **Holt 0.14** | **trend method wins 3×** — naive lags, GBT caps (can't extrapolate ~1219 vs actual 1610) |
| Rossmann retail (store×day) | 0.18 | GBT+promo **0.15** | **ML +15%** — promo/holiday signal (smaller than bike: strong baseline + promo is a fraction of days) |
| M5 (item×store×day, intermittent) | **0.72** | GBT+exo 0.80 | **naive wins** — exogenous helped GBT +20% but intermittency favors the smooth rate (needs Croston / full M5 recipe) |

## 5. The lessons (the explainer's payoff)

1. **Dense + stationary** (taxi) → naive wins; a model has nothing to add.
2. **Sparse** (Olist fine grain) → **pooling** (clusters + global GBT + ensemble) wins.
3. **Exogenous signal present** (bike) → **ML wins decisively (+50%)**. *Signal is the lever, not model complexity.*
4. **Trends** → Holt/ETS or **log-transform**; tree models **can't extrapolate** beyond training range.
5. **Accuracy is a granularity dial** — Olist 0.70→0.08 purely by coarsening, traded against actionability.
6. **Calibration**: empirical odds where data is rich, **NB (pooled dispersion)** where thin; always validate coverage.
7. **Decision**: newsvendor `CR = Cu/(Cu+Co)` — but validate realized cost; a mis-calibrated forecaster makes the formula over-stock.
8. **Intermittency** (M5, 41% zeros) is its own regime — the smooth rate (item×weekday mean ≈ Croston) beats a per-day count GBT; needs intermittent-demand methods, not features.
9. **Calibration under drift**: vanilla conformal breaks (cal≠test); use **adaptive/online** conformal.

## The one-sentence synthesis

**Signal is always the lever; the regime decides whether it's enough — so match the
method to the regime and measure everything.** Exogenous signal wins where it exists
(bike +50%, Rossmann +15%, M5 +20% relative); pooling wins on sparse; Holt wins on
trends (trees can't extrapolate); the smooth rate wins on intermittent; naive wins
when there's no signal. No single model is best — the *eval gates* tell you which is.

## Visual report

`python3 viz.py` → **`report.html`** — a self-contained dashboard (inline SVG, no
deps) of the six headline results: the exogenous-lift-by-regime, the recsys
leaderboard, the bike signal-isolation, the accuracy dial, the trend-extrapolation,
and conformal calibration. Open it in any browser.

## 6. Run it

```bash
python3 run.py          # taxi 2-axis: naive baselines
python3 run_ml.py       # taxi: GBT vs naive (naive wins)
python3 run_olist.py    # Olist 3-axis: GBT vs naive (naive wins)
python3 run_accuracy.py # the granularity dial (8× swing)
python3 run_best.py     # best defensible hybrid (beats naive on sparse grain)
python3 run_bike.py     # exogenous signal (weather) → ML wins +50%
python3 run_rossmann.py # retail + promo signal → ML +15% (inventory domain)
python3 run_m5.py       # full cube: item×store×day + price/SNAP/events, intermittent
python3 run_trend.py    # Holt/ETS — trends (trees can't extrapolate)
python3 run_quantile.py # calibrated odds (which distribution is honest)
python3 run_conformal.py # conformal calibration: static fails under drift → adaptive
python3 run_newsvendor.py / run_online.py  # cost-optimal stock + adaptive loop
python3 query.py        # the product: (area, product, time) → count + odds
python3 run_clustering.py  # data-driven axis grouping (demand archetypes)
```

Detailed findings: [lessons.md](lessons.md).
