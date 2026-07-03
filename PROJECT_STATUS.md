# forecast-lab — PROJECT STATUS

Last updated: 2026-07-03

## Why / What

**forecast-lab is an eval-first ML learning lab, not a sellable product.** One
through-line across forecasting and recommendation: the best method depends on
the data regime, so measure everything and never trust a model that doesn't
beat the dumb baseline on held-out data. The thesis was seeded when the
original event-forecast model **collapsed to a constant on real data** and only
the eval harness caught it — the harness, and the lessons it produces, are the
real asset.

**Users:** the repo owner (learner) and fleet agents. No external users.

**In scope:**
- Learning + eval harnesses: honest temporal/held-out benchmarks for
  forecasting (demand-forecast) and recommenders (recsys-lab).
- Distilled findings as docs: `TUTORIAL.md` guided path,
  `demand-forecast/docs/EXPLAINER.md`, per-lab `results.md` leaderboards,
  `docs/learning/new-things.md` study queue.
- The event-forecast Rust service as local/research code and origin story.

**Out of scope:**
- A production forecasting product, deployment, auth, or paying users.
- Shipping any model that doesn't beat its naive/popularity baseline.

Three sub-projects share the philosophy, not the code:

| Folder | What | Language |
|---|---|---|
| `demand-forecast/` | forecasting explainer: methods × dataset regimes (taxi · Olist · bike · Rossmann · M5) × eval gates | Python |
| `recsys-lab/` | recommender benchmark ladder: popularity → item-KNN → ALS → BPR → Markov → SASRec on MovieLens-1M | Python |
| `event-forecast/` | where it started — next-event forecaster whose model collapsed on real data (has its own `PROJECT_STATUS.md`) | Rust |

## Dependencies

### External
- **Python labs** (`demand-forecast/`, `recsys-lab/`): numpy, scipy, pandas,
  scikit-learn (demand-forecast); torch optional for SASRec (recsys-lab).
  Managed per-lab via `pyproject.toml` + `uv.lock`, pytest for tests.
- **event-forecast:** Rust + Rocket 0.5, sqlx, TimescaleDB/PostgreSQL via
  local Docker Compose (:54329), Leaflet single-file explorer UI. Full list
  in `event-forecast/PROJECT_STATUS.md`.
- **Datasets:** gitignored; auto-downloaded by each lab's run scripts
  (NYC taxi, Olist, bike-sharing, Rossmann, M5, MovieLens-1M).

### Internal (fleet)
- None at runtime. Fleet standards (AGENTS.md at fleet root) govern process.
- No production Cloudflare surface; `package.json` `deploy` script fails
  closed by design (CI is the release gate for this lab repo).

## Timeline

- **pre-2026-06-21** — sub-projects built in their own repos: event-forecast
  (Rust service, phases 1–5; 2026-06-20 eval found the baseline ties/loses to
  a majority-class guesser on real data), recsys-lab, demand-forecast.
- **2026-06-21** — consolidated the eval-first ML exploration into one repo;
  added shared GBT helper + guided learning tutorial (`TUTORIAL.md`).
- **2026-06-24** — event-forecast perf pass (hashset/hashmap) (#1).
- **2026-06-28** — added fleet learning track (`docs/learning/new-things.md`).
- **2026-07-02** — explicit deploy guard (no production deploy path).

## Products

- **No production deployments.** Everything runs locally.
- **GitHub repo:** `sarthak-fleet/forecast-lab` (private), CI on GitHub
  Actions (`.github/workflows/ci.yml`): `cargo test` for event-forecast +
  pytest matrix for the Python labs.
- **Local surfaces:**
  - `demand-forecast/query.py` — pick area × product × time → count + odds +
    stocking quantity; `viz.py` → `report.html` (six-chart story).
  - `recsys-lab/recommend.py` — real top-10 recommendation lists;
    `results.md` leaderboard.
  - `event-forecast` — Rocket API on :8088 + Leaflet explorer + `evaluate` /
    `load_events` CLI binaries (see its `PROJECT_STATUS.md`).
- **Docs as product:** `TUTORIAL.md` (run-it-in-order learning path),
  `demand-forecast/docs/EXPLAINER.md`, per-lab `docs/lessons.md`.

## Features (shipped)

### demand-forecast (Python)
- Baselines + models: naive, seasonal-naive, moving average, Holt/ETS,
  gradient boosting (lags + calendar + exogenous), pooled GBT + shape-based
  clustering, naive/model ensemble.
- Probabilistic layer: empirical / Poisson / Negative-Binomial quantiles;
  split + adaptive **online conformal** calibration; pinball-loss scoring.
- Decision layer: newsvendor cost-optimal stocking; online adaptive loop.
- Five dataset regimes with recorded findings (`results.md`, `docs/lessons.md`):
  NYC taxi (LastWeek lag-168h beats GBT — no exogenous signal), bike-sharing
  (weather signal → ML +50%), Rossmann (promo → +15%), M5 (intermittent —
  naive holds), Olist (sparsity/pooling); granularity dial: wMAPE 0.70 → 0.08
  by aggregating.
- Query surface (`query.py`), report generator (`viz.py` → `report.html`),
  pytest suite, per-run results JSON.

### recsys-lab (Python)
- Six-model ladder on MovieLens-1M: Popularity, ItemKNN, ALS, BPR,
  Markov (1st-order), SASRec (2-block causal transformer).
- Honest eval: leave-one-out temporal split, **full-ranking** (no sampled
  negatives), optimistic tie handling, Recall/NDCG/MRR/Coverage.
- Recorded leaderboard (`results.md`): SASRec NDCG@10 +388% vs popularity;
  sequence models dominate order-blind MF/KNN; undertrained SASRec came
  last — the eval catches under-training too.
- `recommend.py` top-10 surface, BPR sweep script, pytest suite, learning
  curriculum (`docs/learning/roadmap.md`, `metrics.md`).

### event-forecast (Rust)
- Full inventory lives in `event-forecast/PROJECT_STATUS.md` (kept as the
  canonical status for that sub-project): Rocket JSON API, transparent
  transition/median baseline, TimescaleDB ingest, heatmap/anomaly/decision/
  replay endpoints, Leaflet explorer, fixture tests.
- Its lasting contribution here is the **negative result**: the baseline
  ties/loses to a majority-class guesser on real data; the eval harness is
  what survived.

### Repo-level
- `TUTORIAL.md` guided learning path across all three labs.
- `docs/learning/new-things.md` study queue (7 stubs, `Why here:` left TBD
  for the learner per fleet learning-track standard).
- CI for both languages; deploy script that fails closed.

## Todo / Planned / Deferred / Blocked

### Planned
1. Fill `Why here:` stubs in `docs/learning/new-things.md` after studying
   each topic (owner: user — per the learning-track standard, agents must
   not pre-fill rationale).
2. recsys-lab curriculum growth per `docs/learning/roadmap.md`: fill
   remaining `Source:` links and add a short lessons entry per implemented
   tier.
3. event-forecast planned items tracked in its own `PROJECT_STATUS.md`
   (real-data-volume validation gate, `augurs` benchmark spike, first real
   event source, alert profile editor).

### Deferred
- Any production deployment or productization — this is a learning lab;
  the deploy guard enforces it.
- event-forecast deferrals (ML frameworks, persisted alerts, geocoding,
  ROADMAP phases 6+) — see `event-forecast/PROJECT_STATUS.md`.

### Blocked
- event-forecast accuracy validation blocked on a larger real stream
  (sample is 14 events / 4 entities — fixture/demo quality only).
