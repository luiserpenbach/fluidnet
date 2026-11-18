import React, { useEffect, useState } from 'react';
import { Comp, Model, Sim, displayUnit, fmt } from './model';
import { FLUIDS } from './engine';
import { Symbol } from './symbols';
import type { AlignMode } from './graph';
import type { ResultSet } from './Results';
import { SERIES_COLORS } from './Results';

function valueToText(v: any): string {
  if (v === null || v === undefined) return '';
  return String(v);
}

function textToValue(t: string): any {
  const s = t.trim();
  if (s === '') return null;
  const n = Number(s);
  return isFinite(n) && /^[-+0-9.eE]+$/.test(s) ? n : s;
}

function tableToText(v: any[][]): string {
  return v.map((r) => (Array.isArray(r) ? r.join(', ') : String(r))).join('\n');
}

function textToTable(t: string): any[][] {
  return t
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => l.split(',').map((c) => textToValue(c)));
}

/** Text field that commits on blur / Enter, so typing does not re-run checks per keystroke. */
function Field({ id, value, placeholder, onCommit, mono = true }: { id: string; value: string; placeholder?: string; onCommit: (s: string) => void; mono?: boolean }) {
  const [v, setV] = useState(value);
  useEffect(() => setV(value), [value]);
  return (
    <input
      id={id}
      className={`field${mono ? ' mono' : ''}`}
      value={v}
      placeholder={placeholder}
      spellCheck={false}
      onChange={(e) => setV(e.target.value)}
      onBlur={() => v !== value && onCommit(v)}
      onKeyDown={(e) => {
        if (e.key === 'Enter') (e.target as HTMLInputElement).blur();
        if (e.key === 'Escape') setV(value);
      }}
    />
  );
}

function TableField({ id, value, onCommit }: { id: string; value: any[][]; onCommit: (v: any[][]) => void }) {
  const text = tableToText(value);
  const [v, setV] = useState(text);
  useEffect(() => setV(text), [text]);
  return (
    <textarea
      id={id}
      className="field mono table-field"
      rows={Math.max(3, value.length + 1)}
      value={v}
      spellCheck={false}
      onChange={(e) => setV(e.target.value)}
      onBlur={() => v !== text && onCommit(textToTable(v))}
    />
  );
}

