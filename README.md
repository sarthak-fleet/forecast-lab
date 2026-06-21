# forecast-lab

One project, one through-line: **eval-first ML** — across forecasting and
recommendation, the best method depends on the data regime, so *measure
everything* and never trust a model that doesn't beat the dumb baseline on
held-out data.

It grew as one continuous exploration, in three parts (kept as clean parallel
subfolders — they share the *philosophy*, not the code: forecasting metrics ≠
ranking metrics ≠ a Rust service):

| Folder | What | Language |
|---|---|---|
| **[demand-forecast/](demand-forecast/)** | the culmination — a forecasting **explainer**: methods × dataset regimes (taxi · Olist · bike · Rossmann · M5) × eval gates. Quantile odds, newsvendor inventory, clustering, a query surface, and `report.html`. **Start at [docs/EXPLAINER.md](demand-forecast/docs/EXPLAINER.md).** | Python |
| **[recsys-lab/](recsys-lab/)** | the recommender benchmark ladder — popularity → item-KNN → ALS → BPR → Markov → SASRec on MovieLens-1M, honest full-ranking eval. | Python |
| **[event-forecast/](event-forecast/)** | where it started — a next-event forecaster whose model collapsed on real data, which seeded the eval-first thesis. | Rust |

## The one-sentence synthesis

**Signal is always the lever; the regime decides whether it's enough — match the
method to the regime, and measure everything.**

(Datasets are gitignored — re-fetch via each subfolder's run scripts.)
