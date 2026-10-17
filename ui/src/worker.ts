// Web worker hosting the fluidnet WebAssembly engine (C ABI, no bindings).
let ex: any = null;
let lastProgress = 0;

const enc = new TextEncoder();
const dec = new TextDecoder();

function put(bytes: Uint8Array): [number, number] {
  const p = ex.fn_alloc(bytes.length);
  new Uint8Array(ex.memory.buffer, p, bytes.length).set(bytes);
  return [p, bytes.length];
}
function outJson(): any {
  const s = dec.decode(new Uint8Array(ex.memory.buffer, ex.fn_out_json_ptr(), ex.fn_out_json_len()));
  return JSON.parse(s);
}

self.onmessage = async (ev: MessageEvent) => {
  const { id, type } = ev.data;
  try {
    if (type === 'init') {
      const { instance } = await WebAssembly.instantiate(ev.data.wasm, {
        env: {
          fn_progress: (t: number) => {
            const now = performance.now();
            if (now - lastProgress > 60) {
              lastProgress = now;
              (self as any).postMessage({ type: 'progress', t });
            }
            return 1;
          },
        },
      });
      ex = instance.exports;
      ex.fn_catalog();
      (self as any).postMessage({ id, ok: true, result: outJson() });
    } else if (type === 'fluid') {
      const [p, n] = put(new Uint8Array(ev.data.bytes));
      const r = ex.fn_load_fluid(p, n);
      ex.fn_free(p, n);
      (self as any).postMessage({ id, ok: r === 0, result: outJson() });
    } else if (type === 'check') {
      const [p, n] = put(enc.encode(ev.data.model));
      const r = ex.fn_check(p, n);
      ex.fn_free(p, n);
      (self as any).postMessage({ id, ok: r >= 0, result: outJson() });
    } else if (type === 'run') {
      const [p, n] = put(enc.encode(ev.data.model));
      const t0 = performance.now();
      const r = ex.fn_run(p, n, ev.data.mode === 'steady' ? 0 : 1);
      const ms = performance.now() - t0;
      ex.fn_free(p, n);
      const meta = outJson();
      if (r < 0) {
        (self as any).postMessage({ id, ok: false, result: meta });
        return;
      }
      const src = new Float64Array(ex.memory.buffer, ex.fn_out_data_ptr(), ex.fn_out_data_len());
      const data = new Float64Array(src); // copy out of wasm memory
      meta.elapsed_ms = ms;
      (self as any).postMessage({ id, ok: true, result: { meta, data } }, [data.buffer]);
    }
  } catch (e: any) {
    (self as any).postMessage({ id, ok: false, result: { error: String(e?.message ?? e) } });
  }
};
