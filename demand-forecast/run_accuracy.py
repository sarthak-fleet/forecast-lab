"""Turn the highest-ROI accuracy dials and measure what each buys.

Dials (no new data needed):
  - TIME granularity   : weekly vs monthly (coarser averages out noise)
  - PLACE/PRODUCT grain : raw state×category vs clustered archetypes (denser cells)
  - ENSEMBLE           : blend last-period + 3-period moving average

Accuracy here = wMAPE of a naive last-period forecast, on a temporal backtest
holding the test span ~constant (~8 weeks). Lower = better.
  cd demand-forecast && python3 run_accuracy.py
"""
import numpy as np
import pandas as pd

from demand.olist import load_olist_orders
from demand.clustering import cluster_by_profile
from demand.eval import metrics

o = load_olist_orders()
plab, _, _ = cluster_by_profile(o, "category", "month", k=6, min_count=300)
slab, _, _ = cluster_by_profile(o, "state", "month", k=4, min_count=300)
o["pclust"] = "P" + o.category.map(plab).fillna(-1).astype(int).astype(str)
o["sclust"] = "S" + o.state.map(slab).fillna(-1).astype(int).astype(str)

TEST = {"W": 8, "M": 2}     # periods, ~8 weeks of test span either way


def run(place, prod, freq):
    df = o.copy()
    df["period"] = df.ts.dt.to_period(freq).dt.start_time
    g = df.groupby([place, prod, "period"]).size().rename("y").reset_index()
    places, prods = sorted(g[place].unique()), sorted(g[prod].unique())
    periods = sorted(g.period.unique())
    panel = (pd.MultiIndex.from_product([places, prods, periods], names=[place, prod, "period"])
             .to_frame(index=False).merge(g, on=[place, prod, "period"], how="left"))
    panel["y"] = panel.y.fillna(0).astype(float)
    panel = panel.sort_values([place, prod, "period"])
    gb = panel.groupby([place, prod], group_keys=False)
    panel["lag1"] = gb.y.shift(1)
    panel["ma3"] = gb.y.apply(lambda s: s.shift(1).rolling(3).mean())
    te = panel[panel.period >= periods[-TEST[freq]]].dropna(subset=["lag1"]).copy()
    te["ma3"] = te.ma3.fillna(te.lag1)
    y = te.y.to_numpy()
    last = metrics(y, te.lag1.to_numpy())["wMAPE"]
    ens = metrics(y, 0.5 * te.lag1.to_numpy() + 0.5 * te.ma3.to_numpy())["wMAPE"]
    return len(panel), panel.y.mean(), last, ens


print(f"{'place':9s}{'product':10s}{'time':5s}{'cells':>8s}{'mean/cell':>11s}"
      f"{'wMAPE last':>12s}{'wMAPE ens':>11s}")
for place, prod, freq in [("state", "category", "W"), ("state", "category", "M"),
                          ("sclust", "pclust", "W"), ("sclust", "pclust", "M")]:
    cells, mean, last, ens = run(place, prod, freq)
    print(f"{place:9s}{prod:10s}{freq:5s}{cells:8d}{mean:11.1f}{last:12.3f}{ens:11.3f}")
