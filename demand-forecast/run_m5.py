"""Capstone cell: M5 (Walmart) — the full demand cube. item × store × day unit
sales WITH price + SNAP + calendar events. Intermittent retail demand — the
hardest, most complete regime. (Subset: store CA_1, FOODS category.)

Isolates the exogenous lift on the M5 task (last 28 days held out):
  seasonal baseline → GBT(item+calendar) → GBT(+ price + SNAP + events)

  cd demand-forecast && python3 run_m5.py
"""
import numpy as np
import pandas as pd
from demand.eval import metrics
from demand.gbt import gbt_fit_predict

sales = pd.read_csv("data/m5/sales_train_evaluation.csv")
sales = sales[(sales.store_id == "CA_1") & (sales.cat_id == "FOODS")]
cal = pd.read_csv("data/m5/calendar.csv")
prices = pd.read_csv("data/m5/sell_prices.csv")
prices = prices[prices.store_id == "CA_1"]

dcols = [c for c in sales.columns if c.startswith("d_")]
long = sales.melt(id_vars=["item_id", "dept_id"], value_vars=dcols,
                  var_name="d", value_name="sales")
long["dn"] = long.d.str.slice(2).astype(int)
long = long.merge(cal[["d", "wm_yr_wk", "wday", "month", "year",
                       "event_name_1", "event_type_1", "snap_CA"]], on="d", how="left")
long = long.merge(prices[["item_id", "wm_yr_wk", "sell_price"]],
                  on=["item_id", "wm_yr_wk"], how="left")
long["sell_price"] = (long.groupby("item_id").sell_price
                      .transform(lambda s: s.fillna(s.median())).fillna(long.sell_price.median()))
long["event"] = long.event_name_1.notna().astype(int)
long["etype"] = long.event_type_1.astype("category").cat.codes
long["item_code"] = long.item_id.astype("category").cat.codes
long["dept_code"] = long.dept_id.astype("category").cat.codes

tr, te = long[long.dn <= 1913], long[long.dn > 1913]          # M5 horizon = last 28 days
y = te.sales.to_numpy(float)

sn = tr.groupby(["item_id", "wday"]).sales.mean().rename("p").reset_index()
sn_pred = te.merge(sn, on=["item_id", "wday"], how="left").p.fillna(tr.sales.mean()).to_numpy()

BASE = ["item_code", "dept_code", "wday", "month"]
WX = BASE + ["sell_price", "snap_CA", "event", "etype"]


def gbt(f):
    return gbt_fit_predict(tr[f], tr.sales, te[f], max_iter=300)


print(f"M5 · store CA_1 · FOODS · {sales.shape[0]} items × {len(dcols)} days · "
      f"last 28 held out · {(y == 0).mean() * 100:.0f}% zero-sales days (intermittent)\n")
print(f"  {'method':40s}{'MAE':>8s}{'RMSE':>8s}{'wMAPE':>8s}")
base = None
for name, pred in [("SeasonalNaive (item × weekday mean)", sn_pred),
                   ("GBT (item + calendar)", gbt(BASE)),
                   ("GBT (+ price + SNAP + events)", gbt(WX))]:
    m = metrics(y, pred)
    if base is None:
        base = m["wMAPE"]
    tag = "" if "Seasonal" in name else f"  ({(1 - m['wMAPE'] / base) * 100:+.0f}% vs naive)"
    print(f"  {name:40s}{m['MAE']:8.2f}{m['RMSE']:8.2f}{m['wMAPE']:8.3f}{tag}")
