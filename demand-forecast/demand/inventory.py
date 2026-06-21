"""Newsvendor inventory layer — baked onto the demand forecast.

The business decision: given per-unit understock cost Cu (lost margin on a missed
order) and overstock cost Co (spoilage / holding on an excess unit), the
profit-optimal service level is the **critical ratio**

    CR = Cu / (Cu + Co)

and you stock the CR-quantile of forecast demand. This module turns a cost ratio
straight into a stocking decision + its realized cost — no manual quantile picking.
"""
import numpy as np


class NaiveQuantileForecaster:
    """The winning distributional model: location = lag1 (repeat last week),
    spread = empirical residual quantiles (y - lag1) pooled over train.
    Best-calibrated + simplest of the methods we tried."""

    def fit(self, tr):
        self.resid = (tr.y - tr.lag1).to_numpy()
        return self

    def quantile(self, te, tau):
        return np.clip(te.lag1.to_numpy() + np.quantile(self.resid, tau), 0, None)


def weighted_quantile(values, weights, tau):
    """Recency-weighted empirical quantile (for drift adaptation)."""
    i = np.argsort(values)
    v, w = values[i], weights[i]
    cw = np.cumsum(w)
    cw /= cw[-1]
    return float(np.interp(tau, cw, v))


def critical_ratio(cu, co):
    """Optimal service level = optimal quantile to stock."""
    return cu / (cu + co)


def policy_cost(stock, demand, cu, co):
    under = np.clip(demand - stock, 0, None)
    over = np.clip(stock - demand, 0, None)
    return {
        "service_level": float((stock >= demand).mean()),
        "overstock": float(over.mean()),
        "understock": float(under.mean()),
        "cost": float((cu * under + co * over).mean()),
    }


def optimal_stock(forecaster, te, cu, co):
    """Cost ratio in -> (target service level, per-cell stock quantities) out."""
    tau = critical_ratio(cu, co)
    return tau, forecaster.quantile(te, tau)
