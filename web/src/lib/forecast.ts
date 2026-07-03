/**
 * Method-ladder forecast — TypeScript port of the demand-forecast lab's
 * statistical methods. The lab's thesis: the best method depends on the
 * data regime, so measure everything on a held-out backtest and never trust
 * a model that doesn't beat the naive baseline.
 *
 * Rungs ported (see demand-forecast/docs/EXPLAINER.md):
 *   1. Naive (last value)            — repeat last period
 *   2. Seasonal-naive                — same period last cycle
 *   3. Moving average                — mean of recent periods
 *   4. Holt-Winters (level+trend+seas) — exp smoothing; the trend method trees can't be
 *   5. Ensemble (naive + seasonal)   — anchored downside
 *
 * GBT (gradient boosting on lags+calendar+exogenous) is intentionally NOT
 * ported: it needs heavy native deps and exogenous columns the upload CSV
 * doesn't carry. The statistical ladder above is the illustrative core and
 * tells the regime story (naive vs seasonal vs trend) that the lab teaches.
 */

export interface Metrics {
  MAE: number;
  RMSE: number;
  wMAPE: number;
  bias: number;
}

export interface MethodResult {
  name: string;
  /** Forecast over the held-out backtest window (aligned to actuals). */
  backtest: number[];
  /** Forecast over the future horizon (beyond the uploaded series). */
  future: number[];
  metrics: Metrics;
}

export interface ForecastReport {
  /** ISO date (or index label) per row of the input series. */
  dates: string[];
  /** Observed values. */
  actuals: number[];
  /** Index where the backtest window starts (train ends here). */
  splitIndex: number;
  /** Horizon length for the future forecast. */
  horizon: number;
  /** Per-method results, sorted by wMAPE ascending (best first). */
  methods: MethodResult[];
  /** Best method name by wMAPE. */
  best: string;
}

export interface ForecastOptions {
  /** Fraction of the series to hold out for the backtest (0.1–0.4). */
  testFraction?: number;
  /** Seasonal period for seasonal-naive / Holt-Winters (e.g. 7 daily, 24 hourly). */
  seasonality?: number;
  /** Future forecast horizon (number of periods). */
  horizon?: number;
}

const clampNonNeg = (x: number): number => (x < 0 || !Number.isFinite(x) ? 0 : x);

/** Industry-standard point-accuracy metrics (see demand-forecast/demand/eval.py). */
export function metrics(yTrue: number[], yPred: number[]): Metrics {
  const n = yTrue.length;
  if (n === 0) return { MAE: 0, RMSE: 0, wMAPE: 0, bias: 0 };
  let absSum = 0;
  let sqSum = 0;
  let biasSum = 0;
  let actualSum = 0;
  for (let i = 0; i < n; i++) {
    const e = (yPred[i] ?? 0) - (yTrue[i] ?? 0);
    absSum += Math.abs(e);
    sqSum += e * e;
    biasSum += e;
    actualSum += Math.abs(yTrue[i] ?? 0);
  }
  return {
    MAE: round(absSum / n, 3),
    RMSE: round(Math.sqrt(sqSum / n), 3),
    wMAPE: actualSum > 0 ? round(absSum / actualSum, 4) : 0,
    bias: round(biasSum / n, 3),
  };
}

function round(x: number, dp: number): number {
  const f = 10 ** dp;
  return Math.round(x * f) / f;
}

/** 1. Naive — repeat the last observed value. */
function naiveForecast(train: number[], horizon: number): number[] {
  const last = train.at(-1) ?? 0;
  return Array.from({ length: horizon }, () => last);
}

/** 2. Seasonal-naive — repeat the value from one season ago. */
function seasonalNaiveForecast(train: number[], horizon: number, m: number): number[] {
  const n = train.length;
  if (m <= 0 || m > n) return naiveForecast(train, horizon);
  return Array.from({ length: horizon }, (_, h) => train[n - m + ((h % m + m) % m)] ?? train.at(-1) ?? 0);
}

/** 3. Moving average — mean of the last `window` periods, held flat. */
function movingAverageForecast(train: number[], horizon: number, window: number): number[] {
  const w = Math.min(window, train.length);
  if (w <= 0) return naiveForecast(train, horizon);
  let sum = 0;
  for (let i = train.length - w; i < train.length; i++) sum += train[i] ?? 0;
  const mean = sum / w;
  return Array.from({ length: horizon }, () => mean);
}

/**
 * 4. Additive Holt-Winters — level + linear trend + period-m seasonality.
 * Ported from demand-forecast/run_trend.py `holt_winters`. The trend method
 * the rest of the ladder lacks: trees can't extrapolate a trend, this can.
 */
