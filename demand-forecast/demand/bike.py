"""Pinned UCI Bike Sharing input and timestamp-correct rolling-origin features."""
from hashlib import sha256
from io import BytesIO
from pathlib import Path
from urllib.request import urlopen
from zipfile import ZipFile

import pandas as pd

SOURCE = "https://archive.ics.uci.edu/static/public/275/bike%2Bsharing%2Bdataset.zip"
ARCHIVE_SHA256 = "b70182d0d0508e9abbb79306ce5c0cec34869000f8220175ac83d11dbe845401"
HOUR_SHA256 = "e03de4ee4ef4dc376ac6e04bf829673c6269e8eba5c60fa121640fa2f829504f"
DEFAULT_DATA = Path(__file__).resolve().parents[1] / "data" / "bike" / "hour.csv"
LAGS = ["lag1", "lag24", "lag168"]


def load_bike(path=DEFAULT_DATA):
    """Fetch only the named public CSV; verify cached input on every run."""
    path = Path(path)
    if not path.exists():
        with urlopen(SOURCE, timeout=30) as response:
            archive = response.read(2_000_000)
        if sha256(archive).hexdigest() != ARCHIVE_SHA256:
            raise ValueError("UCI archive checksum changed; review the source before updating")
        with ZipFile(BytesIO(archive)) as zipped:
            csv = zipped.read("hour.csv")
        if sha256(csv).hexdigest() != HOUR_SHA256:
            raise ValueError("Unexpected UCI hourly CSV checksum")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(csv)
    if sha256(path.read_bytes()).hexdigest() != HOUR_SHA256:
        raise ValueError("Cached hourly CSV checksum changed; restore the pinned UCI input")
    return pd.read_csv(path)


def temporal_split(frame, train_fraction=0.8):
    """Fixed training cutoff; each test target may use only earlier observations.

    Missing timestamps stay missing. Row shifts would silently treat a missing
    hour as the preceding hour; exact timestamp lookups avoid that error.
    """
    frame = frame.copy()
    frame["timestamp"] = pd.to_datetime(frame.dteday) + pd.to_timedelta(frame.hr, unit="h")
    frame = frame.sort_values("timestamp").reset_index(drop=True)
    if frame.timestamp.duplicated().any():
        raise ValueError("Duplicate timestamps are ambiguous")
    if not 0 < train_fraction < 1 or len(frame) < 2:
        raise ValueError("A nonempty chronological train and test split is required")
    history = frame.set_index("timestamp").cnt
    for hours in (1, 24, 168):
        frame[f"lag{hours}"] = history.reindex(
            frame.timestamp - pd.Timedelta(hours=hours)
        ).to_numpy()
    cut = int(len(frame) * train_fraction)
    if not 0 < cut < len(frame):
        raise ValueError("A nonempty chronological train and test split is required")
    train = frame.iloc[:cut].dropna(subset=LAGS).copy()
    test = frame.iloc[cut:].dropna(subset=LAGS).copy()
    if train.empty or test.empty:
        raise ValueError("Not enough timestamp-aligned history for weekly lags")
    return frame, train, test, cut
