"""Olist e-commerce -> 3-axis demand panel: (region-group x product-type x week) -> order count.

The item axis forces bucketing (sparsity): top states + 'other' for place, top
categories + 'other' for product-type, weekly buckets for time. Reuses the Olist
CSVs already downloaded under event-forecast/.
"""
from pathlib import Path
import numpy as np
import pandas as pd

OLIST = Path(__file__).resolve().parents[2] / "event-forecast" / "data" / "olist"


def _read(name, **kw):
    return pd.read_csv(OLIST / name, **kw)


def load_olist_orders(start="2017-01-01", end="2018-08-31"):
    """Order-level records (state, category, ts, week, month) — for clustering."""
    trans = _read("product_category_name_translation.csv", encoding="utf-8-sig")
    cat_en = dict(zip(trans.product_category_name, trans.product_category_name_english))
    prod = _read("olist_products_dataset.csv")
    prod_cat = {pid: cat_en.get(c, "unknown") if isinstance(c, str) else "unknown"
                for pid, c in zip(prod.product_id, prod.product_category_name)}
    items1 = _read("olist_order_items_dataset.csv")
    items1 = items1[items1.order_item_id == 1]
    order_cat = dict(zip(items1.order_id, items1.product_id.map(prod_cat)))
    cust = _read("olist_customers_dataset.csv")
    cust_state = dict(zip(cust.customer_id, cust.customer_state))
    o = _read("olist_orders_dataset.csv", parse_dates=["order_purchase_timestamp"])
    o = o[["order_id", "customer_id", "order_purchase_timestamp"]].dropna()
    o["state"] = o.customer_id.map(cust_state)
    o["category"] = o.order_id.map(order_cat)
    o = o.dropna(subset=["state", "category"])
    o = o[(o.order_purchase_timestamp >= start) & (o.order_purchase_timestamp <= end)]
    o = o.rename(columns={"order_purchase_timestamp": "ts"})
    o["week"] = o.ts.dt.to_period("W").dt.start_time
    o["month"] = o.ts.dt.month
    return o[["order_id", "state", "category", "ts", "week", "month"]]


def load_olist_panel(n_states=5, n_cats=10, start="2017-01-01", end="2018-08-31", test_weeks=8):
    trans = _read("product_category_name_translation.csv", encoding="utf-8-sig")
    cat_en = dict(zip(trans.product_category_name, trans.product_category_name_english))
    prod = _read("olist_products_dataset.csv")
    prod_cat = {pid: cat_en.get(c, "unknown") if isinstance(c, str) else "unknown"
                for pid, c in zip(prod.product_id, prod.product_category_name)}
    items = _read("olist_order_items_dataset.csv")
    items1 = items[items.order_item_id == 1]
    order_cat = dict(zip(items1.order_id, items1.product_id.map(prod_cat)))
    cust = _read("olist_customers_dataset.csv")
    cust_state = dict(zip(cust.customer_id, cust.customer_state))

    o = _read("olist_orders_dataset.csv", parse_dates=["order_purchase_timestamp"])
    o = o[["order_id", "customer_id", "order_purchase_timestamp"]].dropna()
    o["state"] = o.customer_id.map(cust_state)
    o["category"] = o.order_id.map(order_cat)
    o = o.dropna(subset=["state", "category"])
    o = o[(o.order_purchase_timestamp >= start) & (o.order_purchase_timestamp <= end)]
    o["week"] = o.order_purchase_timestamp.dt.to_period("W").dt.start_time

    top_states = o.state.value_counts().head(n_states).index
    top_cats = o.category.value_counts().head(n_cats).index
    o["region"] = np.where(o.state.isin(top_states), o.state, "other")
    o["ptype"] = np.where(o.category.isin(top_cats), o.category, "other")

    g = o.groupby(["region", "ptype", "week"]).size().rename("y").reset_index()
    regions = sorted(o.region.unique())
    ptypes = sorted(o.ptype.unique())
    weeks = pd.date_range(g.week.min(), g.week.max(), freq="W-MON")
    panel = (pd.MultiIndex.from_product([regions, ptypes, weeks], names=["region", "ptype", "week"])
             .to_frame(index=False)
             .merge(g, on=["region", "ptype", "week"], how="left"))
    panel["y"] = panel.y.fillna(0).astype(float)

    cutoff = weeks[-test_weeks]
    train = panel[panel.week < cutoff].copy()
    test = panel[panel.week >= cutoff].copy()
    return panel, train, test, cutoff
