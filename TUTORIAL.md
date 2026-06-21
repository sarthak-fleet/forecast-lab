# forecast-lab — Learning Tutorial

A guided path through the repo: **run it in this order, observe the result, learn
the lesson.** The reference docs are the *what* ([recsys roadmap](recsys-lab/docs/learning/roadmap.md),
[forecasting EXPLAINER](demand-forecast/docs/EXPLAINER.md)); this is the *order to
walk it* and why each step matters.

**Setup:** Python + numpy/pandas/scipy/scikit-learn (+ torch for SASRec). Datasets
auto-download via the run scripts; nothing else to install.

## 0. The one idea (read first)

**Eval-first.** Never trust a model that doesn't beat the dumb baseline on a
held-out split. The through-line you'll watch prove itself again and again:
**signal is the lever; the regime decides whether it's enough.**

## Part A — Recommenders (`cd recsys-lab`)

1. **`python3 run.py`** → popularity · item-KNN · ALS · BPR · Markov · SASRec on
   MovieLens-1M. *Observe:* everything beats popularity (personalization works);
   **order dominates** — the sequence models (Markov, SASRec) crush the order-blind
   MF/KNN. *Lesson:* what you watched *last* beats everything you watched, unordered.
2. *Watch the trap:* SASRec lands #1 — **but only after proper training** (undertrained,
   it came dead last). *Lesson:* tuning rigor cuts both ways; the eval catches a
   fancy model that's *under*-trained, not just baselines that are under-tuned.
3. **`python3 recommend.py`** → real top-10 lists. *Lesson:* good recommendations
   ≠ hitting the one held-out movie — the metric is deliberately conservative.
   → deeper: [roadmap.md](recsys-lab/docs/learning/roadmap.md) · [metrics.md](recsys-lab/docs/learning/metrics.md)

## Part B — Forecasting (`cd demand-forecast`)

The loop is **forecast → distribution → decision**, each measured vs. naive on a
temporal backtest.

4. **`run.py`** → the seasonal-naive bar. **`run_ml.py` / `run_olist.py`** → does
   GBT beat it? *No, without signal* (naive wins). *Lesson:* model complexity isn't
   the lever.
5. **`run_accuracy.py`** → the **granularity dial**: 8× accuracy (wMAPE 0.70→0.08)
   purely by aggregating. *Lesson:* accuracy is a *choice*, traded against actionability.
6. **`run_trend.py`** → Holt extrapolates a trend; **trees structurally cannot**
   (they cap at the max they saw). *Lesson:* match the method to the regime.
7. **`run_bike.py`** (weather, +50%) · **`run_rossmann.py`** (promo, +15%) ·
   **`run_m5.py`** (intermittent, naive holds). *Lesson:* **signal is the lever**,
   and its size = how much it drives demand ÷ how good the baseline already is.
8. **`run_quantile.py` · `run_conformal.py` · `run_newsvendor.py` · `run_online.py`**
   → calibrated *odds*, then a cost-optimal stocking decision, then an adaptive loop.
   *Lesson:* the elegant formula (newsvendor, conformal) is only as good as its
   calibration — and needs the *online* variant under drift.
9. **`python3 query.py`** → the product surface: pick **area × product × time → get
   a count + the odds + how much to stock.**
10. **`python3 viz.py && open report.html`** → the whole story in six charts.
    → deeper: [EXPLAINER.md](demand-forecast/docs/EXPLAINER.md) (the full matrix) ·
    [lessons.md](demand-forecast/docs/lessons.md) (every finding)

## Part C — Where it began (`event-forecast/`, Rust)

The eval-first thesis was born here: a next-event forecaster whose model **collapsed
to a constant** on real data. The eval caught it; the model was discarded; the lesson
seeded everything above. (See its `docs/`.)

## The synthesis

**Signal is the lever; the regime decides; match the method to the regime; measure
everything.** The only thing that materially moves accuracy further is *exogenous
signal at a fine grain* — everything else here is a measured map of what works,
what doesn't, and why.
