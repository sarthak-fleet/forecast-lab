# new-things — study queue

Short stubs for non-standard tech in this repo. 3–5 lines each. Fill `Why here:`
yourself after learning; never invent rationale.

## Quantile regression with newsvendor inventory optimization
- What: Turning demand forecasts into cost-optimal stocking decisions using the critical ratio formula
- Why here: TBD
- Gotcha (from code): `demand/inventory.py:37-39` — optimal service level is simply `Cu / (Cu + Co)` — no optimization needed, just pick the right quantile
- Source: https://huggingface.co/blog/forecasting-quantile-newsvendor

## Conformal prediction with adaptive online calibration
- What: Distribution-free uncertainty quantification that guarantees coverage, with online drift correction
- Why here: TBD
- Gotcha (from code): `run_conformal.py:63` — online conformal uses `Q_ad += gamma * (target - cov_ad[-1])` to track drift in real-time, unlike static conformal which degrades
- Source: https://arxiv.org/abs/2107.07511

## SASRec: causal transformer for sequential recommendation
- What: Self-attentive sequential recommendation with causal masking and per-position binary ranking
- Why here: TBD
- Gotcha (from code): `recsys-lab/recsys/sasrec.py:40-43` — embeddings initialized with `std=0.01` instead of default `std≈1` because default init makes dot-product logits too large, wasting early epochs
- Source: https://arxiv.org/abs/1808.09781

## Full-ranking evaluation (no sampled negatives)
- What: Honest evaluation that scores ALL items for each user, not sampled negatives — with optimistic tie handling
- Why here: TBD
- Gotcha (from code): `recsys-lab/recsys/eval.py:9-12` — tie handling uses optimistic rank (`(#items strictly better) + 1`) — matters most for Popularity where many items share counts
- Source: https://dl.acm.org/doi/10.1145/3397271.3401274

## Shape-based clustering for regime detection
- What: KMeans on row-normalized demand profiles to cluster by seasonality shape, not volume
- Why here: TBD
- Gotcha (from code): `demand-forecast/demand/clustering.py:20` — `X = normalize(piv.loc[keep].to_numpy())` normalizes rows so clustering groups by pattern shape, not magnitude
- Source: https://scikit-learn.org/stable/modules/preprocessing.html

## Pinball loss for quantile calibration
- What: Proper scoring rule for quantile forecasts that penalizes asymmetrically — matches the quantile definition
- Why here: TBD
- Gotcha (from code): `run_quantile.py:60` — pinball loss is `np.mean(np.maximum(tau * d, (tau - 1) * d))` where `d = demand - stock` — asymmetric penalty is the point, not a bug
- Source: https://en.wikipedia.org/wiki/Quantile_regression

## Rust ML with TimescaleDB
- What: Event forecasting service in Rust with TimescaleDB for time-series storage
- Why here: TBD
- Gotcha (from code): `event-forecast/src/lib.rs:240` — uses `median(&global_intervals, 60_000)` with a 60-second floor for interval prediction — Rust's stdlib doesn't have median, custom implementation
- Source: https://docs.timescale.com/
