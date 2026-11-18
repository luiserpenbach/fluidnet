// Parameter study: sweep one parameter and solve the steady state at each value.
import React, { useMemo, useState } from 'react';
import { Engine, RunResult } from './engine';
import { Model, toEngineJson, usedFluids } from './model';
import { ResultSet } from './Results';

export interface StudySpec {
  comp: string;
  key: string;
  from: string;
  to: string;
  n: number;
}

/** "25 bar" -> [25, "bar"] */
export function splitQuantity(v: any): [number, string] | null {
  const m = /^\s*([-+]?(?:\d+\.?\d*|\.\d+)(?:[eE][-+]?\d+)?)\s*(.*?)\s*$/.exec(String(v ?? ''));
  return m ? [Number(m[1]), m[2]] : null;
}

/** Parameters of a component that can be swept: scalar quantities. */
export function sweepable(catalog: any, type: string): any[] {
  const spec = catalog?.components.find((c: any) => c.type === type);
  return (spec?.params ?? []).filter((p: any) => p.kind === 'quantity');
}

/**
 * Run the sweep. The result is a ResultSet whose independent variable is the
 * parameter, so plots, tables, the probe and CSV export work on it unchanged.
 */
export async function runStudy(engine: Engine, model: Model, spec: StudySpec, onPoint: (i: number, n: number) => void, cancelled: () => boolean): Promise<ResultSet> {
  const a = splitQuantity(spec.from);
  const b = splitQuantity(spec.to);
  if (!a || !b) throw new Error('From and To must be numbers, optionally with a unit (for example 10 bar).');
  const unit = a[1] || b[1];
  if (a[1] && b[1] && a[1] !== b[1]) throw new Error(`From and To use different units (${a[1]} and ${b[1]}).`);
  const n = Math.max(2, Math.min(200, Math.round(spec.n)));
  if (!model.components.some((c) => c.id === spec.comp)) throw new Error(`No component ${spec.comp} in the model.`);
  await engine.ensureFluids(usedFluids(model));
  const xs = Array.from({ length: n }, (_, i) => a[0] + ((b[0] - a[0]) * i) / (n - 1));
  const runs: (RunResult | null)[] = [];
  const warnings: string[] = [];
  const t0 = performance.now();
  let first: RunResult | null = null;
  for (let i = 0; i < n; i++) {
    if (cancelled()) throw new Error('stopped');
    onPoint(i, n);
    const value = unit ? `${+xs[i].toPrecision(12)} ${unit}` : +xs[i].toPrecision(12);
    const m: Model = { ...model, components: model.components.map((c) => (c.id === spec.comp ? { ...c, params: { ...c.params, [spec.key]: value } } : c)) };
    try {
      const r = await engine.run(toEngineJson(m), 'steady');
      if (r.meta.error) throw new Error(r.meta.error);
      runs.push(r);
      first ??= r;
      for (const w of r.meta.warnings ?? []) if (!warnings.includes(w)) warnings.push(w);
    } catch (e: any) {
      if (String(e.message) === 'stopped') throw e;
      runs.push(null);
      warnings.unshift(`${spec.comp}.${spec.key} = ${value}: no solution (${String(e.message ?? e).replace(/^model error: /, '')})`);
    }
  }
  if (!first) throw new Error(warnings[0] ?? 'No point of the study could be solved.');
  const nch = first.meta.channels.length;
  const data = new Float64Array(n * (nch + 1)).fill(NaN);
  data.set(xs, 0);
  runs.forEach((r, i) => {
    if (!r || r.meta.channels.length !== nch) return;
    const last = r.meta.n_time - 1;
    for (let c = 0; c < nch; c++) data[(c + 1) * n + i] = r.data[(c + 1) * r.meta.n_time + last];
  });
  const solved = runs.filter(Boolean).length;
  return new ResultSet({
    meta: { ...first.meta, mode: 'study', n_time: n, x: { name: `${spec.comp}.${spec.key}`, unit }, warnings, elapsed_ms: performance.now() - t0, study: { ...spec, solved } },
    data,
  });
}

