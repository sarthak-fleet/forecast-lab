"""Shared feature builder for the 3-axis Olist panel (lags + calendar + group mean).

1-step-ahead safe: lags come from actuals up to t-1.
"""
import pandas as pd

FEATS = ["region_id", "ptype_id", "woy", "month", "trend", "lag1", "lag4", "ma4", "gmean"]


def build_features(panel, train, cutoff):
    panel = panel.sort_values(["region", "ptype", "week"])
    gb = panel.groupby(["region", "ptype"], group_keys=False)
    panel["lag1"] = gb.y.shift(1)
    panel["lag4"] = gb.y.shift(4)
    panel["ma4"] = gb.y.apply(lambda s: s.shift(1).rolling(4).mean())
    panel["woy"] = panel.week.dt.isocalendar().week.astype(int)
    panel["month"] = panel.week.dt.month
    panel["trend"] = ((panel.week - panel.week.min()).dt.days // 7).astype(int)
    gmean = train.groupby(["region", "ptype"]).y.mean().rename("gmean")
    panel = panel.merge(gmean, on=["region", "ptype"], how="left")
    reg = {c: i for i, c in enumerate(sorted(panel.region.unique()))}
    pty = {c: i for i, c in enumerate(sorted(panel.ptype.unique()))}
    panel["region_id"] = panel.region.map(reg)
    panel["ptype_id"] = panel.ptype.map(pty)
    tr = panel[panel.week < cutoff].dropna(subset=["lag1", "lag4", "ma4"]).copy()
    te = panel[panel.week >= cutoff].copy()
    for c in ["lag1", "lag4", "ma4"]:
        te[c] = te[c].fillna(te["gmean"])
    return tr, te
