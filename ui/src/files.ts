// Saving and exporting: File System Access where the browser has it, a download otherwise.
import type { ResultSet } from './Results';
import { displayUnit } from './model';

export function downloadText(name: string, text: string, mime = 'text/plain') {
  const url = URL.createObjectURL(new Blob([text], { type: mime }));
  const a = document.createElement('a');
  a.href = url;
  a.download = name;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}

export const canPickFiles = () => typeof (window as any).showSaveFilePicker === 'function' && window.self === window.top;

export const fileStem = (name: string) => name.trim().replace(/[^\w\-]+/g, '_').replace(/^_+|_+$/g, '').toLowerCase() || 'model';

/**
 * Write the model file. Reuses `handle` (an earlier Save or Open) unless
 * `saveAs`; returns the handle written to, null after a plain download, or
 * undefined when the user cancelled.
 */
export async function saveText(text: string, suggested: string, handle: any, saveAs: boolean): Promise<any | null | undefined> {
  if (!canPickFiles()) {
    downloadText(suggested, text, 'application/yaml');
    return null;
  }
  try {
    const h =
      !saveAs && handle
        ? handle
        : await (window as any).showSaveFilePicker({ suggestedName: suggested, types: [{ description: 'fluidnet model', accept: { 'application/yaml': ['.yaml', '.yml'] } }] });
    const w = await h.createWritable();
    await w.write(text);
    await w.close();
    return h;
  } catch (e: any) {
    if (e?.name === 'AbortError') return undefined;
    throw e;
  }
}

const csvCell = (s: string) => (/[",\n]/.test(s) ? `"${s.replace(/"/g, '""')}"` : s);

/** Results as CSV in display units; all channels, or the named ones. */
export function resultsCsv(rs: ResultSet, names?: string[]): string {
  const chans: string[] = names?.length ? names.filter((n) => rs.has(n)) : rs.meta.channels.map((c: any) => c.name);
  const units = chans.map((n) => displayUnit(rs.unit(n)));
  const head = [rs.xName + (rs.xUnit ? ` [${rs.xUnit}]` : ''), ...chans.map((n, i) => `${n}${units[i].label ? ` [${units[i].label}]` : ''}`)];
  const cols = chans.map((n) => rs.series(n)!);
  const x = rs.time;
  const lines = [head.map(csvCell).join(',')];
  for (let k = 0; k < rs.nt; k++) {
    const row = [String(x[k])];
    for (let i = 0; i < cols.length; i++) {
      const v = cols[i][k] * units[i].scale;
      row.push(Number.isFinite(v) ? String(+v.toPrecision(9)) : '');
    }
    lines.push(row.join(','));
  }
  return lines.join('\n') + '\n';
}
