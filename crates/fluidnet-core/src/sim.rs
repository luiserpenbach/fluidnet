//! High-level entry points: compile a model, run steady state or transient,
//! collect output channels.

use serde::Serialize;

use crate::error::{Error, Result};
use crate::model::Model;
use crate::network::{Eval, Network, NodeKind};
use crate::props::FluidLibrary;
use crate::solver::steady::{self, SteadyStats};
use crate::solver::transient::{self, Stats, TransientOptions};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Steady,
    Transient,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChannelData {
    pub name: String,
    pub unit: &'static str,
    pub data: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NodeInfo {
    pub name: String,
    pub fluid: String,
    pub volume: Option<f64>,
    pub boundary: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct CompSummary {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    /// port -> node name
    pub ports: Vec<(String, String)>,
    pub nodes: Vec<String>,
    pub branches: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Results {
    pub name: String,
    pub mode: &'static str,
    pub time: Vec<f64>,
    pub channels: Vec<ChannelData>,
    pub warnings: Vec<String>,
    pub stats: Option<Stats>,
    pub steady: Option<SteadyStats>,
    pub nodes: Vec<NodeInfo>,
    pub components: Vec<CompSummary>,
    pub n_states: usize,
    /// Set when the run stopped early; data up to the failure is kept.
    pub error: Option<String>,
}

impl Results {
    pub fn channel(&self, name: &str) -> Option<&[f64]> {
        self.channels
            .iter()
            .find(|c| c.name == name)
            .map(|c| c.data.as_slice())
    }
    pub fn last(&self, name: &str) -> Option<f64> {
        self.channel(name).and_then(|d| d.last().copied())
    }
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|e| format!("{{\"error\":\"{e}\"}}"))
    }
    pub fn to_csv(&self) -> String {
        let mut s = String::from("time [s]");
        for c in &self.channels {
            s.push_str(&format!(",{} [{}]", c.name, c.unit));
        }
        s.push('\n');
        for (i, t) in self.time.iter().enumerate() {
            s.push_str(&format!("{t:.9e}"));
            for c in &self.channels {
                let v = c.data[i];
                if v.is_finite() {
                    s.push_str(&format!(",{v:.9e}"));
                } else {
                    s.push(',');
                }
            }
            s.push('\n');
        }
        s
    }
}

struct Recorder<'a> {
    net: &'a Network,
    ev: Eval,
    time: Vec<f64>,
    data: Vec<Vec<f64>>,
}

impl<'a> Recorder<'a> {
    fn new(net: &'a Network) -> Self {
        Recorder {
            net,
            ev: Eval::new(net),
            time: vec![],
            data: vec![Vec::new(); net.channels.len()],
        }
    }
    fn record(&mut self, t: f64, y: &[f64]) -> Result<()> {
        self.net.evaluate(t, y, &mut self.ev)?;
        self.time.push(t);
        for (c, d) in self.net.channels.iter().zip(self.data.iter_mut()) {
            d.push(self.net.probe(&c.probe, &self.ev));
        }
        Ok(())
    }
    fn finish(
        self,
        mode: &'static str,
        stats: Option<Stats>,
        steady: Option<SteadyStats>,
    ) -> Results {
        let net = self.net;
        Results {
            name: net.name.clone(),
            mode,
            time: self.time,
            channels: net
                .channels
                .iter()
                .zip(self.data)
                .map(|(c, d)| ChannelData {
                    name: c.name.clone(),
                    unit: c.unit,
                    data: d,
                })
                .collect(),
            warnings: net.warnings.clone(),
            stats,
            steady,
            nodes: net
                .nodes
                .iter()
                .map(|n| NodeInfo {
                    name: n.name.clone(),
                    fluid: net.fluids[n.fluid].key.clone(),
                    volume: match n.kind {
                        NodeKind::Volume { vol, .. } => Some(vol),
                        _ => None,
                    },
                    boundary: matches!(n.kind, NodeKind::Boundary { .. }),
                })
                .collect(),
            components: net
                .comps
                .iter()
                .map(|c| CompSummary {
                    id: c.id.clone(),
                    kind: c.kind.clone(),
                    ports: c
                        .ports
                        .iter()
                        .map(|(p, n)| (p.clone(), net.nodes[*n].name.clone()))
                        .collect(),
                    nodes: c.nodes.iter().map(|&n| net.nodes[n].name.clone()).collect(),
                    branches: c
                        .branches
                        .iter()
                        .map(|&b| net.branches[b].name.clone())
                        .collect(),
                })
                .collect(),
            n_states: net.n_states,
            error: None,
        }
    }
}

/// Solve the steady state at time t (boundary schedules evaluated at t).
pub fn run_steady(net: &Network, t: f64) -> Result<Results> {
    let (y, st) = steady::solve(net, &net.y0, t, 600)?;
    let mut rec = Recorder::new(net);
    rec.record(t, &y)?;
    Ok(rec.finish("steady", None, Some(st)))
}

/// Run the transient defined by the model's simulation settings.
pub fn run_transient(net: &Network, progress: &mut dyn FnMut(f64) -> bool) -> Result<Results> {
    let s = &net.sim;
    if !(s.t_end > 0.0) {
        return Err(Error::Model("simulation.t_end must be > 0".into()));
    }
    let dt = s
        .output_dt
        .unwrap_or(s.t_end / 500.0)
        .max(s.t_end / 200_000.0);
    let n_out = (s.t_end / dt).round() as usize;
    let mut out_times: Vec<f64> = (0..=n_out).map(|k| (k as f64 * dt).min(s.t_end)).collect();
    if (out_times.last().copied().unwrap_or(0.0) - s.t_end).abs() > 1e-12 {
        out_times.push(s.t_end);
    }
    let mut steady_stats = None;
    let y0 = if s.start_from_steady {
        let (y, st) = steady::solve(net, &net.y0, 0.0, 600)?;
        steady_stats = Some(st);
        y
    } else {
        net.y0.clone()
    };
    let opts = TransientOptions {
        t0: 0.0,
        t_end: s.t_end,
        rtol: s.rtol,
        atol_scale: s.atol_scale,
        max_step: s.max_step.unwrap_or(s.t_end / 20.0).max(1e-12),
        max_steps: s.max_steps,
        out_times,
        freeze_time: None,
        steady_tol: None,
    };
    let mut rec = Recorder::new(net);
    let outcome = {
        let mut sink = |t: f64, y: &[f64]| rec.record(t, y);
        transient::integrate(net, &y0, &opts, &mut sink, progress)
    };
    match outcome {
        Ok(stats) => Ok(rec.finish("transient", Some(stats), steady_stats)),
        Err(e) if !rec.time.is_empty() => {
            let mut r = rec.finish("transient", None, steady_stats);
            r.error = Some(e.to_string());
            Ok(r)
        }
        Err(e) => Err(e),
    }
}

/// Parse, compile and run in one call.
pub fn run_model(src: &str, lib: &FluidLibrary, mode: Mode) -> Result<Results> {
    let model = Model::parse(src)?;
    let net = Network::compile(&model, lib)?;
    match mode {
        Mode::Steady => run_steady(&net, 0.0),
        Mode::Transient => run_transient(&net, &mut |_| true),
    }
}
