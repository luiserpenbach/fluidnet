//! Python bindings: `fluidnet._core`. The pure-Python package `fluidnet` wraps
//! these functions with numpy arrays and a small results class.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use fluidnet_core::network::Network;
use fluidnet_core::props::FluidLibrary;
use fluidnet_core::{catalog, run_steady, run_transient, Model};
use pyo3::exceptions::{PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyBytes;

static LIBS: Mutex<Option<HashMap<PathBuf, FluidLibrary>>> = Mutex::new(None);

fn library(dir: &str) -> PyResult<FluidLibrary> {
    let key = PathBuf::from(dir);
    let mut guard = LIBS.lock().unwrap();
    let map = guard.get_or_insert_with(HashMap::new);
    if let Some(l) = map.get(&key) {
        return Ok(l.clone());
    }
    let lib = FluidLibrary::load_dir(&key).map_err(|e| PyValueError::new_err(e.to_string()))?;
    if lib.keys().is_empty() {
        return Err(PyValueError::new_err(format!(
            "no fluid tables (*.fntb) in {dir}"
        )));
    }
    map.insert(key, lib.clone());
    Ok(lib)
}

/// Run a model given as YAML or JSON text.
/// Returns (metadata JSON, data bytes: time then each channel as float64).
#[pyfunction]
#[pyo3(signature = (model, mode, fluids_dir, steady_time=None))]
fn run(
    py: Python<'_>,
    model: &str,
    mode: &str,
    fluids_dir: &str,
    steady_time: Option<f64>,
) -> PyResult<(String, PyObject)> {
    let lib = library(fluids_dir)?;
    let model = Model::parse(model).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let steady = match mode {
        "steady" => true,
        "transient" => false,
        m => {
            return Err(PyValueError::new_err(format!(
                "mode must be 'steady' or 'transient', got '{m}'"
            )))
        }
    };
    let res = py.allow_threads(|| -> Result<_, String> {
        let net = Network::compile(&model, &lib).map_err(|e| e.to_string())?;
        let t0 = std::time::Instant::now();
        let r = if steady {
            run_steady(&net, steady_time.unwrap_or(net.sim.steady_time))
        } else {
            run_transient(&net, &mut |_| true)
        }
        .map_err(|e| e.to_string())?;
        Ok((r, t0.elapsed().as_secs_f64()))
    });
    let (r, elapsed) = res.map_err(PyRuntimeError::new_err)?;
    let nt = r.time.len();
    let mut data: Vec<u8> = Vec::with_capacity(8 * nt * (r.channels.len() + 1));
    for v in r
        .time
        .iter()
        .chain(r.channels.iter().flat_map(|c| c.data.iter()))
    {
        data.extend_from_slice(&v.to_le_bytes());
    }
    let meta = serde_json::json!({
        "name": r.name, "mode": r.mode, "n_time": nt,
        "channels": r.channels.iter().map(|c| serde_json::json!({"name": c.name, "unit": c.unit})).collect::<Vec<_>>(),
        "warnings": r.warnings, "stats": r.stats, "steady": r.steady, "error": r.error,
        "components": r.components, "n_states": r.n_states, "elapsed_s": elapsed,
    });
    Ok((meta.to_string(), PyBytes::new_bound(py, &data).into()))
}

/// Parse YAML or JSON model text and return it as JSON (for editing as a dict).
#[pyfunction]
fn parse_model(text: &str) -> PyResult<String> {
    let m = Model::parse(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
    serde_json::to_string(&m).map_err(|e| PyRuntimeError::new_err(e.to_string()))
}

/// Model as YAML text.
#[pyfunction]
fn to_yaml(text: &str) -> PyResult<String> {
    let m = Model::parse(text).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(m.to_yaml())
}

/// Compile a model and return a structure summary (nodes, branches, states) as JSON.
#[pyfunction]
fn check(model: &str, fluids_dir: &str) -> PyResult<String> {
    let lib = library(fluids_dir)?;
    let model = Model::parse(model).map_err(|e| PyValueError::new_err(e.to_string()))?;
    let net = Network::compile(&model, &lib).map_err(|e| PyValueError::new_err(e.to_string()))?;
    Ok(serde_json::json!({
        "states": net.state_names, "nodes": net.nodes.iter().map(|n| &n.name).collect::<Vec<_>>(),
        "branches": net.branches.iter().map(|b| &b.name).collect::<Vec<_>>(), "warnings": net.warnings,
        "channels": net.channels.iter().map(|c| serde_json::json!({"name": c.name, "unit": c.unit})).collect::<Vec<_>>(),
    })
    .to_string())
}

/// Component catalogue (ports, parameters, units, defaults) as JSON.
#[pyfunction]
fn catalog_json() -> String {
    catalog::catalog_json().to_string()
}

/// Fluid properties at (P [Pa], h [J/kg]) or, with T given instead of h, at (P, T).
#[pyfunction]
#[pyo3(signature = (fluid, p, fluids_dir, h=None, t=None))]
fn props(
    fluid: &str,
    p: f64,
    fluids_dir: &str,
    h: Option<f64>,
    t: Option<f64>,
) -> PyResult<String> {
    let lib = library(fluids_dir)?;
    let fl = lib
        .get(fluid)
        .ok_or_else(|| PyValueError::new_err(format!("unknown fluid {fluid}")))?;
    let h = match (h, t) {
        (Some(h), _) => h,
        (None, Some(t)) => fl
            .h_pt(p, t)
            .map_err(|e| PyValueError::new_err(e.to_string()))?,
        _ => return Err(PyValueError::new_err("give h or t")),
    };
    let pr = fl.props_ph_x(p, h);
    Ok(serde_json::json!({
        "P": pr.p, "h": pr.h, "T": pr.t, "rho": pr.rho, "s": pr.s, "cp": pr.cp, "mu": pr.mu, "k": pr.k,
        "a": pr.a, "x": if pr.x.is_finite() { serde_json::json!(pr.x) } else { serde_json::Value::Null },
    })
    .to_string())
}

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(run, m)?)?;
    m.add_function(wrap_pyfunction!(parse_model, m)?)?;
    m.add_function(wrap_pyfunction!(to_yaml, m)?)?;
    m.add_function(wrap_pyfunction!(check, m)?)?;
    m.add_function(wrap_pyfunction!(catalog_json, m)?)?;
    m.add_function(wrap_pyfunction!(props, m)?)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
