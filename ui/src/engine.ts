// Main-thread client for the engine worker. The main thread fetches the wasm
// module and fluid tables (relative URLs) and hands bytes to the worker, so the
// worker can be an inline blob worker.
import EngineWorker from './worker.ts?worker&inline';

export interface RunResult {
  meta: any;
  data: Float64Array;
}

export const FLUIDS: { key: string; label: string }[] = [
  { key: 'H2', label: 'Hydrogen (para)' },
  { key: 'O2', label: 'Oxygen' },
  { key: 'CH4', label: 'Methane' },
  { key: 'N2', label: 'Nitrogen' },
  { key: 'He', label: 'Helium' },
  { key: 'H2O', label: 'Water' },
  { key: 'N2O', label: 'Nitrous oxide' },
  { key: 'C2H5OH', label: 'Ethanol' },
];

export class Engine {
  private worker: Worker | null = null;
  private seq = 0;
  private pending = new Map<number, { resolve: (v: any) => void; reject: (e: any) => void }>();
  private wasm: ArrayBuffer | null = null;
  private fluidBytes = new Map<string, ArrayBuffer>();
  private loaded = new Set<string>();
  onProgress: (t: number) => void = () => {};
  catalog: any = null;

  private call(msg: any, transfer: Transferable[] = []): Promise<any> {
    const id = ++this.seq;
    return new Promise((resolve, reject) => {
      this.pending.set(id, { resolve, reject });
      this.worker!.postMessage({ ...msg, id }, transfer);
    });
  }

  private spawn() {
    this.worker = new EngineWorker();
    this.worker.onmessage = (ev) => {
      const m = ev.data;
      if (m.type === 'progress') return this.onProgress(m.t);
      const p = this.pending.get(m.id);
      if (!p) return;
      this.pending.delete(m.id);
      m.ok ? p.resolve(m.result) : p.reject(new Error(m.result?.error ?? 'engine error'));
    };
  }

  async init(): Promise<void> {
    if (!this.wasm) {
      const r = await fetch('fluidnet.wasm');
      if (!r.ok) throw new Error(`cannot load fluidnet.wasm (${r.status})`);
      this.wasm = await r.arrayBuffer();
    }
    this.spawn();
    this.loaded.clear();
    this.catalog = await this.call({ type: 'init', wasm: this.wasm.slice(0) });
  }

  async ensureFluids(keys: string[]) {
    for (const k of keys) {
      if (this.loaded.has(k) || !FLUIDS.some((f) => f.key === k)) continue;
      let b = this.fluidBytes.get(k);
      if (!b) {
        const r = await fetch(`fluids/${k}.txt`);
        if (!r.ok) throw new Error(`cannot load fluid table ${k} (${r.status})`);
        const bin = atob((await r.text()).trim());
        const gz = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) gz[i] = bin.charCodeAt(i);
        const stream = new Blob([gz]).stream().pipeThrough(new DecompressionStream('gzip'));
        b = await new Response(stream).arrayBuffer();
        this.fluidBytes.set(k, b);
      }
      const copy = b.slice(0);
      await this.call({ type: 'fluid', bytes: copy }, [copy]);
      this.loaded.add(k);
    }
  }

  check(model: string): Promise<any> {
    return this.call({ type: 'check', model });
  }

  run(model: string, mode: 'steady' | 'transient'): Promise<RunResult> {
    return this.call({ type: 'run', model, mode });
  }

  /** Abort a running simulation: terminate the worker and start a fresh one. */
  async stop() {
    this.worker?.terminate();
    for (const p of this.pending.values()) p.reject(new Error('stopped'));
    this.pending.clear();
    await this.init();
  }
}
