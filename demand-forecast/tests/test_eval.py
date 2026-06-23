"""Smoke tests for demand-forecast eval metrics."""
import numpy as np
from demand.eval import metrics


def test_perfect_predictions():
    y = [10, 20, 30, 0, 5]
    m = metrics(y, y)
    assert m["MAE"] == 0
    assert m["RMSE"] == 0
    assert m["wMAPE"] == 0
    assert m["bias"] == 0


def test_known_values():
    y_true = [10, 20, 30]
    y_pred = [12, 18, 33]
    m = metrics(y_true, y_pred)
    assert m["MAE"] == round((2 + 2 + 3) / 3, 3)
    assert m["bias"] == round((2 - 2 + 3) / 3, 3)


def test_wmape_zero_actual():
    """wMAPE should handle zero-actual rows without NaN."""
    y_true = [0, 10, 0, 20]
    y_pred = [1, 10, 0, 20]
    m = metrics(y_true, y_pred)
    assert m["wMAPE"] >= 0
    assert not np.isnan(m["wMAPE"])
