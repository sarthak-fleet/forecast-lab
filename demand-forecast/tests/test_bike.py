"""Evaluation protocol guards: timestamp gaps, temporal boundary, and output parity."""
import numpy as np
import pandas as pd
import pytest

from demand.bike import load_bike, temporal_split
from demand.eval import metrics
from run_bike import evaluate


def sample():
    dates = pd.date_range("2012-01-01", periods=600, freq="h")
    return pd.DataFrame({
        "dteday": dates.date.astype(str), "hr": dates.hour,
        "cnt": 20 + dates.hour * 2 + np.arange(len(dates)) % 7,
        "season": 1, "yr": 1, "mnth": dates.month, "holiday": 0,
        "weekday": dates.dayofweek, "workingday": (dates.dayofweek < 5).astype(int),
        "weathersit": 1, "temp": 0.5, "atemp": 0.5, "hum": 0.5, "windspeed": 0.1,
    })


def test_gaps_are_not_silently_treated_as_previous_hour():
    frame = sample().drop(index=300)
    aligned, train, test, _ = temporal_split(frame)
    after_gap = aligned[aligned.timestamp == pd.Timestamp("2012-01-13 13:00")]
    assert after_gap.lag1.isna().all()
    assert train.timestamp.max() < test.timestamp.min()
    assert not train.timestamp.isin(after_gap.timestamp).any()


def test_future_target_changes_do_not_change_features_or_fitted_predictions():
    frame = sample()
    _, train, test, cut = temporal_split(frame)
    changed = frame.copy()
    changed.loc[cut, "cnt"] = 9999
    _, train_changed, test_changed, _ = temporal_split(changed)
    pd.testing.assert_frame_equal(train, train_changed)
    assert test.iloc[0].lag1 == test_changed.iloc[0].lag1
    assert test.iloc[0].lag24 == test_changed.iloc[0].lag24
    first, predictions = evaluate(frame, max_iter=5)
    _, altered = evaluate(changed, max_iter=5)
    for column in predictions.columns[2:]:
        assert predictions.iloc[0][column] == altered.iloc[0][column]
        assert metrics(predictions.actual, predictions[column]) == first["metrics"][column]
    assert first["split"]["test_rows"] == len(predictions)
    # Once observed, a past test target is allowed for the next one-hour forecast.
    assert test_changed.iloc[1].lag1 == 9999


def test_duplicate_timestamps_are_rejected():
    frame = sample()
    with pytest.raises(ValueError, match="Duplicate"):
        temporal_split(pd.concat([frame, frame.iloc[:1]]))


def test_corrupted_cached_input_is_rejected_without_replacement(tmp_path):
    path = tmp_path / "hour.csv"
    path.write_text("unverified data")
    with pytest.raises(ValueError, match="Cached hourly CSV checksum"):
        load_bike(path)
    assert path.read_text() == "unverified data"
