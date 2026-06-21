"""Shared gradient-boosting helper — one home for the HistGradientBoosting config
the runners previously copy-pasted. fit_gbt returns a fitted model; gbt_fit_predict
fits and returns clipped predictions in one call.
"""
import numpy as np
from sklearn.ensemble import HistGradientBoostingRegressor


def fit_gbt(X, y, loss="poisson", max_iter=400, learning_rate=0.05, **kw):
    m = HistGradientBoostingRegressor(
        loss=loss, max_iter=max_iter, learning_rate=learning_rate,
        l2_regularization=1.0, random_state=0, **kw)
    m.fit(X, y)
    return m


def gbt_fit_predict(X_tr, y_tr, X_te, clip0=True, **kw):
    p = fit_gbt(X_tr, y_tr, **kw).predict(X_te)
    return np.clip(p, 0, None) if clip0 else p
