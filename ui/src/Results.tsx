import React, { useEffect, useMemo, useRef, useState } from 'react';
import uPlot from 'uplot';
import { displayUnit, fmt } from './model';
import { downloadText, fileStem, resultsCsv } from './files';
import type { RunResult } from './engine';

export const SERIES_COLORS = ['#2563c9', '#d9433f', '#0f9488', '#d9711c', '#7c4dcc', '#4b5563', '#c0397a', '#3f8f3f', '#a16207', '#0e7aa6'];

export class ResultSet {
  meta: any;
  data: Float64Array;
  nt: number;
  index = new Map<string, number>();
  constructor(r: RunResult) {
    this.meta = r.meta;
    this.data = r.data;
    this.nt = r.meta.n_time;
    r.meta.channels.forEach((c: any, i: number) => this.index.set(c.name, i));
  }
  /** The independent variable: time, or the swept parameter of a study. */
  get time(): Float64Array {
    return this.data.subarray(0, this.nt);
  }
  get mode(): 'steady' | 'transient' | 'study' {
    return this.meta.mode;
  }
  get xName(): string {
    return this.meta.x?.name ?? 't';
  }
  get xUnit(): string {
    return this.meta.x?.unit ?? 's';
  }
  /** Index of the sample nearest to x. */
  nearest(x: number): number {
    const t = this.time;
    let lo = 0;
    let hi = this.nt - 1;
    if (hi <= 0) return 0;
    const asc = t[hi] >= t[0];
    while (hi - lo > 1) {
      const mid = (lo + hi) >> 1;
      if (t[mid] <= x === asc) lo = mid;
      else hi = mid;
    }
    return Math.abs(t[lo] - x) <= Math.abs(t[hi] - x) ? lo : hi;
  }
  has(name: string) {
    return this.index.has(name);
  }
  unit(name: string): string {
    const i = this.index.get(name);
    return i === undefined ? '' : this.meta.channels[i].unit;
  }
  series(name: string): Float64Array | null {
    const i = this.index.get(name);
    if (i === undefined) return null;
    return this.data.subarray((i + 1) * this.nt, (i + 2) * this.nt);
  }
  at(name: string, k: number): number {
    const s = this.series(name);
    return s ? s[Math.min(Math.max(k, 0), this.nt - 1)] : NaN;
  }
  /** node name for component port */
  portNode(comp: string, port: string): string | null {
    const c = this.meta.components.find((x: any) => x.id === comp);
    const p = c?.ports.find((x: any) => x[0] === port);
    return p ? p[1] : null;
  }
}

/** Default channels to plot after a run. */
export function defaultSelection(rs: ResultSet): string[] {
  const out: string[] = [];
  const comps = rs.meta.components;
  for (const c of comps) if (c.type === 'combustion_chamber') out.push(`${c.id}.Pc`, `${c.id}.thrust`, `${c.id}.OF`);
  for (const c of comps) if (c.type === 'shaft') out.push(`${c.id}.speed`);
  for (const c of comps) if (['tank', 'junction'].includes(c.type)) out.push(`${c.id}.P`);
  for (const c of comps) {
    if (c.type === 'pipe') out.push(rs.has(`${c.id}.mdot_out`) ? `${c.id}.mdot_out` : `${c.id}.mdot`);
    else if (['pump'].includes(c.type) || !['tank', 'junction', 'boundary', 'shaft', 'combustion_chamber', 'turbine'].includes(c.type)) out.push(`${c.id}.mdot`);
  }
  for (const c of comps) {
    if (c.type === 'pipe' && rs.has(`${c.id}.Q`)) {
      const o = rs.portNode(c.id, 'out');
      if (o) out.push(`${o}.T`);
      if (rs.has(`${c.id}.Tw_max`)) out.push(`${c.id}.Tw_max`);
    }
    if (c.type === 'tank') out.push(`${c.id}.T`);
  }
  const engine = comps.some((c: any) => c.type === 'combustion_chamber');
  const pick = out.filter((n) => rs.has(n));
  if (engine) {
    // keep engine-level channels and the pump flows, skip line-by-line flows
    return pick.filter((n) => !n.includes('.mdot') || comps.some((c: any) => c.type === 'pump' && n === `${c.id}.mdot`)).slice(0, 10);
  }
  return pick.slice(0, 8);
}

