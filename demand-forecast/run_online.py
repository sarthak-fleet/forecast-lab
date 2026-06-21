"""'Improves with time' — walk-forward adaptive loop, hardened.

Three refinements over the basic loop:
  1. REGRET tracking   — cost above the in-hindsight-best τ each week (de-confounds
                          "improvement" from demand size, the trap we hit before).
  2. EMA-smoothed τ    — kill the week-to-week jitter in the learned quantile.
  3. Drift down-weight — recency-weighted residuals so it adapts to non-stationarity.

  cd demand-forecast && python3 run_online.py
"""
import numpy as np
import pandas as pd

from demand.olist import load_olist_panel
from demand.inventory import critical_ratio, policy_cost, weighted_quantile

CU, CO = 1, 3
GRID = [round(0.05 * i, 2) for i in range(1, 19)]
SPAN, VAL = 16, 8
ALPHA = 0.4          # EMA smoothing on τ
HALFLIFE = 12.0      # weeks; drift down-weighting of residuals


def main():
    panel, *_ = load_olist_panel()
    panel = panel.sort_values(["region", "ptype", "week"]).reset_index(drop=True)
    panel["lag1"] = panel.groupby(["region", "ptype"], group_keys=False).y.shift(1)
    weeks = sorted(panel.week.unique())
    cr = critical_ratio(CU, CO)

    rows = []
    tau_ema = cr
    for t in weeks[-SPAN:]:
        hist = panel[panel.week < t].dropna(subset=["lag1"])
        age = ((t - hist.week).dt.days / 7.0).to_numpy()          # weeks before t
        w = 0.5 ** (age / HALFLIFE)                               # drift weights
        resid = (hist.y - hist.lag1).to_numpy()
        cur = panel[panel.week == t].dropna(subset=["lag1"])
        y, l1 = cur.y.to_numpy(), cur.lag1.to_numpy()

        def stock(tau, lag):
            return np.clip(lag + weighted_quantile(resid, w, tau), 0, None)

        # online τ via cost-min on trailing window, then EMA-smoothed
        val = hist[hist.week >= weeks[weeks.index(t) - VAL]]
        vy, vl = val.y.to_numpy(), val.lag1.to_numpy()
        tau_raw = min(GRID, key=lambda tau: policy_cost(stock(tau, vl), vy, CU, CO)["cost"])
        tau_ema = ALPHA * tau_raw + (1 - ALPHA) * tau_ema

        adaptive = policy_cost(stock(tau_ema, l1), y, CU, CO)["cost"]
        static = policy_cost(stock(cr, l1), y, CU, CO)["cost"]
        oracle = min(policy_cost(stock(tau, l1), y, CU, CO)["cost"] for tau in GRID)
        rows.append({"week": t, "tau": round(tau_ema, 3), "adaptive": adaptive,
                     "static": static, "oracle": oracle, "regret": adaptive - oracle})

    df = pd.DataFrame(rows)
    h = len(df) // 2
    print(f"Walk-forward, perishable Cu={CU} Co={CO} (formula CR={cr:.2f}), {SPAN} weeks\n")
    print(f"  cost/cell   static formula = {df.static.mean():.2f}   "
          f"adaptive = {df.adaptive.mean():.2f}   oracle(best-τ) = {df.oracle.mean():.2f}")
    print(f"  REGRET vs oracle   static = {(df.static - df.oracle).mean():.2f}   "
          f"adaptive = {df.regret.mean():.2f}   (lower = closer to optimal)")
    print(f"\n  improves with time?  adaptive REGRET  first half = {df.regret[:h].mean():.2f}   "
          f"second half = {df.regret[h:].mean():.2f}")
    print(f"  τ stability: EMA std = {df.tau.std():.3f}  (was ~0.05 jittery before)")
    print(f"  τ trajectory: {[f'{t:.2f}' for t in df.tau]}")


if __name__ == "__main__":
    main()
