import type { ForecastReport } from '../lib/forecast';

interface Props {
  report: ForecastReport;
  selectedIndex: number;
  onSelect: (i: number) => void;
}

/** Method comparison table — the ladder, ranked by wMAPE (lower = better). */
export function MetricsTable({ report, selectedIndex, onSelect }: Props) {
  return (
    <div className="overflow-x-auto rounded-lg border border-slate-200 bg-white">
      <table className="w-full text-sm">
        <thead className="bg-slate-50 text-left text-slate-600">
          <tr>
            <th className="px-3 py-2 font-medium">Rank</th>
            <th className="px-3 py-2 font-medium">Method</th>
            <th className="px-3 py-2 text-right font-medium">MAE</th>
            <th className="px-3 py-2 text-right font-medium">RMSE</th>
            <th className="px-3 py-2 text-right font-medium">wMAPE</th>
            <th className="px-3 py-2 text-right font-medium">Bias</th>
          </tr>
        </thead>
        <tbody>
          {report.methods.map((m, i) => {
            const isBest = i === 0;
            const isSel = i === selectedIndex;
            return (
              <tr
                key={m.name}
                onClick={() => onSelect(i)}
                className={`cursor-pointer border-t border-slate-100 transition-colors ${
                  isSel ? 'bg-blue-50' : 'hover:bg-slate-50'
                }`}
              >
                <td className="px-3 py-2 text-slate-500">
                  {isBest ? <span className="font-semibold text-blue-600">{i + 1}</span> : i + 1}
                </td>
                <td className="px-3 py-2 font-medium text-slate-800">
                  {m.name}
                  {isBest && (
                    <span className="ml-2 rounded bg-blue-100 px-1.5 py-0.5 text-[11px] font-semibold text-blue-700">
                      best
                    </span>
                  )}
                </td>
                <td className="px-3 py-2 text-right tabular-nums text-slate-700">{m.metrics.MAE}</td>
                <td className="px-3 py-2 text-right tabular-nums text-slate-700">{m.metrics.RMSE}</td>
                <td className="px-3 py-2 text-right tabular-nums font-semibold text-slate-800">
                  {m.metrics.wMAPE}
                </td>
                <td className="px-3 py-2 text-right tabular-nums text-slate-700">
                  {m.metrics.bias > 0 ? '+' : ''}
                  {m.metrics.bias}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
