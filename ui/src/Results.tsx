import React, { useEffect, useMemo, useRef, useState } from 'react';
import uPlot from 'uplot';
import { displayUnit, fmt } from './model';
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
  get time(): Float64Array {
    return this.data.subarray(0, this.nt);
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

function Chart({ rs, names, height, onProbe, probe }: { rs: ResultSet; names: string[]; height: number; onProbe: (k: number) => void; probe: number | null }) {
  const el = useRef<HTMLDivElement>(null);
  const plot = useRef<uPlot | null>(null);
  const unit = rs.unit(names[0]);
  const du = displayUnit(unit);
  useEffect(() => {
    if (!el.current) return;
    const t = Array.from(rs.time);
    const ys = names.map((n) => Array.from(rs.series(n)!, (v) => (isFinite(v) ? v * du.scale : null)));
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
        { stroke: axisText, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, font, label: 't [s]', labelFont: font, labelSize: 14 },
        { stroke: axisText, grid: { stroke: grid, width: 1 }, ticks: { stroke: grid }, font, size: 60, label: du.label, labelFont: font, labelSize: 14 },
      ],
      series: [
        { label: 't', value: (_u, v) => (v == null ? '–' : fmt(v, 5)) },
        ...names.map((n, i) => ({
          label: n,
          stroke: SERIES_COLORS[i % SERIES_COLORS.length],
          width: 1.5,
          points: { show: rs.nt < 40 },
          value: (_u: any, v: number) => (v == null ? '–' : fmt(v, 5)),
        })),
      ],
      hooks: {
        ready: [
          (u) => {
            u.over.addEventListener('click', () => {
              if (u.cursor.idx != null) onProbe(u.cursor.idx);
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
    plot.current = new uPlot(opts, [t, ...ys] as any, el.current);
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
  }, [rs, names.join('|'), height]);
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

function ValuesTable({ rs, k }: { rs: ResultSet; k: number }) {
  const v = (n: string, si = '') => {
    if (!rs.has(n)) return '–';
    const du = displayUnit(si || rs.unit(n));
    return fmt(rs.at(n, k) * du.scale, 5);
  };
  const rows = rs.meta.components.map((c: any) => {
    const nodeLike = ['tank', 'junction', 'boundary', 'shaft', 'combustion_chamber'].includes(c.type);
    const pin = nodeLike ? c.id : rs.portNode(c.id, 'in');
    const pout = nodeLike ? null : rs.portNode(c.id, 'out');
    const mdot = c.type === 'pipe' ? (rs.has(`${c.id}.mdot_in`) ? `${c.id}.mdot_in` : `${c.id}.mdot`) : `${c.id}.mdot`;
    const choked = rs.has(`${c.id}.choked`) && rs.at(`${c.id}.choked`, k) > 0.5;
    const x = pin && rs.has(`${pin}.x`) ? rs.at(`${pin}.x`, k) : NaN;
    return (
      <tr key={c.id}>
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
            <th>Component</th>
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

function Profile({ rs, k }: { rs: ResultSet; k: number }) {
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
  }, [rs, k, pipe?.id]);
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

export function Results({ rs, selected, setSelected, probe, setProbe, height }: { rs: ResultSet; selected: string[]; setSelected: (s: string[]) => void; probe: number | null; setProbe: (k: number | null) => void; height: number }) {
  const steady = rs.meta.mode === 'steady';
  const [tab, setTab] = useState<'plots' | 'table' | 'profile'>(steady ? 'table' : 'plots');
  useEffect(() => setTab(steady ? 'table' : 'plots'), [steady, rs]);
  const groups = useMemo(() => {
    const g = new Map<string, string[]>();
    for (const n of selected) {
      if (!rs.has(n)) continue;
      const u = displayUnit(rs.unit(n)).label;
      if (!g.has(u)) g.set(u, []);
      g.get(u)!.push(n);
    }
    return [...g.values()];
  }, [rs, selected]);
  const k = probe ?? rs.nt - 1;
  const chartH = Math.max(130, Math.floor((height - 74) / Math.max(1, groups.length)) - 34);
  return (
    <div className="results">
      <div className="res-bar">
        <div className="tabs">
          {!steady && (
            <button className={tab === 'plots' ? 'on' : ''} onClick={() => setTab('plots')}>
              Plots
            </button>
          )}
          <button className={tab === 'table' ? 'on' : ''} onClick={() => setTab('table')}>
            {steady ? 'Steady state' : 'Values'}
          </button>
          {rs.meta.components.some((c: any) => c.type === 'pipe' && c.nodes.length > 0) && (
            <button className={tab === 'profile' ? 'on' : ''} onClick={() => setTab('profile')}>
              Line profile
            </button>
          )}
        </div>
        <div className="res-info mono">
          {steady ? 'steady state' : `t = ${fmt(rs.time[k], 5)} s`}
          {!steady && probe != null && (
            <button className="mini" onClick={() => setProbe(null)}>
              end
            </button>
          )}
        </div>
      </div>
      {tab === 'table' ? (
        <ValuesTable rs={rs} k={k} />
      ) : tab === 'profile' ? (
        <Profile rs={rs} k={k} />
      ) : (
        <div className="plots">
          <ChannelBrowser rs={rs} selected={selected} setSelected={setSelected} />
          <div className="charts">
            {groups.length === 0 && <div className="empty">Pick channels on the left to plot them.</div>}
            {groups.map((names) => (
              <Chart key={names.join('|')} rs={rs} names={names} height={chartH} onProbe={setProbe} probe={probe} />
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
