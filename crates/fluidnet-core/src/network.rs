//! Compiled fluid network and its governing equations.
//!
//! States
//!   * volume nodes: pressure P and specific enthalpy h
//!   * pipe segments: mass flow mdot (momentum equation with inertia)
//! Algebraic (quasi-steady) elements: orifices, valves, check valves,
//! regulators, fittings, mass-flow sources.
//!
//! Volume node (rigid volume V, homogeneous equilibrium):
//!   V (drho/dP)_h dP/dt + V (drho/dh)_P dh/dt = sum(mdot_in) - sum(mdot_out)          (mass)
//!   rho V dh/dt - V dP/dt = sum(mdot_in (h_in - h)) + Q                               (energy)
//! Pipe segment (length L, area A):
//!   (L/A) dmdot/dt = P_a - P_b - dP_friction(mdot) - rho g dz

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use crate::catalog::Params;
use crate::error::{Error, Result};
use crate::flow::{nusselt, pipe_dp, restriction_flux, RestrictionModel};
use crate::model::{Model, SimSettings};
use crate::props::{FluidLibrary, FluidTable, Props};
use crate::units::Schedule;

const G0: f64 = 9.80665;
const DEFAULT_JUNCTION_VOLUME: f64 = 1e-6;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Pressure,
    Enthalpy,
    MassFlow,
}

#[derive(Debug, Clone)]
pub enum BSpec {
    T(Schedule),
    X(Schedule),
}

#[derive(Debug, Clone)]
pub enum NodeKind {
    Volume {
        vol: f64,
        sp: usize,
    },
    Boundary {
        p: Schedule,
        spec: BSpec,
        fixed: Option<Props>,
    },
}

#[derive(Debug, Clone)]
pub enum WallMode {
    Temp(Schedule),
    Flux(Schedule),
}

#[derive(Debug, Clone)]
pub enum Heat {
    Fixed(Schedule),
    Wall {
        area: f64,
        d: f64,
        flow_area: f64,
        mode: WallMode,
        branches: Vec<usize>,
    },
}

#[derive(Debug, Clone)]
pub struct Node {
    pub name: String,
    pub comp: Option<usize>,
    pub fluid: usize,
    pub kind: NodeKind,
    pub heat: Vec<Heat>,
    /// (branch index, +1 if this node is the branch's 'b' end (positive flow enters), -1 if 'a')
    pub conn: Vec<(usize, f64)>,
}

#[derive(Debug, Clone, Copy)]
pub enum Charac {
    Linear,
    EqualPct,
    Quick,
}

#[derive(Debug, Clone)]
pub enum Ctrl {
    Fixed,
    Valve {
        opening: Schedule,
        charac: Charac,
        leak: f64,
    },
    Check {
        crack: f64,
        full: f64,
        leak: f64,
    },
    Regulator {
        set: Schedule,
        band: f64,
        leak: f64,
    },
}

#[derive(Debug, Clone)]
pub enum BranchKind {
    Pipe {
        sm: usize,
        l: f64,
        d: f64,
        area: f64,
        rough: f64,
        k: f64,
        dz: f64,
    },
    Restriction {
        cda: f64,
        ctrl: Ctrl,
        model: RestrictionModel,
        recovery: f64,
    },
    MassFlow {
        mdot: Schedule,
    },
}

#[derive(Debug, Clone)]
pub struct Branch {
    pub name: String,
    pub comp: usize,
    pub a: usize,
    pub b: usize,
    pub kind: BranchKind,
}

#[derive(Debug, Clone)]
pub struct CompInfo {
    pub id: String,
    pub kind: String,
    pub nodes: Vec<usize>,
    pub branches: Vec<usize>,
    /// port name -> node index
    pub ports: Vec<(String, usize)>,
}

/// Output channel definition.
#[derive(Debug, Clone)]
pub enum Probe {
    P(usize),
    T(usize),
    Rho(usize),
    H(usize),
    X(usize),
    Mass(usize),
    Mdot(usize),
    Dp(usize),
    Opening(usize),
    Choked(usize),
    HeatSum(Vec<usize>),
    Velocity(usize),
    /// Pressure drop over a whole (multi-segment) pipe component.
    PipeDp(usize),
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub unit: &'static str,
    pub probe: Probe,
}

pub struct Network {
    pub name: String,
    pub fluids: Vec<Arc<FluidTable>>,
    pub nodes: Vec<Node>,
    pub branches: Vec<Branch>,
    pub comps: Vec<CompInfo>,
    pub n_states: usize,
    pub state_kind: Vec<StateKind>,
    pub state_names: Vec<String>,
    pub y0: Vec<f64>,
    pub breakpoints: Vec<f64>,
    pub warnings: Vec<String>,
    pub channels: Vec<Channel>,
    pub sim: SimSettings,
}

/// Evaluation workspace (all intermediate quantities at one (t, y)).
#[derive(Debug, Clone, Default)]
pub struct Eval {
    pub props: Vec<Props>,
    pub mdot: Vec<f64>,
    pub choked: Vec<bool>,
    pub opening: Vec<f64>,
    pub heat: Vec<f64>,
}

impl Eval {
    pub fn new(net: &Network) -> Self {
        Eval {
            props: vec![Props::default(); net.nodes.len()],
            mdot: vec![0.0; net.branches.len()],
            choked: vec![false; net.branches.len()],
            opening: vec![1.0; net.branches.len()],
            heat: vec![0.0; net.nodes.len()],
        }
    }
}

