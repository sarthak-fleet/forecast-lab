"""MovieLens-1M loading + temporal leave-one-out (LOO) split.

Framing: implicit feedback — every observed rating is a positive interaction
(NCF-style; we ignore the star value). Held-out test = each user's
chronologically *last* interaction. This is the standard LOO protocol; it
avoids the future-leak of a random split.
"""
from pathlib import Path
import numpy as np
import pandas as pd
from scipy.sparse import csr_matrix

RATINGS = Path(__file__).resolve().parent.parent / "data" / "ml-1m" / "ratings.dat"
MOVIES = Path(__file__).resolve().parent.parent / "data" / "ml-1m" / "movies.dat"


def load_split(min_interactions=5, rating_threshold=None, return_meta=False):
    df = pd.read_csv(RATINGS, sep="::", engine="python",
                     names=["user", "item", "rating", "ts"])
    if rating_threshold:
        df = df[df.rating >= rating_threshold]
    df = df.sort_values(["user", "ts"], kind="stable")

    counts = df.user.value_counts()
    df = df[df.user.isin(counts[counts >= min_interactions].index)]

    # remap to contiguous 0-based ids
    uids = {u: k for k, u in enumerate(df.user.unique())}
    iids = {m: k for k, m in enumerate(df.item.unique())}
    df["u"] = df.user.map(uids).astype(np.int64)
    df["i"] = df.item.map(iids).astype(np.int64)
    n_users, n_items = len(uids), len(iids)

    # leave-one-out: each user's last interaction by timestamp
    last_idx = df.groupby("u")["ts"].idxmax()
    test = df.loc[last_idx]
    train = df.drop(index=last_idx)

    train_ui = csr_matrix((np.ones(len(train), dtype=np.float64), (train.u, train.i)),
                          shape=(n_users, n_items))
    train_ui.data[:] = 1.0  # binarize (defensive; ratings are unique per user-item)

    test_items = np.full(n_users, -1, dtype=np.int64)
    test_items[test.u.values] = test.i.values

    # per-user train item sequence in timestamp order (for Markov / SASRec)
    grp = train.sort_values(["u", "ts"], kind="stable").groupby("u")["i"].apply(np.asarray)
    train_seqs = [grp.get(u, np.array([], dtype=np.int64)) for u in range(n_users)]

    if return_meta:
        titles = {}
        with open(MOVIES, encoding="latin-1") as f:
            for line in f:
                mid, title, _ = line.rstrip("\n").split("::", 2)
                titles[int(mid)] = title
        inv = {k: m for m, k in iids.items()}            # internal idx -> MovieID
        item_titles = [titles.get(inv[i], f"item{inv[i]}") for i in range(n_items)]
        return train_ui, test_items, n_users, n_items, train_seqs, {"item_titles": item_titles}

    return train_ui, test_items, n_users, n_items, train_seqs
