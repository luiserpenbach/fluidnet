import React, { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
  MiniMap,
  MarkerType,
  applyNodeChanges,
  useReactFlow,
  ConnectionMode,
  type Node,
  type Edge,
  type NodeChange,
  type EdgeChange,
  type Connection,
} from '@xyflow/react';
import { Engine } from './engine';
import { Comp, Model, parseModel, toYaml, toEngineJson, splitPort, fluidMap, usedFluids, uniqueId, fmt, normRot, FLUID_COLORS, emptyModel } from './model';
import { CompNode, CompNodeData } from './CompNode';
import { Inspector } from './Inspector';
import { Results, ResultSet, ResultTab, defaultSelection } from './Results';
import { Symbol, SYMBOL_SIZE } from './symbols';
import { useHistory } from './history';
import { NODE_LIKE, AlignMode, Fragment, alignComps, connKey, edgeId, extractFragment, fragmentFromText, fragmentToText, hasConn, nodeSize, pasteFragment, refKeysOf, removeItems, renameComp, spliceInto, spliceNode } from './graph';
import { Command, CommandPalette, ContextMenu, MenuItem, MOD, ShortcutsDialog, Toast, Toasts } from './Menus';
import { StudyDialog, StudySpec, runStudy } from './Study';
import { canPickFiles, downloadText, fileStem, resultsCsv, saveText } from './files';

import exLox from '../../examples/lox_feed_startup.yaml?raw';
import exGn2 from '../../examples/gn2_blowdown.yaml?raw';
import exHammer from '../../examples/water_hammer.yaml?raw';
import exVenturi from '../../examples/cavitating_venturi.yaml?raw';
import exReg from '../../examples/regulated_pressurization.yaml?raw';
import exLh2 from '../../examples/lh2_heated_channel.yaml?raw';
import exExpStart from '../../examples/expander_start.yaml?raw';
import exExpThrottle from '../../examples/expander_cycle.yaml?raw';
import exRl10 from '../../examples/rl10a33a.yaml?raw';
import exRl10Start from '../../examples/rl10a33a_start.yaml?raw';

const EXAMPLES: [string, string][] = [
  ['RL10A-3-3A (test point 1)', exRl10],
  ['RL10A-3-3A start', exRl10Start],
  ['Closed expander throttling', exExpThrottle],
  ['Closed expander start', exExpStart],
  ['LOX feed startup', exLox],
  ['LH2 heated channel', exLh2],
  ['Regulated tank pressurization', exReg],
  ['GN2 bottle blowdown', exGn2],
  ['Water hammer', exHammer],
  ['Cavitating venturi', exVenturi],
];

const STORE_KEY = 'fluidnet.model.v1';
const PREFS_KEY = 'fluidnet.prefs.v1';
const nodeTypes = { comp: CompNode };

const NEW_DEFAULTS: Record<string, Record<string, any>> = {
  tank: { volume: '10 L', P0: '10 bar', T0: '290 K' },
  boundary: { P: '1 bar', T: '290 K' },
  junction: {},
  pipe: { length: '1 m', diameter: '10 mm' },
  orifice: { diameter: '4 mm' },
  venturi: { diameter: '3 mm' },
  valve: { diameter: '8 mm' },
  check_valve: { diameter: '8 mm' },
  regulator: { diameter: '4 mm', set_pressure: '10 bar' },
  loss: { diameter: '10 mm', K: 0.3 },
  mass_flow: { mdot: '0.1 kg/s' },
  shaft: { inertia: '0.01 kg m2' },
  pump: { design_speed: '30000 rpm', design_mdot: '1 kg/s', design_dp: '50 bar' },
  turbine: { diameter: '10 mm', mean_diameter: '100 mm' },
  combustion_chamber: { throat_diameter: '30 mm' },
};

function loadStored(): Model | null {
  try {
    const s = localStorage.getItem(STORE_KEY);
    return s ? parseModel(s) : null;
  } catch {
    return null;
  }
}

function overlayFor(c: Comp, rs: ResultSet | null, k: number): string[] {
  if (!rs) return [];
  const has = (n: string) => rs.has(n);
  const at = (n: string) => rs.at(n, k);
  const P = (n: string) => `${fmt(at(n) / 1e5, 4)} bar`;
  if (NODE_LIKE.has(c.type)) {
    const out: string[] = [];
    if (has(`${c.id}.P`)) out.push(P(`${c.id}.P`));
    if (has(`${c.id}.T`)) {
      let s = `${fmt(at(`${c.id}.T`), 4)} K`;
      const x = has(`${c.id}.x`) ? at(`${c.id}.x`) : NaN;
      if (isFinite(x) && x > 0.0005 && x < 0.9995) s += ` · x ${fmt(x, 2)}`;
      out.push(s);
    }
    return out;
  }
  if (c.type === 'shaft') return has(`${c.id}.speed`) ? [`${fmt((at(`${c.id}.speed`) * 30) / Math.PI, 4)} rpm`] : [];
  if (c.type === 'combustion_chamber') {
    if (!has(`${c.id}.Pc`)) return [];
    return [
      `Pc ${fmt(at(`${c.id}.Pc`) / 1e5, 4)} bar · O/F ${fmt(at(`${c.id}.OF`), 3)}`,
      `F ${fmt(at(`${c.id}.thrust`) / 1e3, 4)} kN · Isp ${fmt(at(`${c.id}.Isp`), 4)} s`,
    ];
  }
  const m = c.type === 'pipe' && has(`${c.id}.mdot_in`) ? `${c.id}.mdot_in` : `${c.id}.mdot`;
  const out: string[] = [];
  if (has(m)) out.push(`${fmt(at(m), 4)} kg/s`);
  if (c.type === 'pump' && has(`${c.id}.dp`)) {
    out.push(`+${fmt(at(`${c.id}.dp`) / 1e5, 4)} bar · η ${fmt(at(`${c.id}.eff`) * 100, 3)} %`);
  } else if (c.type === 'turbine' && has(`${c.id}.power`)) {
    out.push(`${fmt(at(`${c.id}.power`) / 1e3, 4)} kW · PR ${fmt(at(`${c.id}.pr`), 3)}`);
  } else if (has(`${c.id}.dp`)) {
    out.push(`Δp ${fmt(at(`${c.id}.dp`) / 1e5, 3)} bar`);
  }
  if (has(`${c.id}.Q`)) out.push(`Q ${fmt(at(`${c.id}.Q`) / 1e3, 4)} kW${has(`${c.id}.Tw_max`) ? ` · Tw ${fmt(at(`${c.id}.Tw_max`), 4)} K` : ''}`);
  if (has(`${c.id}.choked`) && at(`${c.id}.choked`) > 0.5) out.push('choked');
  return out;
}