#[inline]
fn smoothstep(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

// ---------------------------------------------------------------------------
// union-find
struct Uf(Vec<usize>);
impl Uf {
    fn new(n: usize) -> Self {
        Uf((0..n).collect())
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.0[r] != r {
            r = self.0[r];
        }
        let mut c = x;
        while self.0[c] != r {
            let n = self.0[c];
            self.0[c] = r;
            c = n;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.0[rb] = ra;
        }
    }
}

#[derive(Clone)]
enum InitSpec {
    Tx {
        p: f64,
        t: Option<f64>,
        x: Option<f64>,
    },
}

impl Network {
    /// Compile a model into a network.
    pub fn compile(model: &Model, lib: &FluidLibrary) -> Result<Network> {
        let mut warnings = Vec::new();
        let defs = &model.components;
        if defs.is_empty() {
            return Err(Error::Model("model has no components".into()));
        }
        // ---- ids and params
        let mut id_index = HashMap::new();
        let mut params = Vec::new();
        for (ci, c) in defs.iter().enumerate() {
            if c.id.trim().is_empty() {
                return Err(Error::Model(format!("component #{ci} has an empty id")));
            }
            if id_index.insert(c.id.as_str(), ci).is_some() {
                return Err(Error::Model(format!("duplicate component id '{}'", c.id)));
            }
            let p = Params::new(&c.id, &c.kind, &c.params)?;
            for k in p.unknown_keys() {
                warnings.push(format!("{}: unknown parameter '{k}' ignored", c.id));
            }
            params.push(p);
        }

        // ---- ports and connectivity
        let mut port_ids: HashMap<(usize, String), usize> = HashMap::new();
        let mut port_list: Vec<(usize, String)> = Vec::new();
        for (ci, p) in params.iter().enumerate() {
            for &pn in p.spec.ports {
                port_ids.insert((ci, pn.to_string()), port_list.len());
                port_list.push((ci, pn.to_string()));
            }
        }
        let mut uf = Uf::new(port_list.len());
        let resolve = |s: &str| -> Result<usize> {
            let s = s.trim();
            let (cid, pn) = match s.rsplit_once('.') {
                Some((a, b)) if id_index.contains_key(a) => (a, Some(b)),
                _ => (s, None),
            };
            let ci = *id_index.get(cid).ok_or_else(|| {
                Error::Model(format!("connection references unknown component '{cid}'"))
            })?;
            let ports = params[ci].spec.ports;
            let pn = match pn {
                Some(p) => p.to_string(),
                None if ports.len() == 1 => ports[0].to_string(),
                None => {
                    return Err(Error::Model(format!(
                        "connection '{s}': component '{cid}' has ports {ports:?}, name one (e.g. {cid}.{})",
                        ports[0]
                    )))
                }
            };
            port_ids.get(&(ci, pn.clone())).copied().ok_or_else(|| {
                Error::Model(format!(
                    "component '{cid}' has no port '{pn}' (ports: {ports:?})"
                ))
            })
        };
        let mut port_conn_count = vec![0usize; port_list.len()];
        for c in &model.connections {
            let (a, b) = c.ends()?;
            let (pa, pb) = (resolve(&a)?, resolve(&b)?);
            if pa == pb {
                return Err(Error::Model(format!(
                    "connection {a} -> {b} connects a port to itself"
                )));
            }
            uf.union(pa, pb);
            port_conn_count[pa] += 1;
            port_conn_count[pb] += 1;
        }

        // groups of ports = network junction points
        let mut group_of_root: HashMap<usize, usize> = HashMap::new();
        let mut group_ports: Vec<Vec<usize>> = Vec::new();
        for pid in 0..port_list.len() {
            let r = uf.find(pid);
            let g = *group_of_root.entry(r).or_insert_with(|| {
                group_ports.push(Vec::new());
                group_ports.len() - 1
            });
            group_ports[g].push(pid);
        }
        let group_of_port: Vec<usize> = (0..port_list.len())
            .map(|pid| group_of_root[&uf.find(pid)])
            .collect();

        // dangling two-port ends
        for (pid, (ci, pn)) in port_list.iter().enumerate() {
            if !params[*ci].spec.node_like && port_conn_count[pid] == 0 {
                return Err(Error::Model(format!(
                    "port {}.{pn} is not connected",
                    defs[*ci].id
                )));
            }
        }

        // ---- nodes from groups
        let mut nodes: Vec<Node> = Vec::new();
        let mut node_fluid_req: Vec<Option<String>> = Vec::new();
        let mut node_init: Vec<Option<InitSpec>> = Vec::new();
        let mut node_vol_extra: Vec<f64> = Vec::new();
        let mut group_node = vec![usize::MAX; group_ports.len()];
        let mut comps: Vec<CompInfo> = defs
            .iter()
            .map(|d| CompInfo {
                id: d.id.clone(),
                kind: d.kind.clone(),
                nodes: vec![],
                branches: vec![],
                ports: vec![],
            })
            .collect();

        let read_init = |p: &Params| -> Result<Option<InitSpec>> {
            let p0 = p.q_opt("P0")?;
            let t0 = p.q_opt("T0")?;
            let x0 = p.q_opt("x0")?;
            Ok(match p0 {
                Some(pv) => {
                    if t0.is_none() && x0.is_none() {
                        return Err(Error::Model(format!("{}: P0 given without T0 or x0", p.id)));
                    }
                    Some(InitSpec::Tx {
                        p: pv,
                        t: t0,
                        x: x0,
                    })
                }
                None => None,
            })
        };

        for (g, ports) in group_ports.iter().enumerate() {
            let owners: Vec<usize> = ports
                .iter()
                .map(|&pid| port_list[pid].0)
                .filter(|&ci| params[ci].spec.node_like)
                .collect();
            if owners.len() > 1 {
                let ids: Vec<&str> = owners.iter().map(|&c| defs[c].id.as_str()).collect();
                return Err(Error::Model(format!(
                    "{} are connected directly; put a flow element (pipe, orifice, valve) between them",
                    ids.join(" and ")
                )));
            }
            let idx = nodes.len();
            group_node[g] = idx;
            if let Some(&ci) = owners.first() {
                let p = &params[ci];
                let fluid = p.str_opt("fluid");
                match p.spec.kind {
                    "boundary" => {
                        let ps = p.sched("P")?;
                        let spec = if p.has("x") {
                            BSpec::X(p.sched("x")?)
                        } else if p.has("T") {
                            BSpec::T(p.sched("T")?)
                        } else {
                            return Err(Error::Model(format!("{}: boundary needs T or x", p.id)));
                        };
                        nodes.push(Node {
                            name: defs[ci].id.clone(),
                            comp: Some(ci),
                            fluid: usize::MAX,
                            kind: NodeKind::Boundary {
                                p: ps,
                                spec,
                                fixed: None,
                            },
                            heat: vec![],
                            conn: vec![],
                        });
                        node_init.push(None);
                    }
                    "tank" | "junction" => {
                        let vol = p.q("volume")?;
                        if !(vol > 0.0) {
                            return Err(Error::Model(format!("{}: volume must be > 0", p.id)));
                        }
                        let mut heat = vec![];
                        if p.spec.kind == "tank" {
                            let q = p.sched("Q")?;
                            if !matches!(q, Schedule::Const(c) if c == 0.0) {
                                heat.push(Heat::Fixed(q));
                            }
                        }
                        nodes.push(Node {
                            name: defs[ci].id.clone(),
                            comp: Some(ci),
                            fluid: usize::MAX,
                            kind: NodeKind::Volume { vol, sp: 0 },
                            heat,
                            conn: vec![],
                        });
                        node_init.push(read_init(p)?);
                    }
                    k => return Err(Error::Model(format!("internal: node-like kind {k}"))),
                }
                node_fluid_req.push(fluid);
                comps[ci].nodes.push(idx);
            } else {
                // implicit junction; name after the first non-pipe port, else first port
                let pick = ports
                    .iter()
                    .find(|&&pid| defs[port_list[pid].0].kind != "pipe")
                    .or_else(|| ports.first())
                    .copied()
                    .unwrap();
                let (ci, pn) = &port_list[pick];
                nodes.push(Node {
                    name: format!("{}.{}", defs[*ci].id, pn),
                    comp: None,
                    fluid: usize::MAX,
                    kind: NodeKind::Volume { vol: 0.0, sp: 0 },
                    heat: vec![],
                    conn: vec![],
                });
                node_init.push(None);
                node_fluid_req.push(None);
            }
            node_vol_extra.push(0.0);
            for &pid in ports {
                let (ci, pn) = &port_list[pid];
                comps[*ci].ports.push((pn.clone(), idx));
            }
        }

        // ---- branches (and pipe-internal nodes)
        let mut branches: Vec<Branch> = Vec::new();
        let mut breakpoints: Vec<f64> = Vec::new();
        let port_node = |ci: usize, pn: &str| -> usize {
            group_node[group_of_port[port_ids[&(ci, pn.to_string())]]]
        };

        for (ci, p) in params.iter().enumerate() {
            if p.spec.node_like {
                if p.spec.kind == "boundary" {
                    for k in ["P", "T", "x"] {
                        if p.has(k) {
                            breakpoints.extend(p.sched(k)?.breakpoints());
                        }
                    }
                }
                if p.spec.kind == "tank" {
                    breakpoints.extend(p.sched("Q")?.breakpoints());
                }
                continue;
            }
            let id = &defs[ci].id;
            let na = port_node(ci, "in");
            let nb = port_node(ci, "out");
            if na == nb {
                return Err(Error::Model(format!(
                    "{id}: inlet and outlet are connected to the same point"
                )));
            }
            let model = |p: &Params| -> Result<RestrictionModel> {
                let s = p.str_opt("model").unwrap_or_else(|| "auto".into());
                RestrictionModel::parse(&s)
                    .ok_or_else(|| Error::Model(format!("{id}: unknown flow model '{s}'")))
            };
            match p.spec.kind {
                "pipe" => {
                    let l = p.q("length")?;
                    let d = p.q("diameter")?;
                    let rough = p.q("roughness")?;
                    let n = p.int("segments")?.clamp(1, 1000) as usize;
                    let dz = p.q("dz")?;
                    let k = p.q("K")?;
                    if !(l > 0.0 && d > 0.0) {
                        return Err(Error::Model(format!(
                            "{id}: length and diameter must be > 0"
                        )));
                    }
                    let area = std::f64::consts::PI * d * d / 4.0;
                    let seg_l = l / n as f64;
                    let seg_v = area * seg_l;
                    let init = read_init(p)?;
                    let fluid = p.str_opt("fluid");
                    // chain of nodes: na, internal 1..n-1, nb
                    let mut chain = vec![na];
                    for s in 1..n {
                        let idx = nodes.len();
                        nodes.push(Node {
                            name: format!("{id}.n{s}"),
                            comp: Some(ci),
                            fluid: usize::MAX,
                            kind: NodeKind::Volume { vol: seg_v, sp: 0 },
                            heat: vec![],
                            conn: vec![],
                        });
                        node_init.push(init.clone());
                        node_fluid_req.push(fluid.clone());
                        node_vol_extra.push(0.0);
                        comps[ci].nodes.push(idx);
                        chain.push(idx);
                    }
                    chain.push(nb);
                    node_vol_extra[na] += 0.5 * seg_v;
                    node_vol_extra[nb] += 0.5 * seg_v;
                    if let Some(f) = &fluid {
                        for &e in [na, nb].iter() {
                            if node_fluid_req[e].is_none() {
                                node_fluid_req[e] = Some(f.clone());
                            }
                        }
                    }
                    if let Some(i) = &init {
                        for &e in [na, nb].iter() {
                            if node_init[e].is_none()
                                && matches!(nodes[e].kind, NodeKind::Volume { .. })
                                && nodes[e].comp.is_none()
                            {
                                node_init[e] = Some(i.clone());
                            }
                        }
                    }
                    let first_branch = branches.len();
                    for s in 0..n {
                        branches.push(Branch {
                            name: if n == 1 {
                                id.clone()
                            } else {
                                format!("{id}.s{}", s + 1)
                            },
                            comp: ci,
                            a: chain[s],
                            b: chain[s + 1],
                            kind: BranchKind::Pipe {
                                sm: 0,
                                l: seg_l,
                                d,
                                area,
                                rough,
                                k: k / n as f64,
                                dz: dz / n as f64,
                            },
                        });
                        comps[ci].branches.push(first_branch + s);
                    }
                    // wall heat
                    let wall = if p.has("T_wall") {
                        Some(WallMode::Temp(p.sched("T_wall")?))
                    } else if p.has("q_wall") {
                        Some(WallMode::Flux(p.sched("q_wall")?))
                    } else {
                        None
                    };
                    if let Some(mode) = wall {
                        match &mode {
                            WallMode::Temp(s) | WallMode::Flux(s) => {
                                breakpoints.extend(s.breakpoints())
                            }
                        }
                        let seg_aw = std::f64::consts::PI * d * seg_l;
                        for (pos, &nd) in chain.iter().enumerate() {
                            let (aw, brs) = if pos == 0 {
                                (0.5 * seg_aw, vec![first_branch])
                            } else if pos == n {
                                (0.5 * seg_aw, vec![first_branch + n - 1])
                            } else {
                                (seg_aw, vec![first_branch + pos - 1, first_branch + pos])
                            };
                            nodes[nd].heat.push(Heat::Wall {
                                area: aw,
                                d,
                                flow_area: area,
                                mode: mode.clone(),
                                branches: brs,
                            });
                        }
                    }
                }
                "orifice" | "venturi" | "loss" | "valve" | "check_valve" | "regulator" => {
                    let recovery = if matches!(p.spec.kind, "orifice" | "venturi") {
                        p.q("recovery")?.clamp(0.0, 0.98)
                    } else {
                        0.0
                    };
                    let (cda, ctrl, rmodel) = match p.spec.kind {
                        "orifice" | "venturi" => (p.cda()?, Ctrl::Fixed, model(p)?),
                        "loss" => {
                            let k = p.q("K")?;
                            if !(k > 0.0) {
                                return Err(Error::Model(format!("{id}: K must be > 0")));
                            }
                            (p.area()? / k.sqrt(), Ctrl::Fixed, RestrictionModel::Auto)
                        }
                        "valve" => {
                            let opening = p.sched("opening")?;
                            breakpoints.extend(opening.breakpoints());
                            let charac =
                                match p.str_opt("characteristic").as_deref().unwrap_or("linear") {
                                    "linear" => Charac::Linear,
                                    "equal_percentage" => Charac::EqualPct,
                                    "quick_opening" => Charac::Quick,
                                    o => {
                                        return Err(Error::Model(format!(
                                            "{id}: unknown characteristic '{o}'"
                                        )))
                                    }
                                };
                            (
                                p.cda()?,
                                Ctrl::Valve {
                                    opening,
                                    charac,
                                    leak: p.q("leak")?.max(0.0),
                                },
                                model(p)?,
                            )
                        }
                        "check_valve" => {
                            let crack = p.q("cracking_dp")?;
                            let full = p.q("full_open_dp")?;
                            if full <= crack {
                                return Err(Error::Model(format!(
                                    "{id}: full_open_dp must exceed cracking_dp"
                                )));
                            }
                            (
                                p.cda()?,
                                Ctrl::Check {
                                    crack,
                                    full,
                                    leak: p.q("leak")?.max(0.0),
                                },
                                model(p)?,
                            )
                        }
                        "regulator" => {
                            let set = p.sched("set_pressure")?;
                            breakpoints.extend(set.breakpoints());
                            let band = p.q("band")?;
                            if !(band > 0.0) {
                                return Err(Error::Model(format!("{id}: band must be > 0")));
                            }
                            (
                                p.cda()?,
                                Ctrl::Regulator {
                                    set,
                                    band,
                                    leak: p.q("leak")?.max(0.0),
                                },
                                model(p)?,
                            )
                        }
                        _ => unreachable!(),
                    };
                    if !(cda > 0.0) {
                        return Err(Error::Model(format!("{id}: effective area must be > 0")));
                    }
                    comps[ci].branches.push(branches.len());
                    branches.push(Branch {
                        name: id.clone(),
                        comp: ci,
                        a: na,
                        b: nb,
                        kind: BranchKind::Restriction {
                            cda,
                            ctrl,
                            model: rmodel,
                            recovery,
                        },
                    });
                }
                "mass_flow" => {
                    let mdot = p.sched("mdot")?;
                    breakpoints.extend(mdot.breakpoints());
                    comps[ci].branches.push(branches.len());
                    branches.push(Branch {
                        name: id.clone(),
                        comp: ci,
                        a: na,
                        b: nb,
                        kind: BranchKind::MassFlow { mdot },
                    });
                }
                k => {
                    return Err(Error::Model(format!(
                        "{id}: component type '{k}' not supported in this build"
                    )))
                }
            }
        }

        // volumes of implicit junctions / pipe-end contributions
        for (i, n) in nodes.iter_mut().enumerate() {
            if let NodeKind::Volume { vol, .. } = &mut n.kind {
                *vol += node_vol_extra[i];
                if *vol <= 0.0 {
                    *vol = DEFAULT_JUNCTION_VOLUME;
                    warnings.push(format!(
                        "{}: two restrictions meet with no volume between them; using {} mL",
                        n.name,
                        DEFAULT_JUNCTION_VOLUME * 1e6
                    ));
                }
            }
        }

        // adjacency
        for (k, b) in branches.iter().enumerate() {
            nodes[b.a].conn.push((k, -1.0));
            nodes[b.b].conn.push((k, 1.0));
        }

        // ---- fluid assignment over connected regions
        let mut fluids: Vec<Arc<FluidTable>> = Vec::new();
        let mut ufn = Uf::new(nodes.len());
        for b in &branches {
            ufn.union(b.a, b.b);
        }
        let mut region_fluid: HashMap<usize, (String, String)> = HashMap::new();
        for (i, req) in node_fluid_req.iter().enumerate() {
            if let Some(f) = req {
                let r = ufn.find(i);
                match region_fluid.get(&r) {
                    Some((g, who)) if !g.eq_ignore_ascii_case(f) => {
                        return Err(Error::Model(format!(
                            "fluid conflict: {} uses {f} but is connected to {who} using {g}",
                            nodes[i].name
                        )))
                    }
                    Some(_) => {}
                    None => {
                        region_fluid.insert(r, (f.clone(), nodes[i].name.clone()));
                    }
                }
            }
        }
        for i in 0..nodes.len() {
            let r = ufn.find(i);
            let (fname, _) = region_fluid.get(&r).ok_or_else(|| {
                Error::Model(format!(
                    "no fluid specified for the circuit containing {}",
                    nodes[i].name
                ))
            })?;
            let tab = lib.get(fname).ok_or_else(|| {
                Error::Model(format!(
                    "fluid '{fname}' not loaded (available: {})",
                    lib.keys().join(", ")
                ))
            })?;
            let fi = match fluids.iter().position(|f| Arc::ptr_eq(f, &tab)) {
                Some(fi) => fi,
                None => {
                    fluids.push(tab);
                    fluids.len() - 1
                }
            };
            nodes[i].fluid = fi;
        }

        // ---- boundary props (fixed ones precomputed) and initial states
        for n in nodes.iter_mut() {
            if let NodeKind::Boundary { p, spec, fixed } = &mut n.kind {
                let fl = &fluids[n.fluid];
                if p.is_const() && matches!(spec, BSpec::T(s) | BSpec::X(s) if s.is_const()) {
                    *fixed = Some(
                        boundary_props(fl, p, spec, 0.0)
                            .map_err(|e| Error::Model(format!("{}: {e}", n.name)))?,
                    );
                } else {
                    boundary_props(fl, p, spec, 0.0)
                        .map_err(|e| Error::Model(format!("{}: {e}", n.name)))?;
                }
            }
        }

        // initial (P, h) per node; BFS from specified nodes and boundaries
        let mut ph: Vec<Option<(f64, f64)>> = vec![None; nodes.len()];
        let mut queue = VecDeque::new();
        for (i, n) in nodes.iter().enumerate() {
            let fl = &fluids[n.fluid];
            let v = match (&n.kind, &node_init[i]) {
                (NodeKind::Boundary { p, spec, .. }, _) => {
                    let pr = boundary_props(fl, p, spec, 0.0)?;
                    Some((pr.p, pr.h))
                }
                (_, Some(InitSpec::Tx { p, t, x })) => {
                    let h = match (t, x) {
                        (_, Some(x)) => fl.h_px(*p, *x),
                        (Some(t), None) => fl.h_pt(*p, *t),
                        _ => unreachable!(),
                    }
                    .map_err(|e| Error::Model(format!("{}: initial state: {e}", n.name)))?;
                    Some((*p, h))
                }
                _ => None,
            };
            if v.is_some() {
                ph[i] = v;
                queue.push_back(i);
            }
        }
        // propagate through elements that are open at t = 0 first; closed valves
        // separate regions with different initial contents (e.g. LOX line vs GOX manifold)
        let closed_at_0 = |b: &Branch| -> bool {
            match &b.kind {
                BranchKind::Restriction {
                    ctrl: Ctrl::Valve { opening, .. },
                    ..
                } => opening.at(0.0) <= 0.0,
                BranchKind::Restriction {
                    ctrl: Ctrl::Check { .. },
                    ..
                } => true,
                BranchKind::MassFlow { .. } => true,
                _ => false,
            }
        };
        let mut deferred = VecDeque::new();
        loop {
            while let Some(i) = queue.pop_front() {
                let val = ph[i];
                for &(k, _) in &nodes[i].conn {
                    let b = &branches[k];
                    let j = if b.a == i { b.b } else { b.a };
                    if ph[j].is_none() {
                        if closed_at_0(b) {
                            deferred.push_back(i);
                            continue;
                        }
                        ph[j] = val;
                        queue.push_back(j);
                    }
                }
            }
            match deferred.pop_front() {
                Some(i) => {
                    // second pass: cross closed elements only into still-unassigned regions
                    let val = ph[i];
                    for &(k, _) in &nodes[i].conn {
                        let b = &branches[k];
                        let j = if b.a == i { b.b } else { b.a };
                        if ph[j].is_none() {
                            ph[j] = val;
                            queue.push_back(j);
                        }
                    }
                }
                None => break,
            }
        }
        #[allow(clippy::never_loop)]
        while let Some(i) = queue.pop_front() {
            let val = ph[i];
            for &(k, _) in &nodes[i].conn {
                let b = &branches[k];
                let j = if b.a == i { b.b } else { b.a };
                if ph[j].is_none() {
                    ph[j] = val;
                    queue.push_back(j);
                }
            }
        }

        // ---- state vector
        let mut n_states = 0;
        let mut state_kind = Vec::new();
        let mut state_names = Vec::new();
        let mut y0 = Vec::new();
        for (i, n) in nodes.iter_mut().enumerate() {
            if let NodeKind::Volume { sp, .. } = &mut n.kind {
                *sp = n_states;
                let (p, h) = ph[i].ok_or_else(|| {
                    Error::Model(format!(
                        "{}: no initial state (set P0/T0 on it or on a connected tank/boundary)",
                        n.name
                    ))
                })?;
                y0.extend([p, h]);
                state_kind.extend([StateKind::Pressure, StateKind::Enthalpy]);
                state_names.extend([format!("{}.P", n.name), format!("{}.h", n.name)]);
                n_states += 2;
            }
        }
        for b in branches.iter_mut() {
            if let BranchKind::Pipe { sm, .. } = &mut b.kind {
                *sm = n_states;
                y0.push(0.0);
                state_kind.push(StateKind::MassFlow);
                state_names.push(format!("{}.mdot", b.name));
                n_states += 1;
            }
        }

        breakpoints.retain(|t| *t > 0.0 && t.is_finite());
        breakpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
        breakpoints.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        let mut net = Network {
            name: model.name.clone(),
            fluids,
            nodes,
            branches,
            comps,
            n_states,
            state_kind,
            state_names,
            y0,
            breakpoints,
            warnings,
            channels: vec![],
            sim: model.simulation.clone(),
        };
        net.channels = net.build_channels();
        Ok(net)
    }

    fn build_channels(&self) -> Vec<Channel> {
        let mut ch: Vec<Channel> = Vec::new();
        let mut push =
            |name: String, unit: &'static str, probe: Probe| ch.push(Channel { name, unit, probe });
        for (i, n) in self.nodes.iter().enumerate() {
            push(format!("{}.P", n.name), "Pa", Probe::P(i));
            push(format!("{}.T", n.name), "K", Probe::T(i));
            push(format!("{}.rho", n.name), "kg/m3", Probe::Rho(i));
            push(format!("{}.h", n.name), "J/kg", Probe::H(i));
            if self.fluids[n.fluid].pcrit > 0.0 {
                push(format!("{}.x", n.name), "-", Probe::X(i));
            }
            if let (NodeKind::Volume { .. }, Some(ci)) = (&n.kind, n.comp) {
                if self.comps[ci].kind == "tank" {
                    push(format!("{}.m", n.name), "kg", Probe::Mass(i));
                }
            }
        }
        for c in &self.comps {
            if c.kind == "pipe" && c.branches.len() > 1 {
                let first = c.branches[0];
                let last = *c.branches.last().unwrap();
                push(format!("{}.mdot_in", c.id), "kg/s", Probe::Mdot(first));
                push(format!("{}.mdot_out", c.id), "kg/s", Probe::Mdot(last));
                push(format!("{}.u_in", c.id), "m/s", Probe::Velocity(first));
                let ci = self.comps.iter().position(|x| x.id == c.id).unwrap();
                push(format!("{}.dp", c.id), "Pa", Probe::PipeDp(ci));
            }
        }
        for (k, b) in self.branches.iter().enumerate() {
            push(format!("{}.mdot", b.name), "kg/s", Probe::Mdot(k));
            match &b.kind {
                BranchKind::Pipe { .. } => {
                    if self.comps[b.comp].branches.len() == 1 {
                        push(format!("{}.dp", b.name), "Pa", Probe::Dp(k));
                        push(format!("{}.u_in", b.name), "m/s", Probe::Velocity(k));
                    }
                }
                BranchKind::Restriction { ctrl, .. } => {
                    push(format!("{}.dp", b.name), "Pa", Probe::Dp(k));
                    push(format!("{}.choked", b.name), "-", Probe::Choked(k));
                    if !matches!(ctrl, Ctrl::Fixed) {
                        push(format!("{}.opening", b.name), "-", Probe::Opening(k));
                    }
                }
                BranchKind::MassFlow { .. } => {
                    push(format!("{}.dp", b.name), "Pa", Probe::Dp(k));
                }
            }
        }
        for c in &self.comps {
            if c.kind == "pipe" {
                let heated: Vec<usize> =
                    std::iter::once(c.ports.iter().find(|p| p.0 == "in").map(|p| p.1))
                        .flatten()
                        .chain(c.nodes.iter().copied())
                        .chain(c.ports.iter().find(|p| p.0 == "out").map(|p| p.1))
                        .filter(|&n| {
                            self.nodes[n]
                                .heat
                                .iter()
                                .any(|h| matches!(h, Heat::Wall { .. }))
                        })
                        .collect();
                if !heated.is_empty() {
                    push(format!("{}.Q", c.id), "W", Probe::HeatSum(heated));
                }
            }
        }
        ch
    }

    pub fn node_index(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    /// Evaluate all intermediate quantities at (t, y).
    pub fn evaluate(&self, t: f64, y: &[f64], ev: &mut Eval) -> Result<()> {
        for (i, n) in self.nodes.iter().enumerate() {
            let fl = &self.fluids[n.fluid];
            ev.props[i] = match &n.kind {
                NodeKind::Volume { sp, .. } => fl.props_ph(y[*sp], y[*sp + 1]),
                NodeKind::Boundary {
                    fixed: Some(pr), ..
                } => *pr,
                NodeKind::Boundary { p, spec, .. } => boundary_props(fl, p, spec, t)?,
            };
        }
        for (k, b) in self.branches.iter().enumerate() {
            ev.choked[k] = false;
            ev.mdot[k] = match &b.kind {
                BranchKind::Pipe { sm, .. } => y[*sm],
                BranchKind::MassFlow { mdot } => mdot.at(t),
                BranchKind::Restriction {
                    cda,
                    ctrl,
                    model,
                    recovery,
                } => {
                    let pa = &ev.props[b.a];
                    let pb = &ev.props[b.b];
                    let u = match ctrl {
                        Ctrl::Fixed => 1.0,
                        Ctrl::Valve {
                            opening,
                            charac,
                            leak,
                        } => {
                            let s = opening.at(t).clamp(0.0, 1.0);
                            let f = match charac {
                                Charac::Linear => s,
                                Charac::EqualPct => (50f64.powf(s) - 1.0) / 49.0,
                                Charac::Quick => s.sqrt(),
                            };
                            f.max(*leak)
                        }
                        Ctrl::Check { crack, full, leak } => {
                            let dp = pa.p - pb.p;
                            leak + (1.0 - leak) * smoothstep((dp - crack) / (full - crack))
                        }
                        Ctrl::Regulator { set, band, leak } => {
                            let e = (set.at(t) - pb.p) / band + 0.5;
                            leak + (1.0 - leak) * smoothstep(e)
                        }
                    };
                    ev.opening[k] = u;
                    let fl = &self.fluids[self.nodes[b.a].fluid];
                    let (up, pdn, sgn) = if pa.p >= pb.p {
                        (pa, pb.p, 1.0)
                    } else {
                        (pb, pa.p, -1.0)
                    };
                    let eps = 10.0 + 1e-5 * up.p;
                    // throat pressure for a restriction with downstream pressure recovery
                    let pthr = if *recovery > 0.0 {
                        (up.p - (up.p - pdn) / (1.0 - recovery)).max(1e-3 * up.p)
                    } else {
                        pdn
                    };
                    let fx = restriction_flux(fl, up, pthr, eps, *model);
                    ev.choked[k] = fx.choked;
                    sgn * cda * u * fx.g
                }
            };
        }
        for (i, n) in self.nodes.iter().enumerate() {
            let mut q = 0.0;
            for h in &n.heat {
                q += match h {
                    Heat::Fixed(s) => s.at(t),
                    Heat::Wall {
                        area,
                        d,
                        flow_area,
                        mode,
                        branches,
                    } => match mode {
                        WallMode::Flux(s) => s.at(t) * area,
                        WallMode::Temp(s) => {
                            let pr = &ev.props[i];
                            let m = branches.iter().map(|&k| ev.mdot[k].abs()).sum::<f64>()
                                / branches.len() as f64;
                            let re = m * d / (flow_area * pr.mu);
                            let prn = (pr.cp * pr.mu / pr.k).clamp(0.1, 1000.0);
                            let hc = nusselt(re, prn) * pr.k / d;
                            hc * area * (s.at(t) - pr.t)
                        }
                    },
                };
            }
            ev.heat[i] = q;
        }
        Ok(())
    }

    /// dy/dt = f(t, y). Returns an error only for invalid boundary states.
    pub fn rhs(&self, t: f64, y: &[f64], dy: &mut [f64], ev: &mut Eval) -> Result<()> {
        self.evaluate(t, y, ev)?;
        for b in self.branches.iter() {
            if let BranchKind::Pipe {
                sm,
                l,
                d,
                area,
                rough,
                k,
                dz,
            } = &b.kind
            {
                let pa = &ev.props[b.a];
                let pb = &ev.props[b.b];
                let rho = 2.0 / (pa.v + pb.v);
                let mu = 0.5 * (pa.mu + pb.mu);
                let m = y[*sm];
                let dpf = pipe_dp(m, rho, mu, *d, *area, *l, *rough, *k);
                dy[*sm] = (area / l) * (pa.p - pb.p - dpf - rho * G0 * dz);
            }
        }
        for (i, n) in self.nodes.iter().enumerate() {
            if let NodeKind::Volume { vol, sp } = n.kind {
                let pr = &ev.props[i];
                let mut msum = 0.0;
                let mut esum = ev.heat[i];
                for &(k, s) in &n.conn {
                    let f = s * ev.mdot[k];
                    msum += f;
                    if f > 0.0 {
                        let b = &self.branches[k];
                        let other = if b.a == i { b.b } else { b.a };
                        esum += f * (ev.props[other].h - pr.h);
                    }
                }
                let rp = pr.drho_dp();
                let rh = pr.drho_dh();
                let rho = pr.rho;
                // [V rp, V rh; -V, rho V] [dP; dh] = [M; E]
                let det = vol * (rp * rho + rh);
                let det = if det.abs() < 1e-300 { 1e-300 } else { det };
                dy[sp] = (msum * rho - rh * esum) / det;
                dy[sp + 1] = (rp * esum + msum) / det;
            }
        }
        Ok(())
    }

    /// Value of a channel from an evaluated workspace.
    pub fn probe(&self, pr: &Probe, ev: &Eval) -> f64 {
        match pr {
            Probe::P(i) => ev.props[*i].p,
            Probe::T(i) => ev.props[*i].t,
            Probe::Rho(i) => ev.props[*i].rho,
            Probe::H(i) => ev.props[*i].h,
            Probe::X(i) => {
                let pr = &ev.props[*i];
                let x = self.fluids[self.nodes[*i].fluid].quality(pr.p, pr.h);
                if x.is_finite() {
                    x.clamp(-0.001, 1.001).clamp(0.0, 1.0)
                } else {
                    f64::NAN
                }
            }
            Probe::Mass(i) => match self.nodes[*i].kind {
                NodeKind::Volume { vol, .. } => ev.props[*i].rho * vol,
                _ => 0.0,
            },
            Probe::Mdot(k) => ev.mdot[*k],
            Probe::Dp(k) => {
                let b = &self.branches[*k];
                ev.props[b.a].p - ev.props[b.b].p
            }
            Probe::PipeDp(ci) => {
                let c = &self.comps[*ci];
                let first = &self.branches[c.branches[0]];
                let last = &self.branches[*c.branches.last().unwrap()];
                ev.props[first.a].p - ev.props[last.b].p
            }
            Probe::Opening(k) => ev.opening[*k],
            Probe::Choked(k) => {
                if ev.choked[*k] {
                    1.0
                } else {
                    0.0
                }
            }
            Probe::HeatSum(ns) => ns.iter().map(|&n| ev.heat[n]).sum(),
            Probe::Velocity(k) => {
                let b = &self.branches[*k];
                match b.kind {
                    BranchKind::Pipe { area, .. } => {
                        let up = if ev.mdot[*k] >= 0.0 { b.a } else { b.b };
                        ev.mdot[*k] / (ev.props[up].rho * area)
                    }
                    _ => f64::NAN,
                }
            }
        }
    }

    /// Absolute tolerance weights per state for error and convergence norms.
    pub fn weights(&self, y: &[f64], rtol: f64, atol_scale: f64, w: &mut [f64]) {
        for i in 0..self.n_states {
            w[i] = match self.state_kind[i] {
                StateKind::Pressure => atol_scale * 1.0 + rtol * y[i].abs(),
                StateKind::Enthalpy => atol_scale * 10.0 + rtol * y[i].abs().max(2e5),
                StateKind::MassFlow => atol_scale * 1e-6 + rtol * y[i].abs(),
            };
        }
    }

    /// Typical magnitudes for finite-difference perturbations.
    pub fn typical(&self, i: usize) -> f64 {
        match self.state_kind[i] {
            StateKind::Pressure => 1e5,
            StateKind::Enthalpy => 1e5,
            StateKind::MassFlow => 1e-2,
        }
    }

    /// Sparsity: for each state column, the rows of dy that depend on it.
    pub fn sparsity(&self) -> Vec<Vec<usize>> {
        let n = self.n_states;
        let node_rows = |i: usize| -> Vec<usize> {
            match self.nodes[i].kind {
                NodeKind::Volume { sp, .. } => vec![sp, sp + 1],
                _ => vec![],
            }
        };
        let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (i, nd) in self.nodes.iter().enumerate() {
            if let NodeKind::Volume { sp, .. } = nd.kind {
                let mut rows = node_rows(i);
                for &(k, _) in &nd.conn {
                    let b = &self.branches[k];
                    let other = if b.a == i { b.b } else { b.a };
                    rows.extend(node_rows(other));
                    if let BranchKind::Pipe { sm, .. } = b.kind {
                        rows.push(sm);
                    }
                    // algebraic neighbours of the neighbour can see this node only through
                    // this branch's flow, already covered by the two end nodes.
                }
                rows.sort_unstable();
                rows.dedup();
                cols[sp] = rows.clone();
                cols[sp + 1] = rows;
            }
        }
        for (k, b) in self.branches.iter().enumerate() {
            if let BranchKind::Pipe { sm, .. } = b.kind {
                let mut rows = vec![sm];
                rows.extend(node_rows(b.a));
                rows.extend(node_rows(b.b));
                // nodes whose wall heat uses this branch's flow
                for (i, nd) in self.nodes.iter().enumerate() {
                    if nd
                        .heat
                        .iter()
                        .any(|h| matches!(h, Heat::Wall { branches, .. } if branches.contains(&k)))
                    {
                        rows.extend(node_rows(i));
                    }
                }
                rows.sort_unstable();
                rows.dedup();
                cols[sm] = rows;
            }
        }
        cols
    }
}

/// Boundary state at time t.
pub fn boundary_props(fl: &FluidTable, p: &Schedule, spec: &BSpec, t: f64) -> Result<Props> {
    let pv = p.at(t);
    if !(pv > 0.0) {
        return Err(Error::Model(format!(
            "boundary pressure must be > 0 (got {pv})"
        )));
    }
    let h = match spec {
        BSpec::T(s) => fl.h_pt(pv, s.at(t))?,
        BSpec::X(s) => fl.h_px(pv, s.at(t).clamp(0.0, 1.0))?,
    };
    Ok(fl.props_ph_x(pv, h))
}
