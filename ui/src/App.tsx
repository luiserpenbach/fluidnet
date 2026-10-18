import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  ReactFlow,
  ReactFlowProvider,
  Background,
  Controls,
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
import { Comp, Model, parseModel, toYaml, toEngineJson, splitPort, fluidMap, usedFluids, uniqueId, fmt, FLUID_COLORS, emptyModel } from './model';
import { CompNode, CompNodeData } from './CompNode';
import { Inspector } from './Inspector';
import { Results, ResultSet, defaultSelection } from './Results';
import { Symbol, SYMBOL_SIZE } from './symbols';

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

const NODE_LIKE = new Set(['tank', 'junction', 'boundary']);
const STORE_KEY = 'fluidnet.model.v1';
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
  if (!catalog) return <div className="palette" />;
  const cats = new Map<string, any[]>();
  for (const c of catalog.components) {
    if (!cats.has(c.category)) cats.set(c.category, []);
    cats.get(c.category)!.push(c);
  }
  return (
    <div className="palette">
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
      <div className="pal-foot">Drag onto the canvas, or click to add.</div>
    </div>
  );
}

function Editor() {
  const engine = useRef(new Engine()).current;
  const rf = useReactFlow();
  const [catalog, setCatalog] = useState<any>(null);
  const [engineErr, setEngineErr] = useState<string | null>(null);
  const [model, setModel] = useState<Model>(() => loadStored() ?? parseModel(exExpThrottle));
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [check, setCheck] = useState<any>(null);
  const [rs, setRs] = useState<ResultSet | null>(null);
  const [selCh, setSelCh] = useState<string[]>([]);
  const [probe, setProbe] = useState<number | null>(null);
  const [running, setRunning] = useState<null | 'steady' | 'transient'>(null);
  const [progress, setProgress] = useState(0);
  const [runErr, setRunErr] = useState<string | null>(null);
  const [yamlOpen, setYamlOpen] = useState(false);
  const [resH, setResH] = useState(320);
  const [nodes, setNodes] = useState<Node[]>([]);
  const fileRef = useRef<HTMLInputElement>(null);
  const autoRan = useRef(false);

  // ---- engine boot
  useEffect(() => {
    engine.onProgress = (t) => setProgress(t);
    engine
      .init()
      .then(() => setCatalog(engine.catalog))
      .catch((e) => setEngineErr(String(e.message ?? e)));
  }, [engine]);

  // ---- autosave
  useEffect(() => {
    try {
      localStorage.setItem(STORE_KEY, toYaml(model));
    } catch {
      /* storage unavailable */
    }
  }, [model]);

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
        setCheck({ error: String(e.message ?? e) });
      }
    }, 250);
    return () => clearTimeout(t);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [checkKey, catalog]);

  // ---- run
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
        setRs(set);
        setProbe(null);
        setSelCh((prev) => {
          const keep = prev.filter((n) => set.has(n));
          return keep.length ? keep : defaultSelection(set);
        });
        if (set.meta.error) setRunErr(`Stopped early: ${set.meta.error}`);
      } catch (e: any) {
        if (String(e.message) !== 'stopped') setRunErr(String(e.message ?? e));
      } finally {
        setRunning(null);
      }
    },
    [running, catalog, engine, model]
  );

  useEffect(() => {
    if (catalog && !autoRan.current && model.components.length) {
      autoRan.current = true;
      run('transient');
    }
  }, [catalog, model.components.length, run]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const tag = (e.target as HTMLElement)?.tagName;
      if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        run('transient');
      } else if (e.key === 'Enter' && e.shiftKey && tag !== 'TEXTAREA') {
        e.preventDefault();
        run('steady');
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [run]);

  // ---- graph derivation
  const fluids = useMemo(() => fluidMap(model), [model]);
  const byId = useMemo(() => new Map(model.components.map((c) => [c.id, c])), [model]);
  const errId = useMemo(() => {
    const m = /^model error: ([A-Za-z0-9_\-]+)[:.]/.exec(check?.error ?? '');
    return m ? m[1] : null;
  }, [check]);
  const k = probe ?? (rs ? rs.nt - 1 : 0);

  useEffect(() => {
    setNodes((prev) => {
      const prevMap = new Map(prev.map((n) => [n.id, n]));
      return model.components.map((c) => {
        const p = prevMap.get(c.id);
        const data: CompNodeData = {
          id: c.id,
          type: c.type,
          flip: !!c.ui.flip,
          nodeLike: NODE_LIKE.has(c.type),
          overlay: overlayFor(c, rs, k),
          color: FLUID_COLORS[fluids.get(c.id) ?? ''] ?? 'var(--ink)',
          error: errId === c.id,
        };
        return {
          ...(p ?? {}),
          id: c.id,
          type: 'comp',
          position: p?.dragging ? p.position : { x: c.ui.x, y: c.ui.y },
          data,
          selected: c.id === selectedId,
        } as Node;
      });
    });
  }, [model.components, rs, k, fluids, selectedId, errId]);

  const edges: Edge[] = useMemo(() => {
    const center = (id: string) => {
      const c = byId.get(id);
      return c ? { x: c.ui.x, y: c.ui.y } : { x: 0, y: 0 };
    };
    const handleFor = (comp: string, port: string | null, other: string) => {
      const c = byId.get(comp);
      if (!c) return null;
      if (!NODE_LIKE.has(c.type)) return port;
      const a = center(comp);
      const b = center(other);
      const dx = b.x - a.x;
      const dy = b.y - a.y;
      if (c.type === 'boundary' || Math.abs(dx) >= Math.abs(dy)) return dx >= 0 ? 'p-r' : 'p-l';
      return dy >= 0 ? 'p-b' : 'p-t';
    };
    const fluidEdges: Edge[] = model.connections.map(([a, b], i) => {
      const [ca, pa] = splitPort(a);
      const [cb, pb] = splitPort(b);
      const fluid = fluids.get(ca) ?? fluids.get(cb) ?? '';
      return {
        id: `e${i}:${a}~${b}`,
        source: ca,
        target: cb,
        sourceHandle: handleFor(ca, pa ?? 'p', cb),
        targetHandle: handleFor(cb, pb ?? 'p', ca),
        type: 'smoothstep',
        style: { stroke: FLUID_COLORS[fluid] ?? 'var(--ink-soft)', strokeWidth: 2 },
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
          className: 'heat-link',
          style: { stroke: 'var(--heat)', strokeWidth: 1.2, strokeDasharray: '2 4' },
        } as Edge);
      }
    }
    return [...fluidEdges, ...links];
  }, [model.connections, model.components, byId, fluids]);

  // ---- mutations
  const mutate = (f: (m: Model) => Model) => setModel((m) => f(m));
  const addComp = (type: string, pos?: { x: number; y: number }) => {
    mutate((m) => {
      const id = uniqueId(m, type === 'check_valve' ? 'check' : type === 'mass_flow' ? 'flow' : type);
      const params: Record<string, any> = { ...(NEW_DEFAULTS[type] ?? {}) };
      if (type === 'tank' || type === 'boundary') {
        const used = usedFluids(m);
        params.fluid = used[0] ?? 'N2';
        params.fluid && Object.assign(params, {});
      }
      const p = pos ?? rf.screenToFlowPosition({ x: window.innerWidth / 2, y: window.innerHeight / 3 });
      setSelectedId(id);
      return { ...m, components: [...m.components, { id, type, params: { ...params }, ui: { x: p.x, y: p.y } }] };
    });
  };

  const onNodesChange = useCallback(
    (changes: NodeChange[]) => {
      setNodes((ns) => applyNodeChanges(changes, ns));
      for (const ch of changes) {
        if (ch.type === 'position' && ch.dragging === false && ch.position) {
          const pos = ch.position;
          mutate((m) => ({ ...m, components: m.components.map((c) => (c.id === ch.id ? { ...c, ui: { ...c.ui, x: pos.x, y: pos.y } } : c)) }));
        } else if (ch.type === 'position' && ch.dragging && ch.position) {
          // keep model in sync for edge handle selection after drag ends
        } else if (ch.type === 'select' && ch.selected) {
          setSelectedId(ch.id);
        } else if (ch.type === 'remove') {
          removeComp(ch.id);
        }
      }
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    []
  );

  const onEdgesChange = useCallback((changes: EdgeChange[]) => {
    const removed = changes.filter((c) => c.type === 'remove').map((c: any) => c.id as string).filter((id) => id.startsWith('e'));
    if (removed.length) {
      mutate((m) => ({ ...m, connections: m.connections.filter(([a, b], i) => !removed.includes(`e${i}:${a}~${b}`)) }));
    }
  }, []);

  const onConnect = useCallback(
    (c: Connection) => {
      if (!c.source || !c.target || c.source === c.target) return;
      if ([c.source, c.target].some((id) => model.components.find((x) => x.id === id)?.type === 'shaft')) return;
      const ref = (id: string, h: string | null | undefined) => {
        const comp = model.components.find((x) => x.id === id);
        if (!comp || comp.type === 'shaft') return id;
        return NODE_LIKE.has(comp.type) ? id : `${id}.${h}`;
      };
      const a = ref(c.source, c.sourceHandle);
      const b = ref(c.target, c.targetHandle);
      mutate((m) => {
        if (m.connections.some(([x, y]) => (x === a && y === b) || (x === b && y === a))) return m;
        return { ...m, connections: [...m.connections, [a, b]] };
      });
    },
    [model.components]
  );

  const removeComp = (id: string) => {
    mutate((m) => ({
      ...m,
      components: m.components.filter((c) => c.id !== id),
      connections: m.connections.filter(([a, b]) => splitPort(a)[0] !== id && splitPort(b)[0] !== id && a !== id && b !== id),
    }));
    setSelectedId((s) => (s === id ? null : s));
  };

  const renameComp = (id: string, nid: string) => {
    if (model.components.some((c) => c.id === nid)) return;
    const rn = (r: string) => {
      if (r === id) return nid;
      const [c, p] = splitPort(r);
      return c === id ? `${nid}.${p}` : r;
    };
    mutate((m) => ({
      ...m,
      components: m.components.map((c) => (c.id === id ? { ...c, id: nid } : c)),
      connections: m.connections.map(([a, b]) => [rn(a), rn(b)] as [string, string]),
    }));
    setSelectedId(nid);
  };

  const setParam = (id: string, key: string, v: any) =>
    mutate((m) => ({
      ...m,
      components: m.components.map((c) => {
        if (c.id !== id) return c;
        const params = { ...c.params };
        if (v === null || v === undefined || v === '') delete params[key];
        else params[key] = v;
        return { ...c, params };
      }),
    }));

  const loadModel = (m: Model, autoRun = false) => {
    setModel(m);
    setSelectedId(null);
    setRs(null);
    setRunErr(null);
    setSelCh([]);
    setTimeout(() => rf.fitView({ padding: 0.25, duration: 200 }), 50);
    if (autoRun) setTimeout(() => setPendingRun(true), 80);
  };
  const [pendingRun, setPendingRun] = useState(false);
  useEffect(() => {
    if (pendingRun) {
      setPendingRun(false);
      run(model.simulation.t_end > 0 ? 'transient' : 'steady');
    }
  }, [pendingRun, run, model]);

  const selected = selectedId ? byId.get(selectedId) ?? null : null;

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
  const stLine = rs
    ? rs.meta.mode === 'steady'
      ? `Steady state solved in ${fmt(rs.meta.elapsed_ms, 3)} ms · ${rs.meta.n_states} states · ${rs.meta.steady?.method ?? ''}`
      : `${fmt(rs.time[rs.nt - 1], 4)} s simulated in ${fmt(rs.meta.elapsed_ms, 3)} ms · ${rs.meta.n_states} states · ${st?.steps ?? 0} steps${st?.sparse ? ' · sparse' : ''}`
    : '';

  return (
    <div className="app" style={{ ['--res-h' as any]: `${resH}px` }}>
      <header className="app-top">
        <div className="brand">
          <span className="brand-mark" />
          fluidnet
        </div>
        <input id="model-name" className="name-input" value={model.name} onChange={(e) => setModel({ ...model, name: e.target.value })} aria-label="System name" />
        <select
          id="examples"
          className="field examples"
          value=""
          onChange={(e) => {
            const ex = EXAMPLES.find((x) => x[0] === e.target.value);
            if (ex) loadModel(parseModel(ex[1]), true);
          }}
        >
          <option value="">Examples</option>
          {EXAMPLES.map(([n]) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
        <button className="btn-ghost" onClick={() => loadModel(emptyModel())}>
          New
        </button>
        <button className="btn-ghost" onClick={() => fileRef.current?.click()}>
          Open
        </button>
        <input
          ref={fileRef}
          id="open-file"
          type="file"
          accept=".yaml,.yml,.json"
          hidden
          onChange={async (e) => {
            const f = e.target.files?.[0];
            if (!f) return;
            try {
              loadModel(parseModel(await f.text()));
            } catch (err: any) {
              setRunErr(`Could not open ${f.name}: ${err.message}`);
            }
            e.target.value = '';
          }}
        />
        <button className="btn-ghost" onClick={() => setYamlOpen(true)}>
          YAML
        </button>
        <div className="spacer" />
        <div className={`status ${check?.error ? 'bad' : check ? 'good' : ''}`} title={check?.error ?? ''}>
          {engineErr ? 'Engine failed to load' : !catalog ? 'Loading engine…' : check?.error ? 'Model has an error' : check ? `${check.states} states` : 'Empty'}
        </div>
        {running ? (
          <>
            <div className="progress">
              <div style={{ width: `${running === 'transient' ? Math.min(100, (100 * progress) / model.simulation.t_end) : 30}%` }} />
            </div>
            <button className="btn-secondary" onClick={() => engine.stop().then(() => setCatalog(engine.catalog))}>
              Stop
            </button>
          </>
        ) : (
          <>
            <button className="btn-secondary" disabled={!catalog || !!check?.error} onClick={() => run('steady')} title="Shift+Enter">
              Steady state
            </button>
            <button className="btn-primary" disabled={!catalog || !!check?.error} onClick={() => run('transient')} title="Ctrl+Enter">
              Run transient
            </button>
          </>
        )}
      </header>

      <aside className="app-left">
        <Palette catalog={catalog} onAdd={(t) => addComp(t)} />
      </aside>

      <main
        className="canvas"
        onDragOver={(e) => {
          e.preventDefault();
          e.dataTransfer.dropEffect = 'move';
        }}
        onDrop={(e) => {
          const t = e.dataTransfer.getData('application/fluidnet');
          if (!t) return;
          e.preventDefault();
          addComp(t, rf.screenToFlowPosition({ x: e.clientX, y: e.clientY }));
        }}
      >
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={nodeTypes}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onPaneClick={() => setSelectedId(null)}
          connectionMode={ConnectionMode.Loose}
          fitView
          fitViewOptions={{ padding: 0.25 }}
          minZoom={0.2}
          maxZoom={3}
          snapToGrid
          snapGrid={[10, 10]}
          deleteKeyCode={['Backspace', 'Delete']}
          proOptions={{ hideAttribution: true }}
        >
          <Background gap={20} size={1} color="var(--dot)" />
          <Controls showInteractive={false} />
        </ReactFlow>
        {engineErr && <div className="banner bad">The simulation engine could not start: {engineErr}</div>}
        <div className="canvas-legend">
          {[...new Set(fluids.values())].map((f) => (
            <span key={f}>
              <i style={{ background: FLUID_COLORS[f] ?? 'var(--ink)' }} />
              {f}
            </span>
          ))}
        </div>
      </main>

      <aside className="app-right">
        <Inspector
          model={model}
          key={selectedId ?? '-'}
          selected={selected}
          catalog={catalog}
          check={check}
          onParam={setParam}
          onRename={renameComp}
          onFlip={(id) => mutate((m) => ({ ...m, components: m.components.map((c) => (c.id === id ? { ...c, ui: { ...c.ui, flip: !c.ui.flip } } : c)) }))}
          onDelete={removeComp}
          onModel={(p) => setModel({ ...model, ...p })}
          onSim={(p) => setModel({ ...model, simulation: { ...model.simulation, ...p } })}
        />
      </aside>

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
        {rs ? (
          <Results rs={rs} selected={selCh} setSelected={setSelCh} probe={probe} setProbe={setProbe} height={resH} />
        ) : (
          <div className="empty">{running ? 'Solving…' : 'Run a transient or solve the steady state to see results here.'}</div>
        )}
        <div className="foot mono">
          <span>{stLine}</span>
          {rs?.meta.warnings?.length ? <span className="warn-count">{rs.meta.warnings.length} warning(s)</span> : null}
        </div>
      </section>

      {yamlOpen && <YamlDialog model={model} onClose={() => setYamlOpen(false)} onApply={(m) => loadModel(m)} />}
    </div>
  );
}

function YamlDialog({ model, onClose, onApply }: { model: Model; onClose: () => void; onApply: (m: Model) => void }) {
  const [text, setText] = useState(() => toYaml(model));
  const [err, setErr] = useState<string | null>(null);
  const [copied, setCopied] = useState(false);
  const canSave = typeof (window as any).showSaveFilePicker === 'function' && window.self === window.top;
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
          {canSave && (
            <button
              className="btn-ghost"
              onClick={async () => {
                try {
                  const h = await (window as any).showSaveFilePicker({ suggestedName: `${model.name.replace(/\W+/g, '_').toLowerCase()}.yaml` });
                  const w = await h.createWritable();
                  await w.write(text);
                  await w.close();
                } catch {
                  /* cancelled */
                }
              }}
            >
              Save as…
            </button>
          )}
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
