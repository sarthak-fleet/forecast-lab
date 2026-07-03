import type { ForecastReport } from './forecast';
import { toCsv } from './csv';

/** Build a downloadable CSV of the per-method accuracy comparison. */
export function metricsToCsv(report: ForecastReport): string {
  const header = ['method', 'MAE', 'RMSE', 'wMAPE', 'bias', 'rank'];
  const rows = report.methods.map((m, i) => [
    m.name,
    m.metrics.MAE,
    m.metrics.RMSE,
    m.metrics.wMAPE,
    m.metrics.bias,
    i + 1,
  ]);
  return toCsv([header, ...rows]);
}

/** Build a downloadable JSON of the full report (actuals + per-method forecasts). */
export function reportToJson(report: ForecastReport): string {
  return JSON.stringify(report, null, 2);
}

/** Build a CSV of the future forecast from every method (one column each). */
export function futureForecastToCsv(report: ForecastReport): string {
  const header = ['horizon_step', ...report.methods.map((m) => m.name)];
  const rows: (string | number)[][] = [header];
  for (let h = 0; h < report.horizon; h++) {
    rows.push([h + 1, ...report.methods.map((m) => round(m.future[h] ?? 0, 3))]);
  }
  return toCsv(rows);
}

function round(x: number, dp: number): number {
  const f = 10 ** dp;
  return Math.round(x * f) / f;
}

/** Trigger a browser download for a text blob. */
export function downloadText(filename: string, text: string, mime = 'text/plain'): void {
  const blob = new Blob([text], { type: `${mime};charset=utf-8` });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = filename;
  document.body.appendChild(a);
  a.click();
  document.body.removeChild(a);
  URL.revokeObjectURL(url);
}