function Chart({ rs, refRs, names, height, onProbe, probe, theme }: { rs: ResultSet; refRs: ResultSet | null; names: string[]; height: number; onProbe: (k: number) => void; probe: number | null; theme: string }) {
  const el = useRef<HTMLDivElement>(null);
  const plot = useRef<uPlot | null>(null);
  const unit = rs.unit(names[0]);
  const du = displayUnit(unit);
  useEffect(() => {
    if (!el.current) return;
    const col = (r: ResultSet, n: string) => Array.from(r.series(n)!, (v) => (isFinite(v) ? v * du.scale : null));
    const refNames = refRs ? names.filter((n) => refRs.has(n)) : [];
    let data: any = [Array.from(rs.time), ...names.map((n) => col(rs, n))];
    // the reference run has its own time base: join the two tables on x
    if (refRs && refNames.length) data = uPlot.join([data, [Array.from(refRs.time), ...refNames.map((n) => col(refRs, n))]]);
    const xl = rs.xUnit ? `${rs.xName} [${rs.xUnit}]` : rs.xName;
    const css = getComputedStyle(document.documentElement);
    const grid = css.getPropertyValue('--grid').trim() || '#eee';
    const axisText = css.getPropertyValue('--muted').trim() || '#666';
    const font = '11px "IBM Plex Mono", ui-monospace, monospace';
    const opts: uPlot.Options = {
      width: el.current.clientWidth || 600,
      height,
      cursor: { sync: { key: 'fluidnet' }, drag: { x: true, y: false } },
      legend: { show: true },
      scales: { x: { time: false } },
      axes: [
        { stroke: axisText, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, font, label: xl, labelFont: font, labelSize: 14 },
        { stroke: axisText, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, font, size: 60, label: du.label, labelFont: font, labelSize: 14 },
      ],
      series: [
        { label: rs.xName, value: (_u, v) => (v == null ? '–' : fmt(v, 5)) },
        ...names.map((n, i) => ({
          label: n,
          stroke: SERIES_COLORS[i % SERIES_COLORS.length],
          width: 1.5,
          spanGaps: true,
          points: { show: rs.nt < 40 },
          value: (_u: any, v: number) => (v == null ? '–' : fmt(v, 5)),
        })),
        ...refNames.map((n) => ({
          label: `${n} (ref)`,
          stroke: SERIES_COLORS[names.indexOf(n) % SERIES_COLORS.length],
          width: 1,
          dash: [5, 4],
          spanGaps: true,
          points: { show: false },
          value: (_u: any, v: number) => (v == null ? '–' : fmt(v, 5)),
        })),
      ],
      hooks: {
        ready: [
          (u) => {
            u.over.addEventListener('click', () => {
              if (u.cursor.idx != null) onProbe(rs.nearest(u.data[0][u.cursor.idx]));
            });
          },
        ],
        draw: [
          (u) => {
            const k = (u as any)._probe;
            if (k == null) return;
            const x = u.valToPos(rs.time[k], 'x', true);
            const ctx = u.ctx;
            ctx.save();
            ctx.strokeStyle = axisText;
            ctx.setLineDash([4, 4]);
            ctx.beginPath();
            ctx.moveTo(x, u.bbox.top);
            ctx.lineTo(x, u.bbox.top + u.bbox.height);
            ctx.stroke();
            ctx.restore();
          },
        ],
      },
    };
    plot.current = new uPlot(opts, data, el.current);
    (plot.current as any)._probe = probe;
    const ro = new ResizeObserver(() => {
      if (el.current && plot.current) plot.current.setSize({ width: el.current.clientWidth, height });
    });
    ro.observe(el.current);
    return () => {
      ro.disconnect();
      plot.current?.destroy();
      plot.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [rs, refRs, names.join('|'), height, theme]);
  useEffect(() => {
    if (plot.current) {
      (plot.current as any)._probe = probe;
      plot.current.redraw(false, false);
    }
  }, [probe]);
  return <div ref={el} className="chart" />;
}

function ChannelBrowser({ rs, selected, setSelected }: { rs: ResultSet; selected: string[]; setSelected: (s: string[]) => void }) {
  const [q, setQ] = useState('');
  const [showInternal, setShowInternal] = useState(false);
  const groups = useMemo(() => {
    const g = new Map<string, string[]>();
    for (const c of rs.meta.channels) {
      const i = c.name.lastIndexOf('.');
      const head = c.name.slice(0, i);
      if (!g.has(head)) g.set(head, []);
      g.get(head)!.push(c.name);
    }
    const order = new Map<string, number>(rs.meta.components.map((c: any, i: number) => [c.id, i]));
    const internal = (h: string) => /\.(n|s)\d+$/.test(h);
    const rank = (h: string) => (order.has(h) ? order.get(h)! : internal(h) ? 1e6 : 1e3);
    return [...g.entries()].sort((a, b) => rank(a[0]) - rank(b[0])).map(([h, n]) => [h, n, internal(h)] as [string, string[], boolean]);
  }, [rs]);
  const ql = q.toLowerCase();
  const toggle = (n: string) => setSelected(selected.includes(n) ? selected.filter((x) => x !== n) : [...selected, n]);
  return (
    <div className="chan">
      <input id="chan-search" className="field" placeholder="Filter channels" value={q} onChange={(e) => setQ(e.target.value)} />
      <div className="chan-list">
        {groups.map(([head, names, internal]) => {
          if (internal && !showInternal && !ql) return null;
          const vis = names.filter((n) => !ql || n.toLowerCase().includes(ql));
          if (!vis.length) return null;
          return (
            <div key={head} className="chan-group">
              <div className="chan-head">{head}</div>
              <div className="chan-items">
                {vis.map((n) => {
                  const on = selected.includes(n);
                  const color = on ? SERIES_COLORS[selected.indexOf(n) % SERIES_COLORS.length] : undefined;
                  return (
                    <button key={n} className={`chip${on ? ' on' : ''}`} style={on ? { borderColor: color, color } : undefined} onClick={() => toggle(n)} title={`${n} [${displayUnit(rs.unit(n)).label}]`}>
                      {n.slice(head.length + 1)}
                    </button>
                  );
                })}
              </div>
            </div>
          );
        })}
      </div>
      <div className="chan-foot">
        {groups.some((g) => g[2]) && (
          <button className="btn-ghost small" onClick={() => setShowInternal(!showInternal)}>
            {showInternal ? 'Hide line internals' : 'Show line internals'}
          </button>
        )}
        {selected.length > 0 && (
          <button className="btn-ghost small" onClick={() => setSelected([])}>
            Clear
          </button>
        )}
      </div>
    </div>
  );
}

function ValuesTable({ rs, k, selIds, onSelectComp }: { rs: ResultSet; k: number; selIds: string[]; onSelectComp: (id: string) => void }) {
  const [q, setQ] = useState('');
  const v = (n: string, si = '') => {
    if (!rs.has(n)) return '–';
    const du = displayUnit(si || rs.unit(n));
    return fmt(rs.at(n, k) * du.scale, 5);
  };
  const ql = q.trim().toLowerCase();
  const rows = rs.meta.components
    .filter((c: any) => !ql || c.id.toLowerCase().includes(ql) || c.type.replace('_', ' ').includes(ql))
    .map((c: any) => {
      const nodeLike = ['tank', 'junction', 'boundary', 'shaft', 'combustion_chamber'].includes(c.type);
      const pin = nodeLike ? c.id : rs.portNode(c.id, 'in');
      const pout = nodeLike ? null : rs.portNode(c.id, 'out');
      const mdot = c.type === 'pipe' ? (rs.has(`${c.id}.mdot_in`) ? `${c.id}.mdot_in` : `${c.id}.mdot`) : `${c.id}.mdot`;
      const choked = rs.has(`${c.id}.choked`) && rs.at(`${c.id}.choked`, k) > 0.5;
      const x = pin && rs.has(`${pin}.x`) ? rs.at(`${pin}.x`, k) : NaN;
      return (
        <tr key={c.id} className={selIds.includes(c.id) ? 'on' : ''} onClick={() => onSelectComp(c.id)} title="Show on the schematic">
          <td className="mono">{c.id}</td>
          <td className="muted">{c.type.replace('_', ' ')}</td>
          <td className="num">{pin ? v(`${pin}.P`) : '–'}</td>
          <td className="num">{pout ? v(`${pout}.P`) : '–'}</td>
          <td className="num">{pin ? v(`${pin}.T`) : '–'}</td>
          <td className="num">{pout ? v(`${pout}.T`) : '–'}</td>
          <td className="num">{pin ? v(`${pin}.rho`) : '–'}</td>
          <td className="num">{isFinite(x) && x > 0 && x < 1 ? fmt(x, 3) : '–'}</td>
          <td className="num">{c.type === 'combustion_chamber' ? v(`${c.id}.mdot`) : nodeLike ? '–' : v(mdot)}</td>
          <td>
            {choked && <span className="pill pill-warn">choked</span>}
            {rs.has(`${c.id}.opening`) && <span className="pill">{fmt(rs.at(`${c.id}.opening`, k) * 100, 3)} % open</span>}
            {c.type === 'tank' && rs.has(`${c.id}.m`) && <span className="pill">{fmt(rs.at(`${c.id}.m`, k), 4)} kg</span>}
            {rs.has(`${c.id}.Q`) && <span className="pill">{fmt(rs.at(`${c.id}.Q`, k) / 1e3, 4)} kW</span>}
            {rs.has(`${c.id}.Tw_max`) && <span className="pill">wall {fmt(rs.at(`${c.id}.Tw_max`, k), 4)} K</span>}
            {c.type === 'pump' && (
              <>
                <span className="pill">+{fmt(rs.at(`${c.id}.dp`, k) / 1e5, 4)} bar</span>
                <span className="pill">η {fmt(rs.at(`${c.id}.eff`, k) * 100, 3)} %</span>
                <span className="pill">{fmt(rs.at(`${c.id}.power`, k) / 1e3, 4)} kW</span>
                <span className="pill">NPSH {fmt(rs.at(`${c.id}.npsh`, k), 4)} m</span>
              </>
            )}
            {c.type === 'turbine' && (
              <>
                <span className="pill">{fmt(rs.at(`${c.id}.power`, k) / 1e3, 4)} kW</span>
                <span className="pill">η {fmt(rs.at(`${c.id}.eff`, k) * 100, 3)} %</span>
                <span className="pill">PR {fmt(rs.at(`${c.id}.pr`, k), 4)}</span>
                <span className="pill">u/c0 {fmt(rs.at(`${c.id}.u_c0`, k), 3)}</span>
              </>
            )}
            {c.type === 'shaft' && <span className="pill">{fmt((rs.at(`${c.id}.speed`, k) * 30) / Math.PI, 5)} rpm</span>}
            {c.type === 'combustion_chamber' && (
              <>
                <span className="pill">Pc {fmt(rs.at(`${c.id}.Pc`, k) / 1e5, 4)} bar</span>
                <span className="pill">O/F {fmt(rs.at(`${c.id}.OF`, k), 4)}</span>
                <span className="pill">F {fmt(rs.at(`${c.id}.thrust`, k) / 1e3, 4)} kN</span>
                <span className="pill">Isp {fmt(rs.at(`${c.id}.Isp`, k), 4)} s</span>
                <span className="pill">c* {fmt(rs.at(`${c.id}.cstar`, k), 4)} m/s</span>
                <span className="pill">Tc {fmt(rs.at(`${c.id}.Tc`, k), 4)} K</span>
              </>
            )}
          </td>
        </tr>
      );
    });
  return (
    <div className="table-wrap">
      <table className="vtable">
        <thead>
          <tr>
            <th>
              <input id="table-filter" className="field th-filter" placeholder="Component" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Filter components" />
            </th>
            <th>Type</th>
            <th>P in [bar]</th>
            <th>P out [bar]</th>
            <th>T in [K]</th>
            <th>T out [K]</th>
            <th>ρ in [kg/m³]</th>
            <th>Quality</th>
            <th>ṁ [kg/s]</th>
            <th />
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </div>
  );
}

function Profile({ rs, k, theme }: { rs: ResultSet; k: number; theme: string }) {
  const pipes = rs.meta.components.filter((c: any) => c.type === 'pipe' && c.nodes.length > 0);
  const [pid, setPid] = useState<string>(pipes[0]?.id ?? '');
  const el = useRef<HTMLDivElement>(null);
  const pipe = pipes.find((p: any) => p.id === pid) ?? pipes[0];
  useEffect(() => {
    if (!el.current || !pipe) return;
    const chain = [rs.portNode(pipe.id, 'in'), ...pipe.nodes, rs.portNode(pipe.id, 'out')].filter(Boolean) as string[];
    const xs = chain.map((_, i) => i / (chain.length - 1));
    const P = chain.map((n) => rs.at(`${n}.P`, k) / 1e5);
    const T = chain.map((n) => rs.at(`${n}.T`, k));
    const css = getComputedStyle(document.documentElement);
    const grid = css.getPropertyValue('--grid').trim();
    const axisText = css.getPropertyValue('--muted').trim();
    const font = '11px "IBM Plex Mono", ui-monospace, monospace';
    const u = new uPlot(
      {
        width: el.current.clientWidth || 600,
        height: 220,
        scales: { x: { time: false }, P: {}, T: {} },
        legend: { show: true },
        axes: [
          { stroke: axisText, grid: { stroke: grid }, ticks: { stroke: grid }, font, label: 'x / L', labelFont: font, labelSize: 14 },
          { scale: 'P', stroke: SERIES_COLORS[0], grid: { stroke: grid }, ticks: { stroke: grid }, font, size: 60, label: 'P [bar]', labelFont: font, labelSize: 14 },
          { scale: 'T', side: 1, stroke: SERIES_COLORS[1], grid: { show: false }, ticks: { stroke: grid }, font, size: 60, label: 'T [K]', labelFont: font, labelSize: 14 },
        ],
        series: [
          { label: 'x/L', value: (_u, v) => (v == null ? '–' : fmt(v, 3)) },
          { label: 'P [bar]', scale: 'P', stroke: SERIES_COLORS[0], width: 1.5, points: { show: true, size: 4 }, value: (_u, v) => (v == null ? '–' : fmt(v, 5)) },
          { label: 'T [K]', scale: 'T', stroke: SERIES_COLORS[1], width: 1.5, points: { show: true, size: 4 }, value: (_u, v) => (v == null ? '–' : fmt(v, 5)) },
        ],
      },
      [xs, P, T] as any,
      el.current
    );
    const ro = new ResizeObserver(() => el.current && u.setSize({ width: el.current.clientWidth, height: 220 }));
    ro.observe(el.current);
    return () => {
      ro.disconnect();
      u.destroy();
    };
  }, [rs, k, pipe?.id, theme]);
  if (!pipes.length) return <div className="empty">No discretised pipes in this model.</div>;
  return (
    <div className="profile">
      <select id="profile-pipe" className="field" style={{ width: 'auto' }} value={pipe.id} onChange={(e) => setPid(e.target.value)}>
        {pipes.map((p: any) => (
          <option key={p.id} value={p.id}>
            {p.id}
          </option>
        ))}
      </select>
      <div ref={el} className="chart" />
    </div>
  );
}

function Messages({ rs, checkWarnings, runErr }: { rs: ResultSet | null; checkWarnings: string[]; runErr: string | null }) {
  const run: string[] = rs?.meta.warnings ?? [];
  const model = checkWarnings.filter((w) => !run.includes(w));
  const stats: [string, any][] = Object.entries({ ...(rs?.meta.stats ?? {}), ...(rs?.meta.steady ?? {}) }).filter(([, v]) => v !== null && typeof v !== 'object');
  return (
    <div className="messages">
      {runErr && <div className="msg msg-err">{runErr}</div>}
      {run.map((w, i) => (
        <div key={`r${i}`} className="msg msg-warn">
          {w}
        </div>
      ))}
      {model.map((w, i) => (
        <div key={`m${i}`} className="msg msg-warn">
          <span className="muted">Model check · </span>
          {w}
        </div>
      ))}
      {!runErr && !run.length && !model.length && <div className="muted">No warnings.</div>}
      {stats.length > 0 && (
        <>
          <div className="section-label">Solver</div>
          <div className="kv stats mono">
            {stats.map(([key, val]) => (
              <React.Fragment key={key}>
                <span>{key.replace(/_/g, ' ')}</span>
                <b>{typeof val === 'number' ? fmt(val, 5) : String(val)}</b>
              </React.Fragment>
            ))}
          </div>
        </>
      )}
    </div>
  );
}

export type ResultTab = 'plots' | 'table' | 'profile' | 'messages';

export function Results(props: {
  rs: ResultSet | null;
  refRs: ResultSet | null;
  setRefRs: (r: ResultSet | null) => void;
  tab: ResultTab;
  setTab: (t: ResultTab) => void;
  selected: string[];
  setSelected: (s: string[]) => void;
  probe: number | null;
  setProbe: (k: number | null) => void;
  playing: boolean;
  setPlaying: (p: boolean) => void;
  height: number;
  theme: string;
  stale: boolean;
  busy: string | null;
  selIds: string[];
  onSelectComp: (id: string) => void;
  checkWarnings: string[];
  runErr: string | null;
}) {
  const { rs, refRs, tab, setTab, selected, setSelected, probe, setProbe, height, theme } = props;
  const steady = rs?.mode === 'steady';
  const groups = useMemo(() => {
    const g = new Map<string, string[]>();
    if (!rs) return [];
    for (const n of selected) {
      if (!rs.has(n)) continue;
      const u = displayUnit(rs.unit(n)).label;
      if (!g.has(u)) g.set(u, []);
      g.get(u)!.push(n);
    }
    return [...g.values()];
  }, [rs, selected]);
  const k = rs ? Math.min(probe ?? rs.nt - 1, rs.nt - 1) : 0;
  const chartH = Math.max(130, Math.floor((height - 74) / Math.max(1, groups.length)) - 34);
  const hasProfile = !!rs?.meta.components.some((c: any) => c.type === 'pipe' && c.nodes.length > 0);
  const nMsg = (rs?.meta.warnings?.length ?? 0) + (props.runErr ? 1 : 0);
  const refOk = !!rs && !!refRs && refRs !== rs && refRs.mode === rs.mode && refRs.xName === rs.xName;
  const view: ResultTab = (tab === 'plots' && steady) || (tab === 'profile' && !hasProfile) ? 'table' : tab;
  const empty = <div className="empty">{props.busy ?? 'Run a transient or solve the steady state to see results here.'}</div>;
  return (
    <div className="results">
      <div className="res-bar">
        <div className="tabs" role="tablist">
          {!steady && (
            <button role="tab" className={view === 'plots' ? 'on' : ''} onClick={() => setTab('plots')}>
              Plots
            </button>
          )}
          <button role="tab" className={view === 'table' ? 'on' : ''} onClick={() => setTab('table')}>
            {steady ? 'Steady state' : 'Values'}
          </button>
          {hasProfile && (
            <button role="tab" className={view === 'profile' ? 'on' : ''} onClick={() => setTab('profile')}>
              Line profile
            </button>
          )}
          <button role="tab" className={view === 'messages' ? 'on' : ''} onClick={() => setTab('messages')}>
            Messages{nMsg ? <span className="badge">{nMsg}</span> : null}
          </button>
        </div>
        {rs && props.stale && (
          <span className="pill pill-warn" title="The model has changed since these results were computed. Run again to update them.">
            out of date
          </span>
        )}
        {rs && (
          <div className="res-info mono">
            {!steady && rs.nt > 1 && (
              <>
                <button className="mini" id="play" onClick={() => props.setPlaying(!props.playing)} title="Play through the result (P)">
                  {props.playing ? 'pause' : 'play'}
                </button>
                <input
                  id="scrub"
                  className="scrub"
                  type="range"
                  min={0}
                  max={rs.nt - 1}
                  value={k}
                  aria-label={`Probe ${rs.xName}`}
                  onChange={(e) => {
                    props.setPlaying(false);
                    setProbe(Number(e.target.value));
                  }}
                />
              </>
            )}
            <span className="res-t">{steady ? 'steady state' : `${rs.xName} = ${fmt(rs.time[k], 5)}${rs.xUnit ? ` ${rs.xUnit}` : ''}`}</span>
            {!steady && probe != null && (
              <button className="mini" onClick={() => setProbe(null)} title="Show the last point">
                end
              </button>
            )}
            {!steady && (
              <button
                className={`mini${refOk ? ' on' : ''}`}
                id="keep-ref"
                onClick={() => props.setRefRs(refRs ? null : rs)}
                title={refRs ? 'Drop the reference run' : 'Keep this run as a dashed reference in the plots, to compare the next run against'}
              >
                {refRs ? 'clear reference' : 'keep as reference'}
              </button>
            )}
            <button className="mini" id="export-csv" onClick={() => downloadText(`${fileStem(rs.meta.name || 'results')}_${rs.mode}.csv`, resultsCsv(rs), 'text/csv')} title="Download all channels as CSV">
              CSV
            </button>
          </div>
        )}
      </div>
      {view === 'messages' ? (
        <Messages rs={rs} checkWarnings={props.checkWarnings} runErr={props.runErr} />
      ) : !rs ? (
        empty
      ) : view === 'table' ? (
        <ValuesTable rs={rs} k={k} selIds={props.selIds} onSelectComp={props.onSelectComp} />
      ) : view === 'profile' ? (
        <Profile rs={rs} k={k} theme={theme} />
      ) : (
        <div className="plots">
          <ChannelBrowser rs={rs} selected={selected} setSelected={setSelected} />
          <div className="charts">
            {groups.length === 0 && <div className="empty">Pick channels on the left to plot them.</div>}
            {groups.map((names) => (
              <Chart key={names.join('|')} rs={rs} refRs={refOk ? refRs : null} names={names} height={chartH} onProbe={setProbe} probe={probe} theme={theme} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
