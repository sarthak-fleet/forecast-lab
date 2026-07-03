import { useMemo, useState } from 'react';
import { BarChart3, Download, Settings2, TrendingUp } from 'lucide-react';
import { UploadZone } from './components/UploadZone';
import { ForecastChart } from './components/ForecastChart';
import { MetricsTable } from './components/MetricsTable';
import { parseCsv, extractTimeSeries, type TimeSeries } from './lib/csv';
import { runMethodLadder, type ForecastReport } from './lib/forecast';
import { sampleDailyCsv } from './lib/sample';
import { downloadText, futureForecastToCsv, metricsToCsv, reportToJson } from './lib/report';

interface RunConfig {
  testFraction: number;
  seasonality: number;
  horizon: number;
}

export default function App() {
  const [series, setSeries] = useState<TimeSeries | null>(null);
  const [fileName, setFileName] = useState<string | undefined>();
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState(0);
  const [cfg, setCfg] = useState<RunConfig>({ testFraction: 0.2, seasonality: 7, horizon: 14 });
  const [autoSeason, setAutoSeason] = useState(true);

  const report: ForecastReport | null = useMemo(() => {
    if (!series || series.values.length < 8) return null;
    try {
      const r = runMethodLadder(series.values, series.dates, {
        testFraction: cfg.testFraction,
        seasonality: autoSeason ? undefined : cfg.seasonality,
        horizon: cfg.horizon,
      });
      setSelected(0);
      return r;
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
      return null;
    }
  }, [series, cfg, autoSeason]);

  const handleText = (text: string, name: string) => {
    setError(null);
    try {
      const parsed = parseCsv(text);
      if (parsed.headers.length < 2) {
        setError('CSV needs at least two columns (a date column and a value column).');
        return;
      }
      const ts = extractTimeSeries(parsed);
      if (ts.values.length < 8) {
        setError(`Only ${ts.values.length} numeric rows found — need at least 8 to run a backtest.`);
        return;
      }
      setSeries(ts);
      setFileName(name);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };

  const handleSample = () => {
    const csv = sampleDailyCsv();
    handleText(csv, 'sample-daily-demand.csv');
  };

  return (
    <div className="min-h-full bg-slate-50 text-slate-900">
      <header className="bg-slate-900 text-white">
        <div className="mx-auto max-w-5xl px-6 py-8">
          <div className="flex items-center gap-2 text-sm text-slate-300">
            <BarChart3 size={16} />
            <span>forecast-lab</span>
          </div>
          <h1 className="mt-2 text-2xl font-bold sm:text-3xl">Method-ladder forecaster</h1>
          <p className="mt-2 max-w-2xl text-slate-300">
            Upload a time-series CSV. We run the method ladder — naive → seasonal-naive → moving
            average → Holt-Winters → ensemble — on a held-out backtest, score each, and forecast
            the future. The lesson the lab teaches:{' '}
            <span className="font-medium text-white">
              the best method depends on the data regime — measure it, don't assume it.
            </span>
          </p>
        </div>
      </header>

      <main className="mx-auto max-w-5xl px-6 py-8">
        <section className="mb-6">
          <UploadZone
            onFile={handleText}
            onSample={handleSample}
            fileName={fileName}
            rowCount={series?.values.length}
          />
          {error && (
            <p className="mt-3 rounded-lg bg-red-50 px-4 py-2 text-sm text-red-700">{error}</p>
          )}
        </section>

        {report && (
          <>
            <section className="mb-6 rounded-xl border border-slate-200 bg-white p-4">
              <div className="mb-3 flex items-center gap-2 text-sm font-medium text-slate-700">
                <Settings2 size={16} />
                Backtest settings
              </div>
              <div className="grid grid-cols-1 gap-4 sm:grid-cols-3">
                <Slider
                  label="Hold-out fraction"
                  value={cfg.testFraction}
                  min={0.1}
                  max={0.4}
                  step={0.05}
                  fmt={(v) => v.toFixed(2)}
                  onChange={(v) => setCfg((c) => ({ ...c, testFraction: v }))}
                />
                <div>
                  <label className="mb-1 block text-xs font-medium text-slate-600">
                    Seasonality (period)
                  </label>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={1}
                      max={Math.max(1, series?.values.length ?? 1)}
                      value={cfg.seasonality}
                      disabled={autoSeason}
                      onChange={(e) =>
                        setCfg((c) => ({
                          ...c,
                          seasonality: Math.max(1, Number(e.target.value) || 1),
                        }))
                      }
                      className="w-20 rounded-md border border-slate-300 px-2 py-1 text-sm disabled:bg-slate-100 disabled:text-slate-400"
                    />
                    <label className="flex items-center gap-1 text-xs text-slate-600">
                      <input
                        type="checkbox"
                        checked={autoSeason}
                        onChange={(e) => setAutoSeason(e.target.checked)}
                      />
                      auto
                    </label>
                  </div>
                </div>
                <Slider
                  label="Future horizon"
                  value={cfg.horizon}
                  min={1}
                  max={60}
                  step={1}
                  fmt={(v) => String(Math.round(v))}
                  onChange={(v) => setCfg((c) => ({ ...c, horizon: Math.round(v) }))}
                />
              </div>
            </section>

            <section className="mb-6 rounded-xl border border-slate-200 bg-white p-4">
              <div className="mb-2 flex items-center justify-between">
                <h2 className="flex items-center gap-2 text-sm font-semibold text-slate-700">
                  <TrendingUp size={16} />
                  Forecast — {report.methods[selected]?.name}
                </h2>
                <span className="text-xs text-slate-500">
                  {series?.values.length ?? 0} points · backtest from{' '}
                  {report.dates[report.splitIndex] ?? report.splitIndex}
                </span>
              </div>
              <ForecastChart report={report} methodIndex={selected} />
            </section>

            <section className="mb-6">
              <h2 className="mb-2 text-sm font-semibold text-slate-700">Method comparison</h2>
              <MetricsTable report={report} selectedIndex={selected} onSelect={setSelected} />
              <p className="mt-2 text-xs text-slate-500">
                wMAPE = sum|error| / sum(actual) — the demand-industry standard, well-defined with
                zeros. Lower is better. Click a row to plot its forecast above.
              </p>
            </section>

            <section className="mb-10 rounded-xl border border-slate-200 bg-white p-4">
              <h2 className="mb-3 flex items-center gap-2 text-sm font-semibold text-slate-700">
                <Download size={16} />
                Download report
              </h2>
              <div className="flex flex-wrap gap-3">
                <DownloadButton
                  label="Metrics CSV"
                  onClick={() =>
                    downloadText('forecast-lab-metrics.csv', metricsToCsv(report), 'text/csv')
                  }
                />
                <DownloadButton
                  label="Future forecast CSV"
                  onClick={() =>
                    downloadText(
                      'forecast-lab-future.csv',
                      futureForecastToCsv(report),
                      'text/csv',
                    )
                  }
                />
                <DownloadButton
                  label="Full report JSON"
                  onClick={() =>
                    downloadText('forecast-lab-report.json', reportToJson(report), 'application/json')
                  }
                />
              </div>
            </section>
          </>
        )}

        {!report && !error && (
          <p className="text-center text-sm text-slate-400">
            Upload a CSV or load the sample to run the method ladder.
          </p>
        )}
      </main>

      <footer className="border-t border-slate-200 py-6 text-center text-xs text-slate-500">
        forecast-lab · method-ladder forecaster · a consulting wedge from the eval-first ML lab.
      </footer>
    </div>
  );
}

function Slider({
  label,
  value,
  min,
  max,
  step,
  fmt,
  onChange,
}: {
  label: string;
  value: number;
  min: number;
  max: number;
  step: number;
  fmt: (v: number) => string;
  onChange: (v: number) => void;
}) {
  return (
    <div>
      <label className="mb-1 block text-xs font-medium text-slate-600">
        {label} <span className="tabular-nums text-slate-800">({fmt(value)})</span>
      </label>
      <input
        type="range"
        min={min}
        max={max}
        step={step}
        value={value}
        onChange={(e) => onChange(Number(e.target.value))}
        className="w-full accent-blue-600"
      />
    </div>
  );
}

function DownloadButton({ label, onClick }: { label: string; onClick: () => void }) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="inline-flex items-center gap-2 rounded-lg border border-slate-300 bg-white px-3 py-1.5 text-sm font-medium text-slate-700 transition-colors hover:bg-slate-50"
    >
      <Download size={14} />
      {label}
    </button>
  );
}
