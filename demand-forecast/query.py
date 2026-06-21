"""The query surface: select (area, product, time) -> expected count + the odds.

For any cell we return the *predictive distribution* of demand, read straight off
that cell's own history (optionally a seasonal slot) — so the odds are
scale-correct and calibrated by construction, and naturally wider where the cell
is volatile/sparse. That's the honest "estimate + odds + count" the system is for.

  cd demand-forecast && python3 query.py
"""
import numpy as np
import pandas as pd

from demand.olist import load_olist_orders

O = load_olist_orders()
WEEKS = pd.date_range(O.week.min(), O.week.max(), freq="W-MON")
MONTH = ["", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"]


def forecast(area, product, month=None, service=0.90):
    df = O[(O.state == area) & (O.category == product)]
    weeks = WEEKS[WEEKS.month == month] if month else WEEKS
    if month:
        df = df[df.month == month]
    series = df.groupby("week").size().reindex(weeks, fill_value=0).to_numpy()

    when = MONTH[month] + " weeks" if month else "any week"
    print(f"\n▶ {area} / {product} / {when}")
    if series.sum() == 0:
        print("   no demand on record for this cell.")
        return
    p10, p50, p90 = np.percentile(series, [10, 50, 90])
    stock = int(np.ceil(np.quantile(series, service)))
    exp = series.mean()
    thr = max(1, int(round(p50)))
    print(f"   expected ~{exp:.1f} orders/week  (median {p50:.0f})")
    print(f"   odds: 80% of weeks land in [{p10:.0f}, {p90:.0f}]  ·  "
          f"{(series >= thr).mean() * 100:.0f}% chance ≥ {thr}  ·  "
          f"{(series > p90).mean() * 100:.0f}% chance you exceed {p90:.0f}")
    print(f"   → stock {stock} to cover ~{service:.0%} of weeks "
          f"(history shows that's met {(series <= stock).mean() * 100:.0f}% of the time)")


def main():
    print("Demand query — select area / product / time → count + odds")
    forecast("SP", "health_beauty")              # dense cell → tight odds
    forecast("SP", "health_beauty", month=11)    # seasonal slot (November)
    forecast("SP", "toys", month=11)             # gift archetype in gift season
    forecast("RS", "perfumery")                  # thinner cell → wider odds
    forecast("AC", "furniture_decor")            # tiny state → near-zero / uncertain


if __name__ == "__main__":
    main()
