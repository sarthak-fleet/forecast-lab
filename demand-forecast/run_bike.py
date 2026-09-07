"""Reproducible one-hour rolling-origin evaluation on UCI Bike Sharing.

Run: uv run --frozen python run_bike.py
Actual prior-hour demand is observed before each prediction; the models are fit
once before the held-out tail. Observed target-hour weather is an oracle
comparison, not a deployable weather forecast.
"""
import argparse
import json
from pathlib import Path

import numpy as np
from threadpoolctl import threadpool_limits

from demand.bike import HOUR_SHA256, SOURCE, load_bike, temporal_split
from demand.eval import metrics
from demand.gbt import gbt_fit_predict

CAL = ["season", "yr", "mnth", "hr", "holiday", "weekday", "workingday", "lag24", "lag168"]
WX = CAL + ["weathersit", "temp", "atemp", "hum", "windspeed"]


def evaluate(frame, max_iter=500):
    frame, train, test, cut = temporal_split(frame)
    predictions = {
        "Persistence (previous hour)": test.lag1.to_numpy(),
        "Seasonal naive (24 hours)": test.lag24.to_numpy(),
        "Seasonal naive (168 hours)": test.lag168.to_numpy(),
    }
    # Bound CPU use and numerical scheduling; all fitting stays before cutoff.
    with threadpool_limits(limits=1):
        for name, features in [("GBT (calendar + past demand)", CAL),
                               ("GBT (+ observed weather ORACLE)", WX)]:
            predictions[name] = gbt_fit_predict(
                train[features], train.cnt, test[features], max_iter=max_iter,
            )
    y = test.cnt.to_numpy()
    scored = {name: metrics(y, prediction) for name, prediction in predictions.items()}
    report = {
        "dataset": "UCI Bike Sharing, hourly 2011-2012",
        "source": SOURCE,
        "citation": "Fanaee-T, H. (2013). Bike Sharing. doi:10.24432/C5W894",
        "license": "CC BY 4.0",
        "hour_csv_sha256": HOUR_SHA256,
        "protocol": "Fixed models, rolling one-hour predictions; past test observations available",
        "weather_limit": "Target-hour observed weather is an oracle, not forecast-available input",
        "split": {
            "rows": len(frame), "cutoff": str(frame.iloc[cut].timestamp),
            "train_rows": len(train), "test_rows": len(test),
            "train_start": str(train.timestamp.min()), "train_end": str(train.timestamp.max()),
            "test_start": str(test.timestamp.min()), "test_end": str(test.timestamp.max()),
            "excluded_train_missing_lags": cut - len(train),
            "excluded_test_missing_lags": len(frame) - cut - len(test),
        },
        "model": {"max_iter": max_iter, "random_state": 0, "threads": 1},
        "metrics": scored,
        "units": "MAE/RMSE/bias: rentals per hour; wMAPE: absolute error / actual total",
    }
    output = test[["timestamp", "cnt"]].rename(columns={"cnt": "actual"}).copy()
    for name, prediction in predictions.items():
        if not np.isfinite(prediction).all():
            raise ValueError(f"Non-finite predictions: {name}")
        output[name] = prediction
    return report, output


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=Path(__file__).parent / "data" / "bike-evaluation")
    args = parser.parse_args()
    report, predictions = evaluate(load_bike())
    args.output.mkdir(parents=True, exist_ok=True)
    (args.output / "metrics.json").write_text(json.dumps(report, indent=2) + "\n")
    predictions.to_csv(args.output / "predictions.csv", index=False)
    print(f"Bike Sharing: {report['split']['test_rows']} scored held-out hours")
    print(report["protocol"])
    print(report["weather_limit"])
    print(f"{'method':38s}{'MAE':>10s}{'RMSE':>10s}{'wMAPE':>10s}")
    for name, score in report["metrics"].items():
        print(f"{name:38s}{score['MAE']:10.3f}{score['RMSE']:10.3f}{score['wMAPE']:10.4f}")
    print(f"Split, provenance and metrics: {args.output / 'metrics.json'}")
    print(f"Actuals and predictions: {args.output / 'predictions.csv'}")


if __name__ == "__main__":
    main()
