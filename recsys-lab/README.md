# recsys-lab

A **learning project** (not a sellable product): build a handful of recommender
systems, dumb → fancy, and evaluate them honestly against each other on a shared
held-out benchmark. The point is to *learn the field by measuring it* — the same
eval-first discipline that killed the event-forecast model.

## Working thesis (to be tested, not assumed)

- The center of gravity of recsys is **statistics + linear algebra** (counting +
  matrix factorization), not deep learning, and not LLMs.
- Every model must **beat a popularity baseline** on a **temporal split**. If it
  doesn't, it doesn't ship — no matter how fancy.
- LLMs are probably wrong for the core retrieval loop; their honest home is the
  edges (cold-start, reranking a small candidate set, explanations).

These are hypotheses. The lab exists to confirm or break them with numbers.

## Layout

| Path | Role |
|---|---|
| `docs/learning/roadmap.md` | **What I need to learn** — the running curriculum. Start here. |
| `docs/research/` | The landscape survey (libraries, products, datasets, benchmarks). |
| `docs/` | Decisions + lessons as we go (added once we start building). |

## Status

Scaffolding. Landscape research in flight; first models not yet implemented.
See `docs/learning/roadmap.md` for the plan.