function Palette({ catalog, onAdd }: { catalog: any; onAdd: (type: string) => void }) {
  const [q, setQ] = useState('');
  if (!catalog) return <div className="palette" />;
  const cats = new Map<string, any[]>();
  const ql = q.trim().toLowerCase();
  for (const c of catalog.components) {
    if (ql && !`${c.label} ${c.type} ${c.category}`.toLowerCase().includes(ql)) continue;
    if (!cats.has(c.category)) cats.set(c.category, []);
    cats.get(c.category)!.push(c);
  }
  return (
    <div className="palette">
      <input id="pal-search" className="field" placeholder="Filter parts" value={q} onChange={(e) => setQ(e.target.value)} aria-label="Filter parts" />
      {cats.size === 0 && <div className="pal-foot">No part matches.</div>}
      {[...cats.entries()].map(([cat, items]) => (
        <div key={cat} className="pal-group">
          <div className="section-label">{cat}</div>
          {items.map((c) => (
            <button
              key={c.type}
              className="pal-item"
              draggable
              title={c.help}
              onDragStart={(e) => {
                e.dataTransfer.setData('application/fluidnet', c.type);
                e.dataTransfer.effectAllowed = 'move';
              }}
              onClick={() => onAdd(c.type)}
            >
              <span className="pal-sym">
                <Symbol type={c.type} size={Math.min(0.6, 42 / (SYMBOL_SIZE[c.type]?.[0] ?? 48))} />
              </span>
              {c.label}
            </button>
          ))}
        </div>
      ))}
      <div className="pal-foot">Drag onto the canvas or onto a line, or click to add.</div>
    </div>
  );
}

const Icon = ({ d, size = 15 }: { d: string; size?: number }) => (
  <svg width={size} height={size} viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth={1.5} strokeLinecap="round" strokeLinejoin="round" aria-hidden>
    <path d={d} />
  </svg>
);
const IC = {
  undo: 'M6 3 2.5 6.5 6 10M3 6.5h6.5a4 4 0 0 1 0 8H7',
  redo: 'M10 3l3.5 3.5L10 10M13 6.5H6.5a4 4 0 0 0 0 8H9',
  search: 'M7 12A5 5 0 1 0 7 2a5 5 0 0 0 0 10zM14 14l-3.4-3.4',
  sun: 'M8 11a3 3 0 1 0 0-6 3 3 0 0 0 0 6zM8 1v1.5M8 13.5V15M1 8h1.5M13.5 8H15M3 3l1.1 1.1M11.9 11.9 13 13M3 13l1.1-1.1M11.9 4.1 13 3',
  moon: 'M13.5 9.5A6 6 0 0 1 6.5 2.5a6 6 0 1 0 7 7z',
  help: 'M8 15A7 7 0 1 0 8 1a7 7 0 0 0 0 14zM6 6.2a2 2 0 1 1 3 1.7c-.6.4-1 .8-1 1.6M8 11.8v.2',
};

interface Prefs {
  theme: 'light' | 'dark';
  values: boolean;
  snap: boolean;
  minimap: boolean;
  left: boolean;
  right: boolean;
  dock: boolean;
}
function loadPrefs(): Prefs {
  const d: Prefs = { theme: 'light', values: true, snap: true, minimap: false, left: true, right: true, dock: true };
  try {
    return { ...d, ...JSON.parse(localStorage.getItem(PREFS_KEY) ?? '{}') };
  } catch {
    return d;
  }
}

const isEditing = (t: EventTarget | null) => {
  const el = t as HTMLElement | null;
  return !!el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA' || el.tagName === 'SELECT' || el.isContentEditable);
};

/** Component the model check complains about, if the message names one. */
function errorComp(msg: string | undefined, ids: string[]): string | null {
  if (!msg) return null;
  const text = msg.replace(/^model error: /, '');
  const has = (id?: string) => (id && ids.includes(id) ? id : null);
  return (
    has(/^([A-Za-z0-9_\-]+)[:.]/.exec(text)?.[1]) ??
    has(/^port ([A-Za-z0-9_\-]+)\./.exec(text)?.[1]) ??
    has(/component '([^']+)'/.exec(text)?.[1]) ??
    [...ids].sort((a, b) => b.length - a.length).find((id) => new RegExp(`(^|[^A-Za-z0-9_\\-])${id.replace(/[-]/g, '\\-')}([^A-Za-z0-9_\\-]|$)`).test(text)) ??
    null
  );
}

