"""Bake the inventory decision into the forecast: cost ratio -> optimal stock.

Shows that the same demand model, fed different cost structures, automatically
picks different service levels — and that the newsvendor formula CR=Cu/(Cu+Co)
actually lands at the realized-cost-minimizing stock level.

  cd demand-forecast && python3 run_newsvendor.py
"""
import numpy as np

from demand.olist import load_olist_panel
from demand.features import build_features
from demand.inventory import NaiveQuantileForecaster, critical_ratio, policy_cost, optimal_stock


def main():
    panel, train, test, cutoff = load_olist_panel()
    tr, te = build_features(panel, train, cutoff)
    y = te.y.to_numpy()
    f = NaiveQuantileForecaster().fit(tr)

    scenarios = [
        ("Perishable food   (overstock hurts)  Cu=1 Co=3", 1, 3),
        ("Balanced                              Cu=1 Co=1", 1, 1),
        ("High-margin/scarce (stockout hurts)   Cu=4 Co=1", 4, 1),
    ]
    print("Cost structure -> automatic stocking decision (Olist test set):\n")
    for name, cu, co in scenarios:
        tau, stock = optimal_stock(f, te, cu, co)
        r = policy_cost(stock, y, cu, co)
        print(f"{name}")
        print(f"   target service CR={tau:.2f}  ->  achieved={r['service_level']:.2f}  "
              f"overstock={r['overstock']:.1f}/cell  understock={r['understock']:.1f}/cell  "
              f"cost={r['cost']:.2f}/cell\n")

    # Validate the formula: sweep tau, confirm the cost-minimizing tau ~= CR.
    cu, co = 1, 3
    cr = critical_ratio(cu, co)
    print(f"Validation — Cu={cu} Co={co}  (formula CR={cr:.2f}); realized cost by tau:")
    best = (None, 1e9)
    for tau in [0.10, 0.20, 0.25, 0.30, 0.40, 0.50, 0.70, 0.90]:
        c = policy_cost(f.quantile(te, tau), y, cu, co)["cost"]
        if c < best[1]:
            best = (tau, c)
        print(f"   tau={tau:.2f}  cost/cell={c:.2f}" + ("   <- formula CR" if abs(tau - cr) < 1e-9 else ""))
    print(f"\n   empirical cost-min at tau={best[0]:.2f}; formula says {cr:.2f} "
          f"(gap = calibration error — aim higher when the forecaster under-covers).")


if __name__ == "__main__":
    main()
