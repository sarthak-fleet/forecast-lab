"""Explainer cell: Bike-Sharing (hourly demand WITH weather + calendar).

The regime Olist/taxi lacked — real exogenous signal. Isolates the lift at each
step so you can SEE what each method/feature-set buys:
  naive → seasonal-naive → GBT(calendar) → GBT(calendar+weather)

  cd demand-forecast && python3 run_bike.py
"""
import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingRegressor

from demand.eval import metrics

df = pd.read_csv("data/bike/hour.csv").sort_values("instant").reset_index(drop=True)
df["lag1"] = df.cnt.shift(1)        # last hour
df["lag24"] = df.cnt.shift(24)      # same hour yesterday
df["lag168"] = df.cnt.shift(168)    # same hour last week

cut = int(len(df) * 0.8)            # temporal backtest: last 20% held out
tr = df.iloc[:cut].dropna(subset=["lag1", "lag24", "lag168"]).copy()
te = df.iloc[cut:].copy()
y = te.cnt.to_numpy()

CAL = ["season", "yr", "mnth", "hr", "holiday", "weekday", "workingday", "lag24", "lag168"]
WX = CAL + ["weathersit", "temp", "atemp", "hum", "windspeed"]


def gbt(feats):
    m = HistGradientBoostingRegressor(loss="poisson", max_iter=500, learning_rate=0.05,
                                      l2_regularization=1.0, random_state=0)
    m.fit(tr[feats], tr.cnt)
    return np.clip(m.predict(te[feats]), 0, None)


rows = [
    ("Naive (lag-1 hour)", te.lag1.to_numpy()),
    ("SeasonalNaive (lag-24, same hr yest.)", te.lag24.to_numpy()),
    ("SeasonalNaive (lag-168, same hr last wk)", te.lag168.to_numpy()),
    ("GBT (calendar only)", gbt(CAL)),
    ("GBT (calendar + WEATHER)", gbt(WX)),
]
print(f"Bike-Sharing hourly demand · {len(df)} rows · last 20% held out · "
      f"mean {y.mean():.0f}/hr\n")
print(f"  {'method':42s}{'MAE':>8s}{'RMSE':>8s}{'wMAPE':>8s}")
base = None
for name, pred in rows:
    m = metrics(y, pred)
    if base is None:
        base = m["wMAPE"]
    tag = "" if name.startswith("Naive") else f"  ({(1 - m['wMAPE'] / base) * 100:+.0f}% vs naive)"
    print(f"  {name:42s}{m['MAE']:8.1f}{m['RMSE']:8.1f}{m['wMAPE']:8.3f}{tag}")
