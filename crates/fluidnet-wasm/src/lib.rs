//! WebAssembly bindings with a plain C ABI (no wasm-bindgen), used by the UI's
//! web worker.
//!
//! Memory protocol: JS calls `fn_alloc(len)`, writes bytes, passes (ptr, len).
//! Results are kept in module-global buffers and read via `fn_out_*`.
//!
//! Imports (module "env"): `fn_progress(t: f64) -> i32` (return 0 to stop).

use std::cell::RefCell;

use fluidnet_core::network::Network;
use fluidnet_core::props::{FluidLibrary, FluidTable};
use fluidnet_core::{catalog, run_steady, run_transient, Model, Results};

thread_local! {
    static LIB: RefCell<FluidLibrary> = RefCell::new(FluidLibrary::new());
    static OUT_JSON: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static OUT_DATA: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

#[cfg(target_arch = "wasm32")]
#[link(wasm_import_module = "env")]
extern "C" {
    fn fn_progress(t: f64) -> i32;
}

#[cfg(not(target_arch = "wasm32"))]
unsafe fn fn_progress(_t: f64) -> i32 {
    1
}

fn set_json(s: String) -> i32 {
    let n = s.len() as i32;
    OUT_JSON.with(|o| *o.borrow_mut() = s.into_bytes());
    n
}

fn err_json(e: impl std::fmt::Display) -> i32 {
    set_json(serde_json::json!({ "error": e.to_string() }).to_string());
    -1
}

/// # Safety
/// Caller must pass a pointer previously returned by `fn_alloc` with the same length.
unsafe fn read_bytes<'a>(ptr: *const u8, len: usize) -> &'a [u8] {
    std::slice::from_raw_parts(ptr, len)
}

#[no_mangle]
pub extern "C" fn fn_alloc(len: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `ptr`/`len` must come from `fn_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fn_free(ptr: *mut u8, len: usize) {
    drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
}

#[no_mangle]
pub extern "C" fn fn_out_json_ptr() -> *const u8 {
    OUT_JSON.with(|o| o.borrow().as_ptr())
}
#[no_mangle]
pub extern "C" fn fn_out_json_len() -> usize {
    OUT_JSON.with(|o| o.borrow().len())
}
#[no_mangle]
pub extern "C" fn fn_out_data_ptr() -> *const f64 {
    OUT_DATA.with(|o| o.borrow().as_ptr())
}
#[no_mangle]
pub extern "C" fn fn_out_data_len() -> usize {
    OUT_DATA.with(|o| o.borrow().len())
}

/// Load a fluid table (.fntb bytes). Returns 0 or -1 (error in out JSON).
///
/// # Safety
/// `ptr`/`len` must describe a valid buffer from `fn_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fn_load_fluid(ptr: *const u8, len: usize) -> i32 {
    match FluidTable::from_bytes(read_bytes(ptr, len)) {
        Ok(t) => {
            let key = t.key.clone();
            LIB.with(|l| l.borrow_mut().add(t));
            set_json(serde_json::json!({ "loaded": key }).to_string());
            0
        }
        Err(e) => err_json(e),
    }
}

#[no_mangle]
pub extern "C" fn fn_catalog() -> i32 {
    set_json(catalog::catalog_json().to_string())
}

/// Fluid metadata for loaded tables.
#[no_mangle]
pub extern "C" fn fn_fluids() -> i32 {
    let v = LIB.with(|l| {
        let lib = l.borrow();
        lib.keys()
            .iter()
            .filter_map(|k| lib.get(k))
            .map(|t| {
                serde_json::json!({ "key": t.key, "label": t.label, "Tcrit": t.tcrit, "Pcrit": t.pcrit,
                    "Tmin": t.tmin, "Tmax": t.tmax, "Pmin": t.pmin, "Pmax": t.pmax })
            })
            .collect::<Vec<_>>()
    });
    set_json(serde_json::Value::Array(v).to_string())
}

fn compile(src: &str) -> Result<Network, String> {
    let model = Model::parse(src).map_err(|e| e.to_string())?;
    LIB.with(|l| Network::compile(&model, &l.borrow()))
        .map_err(|e| e.to_string())
}

/// Compile only; returns structure summary JSON.
///
/// # Safety
/// `ptr`/`len` must describe a valid UTF-8 buffer from `fn_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fn_check(ptr: *const u8, len: usize) -> i32 {
    let src = String::from_utf8_lossy(read_bytes(ptr, len));
    match compile(&src) {
        Ok(net) => set_json(
            serde_json::json!({
                "states": net.n_states, "nodes": net.nodes.len(), "branches": net.branches.len(),
                "warnings": net.warnings,
            })
            .to_string(),
        ),
        Err(e) => err_json(e),
    }
}

fn pack(res: Results, elapsed_hint: f64) -> i32 {
    // data block: time, then each channel
    let nt = res.time.len();
    let mut data = Vec::with_capacity(nt * (res.channels.len() + 1));
    data.extend_from_slice(&res.time);
    for c in &res.channels {
        data.extend_from_slice(&c.data);
    }
    let meta = serde_json::json!({
        "name": res.name, "mode": res.mode, "n_time": nt,
        "channels": res.channels.iter().map(|c| serde_json::json!({"name": c.name, "unit": c.unit})).collect::<Vec<_>>(),
        "warnings": res.warnings, "stats": res.stats, "steady": res.steady,
        "nodes": res.nodes, "components": res.components, "n_states": res.n_states,
        "error": res.error, "elapsed_hint": elapsed_hint,
    });
    OUT_DATA.with(|o| *o.borrow_mut() = data);
    set_json(meta.to_string())
}

/// Run a model (JSON or YAML). mode 0 = steady state, 1 = transient.
/// Returns JSON length (>= 0) or -1 on error.
///
/// # Safety
/// `ptr`/`len` must describe a valid UTF-8 buffer from `fn_alloc`.
#[no_mangle]
pub unsafe extern "C" fn fn_run(ptr: *const u8, len: usize, mode: i32) -> i32 {
    let src = String::from_utf8_lossy(read_bytes(ptr, len)).into_owned();
    let net = match compile(&src) {
        Ok(n) => n,
        Err(e) => return err_json(e),
    };
    let res = if mode == 0 {
        run_steady(&net, net.sim.steady_time)
    } else {
        run_transient(&net, &mut |t| unsafe { fn_progress(t) } != 0)
    };
    match res {
        Ok(r) => pack(r, 0.0),
        Err(e) => err_json(e),
    }
}
