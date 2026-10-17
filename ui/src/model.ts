import yaml from 'js-yaml';

export interface Comp {
  id: string;
  type: string;
  params: Record<string, any>;
  ui: { x: number; y: number; flip?: boolean };
}

export interface Sim {
  t_end: number;
  output_dt?: number | null;
  rtol: number;
  start_from_steady: boolean;
  max_step?: number | null;
}

export interface Model {
  name: string;
  description: string;
  components: Comp[];
  connections: [string, string][];
  simulation: Sim;
}

export const defaultSim = (): Sim => ({ t_end: 1, output_dt: null, rtol: 1e-4, start_from_steady: false });

export function emptyModel(): Model {
  return { name: 'Untitled system', description: '', components: [], connections: [], simulation: defaultSim() };
}

export function parseModel(text: string): Model {
  const raw: any = text.trim().startsWith('{') ? JSON.parse(text) : yaml.load(text);
  if (!raw || typeof raw !== 'object') throw new Error('not a model file');
  const comps: Comp[] = (raw.components ?? []).map((c: any, i: number) => {
    const { id, type, ui, ...params } = c;
    return {
      id: String(id ?? `c${i}`),
      type: String(type),
      params,
      ui: { x: Number(ui?.x ?? 80 + 160 * (i % 5)), y: Number(ui?.y ?? 100 + 140 * Math.floor(i / 5)), flip: !!ui?.flip },
    };
  });
  const conns: [string, string][] = (raw.connections ?? []).map((c: any) =>
    Array.isArray(c) ? [String(c[0]), String(c[1])] : [String(c.from), String(c.to)]
  );
  const s = raw.simulation ?? {};
  return {
    name: raw.name ?? 'Untitled system',
    description: raw.description ?? '',
    components: comps,
    connections: conns,
    simulation: { ...defaultSim(), ...s },
  };
}

function compObj(c: Comp) {
  const ui: any = { x: Math.round(c.ui.x), y: Math.round(c.ui.y) };
  if (c.ui.flip) ui.flip = true;
  const params: any = {};
  for (const [k, v] of Object.entries(c.params)) if (v !== null && v !== undefined && v !== '') params[k] = v;
  return { id: c.id, type: c.type, ...params, ui };
}

/** YAML in the same compact style as the examples: one line per component. */
export function toYaml(m: Model): string {
  const flow = (o: any) => yaml.dump(o, { flowLevel: 0, lineWidth: -1 }).trim();
  const lines: string[] = [];
  lines.push(`name: ${flow(m.name)}`);
  if (m.description) lines.push(`description: ${flow(m.description)}`);
  lines.push('components:');
  for (const c of m.components) lines.push(`  - ${flow(compObj(c))}`);
  lines.push('connections:');
  for (const [a, b] of m.connections) lines.push(`  - [${a}, ${b}]`);
  const sim: any = {};
  for (const [k, v] of Object.entries(m.simulation)) if (v !== null && v !== undefined) sim[k] = v;
  lines.push(`simulation: ${flow(sim)}`);
  return lines.join('\n') + '\n';
}

/** JSON sent to the engine. */
export function toEngineJson(m: Model): string {
  const sim: any = {};
  for (const [k, v] of Object.entries(m.simulation)) if (v !== null && v !== undefined) sim[k] = v;
  return JSON.stringify({
    name: m.name,
    description: m.description,
    components: m.components.map(compObj),
    connections: m.connections,
    simulation: sim,
  });
}

export function splitPort(ref: string): [string, string | null] {
  const i = ref.lastIndexOf('.');
  return i < 0 ? [ref, null] : [ref.slice(0, i), ref.slice(i + 1)];
}

/** Fluid per component, propagated through connections (for display only). */
export function fluidMap(m: Model): Map<string, string> {
  const adj = new Map<string, Set<string>>();
  const add = (a: string, b: string) => {
    if (!adj.has(a)) adj.set(a, new Set());
    adj.get(a)!.add(b);
  };
  for (const [a, b] of m.connections) {
    const [ca] = splitPort(a);
    const [cb] = splitPort(b);
    add(ca, cb);
    add(cb, ca);
  }
  const out = new Map<string, string>();
  const queue: string[] = [];
  for (const c of m.components) {
    if (c.params.fluid) {
      out.set(c.id, String(c.params.fluid));
      queue.push(c.id);
    }
  }
  while (queue.length) {
    const id = queue.shift()!;
    for (const n of adj.get(id) ?? []) {
      if (!out.has(n)) {
        out.set(n, out.get(id)!);
        queue.push(n);
      }
    }
  }
  return out;
}

export function usedFluids(m: Model): string[] {
  return [...new Set(m.components.map((c) => c.params.fluid).filter(Boolean).map(String))];
}

export function uniqueId(m: Model, base: string): string {
  const ids = new Set(m.components.map((c) => c.id));
  if (!ids.has(base)) return base;
  for (let i = 2; ; i++) if (!ids.has(`${base}${i}`)) return `${base}${i}`;
}

// ---- display units --------------------------------------------------------
export interface DisplayUnit {
  label: string;
  scale: number;
  offset?: number;
}

export function displayUnit(si: string): DisplayUnit {
  switch (si) {
    case 'Pa':
      return { label: 'bar', scale: 1e-5 };
    case 'J/kg':
      return { label: 'kJ/kg', scale: 1e-3 };
    case 'W':
      return { label: 'kW', scale: 1e-3 };
    default:
      return { label: si, scale: 1 };
  }
}

export function fmt(v: number, digits = 4): string {
  if (v === null || v === undefined || !isFinite(v)) return '–';
  const a = Math.abs(v);
  if (a !== 0 && (a >= 1e5 || a < 1e-3)) return v.toExponential(2);
  return Number(v.toPrecision(digits)).toString();
}

export const FLUID_COLORS: Record<string, string> = {
  O2: '#2563c9',
  H2: '#0f9488',
  CH4: '#d9711c',
  N2: '#6b7280',
  He: '#8b5cf6',
  H2O: '#0e7aa6',
  N2O: '#c0397a',
  C2H5OH: '#a16207',
};
