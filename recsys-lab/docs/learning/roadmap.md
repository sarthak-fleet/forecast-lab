# Recommender Systems — Learning Roadmap

The "what I need to learn" spine for recsys-lab. Ordered the way you should
actually learn it: **eval first**, then dumb baselines, then the linear-algebra
core, then up the ladder only as far as the numbers justify.

Each entry is deliberately thin — **what** (one line) · **why it matters here**
(one line) · **source**. Don't re-derive things that have a definitive source;
read the source. Verified links land in `../research/` once the landscape
survey completes; this file is the structure — the citations live in [`../research/landscape.md`](../research/landscape.md).

> How far up this ladder we climb is decided by eval, not ambition. Most of the
> value is in tiers 0–2.

---

## Tier 0 — The discipline ("eval is the game"). Learn this FIRST.

- **Offline eval protocol** — how you split data to score a recommender without
  fooling yourself. *Why here:* a random split leaks the future (same trap as
  the event-forecast timestamp sort); recsys uses **temporal** or
  **leave-one-out** splits. *Source: TBD (research).*
- **Ranking metrics** — Recall@K, NDCG@K, MRR, Hit-Rate@K. *Why here:* recsys is
  a *ranking* problem, not rating prediction; these are the scoreboard.
  → explained with a worked example on our numbers in [metrics.md](metrics.md).
- **Guardrail metrics** — coverage, novelty, diversity; **popularity bias**.
  *Why here:* a model can win on NDCG by only recommending blockbusters — these
  catch that.
- **The reproducibility critique** — Dacrema et al., *"Are We Really Making Much
  Progress?"* (RecSys 2019). *Why here:* it's the empirical backbone of our
  whole thesis — simple baselines often match/beat deep models under fair eval.
  *Source:* [arxiv 1907.06902](https://arxiv.org/abs/1907.06902) (39% of neural
  models reproducible; 86% of those beaten by item-KNN / tuned MF).
- **Evaluation hygiene** — framework choice alone (truncation, tie-handling) can
  fake an 18% NDCG / 35% recall gap on identical algorithms. *Why here:* record
  framework version + similarity-truncation + nDCG tie-handling next to every
  number, or comparisons are meaningless. *Source:*
  [arxiv 2407.13531](https://arxiv.org/pdf/2407.13531).

## Tier 1 — Baselines & statistics (no training)

- **Popularity / trending** — recommend the globally most-popular (or
  recency-weighted) items. *Why here:* THE baseline everything must beat.
- **Co-occurrence / association rules** — "people who interacted with X also
  interacted with Y." *Why here:* counting-only, often shockingly strong.
- **Item-KNN / User-KNN** — similarity over interaction vectors
  (cosine / Jaccard). *Why here:* a well-tuned item-KNN beats many "SOTA" models;
  the honest middle baseline.

## Tier 2 — Matrix factorization (the core math: linear algebra)

- **Low-rank intuition** — approximate the giant sparse user×item matrix as the
  product of two skinny matrices (latent factors). *Why here:* this IS "the real
  recsys"; the elegant, well-understood center of the field.
- **Implicit ALS** — alternating least squares for implicit feedback
  (clicks/watches). *Why here:* the workhorse for the data most products have.
- **BPR** (Bayesian Personalized Ranking) — optimizes ranking directly, not
  rating reconstruction. *Why here:* the right objective for top-K.
- **SVD / FunkSVD** — factorization for explicit ratings. *Why here:* the
  classic (Netflix Prize) entry point if data has star ratings.

## Tier 3 — Classical ML ranking

- **Learning-to-rank** — pointwise / pairwise / listwise framings. *Why here:*
  how industry turns features into a ranked list.
- **Gradient-boosted trees** (LightGBM/XGBoost) for ranking. *Why here:* the
  production ranking layer when you have rich user/item/context features.

## Tier 4 — Embeddings & retrieval

- **Two-tower / neural CF** — learned user-tower + item-tower, dot-product
  retrieval. *Why here:* matrix factorization's neural cousin; the modern
  industrial retrieval default.
- **Approximate nearest neighbor** (FAISS, HNSW, usearch). *Why here:* you can't
  brute-force score a million-item catalog; ANN makes embedding retrieval real.
- **Content-based embeddings** (sentence-transformers on item metadata). *Why
  here:* solves cold-start; the honest, non-LLM use of "semantics."

## Tier 5 — Sequence / session-aware

- **Markov / next-item co-occurrence** — the dumb sequence baseline. *Why here:*
  the direct sibling of the event-forecast model; predict-next-item.
- **GRU4Rec** — RNN over a session. *Why here:* first real sequence model.
- **SASRec / BERT4Rec** — self-attention over interaction order. *Why here:* the
  strong modern sequential baseline; only worth it if order carries signal.

## Tier 6 — LLMs at the edges (and why not the core)

- **Where they fit** — cold-start (no history, rich text), reranking a small
  retrieved candidate set, generating explanations, semantic embeddings.
- **Why NOT core retrieval** — cost, latency, can't rank a large catalog in one
  shot, and (per the thesis) no lift over ALS. *Why here:* learning *where the
  hype stops* is itself a deliverable.

---

## Math prerequisites (the "complicated math," demystified)

- **Linear algebra** — vectors, dot products, low-rank approximation, **SVD**.
  This is the math under matrix factorization and embeddings. *(The one to
  actually study.)*
- **Probability / statistics** — conditional distributions, ranking, sampling
  negatives for implicit feedback.
- **Optimization** — gradient descent, **alternating least squares**,
  regularization, why BPR's loss is shaped the way it is.

## How this doc grows

- Research lands → fill in `Source:` links + the `docs/research/` survey.
- We implement a tier → add a short `docs/lessons.md` entry: what the eval
  actually showed (did it beat popularity? by how much?).
- Keep entries thin and DRY — one concept, one home, link don't repeat.
