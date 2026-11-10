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

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::units::{parse_value, Dim};

/// Version of fluidnet that reads model files (the crate version).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Model {
    /// fluidnet version the file was written for ("0.6" or "0.6.0"). Optional;
    /// a file written for a newer minor version than this build is rejected.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "de_version"
    )]
    pub fluidnet: Option<String>,
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
#[serde(default, deny_unknown_fields)]
pub struct SimSettings {
    /// End time, s.
    #[serde(deserialize_with = "de_time")]
    pub t_end: f64,
    /// Output interval, s (default t_end / 500).
    #[serde(deserialize_with = "de_time_opt")]
    pub output_dt: Option<f64>,
    /// Relative tolerance of the transient integrator.
    pub rtol: f64,
    /// Multiplier on the default absolute tolerances.
    pub atol_scale: f64,
    /// Maximum step size, s.
    #[serde(deserialize_with = "de_time_opt")]
    pub max_step: Option<f64>,
    pub max_steps: usize,
    /// Solve the steady state at t = 0 and start the transient from it.
    pub start_from_steady: bool,
    /// Time at which schedules are evaluated for a steady-state solve
    /// (e.g. the end of a start sequence, when all valves are open).
    #[serde(deserialize_with = "de_time")]
    pub steady_time: f64,
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
            steady_time: 0.0,
        }
    }
}

fn de_time<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<f64, D::Error> {
    let v = Value::deserialize(d)?;
    parse_value(&v, Dim::Time).map_err(serde::de::Error::custom)
}

fn de_time_opt<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<f64>, D::Error> {
    match Value::deserialize(d)? {
        Value::Null => Ok(None),
        v => parse_value(&v, Dim::Time)
            .map(Some)
            .map_err(serde::de::Error::custom),
    }
}

fn de_version<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<Option<String>, D::Error> {
    match Value::deserialize(d)? {
        Value::Null => Ok(None),
        Value::String(s) => Ok(Some(s)),
        Value::Number(n) => Ok(Some(n.to_string())),
        v => Err(serde::de::Error::custom(format!(
            "fluidnet: expected a version like \"0.6\", got {v}"
        ))),
    }
}

/// (major, minor) of a version string such as "0.6" or "0.6.1".
fn major_minor(v: &str) -> Option<(u64, u64)> {
    let mut it = v.trim().split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next().map(|m| m.parse().ok()).unwrap_or(Some(0))?;
    Some((major, minor))
}

impl Model {
    pub fn from_yaml(s: &str) -> Result<Model> {
        let m: Model = serde_yaml::from_str(s).map_err(|e| Error::Parse(format!("YAML: {e}")))?;
        m.check_version()?;
        Ok(m)
    }
    pub fn from_json(s: &str) -> Result<Model> {
        let m: Model = serde_json::from_str(s).map_err(|e| Error::Parse(format!("JSON: {e}")))?;
        m.check_version()?;
        Ok(m)
    }
    fn check_version(&self) -> Result<()> {
        let Some(v) = &self.fluidnet else {
            return Ok(());
        };
        let want = major_minor(v)
            .ok_or_else(|| Error::Parse(format!("fluidnet: cannot read version '{v}'")))?;
        let have = major_minor(VERSION).unwrap_or((0, 0));
        if want > have {
            return Err(Error::Parse(format!(
                "model written for fluidnet {v}; this is fluidnet {VERSION}. Update fluidnet."
            )));
        }
        Ok(())
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
