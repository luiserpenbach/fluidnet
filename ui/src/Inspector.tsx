import React, { useEffect, useState } from 'react';
import { Comp, Model, Sim } from './model';
import { FLUIDS } from './engine';
import { Symbol } from './symbols';

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
  return v.map((r) => `${r[0]}, ${r[1]}`).join('\n');
}

function textToTable(t: string): any[][] {
  return t
    .split('\n')
    .map((l) => l.trim())
    .filter(Boolean)
    .map((l) => {
      const [a, ...rest] = l.split(',');
      return [textToValue(a), textToValue(rest.join(','))];
    });
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

function ParamRow({ comp, p, onChange }: { comp: Comp; p: any; onChange: (key: string, v: any) => void }) {
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
  } else if (isTable) {
    input = <TableField id={fid} value={raw} onCommit={(v) => onChange(p.key, v)} />;
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

export function Inspector(props: {
  model: Model;
  selected: Comp | null;
  catalog: any;
  check: any;
  onParam: (id: string, key: string, v: any) => void;
  onRename: (id: string, newId: string) => void;
  onFlip: (id: string) => void;
  onDelete: (id: string) => void;
  onModel: (patch: Partial<Model>) => void;
  onSim: (patch: Partial<Sim>) => void;
}) {
  const { model, selected, catalog, check } = props;
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
                if (t && t !== selected.id) props.onRename(selected.id, t);
              }}
            />
          </div>
        </div>
        <p className="insp-help">{spec.help}</p>
        <div className="insp-actions">
          <button className="btn-ghost" onClick={() => props.onFlip(selected.id)}>
            Flip
          </button>
          <button className="btn-ghost danger" onClick={() => props.onDelete(selected.id)}>
            Delete
          </button>
        </div>
        <div className="section-label">Parameters</div>
        {spec.params.map((p: any) => (
          <ParamRow key={p.key} comp={selected} p={p} onChange={(k, v) => props.onParam(selected.id, k, v)} />
        ))}
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
            {['1e-2', '1e-3', '1e-4', '1e-5', '1e-6'].map((r) => (
              <option key={r} value={String(Number(r))}>
                {r}
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
      <div className="section-label">Model check</div>
      {check?.error ? (
        <div className="msg msg-err">{check.error}</div>
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
        <li>Drag parts from the left onto the canvas and connect ports by dragging between them.</li>
        <li>Values accept units: <code>300 bar</code>, <code>20 degC</code>, <code>6 mm</code>, <code>50 L</code>.</li>
        <li>Use f(t) on a parameter for a time table, e.g. a valve opening.</li>
        <li>Ctrl+Enter runs the transient. Shift+Enter solves the steady state.</li>
      </ul>
    </div>
  );
}
