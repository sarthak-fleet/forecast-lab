"""Explainer cell: Rossmann retail (store × day sales WITH promo + holidays).

The inventory-domain regime — promotions are known in advance and drive big
demand spikes. Isolates the exogenous lift: seasonal baseline → GBT(store+calendar)
→ GBT(+ PROMO + holidays). 6-week-ahead temporal backtest (the real Rossmann task).

  cd demand-forecast && python3 run_rossmann.py
"""
import numpy as np
import pandas as pd
from demand.eval import metrics
from demand.gbt import gbt_fit_predict

tr_all = pd.read_csv("data/rossmann/train.csv", parse_dates=["Date"], low_memory=False)
store = pd.read_csv("data/rossmann/store.csv")
df = tr_all.merge(store, on="Store", how="left")
df = df[df.Open == 1].copy()                                  # forecast sales on open days
df["StateHolidayF"] = (df.StateHoliday.astype(str) != "0").astype(int)
df["month"], df["day"] = df.Date.dt.month, df.Date.dt.day
for c in ["StoreType", "Assortment"]:
    df[c] = df[c].astype("category").cat.codes
df["CompetitionDistance"] = df.CompetitionDistance.fillna(df.CompetitionDistance.median())

cut = df.Date.max() - pd.Timedelta(days=42)                   # last 6 weeks held out
tr, te = df[df.Date < cut], df[df.Date >= cut]
y = te.Sales.to_numpy()

sn = tr.groupby(["Store", "DayOfWeek"]).Sales.mean().rename("p").reset_index()
sn_pred = te.merge(sn, on=["Store", "DayOfWeek"], how="left").p.fillna(tr.Sales.mean()).to_numpy()

BASE = ["Store", "DayOfWeek", "month", "day", "StoreType", "Assortment",
        "CompetitionDistance", "Promo2"]
WX = BASE + ["Promo", "SchoolHoliday", "StateHolidayF"]


def gbt(feats):
    return gbt_fit_predict(tr[feats], tr.Sales, te[feats], max_iter=400)


print(f"Rossmann retail · {len(df)} open-day records · {df.Store.nunique()} stores · "
      f"6-week backtest · mean {y.mean():.0f}/day\n")
print(f"  {'method':40s}{'MAE':>9s}{'RMSE':>9s}{'wMAPE':>8s}")
base = None
for name, pred in [("SeasonalNaive (store × weekday mean)", sn_pred),
                   ("GBT (store + calendar)", gbt(BASE)),
                   ("GBT (+ PROMO + holidays)", gbt(WX))]:
    m = metrics(y, pred)
    if base is None:
        base = m["wMAPE"]
    tag = "" if "Seasonal" in name else f"  ({(1 - m['wMAPE'] / base) * 100:+.0f}% vs naive)"
    print(f"  {name:40s}{m['MAE']:9.0f}{m['RMSE']:9.0f}{m['wMAPE']:8.3f}{tag}")
