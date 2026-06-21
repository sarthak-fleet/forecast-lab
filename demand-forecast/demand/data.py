"""NYC taxi -> hourly per-zone DEMAND panel + temporal split.

Proxy for Swiggy/Zomato demand: pickups-per-zone-per-hour == orders per area per
window. Reuses the parquet already downloaded under event-forecast/.
"""
from pathlib import Path
import pandas as pd

TAXI = Path(__file__).resolve().parents[2] / "event-forecast" / "data" / "nyc-taxi"
PARQUET = TAXI / "yellow_tripdata_2024-01.parquet"
ZONES = TAXI / "taxi_zone_lookup.csv"


def load_panel(min_zone_total=2000, test_days=7):
    zl = pd.read_csv(ZONES)
    keep = set(zl[zl.Borough != "Unknown"].LocationID)
    zone_name = dict(zip(zl.LocationID, zl.Borough + " / " + zl.Zone))

    df = pd.read_parquet(PARQUET, columns=["tpep_pickup_datetime", "PULocationID"])
    df = df[(df.tpep_pickup_datetime >= "2024-01-01") &
            (df.tpep_pickup_datetime < "2024-02-01")]
    df = df[df.PULocationID.isin(keep)]
    df["hour"] = df.tpep_pickup_datetime.dt.floor("h")

    g = df.groupby(["PULocationID", "hour"]).size().rename("y").reset_index()

    totals = g.groupby("PULocationID")["y"].sum()
    zones = totals[totals >= min_zone_total].index
    hours = pd.date_range("2024-01-01", "2024-01-31 23:00", freq="h")
    panel = (pd.MultiIndex.from_product([zones, hours], names=["PULocationID", "hour"])
             .to_frame(index=False)
             .merge(g, on=["PULocationID", "hour"], how="left"))
    panel["y"] = panel.y.fillna(0).astype(float)
    panel["dow"] = panel.hour.dt.dayofweek
    panel["hod"] = panel.hour.dt.hour
    panel["zone_name"] = panel.PULocationID.map(zone_name)

    cutoff = pd.Timestamp("2024-02-01") - pd.Timedelta(days=test_days)
    train = panel[panel.hour < cutoff].copy()
    test = panel[panel.hour >= cutoff].copy()
    return panel, train, test, zones