function Editor() {
  const engine = useRef(new Engine()).current;
  const rf = useReactFlow();
  const [catalog, setCatalog] = useState<any>(null);
  const [engineErr, setEngineErr] = useState<string | null>(null);
  const hist = useHistory<Model>(() => loadStored() ?? parseModel(exExpThrottle));
  const model = hist.value;
  const commit = hist.commit;
  const [saved, setSaved] = useState<Model>(model);
  const fileHandle = useRef<any>(null);
  const [sel, setSel] = useState<string[]>([]);
  const [selE, setSelE] = useState<string[]>([]);
  const [check, setCheck] = useState<any>(null);
  const [rs, setRs] = useState<ResultSet | null>(null);
  const [refRs, setRefRs] = useState<ResultSet | null>(null);
  const [ranKey, setRanKey] = useState('');
  const [tab, setTab] = useState<ResultTab>('plots');
  const [selCh, setSelCh] = useState<string[]>([]);
  const [probe, setProbe] = useState<number | null>(null);
  const [playing, setPlaying] = useState(false);
  const [running, setRunning] = useState<null | 'steady' | 'transient' | 'study'>(null);
  const [progress, setProgress] = useState(0);
  const [runErr, setRunErr] = useState<string | null>(null);
  const [yamlOpen, setYamlOpen] = useState(false);
  const [studyOpen, setStudyOpen] = useState(false);
  const [studySpec, setStudySpec] = useState<StudySpec | null>(null);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [keysOpen, setKeysOpen] = useState(false);
  const [menu, setMenu] = useState<{ x: number; y: number; items: MenuItem[] } | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [prefs, setPrefs] = useState<Prefs>(loadPrefs);
  const [resH, setResH] = useState(320);
  const [nodes, setNodes] = useState<Node[]>([]);
  const fileRef = useRef<HTMLInputElement>(null);
  const autoRan = useRef(false);
  const clip = useRef<Fragment | null>(null);
  const pointer = useRef({ x: 0, y: 0, inside: false });
  const wasDragging = useRef(false);
  const studyCancel = useRef(false);
  const refKeys = useMemo(() => refKeysOf(catalog), [catalog]);

  const toast = useCallback((text: string, kind: Toast['kind'] = 'info') => {
    const id = Date.now() + Math.random();
    setToasts((t) => [...t.slice(-3), { id, text, kind }]);
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), kind === 'bad' ? 5000 : 2200);
  }, []);
  const pref = (p: Partial<Prefs>) => setPrefs((x) => ({ ...x, ...p }));

  // ---- engine boot
  useEffect(() => {
    engine.onProgress = (t) => setProgress(t);
    engine
      .init()
      .then(() => setCatalog(engine.catalog))
      .catch((e) => setEngineErr(String(e.message ?? e)));
  }, [engine]);

  // ---- autosave, preferences, window title
  useEffect(() => {
    try {
      localStorage.setItem(STORE_KEY, toYaml(model));
    } catch {
      /* storage unavailable */
    }
  }, [model]);
  // before the charts' effects run, so they read the new theme's colours
  useLayoutEffect(() => {
    document.documentElement.dataset.theme = prefs.theme;
  }, [prefs.theme]);
  useEffect(() => {
    try {
      localStorage.setItem(PREFS_KEY, JSON.stringify(prefs));
    } catch {
      /* storage unavailable */
    }
  }, [prefs]);
  const dirty = model !== saved;
  useEffect(() => {
    document.title = `${dirty ? '• ' : ''}${model.name || 'Untitled system'} · fluidnet`;
  }, [dirty, model.name]);

  // ---- model check (debounced, ignores layout-only changes)
  const engineJson = useMemo(() => toEngineJson(model), [model]);
  const checkKey = useMemo(() => toEngineJson({ ...model, components: model.components.map((c) => ({ ...c, ui: { x: 0, y: 0 } })) }), [model]);
  useEffect(() => {
    if (!catalog) return;
    const t = setTimeout(async () => {
      try {
        await engine.ensureFluids(usedFluids(model));
        if (!model.components.length) return setCheck(null);
        setCheck(await engine.check(engineJson));
      } catch (e: any) {
        if (String(e.message) !== 'stopped') setCheck({ error: String(e.message ?? e) });
      }
    }, 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [checkKey, catalog]);

  // ---- run
  const showResult = useCallback((set: ResultSet, key: string) => {
    setRs(set);
    setRanKey(key);
    setProbe(null);
    setPlaying(false);
    setSelCh((prev) => {
      const keep = prev.filter((n) => set.has(n));
      return keep.length ? keep : defaultSelection(set);
    });
    setTab((t) => (t === 'messages' ? (set.mode === 'steady' ? 'table' : 'plots') : t));
  }, []);

  const run = useCallback(
    async (mode: 'steady' | 'transient') => {
      if (running || !catalog) return;
      setRunning(mode);
      setRunErr(null);
      setProgress(0);
      try {
        await engine.ensureFluids(usedFluids(model));
        const r = await engine.run(toEngineJson(model), mode);
        const set = new ResultSet(r);
        showResult(set, checkKey);
        setTab(mode === 'steady' ? 'table' : 'plots');
        if (set.meta.error) setRunErr(`Stopped early: ${set.meta.error}`);
      } catch (e: any) {
        if (String(e.message) !== 'stopped') {
          setRunErr(String(e.message ?? e));
          pref({ dock: true });
        }
      } finally {
        setRunning(null);
      }
    },
    [running, catalog, engine, model, checkKey, showResult]
  );

  const study = useCallback(
    async (spec: StudySpec) => {
      if (running || !catalog) return;
      setStudySpec(spec);
      setStudyOpen(false);
      setRunning('study');
      setRunErr(null);
      setProgress(0);
      studyCancel.current = false;
      try {
        const set = await runStudy(engine, model, spec, (i, n) => setProgress(i / n), () => studyCancel.current);
        showResult(set, checkKey);
        setTab('plots');
        pref({ dock: true });
        const failed = set.nt - set.meta.study.solved;
        if (failed) setRunErr(`${failed} of ${set.nt} points have no solution; see Messages.`);
      } catch (e: any) {
        if (String(e.message) !== 'stopped') setRunErr(String(e.message ?? e));
      } finally {
        setRunning(null);
      }
    },
    [running, catalog, engine, model, checkKey, showResult]
  );

  const stop = () => {
    studyCancel.current = true;
    engine.stop().then(() => setCatalog(engine.catalog));
  };

  useEffect(() => {
    if (catalog && !autoRan.current && model.components.length) {
      autoRan.current = true;
      run('transient');
    }
  }, [catalog, model.components.length, run]);

  // ---- playback of the probe through a result
  useEffect(() => {
    if (!playing || !rs || rs.nt < 2) return;
    const step = Math.max(1, Math.ceil(rs.nt / 240));
    const t = setInterval(() => {
      setProbe((p) => {
        const next = (p ?? -step) + step;
        if (next >= rs.nt - 1) {
          setPlaying(false);
          return null;
        }
        return next;
      });
    }, 40);
    return () => clearInterval(t);
  }, [playing, rs]);

  // ---- selection only ever names things that exist (undo can remove them)
  useEffect(() => {
    const ids = new Set(model.components.map((c) => c.id));
    const eids = new Set(model.connections.map(([a, b]) => edgeId(a, b)));
    setSel((s) => (s.every((id) => ids.has(id)) ? s : s.filter((id) => ids.has(id))));
    setSelE((s) => (s.every((id) => eids.has(id)) ? s : s.filter((id) => eids.has(id))));
  }, [model.components, model.connections]);

  // ---- graph derivation
  const fluids = useMemo(() => fluidMap(model), [model]);
  const byId = useMemo(() => new Map(model.components.map((c) => [c.id, c])), [model]);
  const errId = useMemo(() => errorComp(check?.error, model.components.map((c) => c.id)), [check, model.components]);
  const k = rs ? Math.min(probe ?? rs.nt - 1, rs.nt - 1) : 0;
  const stale = !!rs && ranKey !== checkKey;

  useEffect(() => {
    setNodes((prev) => {
      const prevMap = new Map(prev.map((n) => [n.id, n]));
      return model.components.map((c) => {
        const p = prevMap.get(c.id);
        const data: CompNodeData = {
          id: c.id,
          type: c.type,
          flip: !!c.ui.flip,
          rot: c.ui.rot ?? 0,
          overlay: prefs.values ? overlayFor(c, rs, k) : [],
          color: FLUID_COLORS[fluids.get(c.id) ?? ''] ?? 'var(--ink)',
          error: errId === c.id,
        };
        return {
          ...(p ?? {}),
          id: c.id,
          type: 'comp',
          position: p?.dragging ? p.position : { x: c.ui.x, y: c.ui.y },
          data,
          selected: sel.includes(c.id),
        } as Node;
      });
    });
  }, [model.components, rs, k, fluids, sel, errId, prefs.values]);

  const edges: Edge[] = useMemo(() => {
    const center = (id: string) => {
      const c = byId.get(id);
      if (!c) return { x: 0, y: 0 };
      const [w, h] = nodeSize(c);
      return { x: c.ui.x + w / 2, y: c.ui.y + h / 2 };
    };
    const handleFor = (comp: string, port: string | null, other: string) => {
      const c = byId.get(comp);
      if (!c) return null;
      if (!NODE_LIKE.has(c.type)) return port;
      const a = center(comp);
      const b = center(other);
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      if (c.type === 'boundary') {
        if ((c.ui.rot ?? 0) % 180) return dy >= 0 ? 'p-b' : 'p-t';
        return dx >= 0 ? 'p-r' : 'p-l';
      }
      if (Math.abs(dx) >= Math.abs(dy)) return dx >= 0 ? 'p-r' : 'p-l';
      return dy >= 0 ? 'p-b' : 'p-t';
    };
    const end = (ref: string): [string, string | null] => (byId.has(ref) ? [ref, null] : splitPort(ref));
    // +1: the flow leaves the component through this port, -1: it enters, 0: unknown or at rest
    const outflow = (comp: string, port: string | null): number => {
      const c = byId.get(comp);
      if (!rs || !c || NODE_LIKE.has(c.type) || c.type === 'shaft') return 0;
      if (c.type === 'combustion_chamber') return rs.has(`${comp}.mdot`) && rs.at(`${comp}.mdot`, k) > 1e-9 ? -1 : 0;
      const pipeCh = port === 'in' ? `${comp}.mdot_in` : `${comp}.mdot_out`;
      const name = c.type === 'pipe' && rs.has(pipeCh) ? pipeCh : `${comp}.mdot`;
      const m = rs.has(name) ? rs.at(name, k) : NaN;
      if (!isFinite(m) || Math.abs(m) < 1e-9) return 0;
      return (port === 'out') === m > 0 ? 1 : -1;
    };
    const fluidEdges: Edge[] = model.connections.map(([a, b]) => {
      const [ca, pa] = end(a);
      const [cb, pb] = end(b);
      const fluid = fluids.get(ca) ?? fluids.get(cb) ?? '';
      const stroke = FLUID_COLORS[fluid] ?? 'var(--ink-soft)';
      const id = edgeId(a, b);
      // flow sense along the line: 1 from a to b, -1 from b to a
      const dir = prefs.values ? outflow(ca, pa) || -outflow(cb, pb) : 0;
      const marker = { type: MarkerType.ArrowClosed, color: stroke, width: 13, height: 13 };
      return {
        id,
        source: ca,
        target: cb,
        sourceHandle: handleFor(ca, pa ?? 'p', cb),
        targetHandle: handleFor(cb, pb ?? 'p', ca),
        type: 'smoothstep',
        selected: selE.includes(id),
        interactionWidth: 18,
        markerEnd: dir > 0 ? marker : undefined,
        markerStart: dir < 0 ? marker : undefined,
        style: { stroke, strokeWidth: 2 },
      } as Edge;
    });
    const links: Edge[] = [];
    for (const c of model.components) {
      if ((c.type === 'pump' || c.type === 'turbine') && c.params.shaft && byId.has(String(c.params.shaft))) {
        links.push({
          id: `m:${c.id}`,
          source: c.id,
          target: String(c.params.shaft),
          sourceHandle: 'mech',
          targetHandle: 'mech',
          type: 'straight',
          selectable: false,
          deletable: false,
          focusable: false,
          className: 'mech-link',
          style: { stroke: 'var(--ink-soft)', strokeWidth: 1.2, strokeDasharray: '5 4' },
        } as Edge);
      }
      if (c.type === 'pipe' && c.params.hot_chamber && byId.has(String(c.params.hot_chamber))) {
        links.push({
          id: `h:${c.id}`,
          source: String(c.params.hot_chamber),
          target: c.id,
          sourceHandle: 'heat',
          targetHandle: 'heat',
          type: 'smoothstep',
          selectable: false,
          deletable: false,
          focusable: false,
          className: 'heat-link',
          style: { stroke: 'var(--heat)', strokeWidth: 1.2, strokeDasharray: '2 4' },
        } as Edge);
      }
    }
    return [...fluidEdges, ...links];
  }, [model.connections, model.components, byId, fluids, selE, rs, k, prefs.values]);

  // ---- mutations (each call is one undo step)
  const selConnKeys = () => selE.filter((id) => id.startsWith('e:')).map((id) => id.slice(2));
  const select = (ids: string[], edgeIds: string[] = []) => {
    setSel(ids);
    setSelE(edgeIds);
  };
  const portsOf = (type: string): string[] => catalog?.components.find((c: any) => c.type === type)?.ports ?? [];
  /** Whether a part can be inserted into a line: two-port elements always, volumes only between two elements. */
  const fitsLine = (type: string, key: string): boolean => {
    if (!NODE_LIKE.has(type)) {
      const ports = portsOf(type);
      return ports.includes('in') && ports.includes('out');
    }
    const conn = model.connections.find(([a, b]) => connKey(a, b) === key);
    return !!conn && conn.every((r) => !byId.has(r) && !NODE_LIKE.has(byId.get(splitPort(r)[0])?.type ?? ''));
  };
  const centred = (type: string, p: { x: number; y: number }) => {
    const [w, h] = SYMBOL_SIZE[type] ?? [48, 32];
    return { x: Math.round((p.x - w / 2) / 10) * 10, y: Math.round((p.y - h / 2) / 10) * 10 };
  };

  /** Add a part; with `into` (a connection key) it is inserted into that line. */
  const addComp = (type: string, at?: { x: number; y: number }, into?: string) => {
    const base = type === 'check_valve' ? 'check' : type === 'mass_flow' ? 'flow' : type === 'combustion_chamber' ? 'chamber' : type;
    const id = uniqueId(model, base);
    const params: Record<string, any> = { ...(NEW_DEFAULTS[type] ?? {}) };
    if (type === 'tank' || type === 'boundary') params.fluid = usedFluids(model)[0] ?? 'N2';
    const el = document.querySelector('.canvas')?.getBoundingClientRect();
    const p = centred(type, at ?? rf.screenToFlowPosition({ x: el ? el.left + el.width / 2 : window.innerWidth / 2, y: el ? el.top + el.height / 2 : window.innerHeight / 3 }));
    const canSplice = !!into && fitsLine(type, into);
    commit((m) => {
      const next = { ...m, components: [...m.components, { id, type, params, ui: { x: p.x, y: p.y } }] };
      if (!canSplice) return next;
      return NODE_LIKE.has(type) ? spliceNode(next, into!, id) : spliceInto(next, into!, id);
    });
    select([id]);
    if (into && !canSplice) toast(`A ${type.replace('_', ' ')} cannot be inserted into a line; it was placed next to it.`);
    else if (into) toast(`Inserted ${id} into the line`);
  };

  const deleteSelection = () => {
    const keys = selConnKeys();
    if (!sel.length && !keys.length) return;
    commit((m) => removeItems(m, sel, keys, refKeys));
    select([]);
  };

  const patchUi = (ids: string[], f: (c: Comp) => Partial<Comp['ui']>) => {
    if (!ids.length) return;
    commit((m) => ({ ...m, components: m.components.map((c) => (ids.includes(c.id) ? { ...c, ui: { ...c.ui, ...f(c) } } : c)) }));
  };
  const rotateSel = () =>
    patchUi(sel, (c) => {
      // turn about the symbol centre
      const [w, h] = nodeSize(c);
      const d = (w - h) / 2;
      return { rot: normRot((c.ui.rot ?? 0) + 90), x: Math.round(c.ui.x + d), y: Math.round(c.ui.y - d) };
    });
  const flipSel = () => patchUi(sel, (c) => ({ flip: !c.ui.flip }));

  const copySel = (): Fragment | null => {
    if (!sel.length) return null;
    const f = extractFragment(model, sel);
    clip.current = f;
    return f;
  };
  const paste = (f: Fragment | null, at?: { x: number; y: number }, offset = 30) => {
    if (!f?.components.length) return;
    const r = pasteFragment(model, f, refKeys, at, offset);
    commit(() => r.model);
    select(r.ids);
  };
  const duplicateSel = () => paste(sel.length ? extractFragment(model, sel) : null);
  const pasteAtPointer = (f: Fragment | null) => paste(f, pointer.current.inside ? rf.screenToFlowPosition(pointer.current) : undefined);

  const rename = (id: string, nid: string): string | null => {
    if (model.components.some((c) => c.id === nid)) return `There is already a component called ${nid}.`;
    commit((m) => renameComp(m, id, nid, refKeys));
    setSel((s) => s.map((x) => (x === id ? nid : x)));
    return null;
  };

  const setParam = (id: string, key: string, v: any) =>
    commit((m) => ({
      ...m,
      components: m.components.map((c) => {
        if (c.id !== id) return c;
        const params = { ...c.params };
        if (v === null || v === undefined || v === '') delete params[key];
        else params[key] = v;
        return { ...c, params };
      }),
    }));

  const fit = () => rf.fitView({ padding: 0.25, duration: 200 });
  const goTo = (id: string) => {
    const c = byId.get(id);
    if (!c) return;
    select([id]);
    pref({ right: true });
    const [w, h] = nodeSize(c);
    rf.setCenter(c.ui.x + w / 2, c.ui.y + h / 2, { zoom: Math.max(rf.getZoom(), 1), duration: 300 });
  };

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      setNodes((ns) => applyNodeChanges(changes.filter((c) => c.type !== 'remove'), ns));
      const moved = new Map<string, { x: number; y: number }>();
      const picks: [string, boolean][] = [];
      for (const ch of changes) {
        if (ch.type === 'position') {
          if (ch.dragging) wasDragging.current = true;
          else if (ch.position) moved.set(ch.id, ch.position);
        } else if (ch.type === 'select') {
          picks.push([ch.id, ch.selected]);
        }
      }
      if (moved.size) {
        // a drag is one undo step; repeated arrow-key nudges merge into one
        const nudge = !wasDragging.current;
        wasDragging.current = false;
        commit(
          (m) => {
            let any = false;
            const components = m.components.map((c) => {
              const p = moved.get(c.id);
              if (!p || (Math.round(p.x) === Math.round(c.ui.x) && Math.round(p.y) === Math.round(c.ui.y))) return c;
              any = true;
              return { ...c, ui: { ...c.ui, x: p.x, y: p.y } };
            });
            return any ? { ...m, components } : m;
          },
          nudge ? `nudge:${[...moved.keys()].join(',')}` : undefined
        );
      }
      if (picks.length) {
        setSel((prev) => {
          const s = new Set(prev);
          for (const [id, on] of picks) on ? s.add(id) : s.delete(id);
          return s.size === prev.length && prev.every((x) => s.has(x)) ? prev : [...s];
        });
      }
    },
    [commit]
  );

  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    const picks = changes.filter((c) => c.type === 'select') as { id: string; selected: boolean }[];
    if (!picks.length) return;
    setSelE((prev) => {
      const s = new Set(prev);
      for (const p of picks) p.selected ? s.add(p.id) : s.delete(p.id);
      return s.size === prev.length && prev.every((x) => s.has(x)) ? prev : [...s];
    });
  }, []);

  const connectable = (c: { source: string | null; target: string | null; sourceHandle?: string | null; targetHandle?: string | null }) =>
    !!c.source && !!c.target && c.source !== c.target && ![c.source, c.target].some((id) => byId.get(id)?.type === 'shaft');

  const onConnect = (c: Connection) => {
    if (!connectable(c)) return;
    const ref = (id: string, h: string | null | undefined) => (NODE_LIKE.has(byId.get(id)?.type ?? '') ? id : `${id}.${h}`);
    const a = ref(c.source!, c.sourceHandle);
    const b = ref(c.target!, c.targetHandle);
    if (hasConn(model, a, b)) return toast('These two ports are already connected.');
    if (byId.has(a) && byId.has(b)) return toast('Two volumes cannot be joined directly. Put a pipe, valve or orifice between them.');
    commit((m) => (hasConn(m, a, b) ? m : { ...m, connections: [...m.connections, [a, b]] }));
    select([], [edgeId(a, b)]);
  };

  const loadModel = (m: Model, opts: { autoRun?: boolean; handle?: any } = {}) => {
    commit(() => m);
    setSaved(m);
    fileHandle.current = opts.handle ?? null;
    select([]);
    setRs(null);
    setRefRs(null);
    setRunErr(null);
    setSelCh([]);
    setPlaying(false);
    setTimeout(fit, 50);
    if (opts.autoRun) setTimeout(() => setPendingRun(true), 80);
  };
  const [pendingRun, setPendingRun] = useState(false);
  useEffect(() => {
    if (pendingRun) {
      setPendingRun(false);
      run(model.simulation.t_end > 0 ? 'transient' : 'steady');
    }
  }, [pendingRun, run, model]);

  // ---- files
  const openText = (text: string, name: string, handle?: any) => {
    try {
      loadModel(parseModel(text), { handle });
      toast(`Opened ${name}`);
    } catch (err: any) {
      toast(`Could not open ${name}: ${err.message}`, 'bad');
    }
  };
  const openFile = async () => {
    if (!canPickFiles() || !(window as any).showOpenFilePicker) return fileRef.current?.click();
    try {
      const [h] = await (window as any).showOpenFilePicker({ types: [{ description: 'fluidnet model', accept: { 'application/yaml': ['.yaml', '.yml'], 'application/json': ['.json'] } }] });
      const f = await h.getFile();
      openText(await f.text(), f.name, f.name.endsWith('.json') ? undefined : h);
    } catch {
      /* cancelled */
    }
  };
  const save = async (saveAs = false) => {
    try {
      const h = await saveText(toYaml(model), `${fileStem(model.name)}.yaml`, fileHandle.current, saveAs);
      if (h === undefined) return;
      fileHandle.current = h;
      setSaved(model);
      toast(h ? `Saved ${h.name}` : `Downloaded ${fileStem(model.name)}.yaml`);
    } catch (e: any) {
      toast(`Could not save: ${e.message ?? e}`, 'bad');
    }
  };

  // ---- context menus
  const addItems = (at: { x: number; y: number }, into?: string): MenuItem[] =>
    (catalog?.components ?? [])
      .filter((c: any) => !into || fitsLine(c.type, into))
      .map((c: any) => ({ label: c.label, onClick: () => addComp(c.type, at, into) }));
  const alignItems = (): MenuItem[] =>
    (
      [
        ['left', 'Left edges'],
        ['hcenter', 'Centres'],
        ['right', 'Right edges'],
        ['top', 'Top edges'],
        ['vcenter', 'Middles'],
        ['bottom', 'Bottom edges'],
        ['hdist', 'Space evenly across'],
        ['vdist', 'Space evenly down'],
      ] as [AlignMode, string][]
    ).map(([mode, label]) => ({ label, disabled: mode.endsWith('dist') && sel.length < 3, onClick: () => align(mode) }));
  const align = (mode: AlignMode) => commit((m) => alignComps(m, sel, mode));
  const selectionMenu = (n: number): MenuItem[] => [
    { label: 'Duplicate', hint: `${MOD}+D`, onClick: () => act.current.duplicateSel() },
    { label: 'Copy', hint: `${MOD}+C`, onClick: () => act.current.copy() },
    { label: 'Cut', hint: `${MOD}+X`, onClick: () => act.current.cut() },
    { separator: true },
    { label: 'Rotate', hint: 'R', onClick: () => act.current.rotateSel() },
    { label: 'Flip', hint: 'F', onClick: () => act.current.flipSel() },
    ...(n > 1 ? [{ label: 'Align', items: alignItems() } as MenuItem] : []),
    { separator: true },
    { label: n > 1 ? `Delete ${n} components` : 'Delete', hint: 'Del', danger: true, onClick: () => act.current.deleteSelection() },
  ];
  const openMenu = (e: React.MouseEvent | MouseEvent, items: MenuItem[]) => {
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY, items });
  };

  // ---- keyboard and clipboard. Handlers are registered once and read the
  // current closures through this ref.
  const act = useRef<any>({});
  act.current = {
    run,
    undo: hist.undo,
    redo: hist.redo,
    deleteSelection,
    duplicateSel,
    rotateSel,
    flipSel,
    fit,
    save,
    openFile,
    selectAll: () => select(model.components.map((c) => c.id), model.connections.map(([a, b]) => edgeId(a, b))),
    clear: () => select([]),
    copy: () => {
      const f = copySel();
      if (f) {
        navigator.clipboard?.writeText(fragmentToText(f)).catch(() => {});
        toast(`Copied ${f.components.length} component${f.components.length > 1 ? 's' : ''}`);
      }
    },
    cut: () => {
      const f = copySel();
      if (f) {
        navigator.clipboard?.writeText(fragmentToText(f)).catch(() => {});
        deleteSelection();
      }
    },
    copySel,
    pasteAtPointer,
    hasSel: sel.length > 0,
    canPlay: !!rs && rs.mode !== 'steady' && rs.nt > 1,
    togglePlay: () => setPlaying((p) => !p),
    overlayOpen: yamlOpen || studyOpen || paletteOpen || keysOpen,
  };
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const a = act.current;
      const mod = e.ctrlKey || e.metaKey;
      const key = e.key.toLowerCase();
      const editing = isEditing(e.target);
      if (key === 'enter' && mod) {
        e.preventDefault();
        return a.run('transient');
      }
      if (key === 'enter' && e.shiftKey && (e.target as HTMLElement)?.tagName !== 'TEXTAREA') {
        e.preventDefault();
        return a.run('steady');
      }
      if (mod && key === 's') {
        e.preventDefault();
        return a.save(e.shiftKey);
      }
      if (mod && key === 'o') {
        e.preventDefault();
        return a.openFile();
      }
      if (mod && key === 'k') {
        e.preventDefault();
        return setPaletteOpen((o) => !o);
      }
      if (editing || a.overlayOpen) return;
      if (mod && key === 'z') {
        e.preventDefault();
        return e.shiftKey ? a.redo() : a.undo();
      }
      if (mod && key === 'y') {
        e.preventDefault();
        return a.redo();
      }
      if (mod && key === 'a') {
        e.preventDefault();
        return a.selectAll();
      }
      if (mod && key === 'd') {
        e.preventDefault();
        return a.duplicateSel();
      }
      if (mod || e.altKey) return;
      if (key === 'delete' || key === 'backspace') {
        e.preventDefault();
        a.deleteSelection();
      } else if (key === 'r') a.rotateSel();
      else if (key === 'f') a.flipSel();
      else if (key === '1') a.fit();
      else if (key === '?') setKeysOpen(true);
      else if (key === 'escape') a.clear();
      else if (key === 'p' && a.canPlay) a.togglePlay();
    };
    const textSelected = () => !!window.getSelection()?.toString();
    const onCopy = (e: ClipboardEvent) => {
      const a = act.current;
      if (isEditing(e.target) || a.overlayOpen || textSelected() || !a.hasSel) return;
      const f = a.copySel();
      e.clipboardData?.setData('text/plain', fragmentToText(f));
      e.preventDefault();
      if (e.type === 'cut') a.deleteSelection();
      else toast(`Copied ${f.components.length} component${f.components.length > 1 ? 's' : ''}`);
    };
    const onPaste = (e: ClipboardEvent) => {
      const a = act.current;
      if (isEditing(e.target) || a.overlayOpen) return;
      const f = fragmentFromText(e.clipboardData?.getData('text/plain') ?? '') ?? clip.current;
      if (!f) return;
      e.preventDefault();
      a.pasteAtPointer(f);
    };
    window.addEventListener('keydown', onKey);
    document.addEventListener('copy', onCopy);
    document.addEventListener('cut', onCopy);
    document.addEventListener('paste', onPaste);
    return () => {
      window.removeEventListener('keydown', onKey);
      document.removeEventListener('copy', onCopy);
      document.removeEventListener('cut', onCopy);
      document.removeEventListener('paste', onPaste);
    };
  }, [toast]);

  // ---- command palette
  const commands = useMemo<Command[]>(() => {
    if (!paletteOpen) return [];
    const out: Command[] = [
      { group: 'Run', label: 'Run transient', hint: `${MOD}+Enter`, run: () => run('transient') },
      { group: 'Run', label: 'Solve steady state', hint: 'Shift+Enter', run: () => run('steady') },
      { group: 'Run', label: 'Parameter study…', keywords: 'sweep', run: () => setStudyOpen(true) },
      { group: 'File', label: 'Save', hint: `${MOD}+S`, run: () => save() },
      { group: 'File', label: 'Save as…', hint: `${MOD}+Shift+S`, run: () => save(true) },
      { group: 'File', label: 'Open…', hint: `${MOD}+O`, run: openFile },
      { group: 'File', label: 'New model', run: () => loadModel(emptyModel()) },
      { group: 'File', label: 'Edit as YAML', run: () => setYamlOpen(true) },
      { group: 'Edit', label: 'Undo', hint: `${MOD}+Z`, run: hist.undo },
      { group: 'Edit', label: 'Redo', hint: `${MOD}+Shift+Z`, run: hist.redo },
      { group: 'Edit', label: 'Select all', hint: `${MOD}+A`, run: () => act.current.selectAll() },
      { group: 'View', label: 'Fit schematic to view', hint: '1', run: fit },
      { group: 'View', label: prefs.theme === 'dark' ? 'Light theme' : 'Dark theme', keywords: 'theme dark light', run: () => pref({ theme: prefs.theme === 'dark' ? 'light' : 'dark' }) },
      { group: 'View', label: `${prefs.values ? 'Hide' : 'Show'} values on the schematic`, run: () => pref({ values: !prefs.values }) },
      { group: 'View', label: `${prefs.minimap ? 'Hide' : 'Show'} minimap`, run: () => pref({ minimap: !prefs.minimap }) },
      { group: 'View', label: 'Keyboard shortcuts', hint: '?', run: () => setKeysOpen(true) },
    ];
    if (rs) out.push({ group: 'Results', label: 'Export results as CSV', run: () => downloadText(`${fileStem(model.name)}_${rs.mode}.csv`, resultsCsv(rs), 'text/csv') });
    for (const c of model.components) out.push({ group: 'Go to', label: c.id, keywords: c.type.replace('_', ' '), hint: c.type.replace('_', ' '), run: () => goTo(c.id) });
    for (const c of catalog?.components ?? []) out.push({ group: 'Add', label: c.label, keywords: `${c.type} ${c.category}`, run: () => addComp(c.type) });
    for (const [n, text] of EXAMPLES) out.push({ group: 'Example', label: n, run: () => loadModel(parseModel(text), { autoRun: true }) });
    return out;
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [paletteOpen]);

  const single = sel.length === 1 ? byId.get(sel[0]) ?? null : null;
  const multi = useMemo(() => model.components.filter((c) => sel.includes(c.id)), [model.components, sel]);

  // ---- results panel resize
  const drag = useRef<{ y: number; h: number } | null>(null);
  useEffect(() => {
    const mv = (e: PointerEvent) => {
      if (!drag.current) return;
      setResH(Math.min(window.innerHeight - 160, Math.max(140, drag.current.h + (drag.current.y - e.clientY))));
    };
    const up = () => (drag.current = null);
    window.addEventListener('pointermove', mv);
    window.addEventListener('pointerup', up);
    return () => {
      window.removeEventListener('pointermove', mv);
      window.removeEventListener('pointerup', up);
    };
  }, []);

  const st = rs?.meta.stats;
  const stLine = !rs
    ? ''
    : rs.mode === 'steady'
      ? `Steady state solved in ${fmt(rs.meta.elapsed_ms, 3)} ms · ${rs.meta.n_states} states · ${rs.meta.steady?.method ?? ''}`
      : rs.mode === 'study'
        ? `Parameter study of ${rs.xName}: ${rs.meta.study.solved} of ${rs.nt} points solved in ${fmt(rs.meta.elapsed_ms, 3)} ms`
        : `${fmt(rs.time[rs.nt - 1], 4)} s simulated in ${fmt(rs.meta.elapsed_ms, 3)} ms · ${rs.meta.n_states} states · ${st?.steps ?? 0} steps${st?.sparse ? ' · sparse' : ''}`;
  const pct = running === 'transient' ? Math.min(100, (100 * progress) / model.simulation.t_end) : running === 'study' ? 100 * progress : 30;
  const gridStyle: any = { ['--res-h']: prefs.dock ? `${resH}px` : '0px', ['--left-w']: prefs.left ? '216px' : '0px', ['--right-w']: prefs.right ? '312px' : '0px' };
  const edgeAt = (x: number, y: number): string | undefined => {
    for (const el of document.elementsFromPoint(x, y)) {
      const id = (el as HTMLElement).closest?.('.react-flow__edge')?.getAttribute('data-id');
      if (id?.startsWith('e:')) return id.slice(2);
    }
    return undefined;
  };
  const tools: [keyof Prefs, string, string][] = [
    ['left', 'Parts', 'Show or hide the parts list'],
    ['right', 'Inspector', 'Show or hide the inspector'],
    ['dock', 'Results', 'Show or hide the results'],
    ['values', 'Values', 'Show results and flow direction on the schematic'],
    ['snap', 'Snap', 'Snap parts to a 10 px grid'],
    ['minimap', 'Map', 'Show an overview map'],
  ];

  return (
    <div className="app" style={gridStyle}>
      <header className="app-top">
        <div className="brand">
          <span className="brand-mark" />
          fluidnet
        </div>
        <input id="model-name" className="name-input" value={model.name} onChange={(e) => commit((m) => ({ ...m, name: e.target.value }), 'name')} aria-label="System name" />
        {dirty && <span className="dirty" title="Changes since the last save. The model is also kept in this browser.">edited</span>}
        <select
          id="examples"
          className="field examples"
          value=""
          onChange={(e) => {
            const ex = EXAMPLES.find((x) => x[0] === e.target.value);
            if (ex) loadModel(parseModel(ex[1]), { autoRun: true });
          }}
        >
          <option value="">Examples</option>
          {EXAMPLES.map(([n]) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
        <button className="btn-ghost" onClick={() => loadModel(emptyModel())} title="Start an empty model (undo brings the current one back)">
          New
        </button>
        <button className="btn-ghost" onClick={openFile} title={`Open a model file (${MOD}+O)`}>
          Open
        </button>
        <button className="btn-ghost" id="save" onClick={() => save()} title={`Save the model as YAML (${MOD}+S)`}>
          Save
        </button>
        <input
          ref={fileRef}
          id="open-file"
          type="file"
          accept=".yaml,.yml,.json"
          hidden
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (f) openText(await f.text(), f.name);
            e.target.value = '';
          }}
        />
        <button className="btn-ghost" onClick={() => setYamlOpen(true)}>
          YAML
        </button>
        <span className="top-sep" />
        <button className="btn-icon" id="undo" disabled={!hist.canUndo} onClick={hist.undo} title={`Undo (${MOD}+Z)`} aria-label="Undo">
          <Icon d={IC.undo} />
        </button>
        <button className="btn-icon" id="redo" disabled={!hist.canRedo} onClick={hist.redo} title={`Redo (${MOD}+Shift+Z)`} aria-label="Redo">
          <Icon d={IC.redo} />
        </button>
        <button className="btn-icon" id="find" onClick={() => setPaletteOpen(true)} title={`Find a component or command (${MOD}+K)`} aria-label="Find">
          <Icon d={IC.search} />
        </button>
        <div className="spacer" />
        <button
          className={`status ${check?.error ? 'bad' : check ? 'good' : ''}`}
          title={check?.error ? String(check.error).replace(/^model error: /, '') : 'Model check'}
          onClick={() => {
            pref({ right: true });
            if (check?.error && errId) goTo(errId);
            else select([]);
          }}
        >
          {engineErr ? 'Engine failed to load' : !catalog ? 'Loading engine…' : check?.error ? 'Model has an error' : check ? `${check.states} states` : 'Empty'}
        </button>
        {running ? (
          <>
            <div className="progress">
              <div style={{ width: `${pct}%` }} />
            </div>
            <button className="btn-secondary" onClick={stop}>
              Stop
            </button>
          </>
        ) : (
          <>
            <button className="btn-ghost" id="study" disabled={!catalog || !!check?.error} onClick={() => setStudyOpen(true)} title="Sweep a parameter and solve the steady state at each value">
              Study
            </button>
            <button className="btn-secondary" disabled={!catalog || !!check?.error} onClick={() => run('steady')} title="Shift+Enter">
              Steady state
            </button>
            <button className="btn-primary" disabled={!catalog || !!check?.error} onClick={() => run('transient')} title={`${MOD}+Enter`}>
              Run transient
            </button>
          </>
        )}
        <button className="btn-icon" id="theme" onClick={() => pref({ theme: prefs.theme === 'dark' ? 'light' : 'dark' })} title={prefs.theme === 'dark' ? 'Light theme' : 'Dark theme'} aria-label="Toggle theme">
          <Icon d={prefs.theme === 'dark' ? IC.sun : IC.moon} />
        </button>
        <button className="btn-icon" onClick={() => setKeysOpen(true)} title="Keyboard shortcuts (?)" aria-label="Keyboard shortcuts">
          <Icon d={IC.help} />
        </button>
      </header>

      {prefs.left && (
        <aside className="app-left">
          <Palette catalog={catalog} onAdd={(t) => addComp(t)} />
        </aside>
      )}

      <main
        className="canvas"
        onPointerMove={(e) => (pointer.current = { x: e.clientX, y: e.clientY, inside: true })}
        onPointerLeave={() => (pointer.current.inside = false)}
        onDragOver={(e) => {
          e.preventDefault();
          e.dataTransfer.dropEffect = e.dataTransfer.types.includes('Files') ? 'copy' : 'move';
        }}
        onDrop={async (e) => {
          e.preventDefault();
          const file = e.dataTransfer.files?.[0];
          if (file) return openText(await file.text(), file.name);
          const t = e.dataTransfer.getData('application/fluidnet');
          if (t) addComp(t, rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }), edgeAt(e.clientX, e.clientY));
        }}
      >
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          isValidConnection={connectable}
          onNodeContextMenu={(e, n) => {
            const ids = sel.includes(n.id) ? sel : [n.id];
            if (ids !== sel) select(ids);
            openMenu(e, selectionMenu(ids.length));
          }}
          onSelectionContextMenu={(e) => openMenu(e, selectionMenu(sel.length))}
          onEdgeContextMenu={(e, edge) => {
            if (!edge.id.startsWith('e:')) return e.preventDefault();
            select([], [edge.id]);
            const at = rf.screenToFlowPosition({ x: e.clientX, y: e.clientY });
            openMenu(e, [
              { label: 'Insert into this line', items: addItems(at, edge.id.slice(2)) },
              { separator: true },
              { label: 'Delete connection', hint: 'Del', danger: true, onClick: () => act.current.deleteSelection() },
            ]);
          }}
          onPaneContextMenu={(e) => {
            const at = rf.screenToFlowPosition({ x: e.clientX, y: e.clientY });
            openMenu(e, [
              { label: 'Add', items: addItems(at) },
              { label: 'Paste here', hint: `${MOD}+V`, disabled: !clip.current, onClick: () => paste(clip.current, at) },
              { separator: true },
              { label: 'Select all', hint: `${MOD}+A`, onClick: () => act.current.selectAll() },
              { label: 'Fit to view', hint: '1', onClick: fit },
            ]);
          }}
          connectionMode={ConnectionMode.Loose}
          fitView
          fitViewOptions={{ padding: 0.25 }}
          minZoom={0.2}
          maxZoom={3}
          snapToGrid={prefs.snap}
          snapGrid={[10, 10]}
          deleteKeyCode={null}
          multiSelectionKeyCode={['Meta', 'Control', 'Shift']}
          selectionKeyCode="Shift"
          connectionRadius={24}
          proOptions={{ hideAttribution: true }}
        >
          <Background gap={20} size={1} color="var(--dot)" />
          <Controls showInteractive={false} />
          {prefs.minimap && <MiniMap pannable zoomable nodeColor="var(--line-strong)" maskColor="var(--mask)" bgColor="var(--side)" nodeStrokeWidth={0} />}
        </ReactFlow>
        {engineErr && <div className="banner bad">The simulation engine could not start: {engineErr}</div>}
        <div className="canvas-tools" role="toolbar" aria-label="View">
          {tools.map(([key, label, title]) => (
            <button key={key} className={`tool${prefs[key] ? ' on' : ''}`} aria-pressed={!!prefs[key]} title={title} onClick={() => pref({ [key]: !prefs[key] } as any)}>
              {label}
            </button>
          ))}
        </div>
        {catalog && model.components.length === 0 && (
          <div className="canvas-empty">
            <b>Empty model</b>
            <span>Drag parts from the left, right-click to add one here, or load one of the examples.</span>
          </div>
        )}
        <div className="canvas-legend">
          {[...new Set(fluids.values())].map((f) => (
            <span key={f}>
              <i style={{ background: FLUID_COLORS[f] ?? 'var(--ink)' }} />
              {f}
            </span>
          ))}
        </div>
      </main>

      {prefs.right && (
        <aside className="app-right">
          <Inspector
            model={model}
            key={single?.id ?? '-'}
            selected={single}
            multi={multi}
            nConn={selE.length}
            catalog={catalog}
            check={check}
            errId={errId}
            rs={rs}
            k={k}
            plotted={selCh}
            onTogglePlot={(n) => {
              setSelCh((s) => (s.includes(n) ? s.filter((x) => x !== n) : [...s, n]));
              if (rs && rs.mode !== 'steady') {
                setTab('plots');
                pref({ dock: true });
              }
            }}
            onParam={setParam}
            onRename={rename}
            onFlip={flipSel}
            onRotate={rotateSel}
            onDuplicate={duplicateSel}
            onDelete={deleteSelection}
            onAlign={align}
            onSelect={goTo}
            onModel={(p) => commit((m) => ({ ...m, ...p }), 'meta')}
            onSim={(p) => commit((m) => ({ ...m, simulation: { ...m.simulation, ...p } }))}
          />
        </aside>
      )}

      {prefs.dock && (
        <section className="app-dock">
          <div
            className="resizer"
            onPointerDown={(e) => {
              drag.current = { y: e.clientY, h: resH };
            }}
          />
          {runErr && (
            <div className="banner bad inline">
              {runErr}
              <button className="mini" onClick={() => setRunErr(null)}>
                dismiss
              </button>
            </div>
          )}
          <Results
            rs={rs}
            refRs={refRs}
            setRefRs={setRefRs}
            tab={tab}
            setTab={setTab}
            selected={selCh}
            setSelected={setSelCh}
            probe={probe}
            setProbe={setProbe}
            playing={playing}
            setPlaying={setPlaying}
            height={resH}
            theme={prefs.theme}
            stale={stale}
            busy={running ? (running === 'study' ? 'Running the study…' : 'Solving…') : null}
            selIds={sel}
            onSelectComp={goTo}
            checkWarnings={check?.warnings ?? []}
            runErr={runErr}
          />
          <div className="foot mono">
            <span>{stLine}</span>
            {rs?.meta.warnings?.length ? (
              <button className="warn-count" onClick={() => setTab('messages')}>
                {rs.meta.warnings.length} warning(s)
              </button>
            ) : null}
          </div>
        </section>
      )}

      {menu && <ContextMenu {...menu} onClose={() => setMenu(null)} />}
      {yamlOpen && <YamlDialog model={model} onClose={() => setYamlOpen(false)} onApply={(m) => {
            commit(() => m);
            select([]);
            setTimeout(fit, 50);
          }} />}
      {studyOpen && <StudyDialog model={model} catalog={catalog} initial={studySpec} selectedId={single?.id ?? null} onRun={study} onClose={() => setStudyOpen(false)} />}
      {paletteOpen && <CommandPalette commands={commands} onClose={() => setPaletteOpen(false)} />}
      {keysOpen && <ShortcutsDialog onClose={() => setKeysOpen(false)} />}
      <Toasts toasts={toasts} />
    </div>
  );
}

