"""Trend regime: add Holt-Winters (level+trend+seasonality) — the method the rest
of the stack lacks (naive lags a trend; tree models can't extrapolate one).

Hand-rolled additive Holt-Winters (transparent for the explainer) + a log variant
for multiplicative/exponential growth. Run on bike DAILY demand (has trend +
weekly + annual seasonality), multi-step forecast of the held-out tail.

  cd demand-forecast && python3 run_trend.py
"""
import numpy as np
import pandas as pd
from sklearn.ensemble import HistGradientBoostingRegressor

from demand.eval import metrics

H = 56  # forecast horizon (days held out)


def holt_winters(y, m=7, a=0.3, b=0.1, g=0.3, horizon=H):
    """Additive Holt-Winters: level + linear trend + period-m seasonality."""
    y = np.asarray(y, float)
    level = y[:m].mean()
    trend = (y[m:2 * m].mean() - y[:m].mean()) / m
    seas = list(y[:m] - level)
    for t in range(len(y)):
        i = t % m
        prev = level
        level = a * (y[t] - seas[i]) + (1 - a) * (level + trend)
        trend = b * (level - prev) + (1 - b) * trend
        seas[i] = g * (y[t] - level) + (1 - g) * seas[i]
    return np.array([max(0.0, level + (h + 1) * trend + seas[(len(y) + h) % m])
                     for h in range(horizon)])


df = pd.read_csv("data/bike/day.csv")
y = df.cnt.to_numpy(float)
tr_y, te_y = y[:-H], y[-H:]

naive = np.full(H, tr_y[-1])                                   # persistence
seasonal = np.array([tr_y[-7 + (h % 7)] for h in range(H)])    # repeat last week
hw = holt_winters(tr_y)
hw_log = np.expm1(holt_winters(np.log1p(tr_y)))                # multiplicative

FE = ["mnth", "weekday", "workingday", "holiday", "season", "yr",
      "weathersit", "temp", "atemp", "hum", "windspeed"]
gb = HistGradientBoostingRegressor(loss="poisson", max_iter=400, learning_rate=0.05,
                                   random_state=0)
gb.fit(df.iloc[:-H][FE], tr_y)
gbt = np.clip(gb.predict(df.iloc[-H:][FE]), 0, None)

print(f"Bike DAILY demand · forecast last {H} days (multi-step) · mean {te_y.mean():.0f}/day\n")
print(f"  {'method':34s}{'MAE':>8s}{'RMSE':>8s}{'wMAPE':>8s}")
for name, pred in [("Naive (persistence)", naive),
                   ("SeasonalNaive (repeat last week)", seasonal),
                   ("Holt-Winters (trend + weekly)", hw),
                   ("Holt-Winters log (multiplicative)", hw_log),
                   ("GBT (calendar + weather)", gbt)]:
    m = metrics(te_y, pred)
    print(f"  {name:34s}{m['MAE']:8.1f}{m['RMSE']:8.1f}{m['wMAPE']:8.3f}")


# --- Trend-DOMINATED case: Olist weekly orders, train 2017 -> forecast 2018 growth ---
# where a trend method should win and tree models structurally cannot extrapolate.
from demand.olist import load_olist_orders


def holt(yv, a=0.3, b=0.1, horizon=12):
    yv = np.asarray(yv, float)
    level, trend = yv[0], yv[1] - yv[0]
    for t in range(1, len(yv)):
        prev = level
        level = a * yv[t] + (1 - a) * (level + trend)
        trend = b * (level - prev) + (1 - b) * trend
    return np.array([max(0.0, level + (h + 1) * trend) for h in range(horizon)])


o = load_olist_orders(start="2017-01-01", end="2018-03-31")
wk = o.groupby("week").size().sort_index()
tr2 = wk[wk.index < "2018-01-01"].to_numpy(float)
te2 = wk[wk.index >= "2018-01-01"].to_numpy(float)
Hh = len(te2)
idx = np.arange(len(wk)).reshape(-1, 1)
gb2 = HistGradientBoostingRegressor(max_iter=300, random_state=0).fit(idx[:len(tr2)], tr2)

print(f"\nTrend-dominated: Olist weekly orders, train 2017 → forecast {Hh} wks of 2018 "
      f"(actual mean {te2.mean():.0f}/wk, 2017 ended ~{tr2[-1]:.0f}/wk)\n")
for name, pred in [("Naive (flat = last 2017 wk)", np.full(Hh, tr2[-1])),
                   ("GBT (time-index)", np.clip(gb2.predict(idx[len(tr2):]), 0, None)),
                   ("Holt (level + trend)", holt(tr2, horizon=Hh))]:
    m = metrics(te2, pred)
    print(f"  {name:28s} wMAPE={m['wMAPE']:.3f}  predicts ~{pred.mean():.0f}/wk (actual {te2.mean():.0f})")
