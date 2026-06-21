"""Quantile / service-level layer: turn the demand forecast into an inventory
decision and measure the probability you stocked enough.

For each (region, product-type, week) we forecast a high quantile of demand, "stock"
that many, and measure:
  - fill rate   = P(stock >= demand)  -> the service level / "probability"
  - overstock   = mean excess units carried (the cost of safety stock)
  - understock  = mean shortfall (lost sales)
  - pinball     = proper score for the quantile forecast
  - CALIBRATION = does aiming for tau actually deliver fill rate ~= tau?

Methods: quantile-GBT (ML) vs naive (lastweek + residual spread) vs Poisson(lastweek).
  cd demand-forecast && python3 run_quantile.py
"""
import json
from pathlib import Path

import numpy as np
import pandas as pd
from scipy.stats import poisson
from sklearn.ensemble import HistGradientBoostingRegressor

from demand.olist import load_olist_panel

HERE = Path(__file__).resolve().parent
QUANTILES = [0.5, 0.8, 0.9, 0.95]
FEATS = ["region_id", "ptype_id", "woy", "month", "trend", "lag1", "lag4", "ma4", "gmean"]


def build(panel, train, cutoff):
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


def stock_metrics(stock, demand, tau):
    over = np.clip(stock - demand, 0, None)
    under = np.clip(demand - stock, 0, None)
    d = demand - stock
    return {
        "fill_rate": round(float((stock >= demand).mean()), 3),
        "overstock": round(float(over.mean()), 2),
        "understock": round(float(under.mean()), 2),
        "pinball": round(float(np.mean(np.maximum(tau * d, (tau - 1) * d))), 3),
    }


def main():
    panel, train, test, cutoff = load_olist_panel()
    tr, te = build(panel, train, cutoff)
    y = te.y.to_numpy()
    print(f"test cells={len(te)}  mean demand/cell={y.mean():.1f}\n")

    preds = {"QuantileGBT": {}, "Naive (lag1+resid)": {}, "Poisson (lag1)": {}}
    resid = (tr.y - tr.lag1).to_numpy()
    lam = np.clip(te.lag1.to_numpy(), 0.1, None)
    for tau in QUANTILES:
        m = HistGradientBoostingRegressor(loss="quantile", quantile=tau,
                                          max_iter=400, learning_rate=0.05,
                                          l2_regularization=1.0, random_state=0)
        m.fit(tr[FEATS], tr.y)
        preds["QuantileGBT"][tau] = np.clip(m.predict(te[FEATS]), 0, None)
        preds["Naive (lag1+resid)"][tau] = np.clip(te.lag1.to_numpy() + np.quantile(resid, tau), 0, None)
        preds["Poisson (lag1)"][tau] = poisson.ppf(tau, lam)

    results = {}
    for name, pq in preds.items():
        results[name] = {tau: stock_metrics(pq[tau], y, tau) for tau in QUANTILES}

    # calibration table: aiming for tau -> achieved fill rate
    print("CALIBRATION — achieved fill rate (want ~= target tau):")
    print(f"  {'method':22s} " + "  ".join(f"tau={t}" for t in QUANTILES))
    for name in preds:
        print(f"  {name:22s} " + "   ".join(f"{results[name][t]['fill_rate']:.2f} " for t in QUANTILES))

    print("\nSERVICE-LEVEL TRADEOFF (overstock units carried per cell):")
    print(f"  {'method':22s} " + "  ".join(f"tau={t}" for t in QUANTILES))
    for name in preds:
        print(f"  {name:22s} " + "  ".join(f"{results[name][t]['overstock']:5.1f} " for t in QUANTILES))

    print("\nPINBALL loss (lower = better-calibrated+sharper), averaged over tau:")
    for name in preds:
        avg = np.mean([results[name][t]["pinball"] for t in QUANTILES])
        print(f"  {name:22s} {avg:.3f}")

    (HERE / "results_quantile.json").write_text(json.dumps(
        {k: {str(t): v for t, v in d.items()} for k, d in results.items()}, indent=2))


if __name__ == "__main__":
    main()
