"""Build the demand panel, run naive baselines on a temporal backtest, report.

  cd demand-forecast && python3 run.py
"""
import json
from pathlib import Path

from demand.data import load_panel
from demand.models import GlobalMean, ZoneMean, SeasonalNaive, LastWeek
from demand.eval import metrics

HERE = Path(__file__).resolve().parent


def main():
    panel, train, test, zones = load_panel()
    print(f"zones={len(zones)}  hours={panel.hour.nunique()}  "
          f"train_cells={len(train)}  test_cells={len(test)}  "
          f"mean_demand/zone/hr={panel.y.mean():.1f}  (test = last 7 days)\n")

    rows = []
    for M in [GlobalMean(), ZoneMean(), SeasonalNaive(), LastWeek()]:
        M.fit(train)
        m = metrics(test.y.to_numpy(), M.predict(test))
        m["model"] = M.name
        rows.append(m)
        print(f"{M.name:34s}  MAE={m['MAE']:.3f}  RMSE={m['RMSE']:.3f}  "
              f"wMAPE={m['wMAPE']:.3f}  bias={m['bias']:+.3f}")

    rows.sort(key=lambda r: r["MAE"])
    cols = ["MAE", "RMSE", "wMAPE", "bias"]
    md = ["# Demand backtest — NYC taxi (Jan 2024, last 7 days held out)", "",
          f"{len(zones)} zones · hourly · lower MAE/RMSE/wMAPE = better.", "",
          "| Model | " + " | ".join(cols) + " |", "|" + "---|" * (len(cols) + 1)]
    for r in rows:
        md.append("| " + r["model"] + " | " + " | ".join(str(r[c]) for c in cols) + " |")
    md = "\n".join(md) + "\n"
    (HERE / "results.md").write_text(md)
    (HERE / "results.json").write_text(json.dumps(rows, indent=2))
    print("\n" + md)


if __name__ == "__main__":
    main()
