/** A realistic sample time series (trend + weekly seasonality + noise) so the
 * user can see the wedge work without their own data. Mimics daily demand. */
export function sampleDailyCsv(): string {
  const rows: string[] = ['date,value'];
  const start = new Date('2025-01-01T00:00:00Z');
  let base = 120;
  for (let i = 0; i < 120; i++) {
    const d = new Date(start.getTime() + i * 86_400_000);
    const dow = d.getUTCDay();
    const weekly = 1 + 0.35 * Math.sin(((dow + 2) / 7) * Math.PI * 2);
    const trend = 0.6 * i;
    const noise = (Math.sin(i * 1.7) + Math.cos(i * 0.9)) * 8;
    const v = Math.max(0, Math.round(base + trend + 40 * weekly + noise));
    rows.push(`${d.toISOString().slice(0, 10)},${v}`);
  }
  return rows.join('\n');
}
