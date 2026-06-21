"""The best-effort predictor at the fine/actionable grain (state x category x week),
where naive lag1 is weak and a pooled model can win.

Hybrid recipe (M5-style): GBT (Poisson) over a global pooled panel with rich
features — multi-lags, rolling mean/std, trend, calendar, and CLUSTER ids
(borrow strength across cells) — ensembled with the naive backbone. Measured
head-to-head vs the naive baselines on a 1-step-ahead backtest.

  cd demand-forecast && python3 run_best.py
"""
import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingRegressor

from demand.olist import load_olist_orders
from demand.clustering import cluster_by_profile
from demand.eval import metrics

TEST_WEEKS = 8


def main():
    o = load_olist_orders()
    plab, _, _ = cluster_by_profile(o, "category", "month", k=6, min_count=300)
    slab, _, _ = cluster_by_profile(o, "state", "month", k=4, min_count=300)

    g = o.groupby(["state", "category", "week"]).size().rename("y").reset_index()
    states, cats = sorted(o.state.unique()), sorted(o.category.unique())
    weeks = pd.date_range(g.week.min(), g.week.max(), freq="W-MON")
    p = (pd.MultiIndex.from_product([states, cats, weeks], names=["state", "category", "week"])
         .to_frame(index=False).merge(g, on=["state", "category", "week"], how="left"))
    p["y"] = p.y.fillna(0).astype(float)
    p = p.sort_values(["state", "category", "week"])

    gb = p.groupby(["state", "category"], group_keys=False).y
    p["lag1"], p["lag2"], p["lag4"] = gb.shift(1), gb.shift(2), gb.shift(4)
    p["rmean4"] = gb.apply(lambda s: s.shift(1).rolling(4).mean())
    p["rmean8"] = gb.apply(lambda s: s.shift(1).rolling(8).mean())
    p["rstd4"] = gb.apply(lambda s: s.shift(1).rolling(4).std())
    p["woy"] = p.week.dt.isocalendar().week.astype(int)
    p["month"] = p.week.dt.month
    p["trend"] = ((p.week - p.week.min()).dt.days // 7).astype(int)
    p["pcl"] = p.category.map(plab).fillna(-1).astype(int)
    p["scl"] = p.state.map(slab).fillna(-1).astype(int)
    p["state_id"] = p.state.astype("category").cat.codes
    p["cat_id"] = p.category.astype("category").cat.codes

    cutoff = weeks[-TEST_WEEKS]
    tr, te = p[p.week < cutoff].copy(), p[p.week >= cutoff].copy()
    for d, src in [(tr, tr), (te, tr)]:
        d["seas"] = d.merge(src.groupby(["state", "category", "month"]).y.mean().rename("s"),
                            on=["state", "category", "month"], how="left").s.to_numpy()
        d["cmean"] = d.category.map(src.groupby("category").y.mean())
        d["smean"] = d.state.map(src.groupby("state").y.mean())
    glob = tr.y.mean()
    FE = ["lag1", "lag2", "lag4", "rmean4", "rmean8", "rstd4", "woy", "month", "trend",
          "pcl", "scl", "state_id", "cat_id", "seas", "cmean", "smean"]
    tr = tr.dropna(subset=["lag1", "lag2", "lag4", "rmean4"]).copy()
    for d in (tr, te):
        d[FE] = d[FE].fillna(glob)

    gbt = HistGradientBoostingRegressor(loss="poisson", max_iter=600, learning_rate=0.05,
                                        max_leaf_nodes=63, l2_regularization=1.0, random_state=0)
    gbt.fit(tr[FE], tr.y)
    pred = np.clip(gbt.predict(te[FE]), 0, None)
    ens = 0.5 * pred + 0.5 * te.lag1.to_numpy()

    y = te.y.to_numpy()
    rows = [
        ("Best hybrid (GBT+lag ensemble)", metrics(y, ens)),
        ("GBT (pooled, 16 features)", metrics(y, pred)),
        ("SeasonalNaive (state×cat×month)", metrics(y, te.seas.to_numpy())),
        ("LastWeek (lag1) — naive bar", metrics(y, te.lag1.to_numpy())),
    ]
    print(f"Fine grain (state×category×week), {len(states)}×{len(cats)} cells, "
          f"last {TEST_WEEKS} wks held out:\n")
    for name, m in sorted(rows, key=lambda r: r[1]["wMAPE"]):
        print(f"  {name:34s} wMAPE={m['wMAPE']:.3f}  MAE={m['MAE']:.3f}")


if __name__ == "__main__":
    main()