function YamlDialog({ model, onClose, onApply }: { model: Model; onClose: () => void; onApply: (m: Model) => void }) {
  const [text, setText] = useState(() => toYaml(model));
  const [err, setErr] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  return (
    <div className="modal-back" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-head">
          <b>Model file</b>
          <span className="muted">YAML is the source of truth; the CLI runs this file directly.</span>
        </div>
        <textarea id="yaml-text" className="yaml" spellCheck={false} value={text} onChange={(e) => setText(e.target.value)} />
        {err && <div className="msg msg-err">{err}</div>}
        <div className="modal-actions">
          <button
            className="btn-ghost"
            onClick={async () => {
              try {
                await navigator.clipboard.writeText(text);
                setCopied(true);
              } catch {
                const ta = document.getElementById('yaml-text') as HTMLTextAreaElement;
                ta.select();
              }
            }}
          >
            {copied ? 'Copied' : 'Copy'}
          </button>
          <button className="btn-ghost" onClick={() => downloadText(`${fileStem(model.name)}.yaml`, text, 'application/yaml')}>
            Download
          </button>
          <div className="spacer" />
          <button className="btn-secondary" onClick={onClose}>
            Close
          </button>
          <button
            className="btn-primary"
            onClick={() => {
              try {
                onApply(parseModel(text));
                onClose();
              } catch (e: any) {
                setErr(String(e.message ?? e));
              }
            }}
          >
            Apply
          </button>
        </div>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <ReactFlowProvider>
      <Editor />
    </ReactFlowProvider>
  );
}