function holtWintersForecast(
  train: number[],
  horizon: number,
  m: number,
  a = 0.3,
  b = 0.1,
  g = 0.3,
): number[] {
  const y = train.map((v) => Number(v) || 0);
  const n = y.length;
  if (n < 2 * m) {
    // Not enough data to initialise seasonals — degrade to a simple Holt trend.
    return holtLinearForecast(y, horizon, a, b);
  }
  let level = mean(y.slice(0, m));
  let trend = (mean(y.slice(m, 2 * m)) - mean(y.slice(0, m))) / m;
  const seas = y.slice(0, m).map((v) => v - level);
  for (let t = 0; t < n; t++) {
    const i = t % m;
    const prev = level;
    level = a * (y[t] - seas[i]) + (1 - a) * (level + trend);
    trend = b * (level - prev) + (1 - b) * trend;
    seas[i] = g * (y[t] - level) + (1 - g) * seas[i];
  }
  const out: number[] = [];
  for (let h = 0; h < horizon; h++) {
    out.push(clampNonNeg(level + (h + 1) * trend + seas[(n + h) % m]));
  }
  return out;
}

/** Holt's linear (level + trend, no seasonality) — fallback when seasonals can't init. */
function holtLinearForecast(y: number[], horizon: number, a = 0.3, b = 0.1): number[] {
  if (y.length < 2) return naiveForecast(y, horizon);
  let level = y[0];
  let trend = y[1] - y[0];
  for (let t = 1; t < y.length; t++) {
    const prev = level;
    level = a * y[t] + (1 - a) * (level + trend);
    trend = b * (level - prev) + (1 - b) * trend;
  }
  return Array.from({ length: horizon }, (_, h) => clampNonNeg(level + (h + 1) * trend));
}

function mean(xs: number[]): number {
  if (xs.length === 0) return 0;
  return xs.reduce((s, v) => s + (Number(v) || 0), 0) / xs.length;
}

/**
 * Build a method's backtest predictions by fitting on train[:cut] and
 * forecasting the held-out tail, then refitting on the full train for the
 * future forecast. Walk-forward is approximated as a single-origin forecast
 * (one-step multi-horizon) — matches the lab's `run_trend.py` style.
 */
function backtestPredictions(
  fullTrain: number[],
  splitIndex: number,
  seasonality: number,
  forecastFn: (train: number[], horizon: number, m: number) => number[],
): { backtest: number[]; future: number[] } {
  const trainBeforeHoldout = fullTrain.slice(0, splitIndex);
  const holdout = fullTrain.slice(splitIndex);
  const backtest = forecastFn(trainBeforeHoldout, holdout.length, seasonality);
  const future = forecastFn(fullTrain, 0, seasonality).length
    ? forecastFn(fullTrain, 0, seasonality)
    : [];
  return { backtest, future };
}

/** Run the full method ladder over a univariate series. */
export function runMethodLadder(
  values: number[],
  dates: string[],
  opts: ForecastOptions = {},
): ForecastReport {
  const testFraction = opts.testFraction ?? 0.2;
  const seasonality = opts.seasonality ?? detectSeasonality(values);
  const horizon = opts.horizon ?? Math.max(7, Math.round(values.length * (testFraction)));

  const splitIndex = Math.max(1, Math.floor(values.length * (1 - testFraction)));
  const train = values.slice(0, splitIndex);
  const test = values.slice(splitIndex);

  const maWindow = Math.min(seasonality, Math.max(3, Math.round(seasonality)));

  type Method = { name: string; fn: (tr: number[], h: number, m: number) => number[] };
  const methods: Method[] = [
    { name: 'Naive (last value)', fn: (tr, h) => naiveForecast(tr, h) },
    { name: `Seasonal-naive (m=${seasonality})`, fn: (tr, h, m) => seasonalNaiveForecast(tr, h, m) },
    { name: `Moving average (w=${maWindow})`, fn: (tr, h) => movingAverageForecast(tr, h, maWindow) },
    { name: 'Holt-Winters (level+trend+seas)', fn: (tr, h, m) => holtWintersForecast(tr, h, m) },
    {
      name: 'Ensemble (naive + seasonal)',
      fn: (tr, h, m) => {
        const a = naiveForecast(tr, h);
        const b = seasonalNaiveForecast(tr, h, m);
        return a.map((v, i) => 0.5 * v + 0.5 * (b[i] ?? v));
      },
    },
  ];

  const results: MethodResult[] = methods.map((m) => {
    const { backtest, future } = backtestPredictions(values, splitIndex, seasonality, m.fn);
    // Re-derive future with the requested horizon (backtestPredictions used 0
    // to compute a placeholder; recompute properly here).
    const futureHorizon = m.fn(train, horizon, seasonality);
    return {
      name: m.name,
      backtest,
      future: futureHorizon,
      metrics: metrics(test, backtest),
    };
  });

  results.sort((a, b) => a.metrics.wMAPE - b.metrics.wMAPE);
  const best = results[0]?.name ?? '';

  return {
    dates,
    actuals: values,
    splitIndex,
    horizon,
    methods: results,
    best,
  };
}

/**
 * Heuristic seasonality detection from the series length / spacing.
 * The user can override via the UI. For daily data 7 is the natural cycle;
 * for hourly 24; otherwise fall back to a small window.
 */
export function detectSeasonality(values: number[]): number {
  const n = values.length;
  if (n >= 3 * 24) return 24; // looks hourly
  if (n >= 3 * 7) return 7; // looks daily
  if (n >= 3 * 12) return 12; // looks monthly
  return Math.max(2, Math.min(7, Math.round(n / 4)));
}
