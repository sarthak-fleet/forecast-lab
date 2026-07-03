/**
 * Minimal RFC-4180-ish CSV parser — no external dep. Handles quoted fields,
 * embedded commas, and CRLF line endings. Sufficient for the time-series
 * uploads this wedge targets (date + value columns).
 */
export interface ParsedCsv {
  headers: string[];
  rows: string[][];
}

export function parseCsv(text: string): ParsedCsv {
  const cleaned = text.replace(/^\uFEFF/, '').replace(/\r\n/g, '\n').replace(/\r/g, '\n');
  const rows: string[][] = [];
  let field = '';
  let row: string[] = [];
  let inQuotes = false;

  for (let i = 0; i < cleaned.length; i++) {
    const ch = cleaned[i];
    if (inQuotes) {
      if (ch === '"') {
        if (cleaned[i + 1] === '"') {
          field += '"';
          i++;
        } else {
          inQuotes = false;
        }
      } else {
        field += ch;
      }
    } else if (ch === '"') {
      inQuotes = true;
    } else if (ch === ',') {
      row.push(field);
      field = '';
    } else if (ch === '\n') {
      row.push(field);
      rows.push(row);
      row = [];
      field = '';
    } else {
      field += ch;
    }
  }
  if (field.length > 0 || row.length > 0) {
    row.push(field);
    rows.push(row);
  }

  const nonEmpty = rows.filter((r) => r.some((c) => c.trim() !== ''));
  if (nonEmpty.length === 0) return { headers: [], rows: [] };
  const headers = nonEmpty[0].map((h) => h.trim());
  return { headers, rows: nonEmpty.slice(1) };
}

export interface TimeSeries {
  dates: string[];
  values: number[];
  dateColumn: string;
  valueColumn: string;
}

/** Pick the best-guess date and value columns from parsed headers. */
export function pickColumns(headers: string[]): { dateCol: string; valueCol: string } {
  const lower = headers.map((h) => h.toLowerCase());
  const dateCol =
    headers.find((_, i) => /date|time|ds|timestamp|day|week|month/.test(lower[i])) ?? headers[0];
  const valueCol =
    headers.find((_, i) => {
      if (lower[i] === dateCol.toLowerCase()) return false;
      return /value|y|count|qty|quantity|demand|sales|amount|target/.test(lower[i]);
    }) ??
    headers.find((h) => h !== dateCol) ??
    headers[1] ??
    headers[0];
  return { dateCol, valueCol };
}

/**
 * Extract an ordered univariate time series from parsed CSV. Rows with a
 * non-numeric value are dropped. Dates are kept as strings (sorted by row
 * order — the CSV is assumed to be already chronological).
 */
export function extractTimeSeries(parsed: ParsedCsv): TimeSeries {
  const { dateCol, valueCol } = pickColumns(parsed.headers);
  const dateIdx = parsed.headers.indexOf(dateCol);
  const valueIdx = parsed.headers.indexOf(valueCol);
  const dates: string[] = [];
  const values: number[] = [];
  for (const r of parsed.rows) {
    const raw = r[valueIdx]?.trim();
    if (raw === undefined || raw === '') continue;
    const v = Number(raw);
    if (!Number.isFinite(v)) continue;
    dates.push(r[dateIdx]?.trim() ?? String(dates.length));
    values.push(v);
  }
  return { dates, values, dateColumn: dateCol, valueColumn: valueCol };
}

/** Serialise a report-friendly CSV from a 2-D array. */
export function toCsv(rows: (string | number)[][]): string {
  return rows
    .map((r) =>
      r
        .map((cell) => {
          const s = String(cell);
          return /[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s;
        })
        .join(','),
    )
    .join('\n');
}
