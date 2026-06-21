"""Data-driven grouping of the demand axes by KMeans on normalized demand profiles.

Replaces arbitrary "top-N + other" buckets with demand *archetypes*: products /
locations whose demand moves together get grouped; weeks group into demand regimes.
Profiles are row-normalized so we cluster by *shape* (seasonality), not raw volume.
"""
import numpy as np
from sklearn.cluster import KMeans
from sklearn.preprocessing import normalize


def cluster_by_profile(orders, entity, period, k, min_count=200):
    """Cluster `entity` (e.g. 'category', 'state') by its normalized demand profile
    over `period` (e.g. 'month'). Returns (labels dict, profiles pivot, counts).
    Entities below min_count get label -1 ('rare')."""
    piv = orders.pivot_table(index=entity, columns=period, aggfunc="size", fill_value=0)
    counts = piv.sum(axis=1)
    keep = counts[counts >= min_count].index
    k = min(k, max(2, len(keep)))
    X = normalize(piv.loc[keep].to_numpy().astype(float))      # cluster by shape
    km = KMeans(n_clusters=k, n_init=10, random_state=0).fit(X)
    labels = {e: int(l) for e, l in zip(keep, km.labels_)}
    for e in counts.index:
        labels.setdefault(e, -1)
    return labels, piv, counts


def regime_clusters(weekly_totals, k=3):
    """Cluster time buckets into demand regimes by level (1-D). Returns labels
    ordered low->high (0=lowest)."""
    X = weekly_totals.to_numpy().astype(float).reshape(-1, 1)
    km = KMeans(n_clusters=k, n_init=10, random_state=0).fit(X)
    order = np.argsort([X[km.labels_ == c].mean() for c in range(k)])
    rank = {c: int(r) for r, c in enumerate(order)}
    return {w: rank[l] for w, l in zip(weekly_totals.index, km.labels_)}
