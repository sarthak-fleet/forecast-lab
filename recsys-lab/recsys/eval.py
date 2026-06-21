"""Full-ranking held-out evaluation.

For each test user we score ALL items, mask out the ones seen in train, and
find the rank of the single held-out item among everything that's left
(no sampled negatives — the honest protocol, per Krichene & Rendle 2020).

With one held-out item per user, HitRate@K == Recall@K.

Tie handling: rank = (#items strictly better) + 1. This is the optimistic
("best case") rank for ties — matters most for Popularity, where many items
share a count. We use it consistently across all models so comparisons are
fair, and we record it here as an eval-hygiene decision (see roadmap Tier 0).
"""
import numpy as np


def evaluate(model, train_ui, test_items, Ks=(10, 20), batch=512):
    n_users, n_items = train_ui.shape
    indptr, indices = train_ui.indptr, train_ui.indices
    users = np.array([u for u in range(n_users)
                      if test_items[u] >= 0 and indptr[u + 1] > indptr[u]])
    maxK = max(Ks)

    hits = {K: 0 for K in Ks}
    ndcg = {K: 0.0 for K in Ks}
    rec_items = {K: set() for K in Ks}
    mrr = 0.0

    for start in range(0, len(users), batch):
        bu = users[start:start + batch]
        scores = np.asarray(model.score_batch(bu), dtype=np.float64)
        for r, u in enumerate(bu):
            tr = indices[indptr[u]:indptr[u + 1]]
            scores[r, tr] = -np.inf
            t = test_items[u]
            st = scores[r, t]
            rank = int((scores[r] > st).sum()) + 1
            mrr += 1.0 / rank
            topk = np.argpartition(scores[r], -maxK)[-maxK:]
            topk = topk[np.argsort(scores[r][topk])[::-1]]
            for K in Ks:
                if rank <= K:
                    hits[K] += 1
                    ndcg[K] += 1.0 / np.log2(rank + 1)
                rec_items[K].update(topk[:K].tolist())

    n = len(users)
    out = {"users": int(n)}
    for K in Ks:
        out[f"Recall@{K}"] = round(hits[K] / n, 4)
        out[f"NDCG@{K}"] = round(ndcg[K] / n, 4)
        out[f"Coverage@{K}"] = round(len(rec_items[K]) / n_items, 4)
    out["MRR"] = round(mrr / n, 4)
    return out