/** Shape of a two-column schedule, so a typo in a time table is visible at a glance. */
function Sparkline({ table }: { table: any[][] }) {
  const pts = table.map((r) => [Number(r?.[0]), parseFloat(String(r?.[1]))]).filter(([t, v]) => isFinite(t) && isFinite(v));
  if (pts.length < 2) return null;
  const t0 = pts[0][0];
  const t1 = pts[pts.length - 1][0];
  const lo = Math.min(...pts.map((p) => p[1]));
  const hi = Math.max(...pts.map((p) => p[1]));
  if (!(t1 > t0)) return null;
  const W = 280;
  const H = 34;
  const x = (t: number) => 2 + ((W - 4) * (t - t0)) / (t1 - t0);
  const y = (v: number) => (hi > lo ? H - 3 - ((H - 6) * (v - lo)) / (hi - lo) : H / 2);
  return (
    <svg className="spark" viewBox={`0 0 ${W} ${H}`} preserveAspectRatio="none" aria-hidden>
      <polyline points={pts.map(([t, v]) => `${x(t).toFixed(1)},${y(v).toFixed(1)}`).join(' ')} fill="none" stroke="currentColor" strokeWidth={1.25} vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

function ParamRow({ comp, p, model, onChange }: { comp: Comp; p: any; model: Model; onChange: (key: string, v: any) => void }) {
  const raw = comp.params[p.key];
  const isTable = Array.isArray(raw);
  const fid = `p-${comp.id}-${p.key}`;
  let input: React.ReactNode;
  if (p.kind === 'fluid') {
    input = (
      <select id={fid} className="field" value={raw ?? ''} onChange={(e) => onChange(p.key, e.target.value || null)}>
        <option value="">{p.required ? 'Select fluid' : 'From connected parts'}</option>
        {FLUIDS.map((f) => (
          <option key={f.key} value={f.key}>
            {f.key} · {f.label}
          </option>
        ))}
      </select>
    );
  } else if (p.kind === 'ref') {
    const opts = model.components.filter((c) => c.type === p.target);
    input = (
      <select id={fid} className="field" value={raw ?? ''} onChange={(e) => onChange(p.key, e.target.value || null)}>
        <option value="">{opts.length ? 'None' : `No ${p.target.replace('_', ' ')} in the model`}</option>
        {opts.map((c) => (
          <option key={c.id} value={c.id}>
            {c.id}
          </option>
        ))}
      </select>
    );
  } else if (p.kind === 'choice') {
    input = (
      <select id={fid} className="field" value={raw ?? p.default ?? ''} onChange={(e) => onChange(p.key, e.target.value)}>
        {p.choices.map((c: string) => (
          <option key={c} value={c}>
            {c.replace(/_/g, ' ')}
          </option>
        ))}
      </select>
    );
  } else if (p.kind === 'bool' && !isTable) {
    const on = raw === undefined || raw === null ? String(p.default) === 'true' : raw === true || raw === 'true';
    input = (
      <label className="check">
        <input id={fid} type="checkbox" checked={on} onChange={(e) => onChange(p.key, e.target.checked)} />
        {on ? 'Yes' : 'No'}
      </label>
    );
  } else if (isTable) {
    input = (
      <>
        <TableField id={fid} value={raw} onCommit={(v) => onChange(p.key, v)} />
        {p.schedulable && <Sparkline table={raw} />}
      </>
    );
  } else {
    const ph = p.default != null ? String(p.default) : p.required ? `required (${p.unit || p.si})` : `optional (${p.unit || p.si})`;
    input = <Field id={fid} value={valueToText(raw)} placeholder={ph} onCommit={(s) => onChange(p.key, textToValue(s))} />;
  }
  return (
    <div className="prow">
      <div className="prow-head">
        <label htmlFor={fid} title={p.help}>
          {p.label}
          {p.required && <span className="req">*</span>}
        </label>
        <span className="prow-unit">{p.kind === 'quantity' ? p.unit || p.si : ''}</span>
        {p.schedulable && (
          <button
            className="mini"
            title={isTable ? 'Use a constant value' : 'Use a time table: one "t, value" pair per line'}
            onClick={() => {
              if (isTable) onChange(p.key, raw.length ? raw[raw.length - 1][1] : null);
              else {
                const base = raw ?? p.default ?? 0;
                onChange(p.key, [[0, base], [1, base]]);
              }
            }}
          >
            {isTable ? 'const' : 'f(t)'}
          </button>
        )}
      </div>
      {input}
      <div className="prow-help">{p.help}</div>
    </div>
  );
}

/** Channels of one component at the probed time; click a row to plot it. */
function CompResults({ comp, rs, k, plotted, onToggle }: { comp: Comp; rs: ResultSet; k: number; plotted: string[]; onToggle: (name: string) => void }) {
  const rows: [string, string][] = [];
  const add = (label: string, name: string) => rs.has(name) && rows.push([label, name]);
  const info = rs.meta.components.find((c: any) => c.id === comp.id);
  if (!info) return null;
  for (const [port, node] of info.ports as [string, string][]) {
    if (node === comp.id) continue;
    add(`P ${port}`, `${node}.P`);
    add(`T ${port}`, `${node}.T`);
  }
  for (const c of rs.meta.channels) if (c.name.startsWith(`${comp.id}.`) && !c.name.slice(comp.id.length + 1).includes('.')) rows.push([c.name.slice(comp.id.length + 1), c.name]);
  if (!rows.length) return null;
  return (
    <>
      <div className="section-label">
        Results{rs.mode === 'steady' ? '' : ` at ${rs.xName} = ${fmt(rs.time[k], 4)}${rs.xUnit ? ` ${rs.xUnit}` : ''}`}
      </div>
      <div className="cres">
        {rows.map(([label, name]) => {
          const du = displayUnit(rs.unit(name));
          const i = plotted.indexOf(name);
          return (
            <button key={`${label}:${name}`} className={`cres-row${i >= 0 ? ' on' : ''}`} onClick={() => onToggle(name)} title={i >= 0 ? `Remove ${name} from the plots` : `Plot ${name}`}>
              <i style={{ background: i >= 0 ? SERIES_COLORS[i % SERIES_COLORS.length] : 'transparent' }} />
              <span>{label}</span>
              <b className="num">{fmt(rs.at(name, k) * du.scale, 5)}</b>
              <span className="prow-unit">{du.label}</span>
            </button>
          );
        })}
      </div>
    </>
  );
}

const ALIGN: [AlignMode, string][] = [
  ['left', 'Align left'],
  ['hcenter', 'Align centres'],
  ['right', 'Align right'],
  ['top', 'Align top'],
  ['vcenter', 'Align middles'],
  ['bottom', 'Align bottom'],
  ['hdist', 'Space evenly across'],
  ['vdist', 'Space evenly down'],
];

export function Inspector(props: {
  model: Model;
  selected: Comp | null;
  multi: Comp[];
  nConn: number;
  catalog: any;
  check: any;
  errId: string | null;
  rs: ResultSet | null;
  k: number;
  plotted: string[];
  onTogglePlot: (name: string) => void;
  onParam: (id: string, key: string, v: any) => void;
  onRename: (id: string, newId: string) => string | null;
  onFlip: () => void;
  onRotate: () => void;
  onDuplicate: () => void;
  onDelete: () => void;
  onAlign: (mode: AlignMode) => void;
  onSelect: (id: string) => void;
  onModel: (patch: Partial<Model>) => void;
  onSim: (patch: Partial<Sim>) => void;
}) {
  const { model, selected, catalog, check, multi } = props;
  const [renameErr, setRenameErr] = useState<string | null>(null);
  useEffect(() => setRenameErr(null), [selected?.id]);
  if (multi.length > 1 || (!selected && props.nConn > 0)) {
    const types = new Map<string, number>();
    for (const c of multi) types.set(c.type, (types.get(c.type) ?? 0) + 1);
    return (
      <div className="inspector">
        <div className="section-label">Selection</div>
        <div className="kv">
          {multi.length > 0 && (
            <>
              <span>Components</span>
              <b className="num">{multi.length}</b>
            </>
          )}
          {props.nConn > 0 && (
            <>
              <span>Connections</span>
              <b className="num">{props.nConn}</b>
            </>
          )}
        </div>
        {multi.length > 0 && <div className="muted small">{[...types.entries()].map(([t, n]) => `${n} ${t.replace('_', ' ')}`).join(', ')}</div>}
        <div className="insp-actions">
          {multi.length > 0 && (
            <>
              <button className="btn-ghost" onClick={props.onDuplicate}>
                Duplicate
              </button>
              <button className="btn-ghost" onClick={props.onRotate}>
                Rotate
              </button>
            </>
          )}
          <button className="btn-ghost danger" onClick={props.onDelete}>
            Delete
          </button>
        </div>
        {multi.length > 1 && (
          <>
            <div className="section-label">Arrange</div>
            <div className="arrange">
              {ALIGN.map(([mode, label]) => (
                <button key={mode} className="btn-secondary small" disabled={mode.endsWith('dist') && multi.length < 3} onClick={() => props.onAlign(mode)}>
                  {label}
                </button>
              ))}
            </div>
          </>
        )}
      </div>
    );
  }
  if (selected) {
    const spec = catalog?.components.find((c: any) => c.type === selected.type);
    if (!spec) return <div className="inspector">Unknown component type {selected.type}</div>;
    return (
      <div className="inspector">
        <div className="insp-head">
          <div className="insp-sym">
            <Symbol type={selected.type} />
          </div>
          <div>
            <div className="insp-type">{spec.label}</div>
            <Field
              id={`rename-${selected.id}`}
              value={selected.id}
              mono
              onCommit={(s) => {
                const t = s.trim().replace(/[^A-Za-z0-9_\-]/g, '_');
                setRenameErr(t && t !== selected.id ? props.onRename(selected.id, t) : null);
              }}
            />
          </div>
        </div>
        {renameErr && <div className="msg msg-err">{renameErr}</div>}
        {props.errId === selected.id && check?.error && <div className="msg msg-err">{String(check.error).replace(/^model error: /, '')}</div>}
        <p className="insp-help">{spec.help}</p>
        <div className="insp-actions">
          <button className="btn-ghost" onClick={props.onRotate} title="Rotate a quarter turn (R)">
            Rotate
          </button>
          <button className="btn-ghost" onClick={props.onFlip} title="Mirror the symbol (F)">
            Flip
          </button>
          <button className="btn-ghost" onClick={props.onDuplicate} title="Duplicate">
            Duplicate
          </button>
          <button className="btn-ghost danger" onClick={props.onDelete} title="Delete">
            Delete
          </button>
        </div>
        <div className="section-label">Parameters</div>
        {spec.params.map((p: any) => (
          <ParamRow key={p.key} comp={selected} p={p} model={model} onChange={(k, v) => props.onParam(selected.id, k, v)} />
        ))}
        {props.rs && <CompResults comp={selected} rs={props.rs} k={props.k} plotted={props.plotted} onToggle={props.onTogglePlot} />}
      </div>
    );
  }
  const s = model.simulation;
  return (
    <div className="inspector">
      <div className="section-label">System</div>
      <div className="prow">
        <label htmlFor="m-desc">Description</label>
        <textarea
          id="m-desc"
          className="field"
          rows={3}
          value={model.description}
          onChange={(e) => props.onModel({ description: e.target.value })}
        />
      </div>
      <div className="section-label">Transient run</div>
      <div className="grid2">
        <div className="prow">
          <label htmlFor="s-tend">End time</label>
          <div className="with-unit">
            <Field id="s-tend" value={String(s.t_end)} onCommit={(v) => props.onSim({ t_end: Math.max(1e-9, Number(v) || s.t_end) })} />
            <span>s</span>
          </div>
        </div>
        <div className="prow">
          <label htmlFor="s-dt">Output interval</label>
          <div className="with-unit">
            <Field
              id="s-dt"
              value={s.output_dt == null ? '' : String(s.output_dt)}
              placeholder={String(+(s.t_end / 500).toPrecision(3))}
              onCommit={(v) => props.onSim({ output_dt: v.trim() === '' ? null : Number(v) })}
            />
            <span>s</span>
          </div>
        </div>
        <div className="prow">
          <label htmlFor="s-rtol">Relative tolerance</label>
          <select id="s-rtol" className="field" value={String(s.rtol)} onChange={(e) => props.onSim({ rtol: Number(e.target.value) })}>
            {[...new Set(['1e-2', '1e-3', '1e-4', '1e-5', '1e-6'].map((r) => String(Number(r))).concat(String(s.rtol)))].map((r) => (
              <option key={r} value={r}>
                {Number(r).toExponential(0)}
              </option>
            ))}
          </select>
        </div>
        <div className="prow">
          <label htmlFor="s-max">Max step</label>
          <div className="with-unit">
            <Field
              id="s-max"
              value={s.max_step == null ? '' : String(s.max_step)}
              placeholder="auto"
              onCommit={(v) => props.onSim({ max_step: v.trim() === '' ? null : Number(v) })}
            />
            <span>s</span>
          </div>
        </div>
      </div>
      <label className="check">
        <input id="s-steady" type="checkbox" checked={s.start_from_steady} onChange={(e) => props.onSim({ start_from_steady: e.target.checked })} />
        Start from the steady state at t = 0
      </label>
      <div className="prow">
        <label htmlFor="s-steadyt" title="Schedules (valve openings, boundary pressures) are evaluated at this time for the steady-state solve">
          Steady state evaluated at
        </label>
        <div className="with-unit">
          <Field id="s-steadyt" value={s.steady_time == null ? '' : String(s.steady_time)} placeholder="0" onCommit={(v) => props.onSim({ steady_time: v.trim() === '' ? null : Number(v) })} />
          <span>s</span>
        </div>
      </div>
      <div className="section-label">Model check</div>
      {check?.error ? (
        <div className="msg msg-err">
          {String(check.error).replace(/^model error: /, '')}
          {props.errId && (
            <button className="mini" onClick={() => props.onSelect(props.errId!)}>
              show {props.errId}
            </button>
          )}
        </div>
      ) : check ? (
        <div className="kv">
          <span>States</span>
          <b className="num">{check.states}</b>
          <span>Volumes and boundaries</span>
          <b className="num">{check.nodes}</b>
          <span>Flow elements</span>
          <b className="num">{check.branches}</b>
        </div>
      ) : (
        <div className="muted">Add components to begin.</div>
      )}
      {check?.warnings?.map((w: string, i: number) => (
        <div key={i} className="msg msg-warn">
          {w}
        </div>
      ))}
      <div className="section-label">Tips</div>
      <ul className="tips">
        <li>Drag parts from the left onto the canvas, or onto a line to insert them into it.</li>
        <li>Connect ports by dragging between them. Click a line to select it, Delete removes it.</li>
        <li>Values accept units: <code>300 bar</code>, <code>20 degC</code>, <code>6 mm</code>, <code>50 L</code>.</li>
        <li>Use f(t) on a parameter for a time table, e.g. a valve opening.</li>
        <li>Right-click for more, and press <code>?</code> for the shortcuts.</li>
      </ul>
    </div>
  );
}
