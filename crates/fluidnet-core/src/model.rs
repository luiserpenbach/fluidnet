//! Model file schema (YAML or JSON).
//!
//! ```yaml
//! name: GN2 blowdown
//! components:
//!   - {id: tank, type: tank, fluid: N2, volume: 50 L, P0: 300 bar, T0: 290 K}
//!   - {id: line, type: pipe, length: 2 m, diameter: 10 mm, segments: 8}
//!   - {id: orf, type: orifice, diameter: 3 mm, cd: 0.8}
//!   - {id: amb, type: boundary, fluid: N2, P: 1 bar, T: 290 K}
//! connections:
//!   - [tank, line.in]
//!   - [line.out, orf.in]
//!   - [orf.out, amb]
//! simulation: {t_end: 20, output_dt: 0.05}
//! ```

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub components: Vec<ComponentDef>,
    #[serde(default)]
    pub connections: Vec<Connection>,
    #[serde(default)]
    pub simulation: SimSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentDef {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(flatten)]
    pub params: Map<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Connection {
    Pair(Vec<String>),
    Obj { from: String, to: String },
}

impl Connection {
    pub fn ends(&self) -> Result<(String, String)> {
        match self {
            Connection::Pair(v) if v.len() == 2 => Ok((v[0].clone(), v[1].clone())),
            Connection::Pair(v) => Err(Error::Model(format!(
                "connection needs exactly 2 ports, got {v:?}"
            ))),
            Connection::Obj { from, to } => Ok((from.clone(), to.clone())),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SimSettings {
    /// End time, s.
    pub t_end: f64,
    /// Output interval, s (default t_end / 500).
    pub output_dt: Option<f64>,
    /// Relative tolerance of the transient integrator.
    pub rtol: f64,
    /// Multiplier on the default absolute tolerances.
    pub atol_scale: f64,
    /// Maximum step size, s.
    pub max_step: Option<f64>,
    pub max_steps: usize,
    /// Solve the steady state at t = 0 and start the transient from it.
    pub start_from_steady: bool,
}

impl Default for SimSettings {
    fn default() -> Self {
        SimSettings {
            t_end: 1.0,
            output_dt: None,
            rtol: 1e-4,
            atol_scale: 1.0,
            max_step: None,
            max_steps: 2_000_000,
            start_from_steady: false,
        }
    }
}

impl Model {
    pub fn from_yaml(s: &str) -> Result<Model> {
        serde_yaml::from_str(s).map_err(|e| Error::Parse(format!("YAML: {e}")))
    }
    pub fn from_json(s: &str) -> Result<Model> {
        serde_json::from_str(s).map_err(|e| Error::Parse(format!("JSON: {e}")))
    }
    /// Accepts JSON or YAML.
    pub fn parse(s: &str) -> Result<Model> {
        let t = s.trim_start();
        if t.starts_with('{') {
            Self::from_json(s)
        } else {
            Self::from_yaml(s)
        }
    }
    pub fn to_yaml(&self) -> String {
        serde_yaml::to_string(self).unwrap_or_default()
    }
}
