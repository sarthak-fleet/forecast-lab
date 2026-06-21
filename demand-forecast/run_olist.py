"""3-axis demand forecast on Olist: (region x product-type x week) -> order count.

1-step-ahead weekly backtest (forecast each week given all prior weeks). Lags are
computed from actuals up to t-1, which is legitimate for a 1-step-ahead forecast.
Does GBT beat the naive lag baselines once there's a trend + many series + the
right features?  cd demand-forecast && python3 run_olist.py
"""
import json
from pathlib import Path

import numpy as np
import pandas as pd

from demand.olist import load_olist_panel
from demand.eval import metrics

HERE = Path(__file__).resolve().parent


def main():
    try:
        from sklearn.ensemble import HistGradientBoostingRegressor
    except ImportError:
        print("scikit-learn needed for the GBT tier.")
        return

    panel, train, test, cutoff = load_olist_panel()
    nz = (panel.y > 0).mean()
    print(f"cells={len(panel)}  regions={panel.region.nunique()}  "
          f"types={panel.ptype.nunique()}  weeks={panel.week.nunique()}  "
          f"non-zero={nz:.0%}  mean/cell={panel.y.mean():.1f}  "
          f"test=last {test.week.nunique()} weeks\n")

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
    for col in ["lag1", "lag4", "ma4"]:
        te[col] = te[col].fillna(te["gmean"])

    FEATS = ["region_id", "ptype_id", "woy", "month", "trend", "lag1", "lag4", "ma4", "gmean"]
    gbt = HistGradientBoostingRegressor(loss="poisson", max_iter=500, learning_rate=0.05,
                                        l2_regularization=1.0, random_state=0)
    gbt.fit(tr[FEATS], tr.y)
    pred = np.clip(gbt.predict(te[FEATS]), 0, None)

    y = te.y.to_numpy()
    results = [
        {"model": "GBT (region×type×week, Poisson)", **metrics(y, pred)},
        {"model": "LastWeek (lag1)", **metrics(y, te.lag1.to_numpy())},
        {"model": "MA4 (mean last 4 wks)", **metrics(y, te.ma4.to_numpy())},
        {"model": "GroupMean (static)", **metrics(y, te.gmean.to_numpy())},
    ]
    for r in sorted(results, key=lambda r: r["MAE"]):
        print(f"{r['model']:34s} MAE={r['MAE']:.3f} RMSE={r['RMSE']:.3f} "
              f"wMAPE={r['wMAPE']:.4f} bias={r['bias']:+.3f}")
    (HERE / "results_olist.json").write_text(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
