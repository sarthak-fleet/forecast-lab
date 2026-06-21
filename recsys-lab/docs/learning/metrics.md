# Ranking metrics, by worked example

How we score a recommender in recsys-lab. The formal definitions have canonical
sources (linked below) — this page is the *intuition + a worked example on our
own numbers*, which is the part worth writing down.

## What are we measuring? (ranking, NOT rating)

We are measuring **"what to recommend"** (the quality of a ranked list), **not**
"will the user rate this 4 stars" (rating prediction — the old Netflix-Prize/RMSE
task). We even throw the star values away: every watched movie just counts as
"relevant" (implicit feedback). We predict *what the user will engage with*.

How you grade a list with no answer key: **manufacture one from real behavior.**
We hid each user's chronologically last movie. Train on the rest, ask the model
to rank all ~3,706 items, and check **where that truly-watched movie lands**. A
good recommender ranks things the user actually chose near the top. So the metric
= "did the ranking surface the item the user really picked next?"

*Honest caveat:* a recommended movie the user *would* have loved but didn't
happen to watch in the data scores as a miss — we can't know better. So offline
metrics are deliberately conservative (why 21%, not 90%) and why real systems
also A/B test live. For *comparing models* offline, it's still a fair yardstick.

## The frame

Leave-one-out eval: each user has **exactly one correct answer** (their held-out
last item). The model ranks all ~3,706 items; every metric answers one question
in a different way — **where did the true item land?** Its position = `rank` r
(rank 1 = top = best).

## The three metrics, blunt → refined

**Recall@10 (= HitRate@10 here).** Did the true item land in the top 10? 1/0,
averaged. Simple, but treats rank 1 and rank 10 the same.
> SASRec 0.2151 → "for 21.5% of users the true item was somewhere in the top 10."

**NDCG@10 — rank-aware, the primary metric.** Discounts by position with a log.
For one relevant item it collapses to:
> **NDCG@10 = 1 / log₂(rank + 1)** if rank ≤ 10, else 0, then averaged.

| rank | 1 | 2 | 3 | 5 | 10 | 11+ |
|---|---|---|---|---|---|---|
| score | 1.00 | 0.63 | 0.50 | 0.39 | 0.29 | 0 |

The log says top spots matter a lot (1→2 is a big drop) but deep ranks barely
differ (9 vs 10 ≈ nothing) — mirroring how people scan a list.

**MRR = 1 / rank.** rank 1→1.0, 2→0.5, 10→0.1. Same shape as NDCG but a harsher
decay; cares almost only about getting near #1.

## Reading it off our own numbers (the click)

SASRec: Recall@10 = 0.2151, NDCG@10 = 0.1088 → NDCG/Recall ≈ 0.51.
- If every hit were at rank 1, NDCG would equal Recall.
- A hit contributing ~0.5 means `1/log₂(r+1) = 0.5` → **r ≈ 3**.
- So: *when SASRec puts the true item in the top 10, it's around rank 3 on average.*

Recall = how often it's in the top 10; NDCG = how high; together = "21% of the
time, ~position 3." That's why we report both.

## Don't confuse with: Coverage

Coverage@K = fraction of the catalog that appears in *anyone's* top-K. It's a
**diversity guardrail, not accuracy** — it catches a model that scores well by
only ever recommending blockbusters. Keep it out of accuracy comparisons.

## Sources

- DCG / NDCG — https://en.wikipedia.org/wiki/Discounted_cumulative_gain
- MRR — https://en.wikipedia.org/wiki/Mean_reciprocal_rank
- Implemented in `recsys/eval.py` (full-ranking, ties = best-case rank).
