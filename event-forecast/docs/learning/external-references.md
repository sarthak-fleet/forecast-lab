# External References

Concepts used in the Event Forecast forecasting engine. Each entry: what it is,
why it matters here, authoritative link. No re-explanation of well-documented
material.

---

## Markov Chain (first-order)

**What:** A memoryless probabilistic model where the next state depends only on
the current state, not on the full history.

**Why it matters here:** `event_transitions` in `ForecastModel` is exactly a
first-order Markov transition matrix stored as
`HashMap<current_event_type, HashMap<next_event_type, count>>`.
`predict_event_type` picks the argmax of the row for the current event type,
falling back to global frequency when the current type has no observed successors.

**Source:** [Wikipedia — Markov chain](https://en.wikipedia.org/wiki/Markov_chain)

---

## Transition Matrix

**What:** The matrix representation of a Markov chain where entry `(i, j)` is
the probability of moving from state `i` to state `j`.

**Why it matters here:** `event_transitions` is the unnormalized version of this
matrix. Normalization happens on read: `top_choice` divides a row's winning count
by the row total to get a confidence score.

**Source:** [Wikipedia — Stochastic matrix](https://en.wikipedia.org/wiki/Stochastic_matrix)

---

## Laplace Smoothing (additive smoothing)

**What:** Adding a small pseudocount to every cell of a frequency table so
zero-count transitions get a non-zero probability.

**Why it matters here:** The current engine does NOT apply Laplace smoothing.
Zero-count transitions fall back to the global frequency distribution instead
(see `predict_event_type` fallback path). This is a known limitation noted in the
Phase 5 roadmap entry ("Add seasonal/time-of-day features" implies richer
smoothing will be needed).

**Source:** [Wikipedia — Additive smoothing](https://en.wikipedia.org/wiki/Additive_smoothing)

---

## Median as a robust central tendency estimator

**What:** The median of a dataset is the middle value after sorting. It is
resistant to outliers; the mean is not.

**Why it matters here:** `predict_next_stream` uses the median of per-event-type
inter-arrival intervals to predict the timestamp of the next event. A single
very-long gap (e.g., overnight) would inflate a mean-based timestamp estimate
significantly; the median absorbs it.

**Source:** [Wikipedia — Median](https://en.wikipedia.org/wiki/Median)

---

## Z-score anomaly detection

**What:** A z-score measures how many standard deviations an observation is from
the population mean. Points beyond a threshold (commonly 2 or 3) are flagged as
anomalies.

**Why it matters here:** `detect_anomalies` computes the mean and standard
deviation of all inter-arrival intervals in a stream, then flags any interval
whose z-score exceeds `z_threshold` (default 2.0). This is the simplest univariate
anomaly detector; it assumes intervals are roughly normal.

**Source:** [Wikipedia — Standard score](https://en.wikipedia.org/wiki/Standard_score)

---

## TimescaleDB hypertable

**What:** A TimescaleDB hypertable is a PostgreSQL table partitioned automatically
by time into fixed-size "chunks." Time-range queries skip chunks outside the
range, giving sub-linear scan cost for recent-window queries.

**Why it matters here:** The `events` table is promoted to a hypertable on `ts` in
`0001_timescale_events.sql`. The primary query pattern (`WHERE stream_id = $1 ORDER
BY ts ASC`) benefits from chunk pruning when a `ts` range filter is added.

**Source:** [TimescaleDB docs — Hypertables](https://docs.timescale.com/use-timescale/latest/hypertables/)

---

## Held-out evaluation (train/test split)

**What:** Splitting a dataset into a training portion and a held-out test portion,
fitting a model on training only, then scoring predictions against the test set.

**Why it matters here:** `evaluate_stream` splits the event stream at
`history_ratio` (default 0.6), fits `ForecastModel` on the first 60%, predicts
forward `len(future)` steps, and scores event-type accuracy, per-field property
accuracy, and timestamp error against the actual future events. This is the
primary quality gate before any model upgrade.

**Source:** [Wikipedia — Training, validation, and test data sets](https://en.wikipedia.org/wiki/Training,_validation,_and_test_data_sets)
