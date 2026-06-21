# Recommender Systems — Landscape Survey

The categorized menu to pick our first build batch from. Produced by a
fan-out + 3-vote adversarial-verification research run (31 sources, 149 claims,
22 confirmed / 3 killed), then merged with standard domain knowledge.

**Legend:** ✓ = adversarially verified in this run (citation below) · ○ =
standard/background knowledge, *not* independently verified here (trust
accordingly).

> **The one finding that matters most** (✓, RecSys 2019 Best Paper): of 18
> published neural recommenders, only **7 (39%) were reproducible**, and **6 of
> those 7 (86%) were beaten by simple item-KNN or well-tuned matrix
> factorization**. A 2021 follow-up found 11/12 beaten. Our eval-first thesis
> isn't caution — it's the documented norm. Every model we build must beat
> popularity + item-KNN on a temporal split before it earns any complexity.

---

## 1. What industry actually uses

The real pattern is a **multi-stage funnel**, not one model:

| Stage | Job | Machinery |
|---|---|---|
| **Retrieval** ✓ | billions → hundreds | **Two-tower** neural nets (user-tower × item-tower, run independently for caching) + ANN search. Lineage: Word2Vec + features. |
| **1st-stage rank** ✓ | thousands, cheap | lightweight scorer |
| **2nd-stage rank** ✓ | top ~100, heavy | **MTML** (multi-task multi-label) net: predicts P(click), P(like), P(see_less)… combined by a weighted value model |
| **Rerank** ✓ | final | business rules, diversity, integrity |

- **Verified sources:** Instagram Explore (Meta Eng, 2023) documents all four stages with exact counts; YouTube (Covington et al., RecSys 2016) defined the canonical candidate-gen → ranking split everyone copies.
- **Matrix factorization** remains the workhorse for smaller-scale / cold-start; deep nets dominate only at billion-item scale where the infra is justified. ○
- **Netflix lesson** ✓: consolidating models helps when they rank *similar* targets; risks negative transfer across *different* targets. Multi-task ≠ free lunch.
- **LLMs in production recsys:** the research found **no verified primary source** placing LLMs in the core loop. Open question — their honest niche is widely believed to be cold-start embeddings, reranking a small candidate set, and explanations, but treat that as background, not established here. This *supports* our "LLMs aren't the core" prior.

**Takeaway for us:** mirror the funnel conceptually (retrieve → rank), but a learning project lives at the retrieval/MF layer. Two-tower is the thing to study *after* MF, and it needs more infra than ALS for marginal lift on small data.

---

## 2. Open-source libraries we can use

### Python (mature, where the lab should live)
| Library | Covers | Status | Pick it when… |
|---|---|---|---|
| **implicit** ✓ | ALS, BPR, Logistic-MF, item-KNN (cosine/TFIDF/BM25) | active (v0.7.3, 2026; Cython+OpenMP, CUDA) | you want **fast, production-grade ALS/BPR on implicit feedback**. Our MF workhorse. |
| **RecBole** ✓ | **94 algorithms** (general/sequential/context/knowledge); built-in splits, neg-sampling, metrics | active (v1.2.1) | you want a **standardized multi-algorithm benchmark harness** (and SASRec/BERT4Rec for free). |
| **Cornac** ○ | broad CF + multimodal, strong comparative eval | active | rigorous side-by-side model comparison. |
| **LightFM** ○ | hybrid MF (WARP/BPR) with content features | maintenance-mode | warm+cold hybrid on a small footprint. |
| **Surprise** ○ | explicit-rating CF (SVD, KNN) | low activity | classic star-rating prediction / teaching. |
| **Microsoft Recommenders** ○ | examples + utils across many algos | active | reference implementations & best-practice notebooks. |
| **TorchRec** ○ | production deep recsys (sharded embeddings) | active (Meta) | large-scale deep models — overkill for us. |
| **Merlin / NVTabular** ○ | GPU end-to-end pipeline | active (NVIDIA) | GPU-scale feature eng + training. |
| **Spotlight** ○ | PyTorch MF + sequence | largely unmaintained | skip; reference only. |
| **sentence-transformers + FAISS** ○ | text embeddings + ANN retrieval | active | content-based / cold-start retrieval, semantic similarity. |

### Rust (thin but real — best-tool-per-job)
| Crate | Covers | Status | Pick it when… |
|---|---|---|---|
| **disco-rust** ✓ | MF for implicit (conjugate-gradient) + explicit (SGD), **zero deps** | active (2026) | pure-Rust MF experimentation. (Note: CG, *not* ALS; **no BPR** — gap.) |
| **Qdrant** ✓ | vector DB in Rust, **native recommend API** (positive/negative examples) | active (Series B, 2026) | production ANN retrieval with recsys-native queries. |
| **USearch** ✓ | HNSW ANN, first-class Rust bindings, custom metrics, filtered search | active (v2.25, 2026) | low-latency custom-metric ANN. *(Its headline FAISS-speedup benchmarks were refuted — ignore the marketing numbers.)* |
| **hnsw_rs / instant-distance** ○ | HNSW ANN | varies | alternative pure-Rust ANN. |

**Honest verdict:** Rust has ANN (Qdrant/USearch) and basic MF (disco-rust) but **no mature BPR/ALS+sequence stack**. For breadth-of-models learning, **Python wins**; use Rust deliberately for the MF-from-scratch and ANN pieces if we want the systems-learning.

---

## 3. Managed / SaaS products (awareness only — not a build target) ○

