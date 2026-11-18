// Pure operations on the model graph: delete, rename, copy/paste, splice,
// align, and the port geometry of a (rotated, flipped) schematic symbol.
import yaml from 'js-yaml';
import { Comp, Model, normRot, splitPort } from './model';
import { SYMBOL_SIZE } from './symbols';

export const NODE_LIKE = new Set(['tank', 'junction', 'boundary']);
/** Parameters holding a node reference as free text ("id" or "id.port"). */
const TEXT_REFS = ['sense', 'sense_ref'];
const DEFAULT_REF_KEYS = ['shaft', 'hot_chamber'];

export type Conn = [string, string];
export const connKey = (a: string, b: string) => `${a}~${b}`;
export const edgeId = (a: string, b: string) => `e:${connKey(a, b)}`;

/** Keys of parameters that reference another component by id. */
export function refKeysOf(catalog: any): Set<string> {
  const out = new Set(DEFAULT_REF_KEYS);
  for (const c of catalog?.components ?? []) for (const p of c.params) if (p.kind === 'ref') out.add(p.key);
  return out;
}

/** Rewrite component references in a parameter map. `map` returns the new id, or null to drop the reference. */
function remapParams(params: Record<string, any>, refKeys: Set<string>, map: (id: string) => string | null | undefined): Record<string, any> {
  let out = params;
  const set = (k: string, v: any) => {
    if (out === params) out = { ...params };
    if (v === null) delete out[k];
    else out[k] = v;
  };
  for (const k of Object.keys(params)) {
    const v = params[k];
    if (typeof v !== 'string') continue;
    if (refKeys.has(k)) {
      const n = map(v);
      if (n !== undefined && n !== v) set(k, n);
    } else if (TEXT_REFS.includes(k)) {
      // "id" or "id.port"
      const whole = map(v);
      if (typeof whole === 'string') {
        if (whole !== v) set(k, whole);
        continue;
      }
      const [c, p] = splitPort(v);
      const n = p ? map(c) : undefined;
      if (typeof n === 'string') {
        if (n !== c) set(k, `${n}.${p}`);
      } else if (n === null || (whole === null && !p)) {
        set(k, null);
      }
    }
  }
  return out;
}

const endComp = (ref: string, ids: { has(id: string): boolean }) => (ids.has(ref) ? ref : splitPort(ref)[0]);

/** Component id of a connection end, given the ids in the model (ids may not contain dots, ports are the last segment). */
export function connEnds(m: Model, [a, b]: Conn): [string, string] {
  const ids = new Set(m.components.map((c) => c.id));
  return [endComp(a, ids), endComp(b, ids)];
}

/** Remove components and connections (by connKey). References to removed components are cleared. */
export function removeItems(m: Model, compIds: Iterable<string>, connKeys: Iterable<string>, refKeys: Set<string>): Model {
  const gone = new Set(compIds);
  const cut = new Set(connKeys);
  if (!gone.size && !cut.size) return m;
  const ids = new Set(m.components.map((c) => c.id));
  const components = m.components
    .filter((c) => !gone.has(c.id))
    .map((c) => {
      if (!gone.size) return c;
      const params = remapParams(c.params, refKeys, (id) => (gone.has(id) ? null : undefined));
      return params === c.params ? c : { ...c, params };
    });
  const connections = m.connections.filter(([a, b]) => !cut.has(connKey(a, b)) && !gone.has(endComp(a, ids)) && !gone.has(endComp(b, ids)));
  return { ...m, components, connections };
}

export function renameComp(m: Model, id: string, nid: string, refKeys: Set<string>): Model {
  if (id === nid || m.components.some((c) => c.id === nid)) return m;
  const ids = new Set(m.components.map((c) => c.id));
  const rn = (r: string) => {
    if (r === id) return nid;
    if (ids.has(r)) return r;
    const [c, p] = splitPort(r);
    return c === id ? `${nid}.${p}` : r;
  };
  return {
    ...m,
    components: m.components.map((c) => {
      const params = remapParams(c.params, refKeys, (r) => (r === id ? nid : undefined));
      return c.id === id ? { ...c, id: nid, params } : params === c.params ? c : { ...c, params };
    }),
    connections: m.connections.map(([a, b]) => [rn(a), rn(b)] as Conn),
  };
}

