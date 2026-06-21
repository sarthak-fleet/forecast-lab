"""See ACTUAL top-K recommendations (not just metrics) for a few sample users.

For each user: recent watch history, the held-out 'actual next' movie, and each
model's top-10 — with the actual-next item ★-flagged if it appears. Makes the
Recall@10 / NDCG@10 numbers concrete.

  cd recsys-lab && python3 recommend.py
"""
import numpy as np
from recsys.data import load_split
from recsys.models import Popularity, ItemKNN, Markov

train_ui, test_items, n_users, n_items, train_seqs, meta = load_split(return_meta=True)
titles = meta["item_titles"]

models = {"Popularity": Popularity(), "ItemKNN": ItemKNN(knn=100), "Markov": Markov()}
for m in models.values():
    m.fit(train_ui, train_seqs)


def topk(model, u, k=10):
    s = model.score_batch(np.array([u]))[0].astype(float)
    s[train_ui[u].indices] = -np.inf                      # mask already-seen
    idx = np.argpartition(s, -k)[-k:]
    return idx[np.argsort(s[idx])[::-1]]


SAMPLE = [1, 1500, 4500]
for u in SAMPLE:
    actual = test_items[u]
    recent = train_seqs[u][-6:]
    print(f"\n{'=' * 72}")
    print(f"USER {u} — watched {len(train_seqs[u])} movies")
    print("recent history: " + "  |  ".join(titles[i] for i in recent))
    print(f"ACTUAL next (held out): {titles[actual]}")
    for name, model in models.items():
        recs = topk(model, u, 10)
        hit = "HIT" if actual in recs else "miss"
        print(f"\n  {name}  [{hit}]")
        for rank, i in enumerate(recs, 1):
            star = " ★" if i == actual else ""
            print(f"     {rank:2d}. {titles[i]}{star}")