export function StudyDialog({ model, catalog, initial, selectedId, onRun, onClose }: { model: Model; catalog: any; initial: StudySpec | null; selectedId: string | null; onRun: (s: StudySpec) => void; onClose: () => void }) {
  const comps = useMemo(() => model.components.filter((c) => sweepable(catalog, c.type).length > 0), [model, catalog]);
  const startComp = (selectedId && comps.find((c) => c.id === selectedId)?.id) || (initial && comps.find((c) => c.id === initial.comp)?.id) || comps[0]?.id || '';
  const [comp, setComp] = useState(startComp);
  const c = comps.find((x) => x.id === comp);
  const params = c ? sweepable(catalog, c.type) : [];
  const scalar = (key: string) => (c && !Array.isArray(c.params[key]) ? c.params[key] ?? params.find((p) => p.key === key)?.default : undefined);
  const firstKey = (cc = c) => {
    const ps = cc ? sweepable(catalog, cc.type) : [];
    return (ps.find((p: any) => cc!.params[p.key] != null && !Array.isArray(cc!.params[p.key])) ?? ps[0])?.key ?? '';
  };
  const [key, setKey] = useState(() => (initial && initial.comp === startComp && params.some((p) => p.key === initial.key) ? initial.key : firstKey()));
  const span = (k: string): [string, string] => {
    const q = splitQuantity(scalar(k));
    if (!q) return ['', ''];
    const [v, u] = q;
    const f = (x: number) => `${+x.toPrecision(4)}${u ? ` ${u}` : ''}`;
    return v === 0 ? [f(0), f(1)] : [f(v * 0.5), f(v * 1.5)];
  };
  const keep = initial && initial.comp === startComp && initial.key === key;
  const [from, setFrom] = useState(() => (keep ? initial!.from : span(key)[0]));
  const [to, setTo] = useState(() => (keep ? initial!.to : span(key)[1]));
  const [n, setN] = useState(() => String(initial?.n ?? 11));
  const pick = (k: string) => {
    setKey(k);
    const [f, t] = span(k);
    setFrom(f);
    setTo(t);
  };
  const p = params.find((x) => x.key === key);
  const current = scalar(key);
  const isTable = c && Array.isArray(c.params[key]);
  const ok = !!c && !!p && splitQuantity(from) && splitQuantity(to) && Number(n) >= 2;
  return (
    <div className="modal-back" onClick={onClose}>
      <form
        className="modal narrow"
        onClick={(e) => e.stopPropagation()}
        onSubmit={(e) => {
          e.preventDefault();
          if (ok) onRun({ comp, key, from, to, n: Number(n) });
        }}
      >
        <div className="modal-head">
          <b>Parameter study</b>
          <span className="muted">Solves the steady state for a range of one parameter.</span>
        </div>
        <div className="modal-body">
          {comps.length === 0 ? (
            <div className="muted">Add components to the model first.</div>
          ) : (
            <>
              <div className="grid2">
                <div className="prow">
                  <label htmlFor="st-comp">Component</label>
                  <select
                    id="st-comp"
                    className="field"
                    value={comp}
                    onChange={(e) => {
                      setComp(e.target.value);
                      const cc = comps.find((x) => x.id === e.target.value);
                      const k = firstKey(cc);
                      setKey(k);
                      const q = splitQuantity(cc && !Array.isArray(cc.params[k]) ? cc.params[k] ?? sweepable(catalog, cc.type).find((x: any) => x.key === k)?.default : undefined);
                      const f = (x: number) => `${+x.toPrecision(4)}${q && q[1] ? ` ${q[1]}` : ''}`;
                      setFrom(q ? (q[0] === 0 ? f(0) : f(q[0] * 0.5)) : '');
                      setTo(q ? (q[0] === 0 ? f(1) : f(q[0] * 1.5)) : '');
                    }}
                  >
                    {comps.map((x) => (
                      <option key={x.id} value={x.id}>
                        {x.id}
                      </option>
                    ))}
                  </select>
                </div>
                <div className="prow">
                  <label htmlFor="st-key">Parameter</label>
                  <select id="st-key" className="field" value={key} onChange={(e) => pick(e.target.value)}>
                    {params.map((x: any) => (
                      <option key={x.key} value={x.key}>
                        {x.label}
                      </option>
                    ))}
                  </select>
                </div>
              </div>
              <div className="muted small">
                {p?.help} {isTable ? 'This parameter is a time table in the model; the study replaces it with a constant.' : current != null ? `Model value: ${current}.` : 'Not set in the model.'}
              </div>
              <div className="grid3">
                <div className="prow">
                  <label htmlFor="st-from">From</label>
                  <input id="st-from" className="field mono" value={from} onChange={(e) => setFrom(e.target.value)} placeholder={`e.g. 10 ${p?.unit ?? ''}`} spellCheck={false} />
                </div>
                <div className="prow">
                  <label htmlFor="st-to">To</label>
                  <input id="st-to" className="field mono" value={to} onChange={(e) => setTo(e.target.value)} placeholder={`e.g. 30 ${p?.unit ?? ''}`} spellCheck={false} />
                </div>
                <div className="prow">
                  <label htmlFor="st-n">Points</label>
                  <input id="st-n" className="field mono" type="number" min={2} max={200} value={n} onChange={(e) => setN(e.target.value)} />
                </div>
              </div>
              <div className="muted small">The model itself is not changed. Results open in the plots with the parameter on the x axis; the slider there steps the schematic through the points.</div>
            </>
          )}
        </div>
        <div className="modal-actions">
          <div className="spacer" />
          <button type="button" className="btn-secondary" onClick={onClose}>
            Cancel
          </button>
          <button type="submit" className="btn-primary" id="st-run" disabled={!ok}>
            Run study
          </button>
        </div>
      </form>
    </div>
  );
}