// ---- copy / paste ---------------------------------------------------------
export interface Fragment {
  components: Comp[];
  connections: Conn[];
}

/** The selected components with the connections that run between them. */
export function extractFragment(m: Model, compIds: Iterable<string>): Fragment {
  const pick = new Set(compIds);
  const ids = new Set(m.components.map((c) => c.id));
  return {
    components: m.components.filter((c) => pick.has(c.id)).map((c) => ({ ...c, params: structuredCloneSafe(c.params), ui: { ...c.ui } })),
    connections: m.connections.filter(([a, b]) => pick.has(endComp(a, ids)) && pick.has(endComp(b, ids))).map(([a, b]) => [a, b] as Conn),
  };
}

function structuredCloneSafe<T>(v: T): T {
  return JSON.parse(JSON.stringify(v));
}

/** Clipboard text for a fragment: a YAML snippet that also reads as part of a model file. */
export function fragmentToText(f: Fragment): string {
  const flow = (o: any) => yaml.dump(o, { flowLevel: 0, lineWidth: -1 }).trim();
  const lines = ['# fluidnet components', 'components:'];
  for (const c of f.components) {
    const ui: any = { x: Math.round(c.ui.x), y: Math.round(c.ui.y) };
    if (c.ui.flip) ui.flip = true;
    if (c.ui.rot) ui.rot = c.ui.rot;
    lines.push(`  - ${flow({ id: c.id, type: c.type, ...c.params, ui })}`);
  }
  lines.push('connections:');
  for (const [a, b] of f.connections) lines.push(`  - [${a}, ${b}]`);
  return lines.join('\n') + '\n';
}

export function fragmentFromText(text: string): Fragment | null {
  try {
    const raw: any = yaml.load(text);
    if (!raw || typeof raw !== 'object' || !Array.isArray(raw.components) || !raw.components.length) return null;
    const components: Comp[] = raw.components.map((c: any, i: number) => {
      const { id, type, ui, ...params } = c;
      if (typeof type !== 'string') throw new Error('no type');
      return { id: String(id ?? type), type, params, ui: { x: Number(ui?.x ?? 40 * i), y: Number(ui?.y ?? 40 * i), flip: !!ui?.flip, rot: normRot(ui?.rot) } };
    });
    const connections: Conn[] = (raw.connections ?? []).map((c: any) => (Array.isArray(c) ? [String(c[0]), String(c[1])] : [String(c.from), String(c.to)]));
    return { components, connections };
  } catch {
    return null;
  }
}

/** A free id derived from `id`: valve -> valve_2 -> valve_3. */
export function freshId(id: string, taken: { has(id: string): boolean }): string {
  if (!taken.has(id)) return id;
  const m = /^(.*)_(\d+)$/.exec(id);
  const stem = m ? m[1] : id;
  for (let i = m ? Number(m[2]) + 1 : 2; ; i++) if (!taken.has(`${stem}_${i}`)) return `${stem}_${i}`;
}

/**
 * Add a fragment to the model with fresh ids. `at` places the fragment's
 * bounding-box centre; otherwise it is offset from where it was copied.
 */
export function pasteFragment(m: Model, f: Fragment, refKeys: Set<string>, at?: { x: number; y: number }, offset = 30): { model: Model; ids: string[] } {
  if (!f.components.length) return { model: m, ids: [] };
  const taken = new Set(m.components.map((c) => c.id));
  const fragIds = new Set(f.components.map((c) => c.id));
  const idMap = new Map<string, string>();
  for (const c of f.components) {
    const nid = freshId(c.id, taken);
    taken.add(nid);
    idMap.set(c.id, nid);
  }
  let dx = offset;
  let dy = offset;
  if (at) {
    const b = bounds(f.components);
    dx = Math.round((at.x - (b.x0 + b.x1) / 2) / 10) * 10;
    dy = Math.round((at.y - (b.y0 + b.y1) / 2) / 10) * 10;
  }
  const existing = new Set(m.components.map((c) => c.id));
  const comps = f.components.map((c) => ({
    ...c,
    id: idMap.get(c.id)!,
    // references inside the fragment follow the copy; references to parts that
    // do not exist in this model are dropped
    params: remapParams(structuredCloneSafe(c.params), refKeys, (r) => (fragIds.has(r) ? idMap.get(r)! : existing.has(r) ? undefined : null)),
    ui: { ...c.ui, x: c.ui.x + dx, y: c.ui.y + dy },
  }));
  const rn = (r: string) => {
    if (idMap.has(r)) return idMap.get(r)!;
    const [c, p] = splitPort(r);
    return idMap.has(c) ? `${idMap.get(c)}.${p}` : r;
  };
  const conns = f.connections.map(([a, b]) => [rn(a), rn(b)] as Conn);
  return { model: { ...m, components: [...m.components, ...comps], connections: [...m.connections, ...conns] }, ids: comps.map((c) => c.id) };
}

