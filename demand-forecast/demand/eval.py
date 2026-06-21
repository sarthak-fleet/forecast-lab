"""Forecast-accuracy metrics (regression, not ranking).

MAE / RMSE in orders-per-hour; wMAPE = sum|err|/sum(actual) — the demand-industry
standard because it's well-defined when many hours have zero demand.
"""
import numpy as np


def metrics(y_true, y_pred):
    e = np.asarray(y_pred) - np.asarray(y_true)
    return {
        "MAE": round(float(np.mean(np.abs(e))), 3),
        "RMSE": round(float(np.sqrt(np.mean(e ** 2))), 3),
        "wMAPE": round(float(np.sum(np.abs(e)) / np.sum(y_true)), 4),
        "bias": round(float(np.mean(e)), 3),
    }
