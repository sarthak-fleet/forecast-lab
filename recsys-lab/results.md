# Leaderboard — MovieLens-1M (LOO, full-ranking)

users=6040 · items=3706 · protocol: leave-one-out, full-ranking (no sampled negatives), ties = best-case rank.

| Model | Recall@10 | NDCG@10 | MRR | Recall@20 | NDCG@20 | Coverage@10 | fit_s | vs Pop NDCG@10 |
|---|---|---|---|---|---|---|---|---|
| SASRec (2 blocks, d=64) | 0.2151 | 0.1088 | 0.094 | 0.3353 | 0.1392 | 0.6668 | 306.5 | +388% |
| Markov (1st-order) | 0.1637 | 0.0824 | 0.0736 | 0.2493 | 0.1039 | 0.7329 | 0.1 | +270% |
| ItemKNN (cosine, k=100) | 0.0745 | 0.04 | 0.0384 | 0.1114 | 0.0493 | 0.1379 | 1.7 | +79% |
| BPR (f=64, epochs=120) | 0.0735 | 0.036 | 0.0362 | 0.1263 | 0.0493 | 0.5065 | 157.5 | +61% |
| ALS (f=64, iters=15, a=40) | 0.071 | 0.0341 | 0.0352 | 0.1315 | 0.0493 | 0.6282 | 354.3 | +53% |
| Popularity | 0.0437 | 0.0223 | 0.0231 | 0.0719 | 0.0294 | 0.0534 | 0.2 | baseline |