// ---- connections ----------------------------------------------------------
export function hasConn(m: Model, a: string, b: string): boolean {
  return m.connections.some(([x, y]) => (x === a && y === b) || (x === b && y === a));
}

/** Put a two-port component into an existing connection: a - b becomes a - comp.in, comp.out - b. */
export function spliceInto(m: Model, key: string, compId: string, inPort = 'in', outPort = 'out'): Model {
  const i = m.connections.findIndex(([a, b]) => connKey(a, b) === key);
  if (i < 0) return m;
  let [a, b] = m.connections[i];
  // keep the flow sense: an element's inlet end of the line is downstream
  const isIn = (r: string) => r.endsWith('.in');
  const isOut = (r: string) => r.endsWith('.out');
  if (isIn(a) || isOut(b)) [a, b] = [b, a];
  const connections = [...m.connections];
  connections.splice(i, 1, [a, `${compId}.${inPort}`], [`${compId}.${outPort}`, b]);
  return { ...m, connections };
}

/** Put a node-like component (junction) into a connection: a - b becomes a - node, node - b. */
export function spliceNode(m: Model, key: string, compId: string): Model {
  const i = m.connections.findIndex(([a, b]) => connKey(a, b) === key);
  if (i < 0) return m;
  const [a, b] = m.connections[i];
  const connections = [...m.connections];
  connections.splice(i, 1, [a, compId], [compId, b]);
  return { ...m, connections };
}

// ---- layout ---------------------------------------------------------------
export function nodeSize(c: { type: string; ui: { rot?: number } }): [number, number] {
  const [w, h] = SYMBOL_SIZE[c.type] ?? [48, 32];
  return (c.ui.rot ?? 0) % 180 ? [h, w] : [w, h];
}

export function bounds(comps: Comp[]) {
  let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
  for (const c of comps) {
    const [w, h] = nodeSize(c);
    x0 = Math.min(x0, c.ui.x);
    y0 = Math.min(y0, c.ui.y);
    x1 = Math.max(x1, c.ui.x + w);
    y1 = Math.max(y1, c.ui.y + h);
  }
  return { x0, y0, x1, y1 };
}

export type AlignMode = 'left' | 'hcenter' | 'right' | 'top' | 'vcenter' | 'bottom' | 'hdist' | 'vdist';

/** Align or evenly distribute components (by symbol centre for the centre and distribute modes). */
export function alignComps(m: Model, compIds: Iterable<string>, mode: AlignMode): Model {
  const pick = new Set(compIds);
  const sel = m.components.filter((c) => pick.has(c.id));
  if (sel.length < 2) return m;
  const b = bounds(sel);
  const pos = new Map<string, { x: number; y: number }>();
  const cx = (c: Comp) => c.ui.x + nodeSize(c)[0] / 2;
  const cy = (c: Comp) => c.ui.y + nodeSize(c)[1] / 2;
  if (mode === 'hdist' || mode === 'vdist') {
    if (sel.length < 3) return m;
    const horiz = mode === 'hdist';
    const sorted = [...sel].sort((p, q) => (horiz ? cx(p) - cx(q) : cy(p) - cy(q)));
    const first = horiz ? cx(sorted[0]) : cy(sorted[0]);
    const last = horiz ? cx(sorted[sorted.length - 1]) : cy(sorted[sorted.length - 1]);
    sorted.forEach((c, i) => {
      const t = first + ((last - first) * i) / (sorted.length - 1);
      const [w, h] = nodeSize(c);
      pos.set(c.id, horiz ? { x: Math.round(t - w / 2), y: c.ui.y } : { x: c.ui.x, y: Math.round(t - h / 2) });
    });
  } else {
    const mx = (b.x0 + b.x1) / 2;
    const my = (b.y0 + b.y1) / 2;
    for (const c of sel) {
      const [w, h] = nodeSize(c);
      let { x, y } = c.ui;
      if (mode === 'left') x = b.x0;
      else if (mode === 'right') x = b.x1 - w;
      else if (mode === 'hcenter') x = Math.round(mx - w / 2);
      else if (mode === 'top') y = b.y0;
      else if (mode === 'bottom') y = b.y1 - h;
      else if (mode === 'vcenter') y = Math.round(my - h / 2);
      pos.set(c.id, { x, y });
    }
  }
  return { ...m, components: m.components.map((c) => (pos.has(c.id) ? { ...c, ui: { ...c.ui, ...pos.get(c.id)! } } : c)) };
}

