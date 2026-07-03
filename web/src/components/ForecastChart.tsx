import { useMemo } from 'react';
import type { ForecastReport } from '../lib/forecast';

interface Props {
  report: ForecastReport;
  /** Which method index to overlay on the chart (best by default). */
  methodIndex?: number;
}

const W = 760;
const H = 320;
const PAD = { l: 48, r: 16, t: 16, b: 40 };

/**
 * Inline-SVG forecast chart — same no-dependency philosophy as the lab's
 * viz.py report. Plots the full actual series, the held-out backtest
 * forecast for the selected method, and that method's future horizon.
 */
export function ForecastChart({ report, methodIndex = 0 }: Props) {
  const method = report.methods[methodIndex];

  const { paths, xTicks, yTicks, splitX } = useMemo(() => {
    const all = [
      ...report.actuals,
      ...method.backtest,
      ...method.future,
    ].filter(Number.isFinite);
    const yMax = Math.max(...all, 1);
    const yMin = Math.min(...all, 0);
    const total = report.actuals.length + report.horizon;
    const plotW = W - PAD.l - PAD.r;
    const plotH = H - PAD.t - PAD.b;

    const x = (i: number): number => PAD.l + (i / Math.max(1, total - 1)) * plotW;
    const y = (v: number): number =>
      PAD.t + plotH - ((v - yMin) / Math.max(1e-9, yMax - yMin)) * plotH;

    const actualPts = report.actuals.map((v, i) => `${x(i)},${y(v)}`).join(' ');
    const backtestStart = report.splitIndex;
    const backtestPts = method.backtest
      .map((v, i) => `${x(backtestStart + i)},${y(v)}`)
      .join(' ');
    const futurePts = method.future
      .map((v, i) => `${x(report.actuals.length + i)},${y(v)}`)
      .join(' ');

    const yStep = niceStep(yMax - yMin, 4);
    const ticks = Array.from({ length: 5 }, (_, i) => yMin + i * yStep);
    const yt = ticks.filter((t) => t <= yMax + 1e-9);

    const xTickCount = Math.min(6, total);
    const xt = Array.from({ length: xTickCount }, (_, i) =>
      Math.round((i / (xTickCount - 1)) * (total - 1)),
    );

    return {
      paths: { actualPts, backtestPts, futurePts, x, y },
      xTicks: xt,
      yTicks: yt,
      splitX: x(report.splitIndex),
    };
  }, [report, method]);

  return (
    <svg viewBox={`0 0 ${W} ${H}`} width="100%" role="img" aria-label="Forecast chart">
      {/* y grid + labels */}
      {yTicks.map((t) => (
        <g key={`y${t}`}>
          <line
            x1={PAD.l}
            x2={W - PAD.r}
            y1={paths.y(t)}
            y2={paths.y(t)}
            stroke="#e2e8f0"
            strokeWidth={1}
          />
          <text x={PAD.l - 8} y={paths.y(t) + 4} fontSize={11} textAnchor="end" fill="#64748b">
            {fmt(t)}
          </text>
        </g>
      ))}
      {/* x labels */}
      {xTicks.map((i) => (
        <text
          key={`x${i}`}
          x={paths.x(i)}
          y={H - PAD.b + 18}
          fontSize={10}
          textAnchor="middle"
          fill="#64748b"
        >
          {report.dates[i] ?? i}
        </text>
      ))}
      {/* train/test split marker */}
      <line
        x1={splitX}
        x2={splitX}
        y1={PAD.t}
        y2={H - PAD.b}
        stroke="#f59e0b"
        strokeWidth={1.5}
        strokeDasharray="4 3"
      />
      <text x={splitX + 4} y={PAD.t + 12} fontSize={10} fill="#b45309">
        backtest start
      </text>
      {/* actuals */}
      <polyline points={paths.actualPts} fill="none" stroke="#0f172a" strokeWidth={2} />
      {/* backtest forecast */}
      <polyline points={paths.backtestPts} fill="none" stroke="#2563eb" strokeWidth={2} />
      {/* future forecast */}
      <polyline points={paths.futurePts} fill="none" stroke="#16a34a" strokeWidth={2} strokeDasharray="5 3" />
      {/* legend */}
      <g transform={`translate(${PAD.l + 8}, ${H - PAD.b - 56})`} fontSize={11} fill="#334155">
        <rect x={-4} y={-12} width={150} height={56} fill="#ffffffcc" rx={6} />
        <line x1={0} x2={18} y1={0} y2={0} stroke="#0f172a" strokeWidth={2} />
        <text x={24} y={4}>actuals</text>
        <line x1={0} x2={18} y1={16} y2={16} stroke="#2563eb" strokeWidth={2} />
        <text x={24} y={20}>backtest ({method.name})</text>
        <line x1={0} x2={18} y1={32} y2={32} stroke="#16a34a" strokeWidth={2} strokeDasharray="5 3" />
        <text x={24} y={36}>future forecast</text>
      </g>
    </svg>
  );
}

function fmt(v: number): string {
  if (Math.abs(v) >= 1000) return v.toFixed(0);
  if (Math.abs(v) >= 10) return v.toFixed(1);
  return v.toFixed(2);
}

function niceStep(range: number, count: number): number {
  if (range <= 0) return 1;
  const raw = range / count;
  const mag = 10 ** Math.floor(Math.log10(raw));
  const norm = raw / mag;
  const nice = norm < 1.5 ? 1 : norm < 3 ? 2 : norm < 7 ? 5 : 10;
  return nice * mag;
}
