"""Discover data-driven groups on all three axes (product, location, time) by
clustering demand profiles — the learned version of the bucketing.

  cd demand-forecast && python3 run_clustering.py
"""
import numpy as np

from demand.olist import load_olist_orders
from demand.clustering import cluster_by_profile, regime_clusters

MONTH = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]


def show(title, labels, piv, counts, k, topn=6):
    print(f"\n=== {title} — {k} archetypes (clustered by demand shape) ===")
    prof = piv.div(piv.sum(axis=1), axis=0)                    # row-normalized
    for c in range(k):
        members = [e for e, l in labels.items() if l == c]
        if not members:
            continue
        peak = MONTH[int(prof.loc[members].mean().idxmax())]
        members = sorted(members, key=lambda e: -counts[e])
        vol = int(counts[members].sum())
        print(f"  cluster {c}  (peak {peak}, {vol} orders): "
              + ", ".join(members[:topn]) + (" ..." if len(members) > topn else ""))
    rare = [e for e, l in labels.items() if l == -1]
    if rare:
        print(f"  rare (< min): {len(rare)} entities, {int(counts[rare].sum())} orders")


def main():
    o = load_olist_orders()
    print(f"orders={len(o)}  states={o.state.nunique()}  categories={o.category.nunique()}")

    pl, ppiv, pc = cluster_by_profile(o, "category", "month", k=6, min_count=300)
    show("PRODUCT", pl, ppiv, pc, k=6)

    sl, spiv, sc = cluster_by_profile(o, "state", "month", k=4, min_count=300)
    show("LOCATION", sl, spiv, sc, k=4)

    wk = o.groupby("week").size()
    reg = regime_clusters(wk, k=3)
    names = {0: "low", 1: "normal", 2: "peak"}
    print("\n=== TIME — 3 demand regimes (weeks clustered by total demand) ===")
    for r in (2, 1, 0):
        wks = [w for w, lab in reg.items() if lab == r]
        avg = wk[wks].mean()
        print(f"  {names[r]:6s}: {len(wks)} weeks, avg {avg:.0f} orders/wk", end="")
        if r == 2:
            peak_wks = sorted(wks, key=lambda w: -wk[w])[:3]
            print("  e.g. " + ", ".join(str(w.date()) for w in peak_wks), end="")
        print()

    print("\nClusters are interpretable demand archetypes — they replace 'top-N + other'\n"
          "as the place/product axes, and the time regime becomes a model feature.")


if __name__ == "__main__":
    main()