// ---- symbol geometry ------------------------------------------------------
export type Side = 'left' | 'right' | 'top' | 'bottom';
export interface HandleGeom {
  id: string;
  side: Side;
  /** CSS offset along the side (top for left/right sides, left for top/bottom sides), px */
  along: number;
  kind: 'port' | 'anchor';
  cls?: string;
  title?: string;
}
export interface NodeGeom {
  /** symbol size before rotation */
  w: number;
  h: number;
  /** node box after rotation */
  W: number;
  H: number;
  handles: HandleGeom[];
}

/** Handles of a symbol after flipping (mirror in x) and rotating clockwise by `rot` degrees. */
export function nodeGeometry(type: string, rot = 0, flip = false): NodeGeom {
  const [w, h] = SYMBOL_SIZE[type] ?? [48, 32];
  const r = normRot(rot);
  const [W, H] = r % 180 ? [h, w] : [w, h];
  const handles: HandleGeom[] = [];
  if (NODE_LIKE.has(type)) {
    // volumes connect on whichever side faces the neighbour; ids are absolute sides
    const sides: Side[] = type === 'boundary' ? (r % 180 ? ['top', 'bottom'] : ['left', 'right']) : ['left', 'right', 'top', 'bottom'];
    for (const s of sides) handles.push({ id: `p-${s[0]}`, side: s, along: s === 'left' || s === 'right' ? H / 2 : W / 2, kind: 'port' });
    return { w, h, W, H, handles };
  }
  const raw: { id: string; x: number; y: number; kind: 'port' | 'anchor'; cls?: string; title?: string }[] = [];
  if (type === 'shaft') {
    raw.push({ id: 'mech', x: 0, y: -h / 2, kind: 'anchor' });
  } else if (type === 'combustion_chamber') {
    raw.push({ id: 'fuel', x: -w / 2, y: -9, kind: 'port', title: 'fuel' });
    raw.push({ id: 'ox', x: -w / 2, y: 9, kind: 'port', title: 'oxidizer' });
    raw.push({ id: 'heat', x: 30 - w / 2, y: -h / 2, kind: 'anchor' });
  } else {
    raw.push({ id: 'in', x: -w / 2, y: 0, kind: 'port', cls: 'port-in', title: 'inlet' });
    raw.push({ id: 'out', x: w / 2, y: 0, kind: 'port', cls: 'port-out', title: 'outlet' });
    if (type === 'pump' || type === 'turbine') raw.push({ id: 'mech', x: 0, y: h / 2, kind: 'anchor' });
    if (type === 'pipe') raw.push({ id: 'heat', x: 0, y: -h / 2, kind: 'anchor' });
  }
  for (const p of raw) {
    let x = flip ? -p.x : p.x;
    let y = p.y;
    for (let i = 0; i < r / 90; i++) [x, y] = [-y, x];
    const eps = 0.01;
    const side: Side = x <= -W / 2 + eps ? 'left' : x >= W / 2 - eps ? 'right' : y <= -H / 2 + eps ? 'top' : 'bottom';
    handles.push({ id: p.id, side, along: side === 'left' || side === 'right' ? H / 2 + y : W / 2 + x, kind: p.kind, cls: p.cls, title: p.title });
  }
  return { w, h, W, H, handles };
}
