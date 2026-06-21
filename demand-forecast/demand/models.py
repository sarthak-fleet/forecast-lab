"""Naive demand baselines — the bar that any ML model must beat."""
import numpy as np
import pandas as pd


class GlobalMean:
    name = "GlobalMean"
    def fit(self, train):
        self.mu = train.y.mean()
    def predict(self, test):
        return np.full(len(test), self.mu)


class ZoneMean:
    name = "ZoneMean"
    def fit(self, train):
        self.m = train.groupby("PULocationID").y.mean()
        self.glob = train.y.mean()
    def predict(self, test):
        return test.PULocationID.map(self.m).fillna(self.glob).to_numpy()


class SeasonalNaive:
    """Average by (zone, day-of-week, hour-of-day) — the demand 'beat popularity'."""
    name = "SeasonalNaive (zone x dow x hod)"
    def fit(self, train):
        self.m = train.groupby(["PULocationID", "dow", "hod"]).y.mean()
        self.zone = train.groupby("PULocationID").y.mean()
        self.glob = train.y.mean()
    def predict(self, test):
        s = self.m.reindex(list(zip(test.PULocationID, test.dow, test.hod))).to_numpy()
        s = np.where(np.isnan(s), test.PULocationID.map(self.zone).to_numpy(), s)
        return np.where(np.isnan(s), self.glob, s)


class LastWeek:
    """Predict (zone, hour) = same zone & hour one week earlier (lag 168h)."""
    name = "LastWeek (lag 168h)"
    def fit(self, train):
        self.hist = train.set_index(["PULocationID", "hour"]).y
        self.season = train.groupby(["PULocationID", "dow", "hod"]).y.mean()
        self.glob = train.y.mean()
    def predict(self, test):
        prev = test.hour - pd.Timedelta(days=7)
        s = self.hist.reindex(list(zip(test.PULocationID, prev))).to_numpy()
        sfill = self.season.reindex(list(zip(test.PULocationID, test.dow, test.hod))).to_numpy()
        s = np.where(np.isnan(s), sfill, s)
        return np.where(np.isnan(s), self.glob, s)