Sourced from vendor/blog material, not adversarially verified.

- **AWS Personalize** — managed pipeline (HRNN/sequence + similar-items), tied to AWS.
- **Google Vertex AI** — recommendations as part of the GCP ML platform.
- **Azure Personalizer** — contextual-bandit personalization (being sunset; verify).
- **Recombee** — turnkey recsys API, fast integration, usage-priced.
- **Algolia Recommend** — recommendations bolted onto Algolia search.
- **Shaped.ai** — modern real-time recsys-as-a-service.
- **NVIDIA Merlin** — not SaaS; a self-hosted GPU framework (listed for completeness).

Relevance to us: zero as a build target; useful only as a reference for what a "done" product API looks like.

---

## 4. Datasets + evaluation

### Datasets
| Dataset | Shape | Good for |
|---|---|---|
| **MovieLens 100K / 1M / 25M** ○ | ratings + ts | the canonical benchmark; **start here** (known-good numbers validate our harness) |
| **Amazon Reviews (2023)** ○ | huge, per-category, ts | scale + cold-start + metadata |
| **Retailrocket** ○ | e-comm events (view/cart/buy) | implicit-feedback, sessions |
| **Yoochoose / RecSys'15** ○ | clickstream sessions | session-based / sequence models |
| **Last.fm** ○ | music plays | implicit, repeat-consumption |
| **Steam** ○ | plays + reviews | implicit + sequence |
| **Goodbooks-10k** ○ | book ratings | small, clean, teaching |
| **RecSysDatasets** ✓ (RUCAIBox) | index of many in a common format | one-stop loader for the above |
| **Our own** | Olist (built), anime_list / looptv / everythingrated (TBD) | the "is there a real product here" test once the harness is trusted |

### Offline protocol & metrics
- **Split:** temporal (train on past, test on future) or **leave-one-out** (hide each user's last interaction). Never random — it leaks the future. RecBole default ✓: group-by-user, 80/10/10, Recall@10 / MRR@10 / NDCG@10 / Hit@10 / Precision@10.
- **Metrics:** Recall@K, NDCG@K, MRR, Hit-Rate@K (accuracy) + **coverage / novelty** (anti-blockbuster guardrails).
- **Eval hygiene** ✓ (important, subtle): *framework choice alone* moved ItemKNN nDCG by 18% / recall by 35% between RecBole and LensKit — the gap was **similarity-matrix top-K truncation**, not the algorithm, and vanished after alignment. **Always record framework version, similarity-truncation setting, and the nDCG tie-handling** next to any number, or comparisons are meaningless.

---

## 5. Recommended first batch to implement + benchmark

Dumb → fancy, each must beat the one below it on a temporal split (Recall@10, NDCG@10, MRR) on **MovieLens-1M** first:

1. **Popularity** (hand-rolled) — the control. The bar.
2. **Item-KNN** (hand-rolled, then cross-check vs `implicit`) — the baseline that humbles deep models; also our eval-hygiene lesson in practice.
3. **ALS** (`implicit`) — implicit-feedback matrix factorization; the workhorse.
4. **BPR** (`implicit`) — ranking-objective MF; the "right loss for top-K."
5. **SASRec** (`RecBole`) — one modern sequence model; tests whether order carries signal.
6. *(optional)* **Two-tower / embedding retrieval** (sentence-transformers + FAISS or Qdrant) — the industry retrieval pattern, after MF is mastered.
7. *(optional)* **LLM reranker** over ALS's top-50 — the edge experiment: does it beat plain ALS@10? (Likely no — that's a finding.)

**Stack call:** Python lab (`implicit` + a small hand-rolled harness for 1–2, `RecBole` for 5). Optionally re-implement ALS/item-KNN in **Rust (disco-rust + USearch)** as a systems-learning side-quest.

---

## Sources (verified findings)

- Dacrema et al., *Are We Really Making Much Progress?* — https://arxiv.org/abs/1907.06902 · repro repo https://github.com/MaurizioFD/RecSys2019_DeepLearning_Evaluation
- Instagram Explore architecture (Meta Eng, 2023) — https://engineering.fb.com/2023/08/09/ml-applications/scaling-instagram-explore-recommendations-system/
- YouTube deep recommendations (Covington et al., 2016) — https://research.google/pubs/deep-neural-networks-for-youtube-recommendations/
- Netflix model consolidation lessons — https://netflixtechblog.medium.com/lessons-learnt-from-consolidating-ml-models-in-a-large-scale-recommendation-system-870c5ea5eb4a
- Framework-choice eval artifacts (ItemKNN, 2024) — https://arxiv.org/pdf/2407.13531
- RecBole — https://github.com/RUCAIBox/RecBole · datasets https://github.com/RUCAIBox/RecSysDatasets
- implicit — https://github.com/benfred/implicit
- disco-rust — https://github.com/ankane/disco-rust · USearch — https://github.com/unum-cloud/usearch · Qdrant — https://github.com/qdrant/qdrant
- applied-ml (industry blog index) — https://github.com/eugeneyan/applied-ml

**Refuted in verification (do not cite):** Netflix "single consolidated model via task_type" (1-2); USearch 9.6×/20× FAISS-speed claims (0-3); hnswlib-rs star/maintenance specifics (1-2).

**Open questions:** exact LLM integration points in 2025-26 production; reproducibility of *post-2021* models (SASRec/BERT4Rec/LightGCN) under strict temporal splits; real ALS-vs-two-tower gap on ML-1M; any mature Rust BPR/ALS beyond disco-rust.
