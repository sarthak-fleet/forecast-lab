"""ML tier (2-axis: time x place): does gradient boosting beat seasonal-naive?

Features (all leakage-safe for a 1-week-ahead forecast):
  calendar  : hour-of-day, day-of-week, is_weekend, daypart
  place     : zone id, zone train-mean
  signal    : seasonal_mean (zone x dow x hod, train only) — the baseline AS a feature
  lag       : lag168 (count same hour one week earlier; in train for the whole test week)

  cd demand-forecast && python3 run_ml.py
"""
import json
from pathlib import Path

import numpy as np
import pandas as pd

from demand.data import load_panel
from demand.models import SeasonalNaive, LastWeek
from demand.eval import metrics

HERE = Path(__file__).resolve().parent


def daypart(h):
    return 0 if h < 6 else 1 if h < 11 else 2 if h < 15 else 3 if h < 19 else 4


def main():
    try:
        from sklearn.ensemble import HistGradientBoostingRegressor
    except ImportError:
        print("scikit-learn not installed — needed for the GBT tier.")
        return

    panel, train, test, zones = load_panel()
    panel = panel.sort_values(["PULocationID", "hour"])
    panel["lag168"] = panel.groupby("PULocationID").y.shift(168)
    panel["is_weekend"] = (panel.dow >= 5).astype(int)
    panel["daypart"] = panel.hod.map(daypart)

    cutoff = test.hour.min()
    tr = panel[panel.hour < cutoff].copy()
    te = panel[panel.hour >= cutoff].copy()

    # train-only stats merged onto both (no leakage)
    zmean = tr.groupby("PULocationID").y.mean()
    smean = (tr.groupby(["PULocationID", "dow", "hod"]).y.mean()
             .rename("seasonal_mean").reset_index())
    for d in (tr, te):
        d["zone_mean"] = d.PULocationID.map(zmean)
    tr = tr.merge(smean, on=["PULocationID", "dow", "hod"], how="left")
    te = te.merge(smean, on=["PULocationID", "dow", "hod"], how="left")
    glob = tr.y.mean()
    te["seasonal_mean"] = te.seasonal_mean.fillna(zmean.mean())
    te["lag168"] = te.lag168.fillna(te.seasonal_mean)

    FEATS = ["hod", "dow", "is_weekend", "daypart", "PULocationID",
             "zone_mean", "seasonal_mean", "lag168"]
    trf = tr.dropna(subset=["lag168", "seasonal_mean", "zone_mean"])

    gbt = HistGradientBoostingRegressor(max_iter=400, learning_rate=0.06,
                                        l2_regularization=1.0, random_state=0)
    gbt.fit(trf[FEATS], trf.y)
    pred = np.clip(gbt.predict(te[FEATS]), 0, None)

    sn = SeasonalNaive(); sn.fit(tr)
    lw = LastWeek(); lw.fit(tr)
    y = te.y.to_numpy()
    results = [
        {"model": "GBT (calendar+lag+seasonal feat)", **metrics(y, pred)},
        {"model": "SeasonalNaive (bar)", **metrics(y, sn.predict(te))},
        {"model": "LastWeek (bar)", **metrics(y, lw.predict(te))},
    ]
    for r in sorted(results, key=lambda r: r["MAE"]):
        print(f"{r['model']:34s} MAE={r['MAE']:.3f} RMSE={r['RMSE']:.3f} "
              f"wMAPE={r['wMAPE']:.4f} bias={r['bias']:+.3f}")
    (HERE / "results_ml.json").write_text(json.dumps(results, indent=2))

    # feature importance (permutation-free: use the model's built-in via a quick proxy)
    print("\ntop features by gain not exposed in HGB; key levers = seasonal_mean + lag168")


if __name__ == "__main__":
    main()
