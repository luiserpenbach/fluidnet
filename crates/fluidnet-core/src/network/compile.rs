//! Model -> network compilation.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use super::*;
use crate::catalog::Params;
use crate::combustion;
use crate::machines::material;
use crate::model::Model;
use crate::props::FluidLibrary;

const DEFAULT_JUNCTION_VOLUME: f64 = 1e-6;
const NODE_LIKE: &[&str] = &["boundary", "tank", "junction"];

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
struct InitSpec {
    p: f64,
    t: Option<f64>,
    x: Option<f64>,
}

/// Pump design data finalised once initial states are known.
struct PumpDesign {
    branch: usize,
    w_d: f64,
    mdot_d: f64,
    dp_d: f64,
    rho_d: Option<f64>,
}

/// Wall data finalised once initial states are known.
struct WallInit {
    wall: usize,
    t0: Option<f64>,
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
        let mut id_index: HashMap<&str, usize> = HashMap::new();
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
        let kind_of = |ci: usize| -> &str { params[ci].spec.kind };
        // reference to another component of a given type
        let lookup_ref = |p: &Params, key: &str, target: &str| -> Result<Option<usize>> {
            match p.str_opt(key) {
                None => Ok(None),
                Some(rid) => {
                    let ci = *id_index.get(rid.as_str()).ok_or_else(|| {
                        Error::Model(format!("{}.{key}: no component '{rid}'", p.id))
                    })?;
                    if params[ci].spec.kind != target {
                        return Err(Error::Model(format!(
                            "{}.{key}: '{rid}' is a {}, expected a {target}",
                            p.id, params[ci].spec.kind
                        )));
                    }
                    Ok(Some(ci))
                }
            }
        };

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
            if ports.is_empty() {
                return Err(Error::Model(format!(
                    "'{cid}' ({}) has no fluid ports",
                    params[ci].spec.kind
                )));
            }
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

        for (pid, (ci, pn)) in port_list.iter().enumerate() {
            if !NODE_LIKE.contains(&kind_of(*ci)) && port_conn_count[pid] == 0 {
                return Err(Error::Model(format!(
                    "port {}.{pn} is not connected",
                    defs[*ci].id
                )));
            }
        }

        let mut breakpoints: Vec<f64> = Vec::new();
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

        // ---- shafts
        let mut shafts: Vec<Shaft> = Vec::new();
        let mut shaft_of_comp: HashMap<usize, usize> = HashMap::new();
        for (ci, p) in params.iter().enumerate() {
            if p.spec.kind != "shaft" {
                continue;
            }
            let speed = p.sched_opt("speed")?;
            if let Some(s) = &speed {
                breakpoints.extend(s.breakpoints());
            }
            let inertia = p.q("inertia")?;
            if speed.is_none() && !(inertia > 0.0) {
                return Err(Error::Model(format!("{}: inertia must be > 0", p.id)));
            }
            shaft_of_comp.insert(ci, shafts.len());
            shafts.push(Shaft {
                name: defs[ci].id.clone(),
                comp: Some(ci),
                state: None,
                speed,
                inertia,
                bearing: p.q("bearing_torque")?.max(0.0),
                viscous: p.q("viscous_loss")?.max(0.0),
                w0: p.q("N0")?,
                w_guess: 0.0,
                hold: None,
            });
        }

        // ---- chambers (port nodes are linked below)
        struct ChamberDef {
            comp: usize,
            vol: f64,
            at: f64,
            dt: f64,
        }
        let mut chamber_defs: Vec<ChamberDef> = Vec::new();
        let mut chamber_of_comp: HashMap<usize, usize> = HashMap::new();
        for (ci, p) in params.iter().enumerate() {
            if p.spec.kind != "combustion_chamber" {
                continue;
            }
            let at = if p.has("throat_area") {
                p.q("throat_area")?
            } else if p.has("throat_diameter") {
                let d = p.q("throat_diameter")?;
                std::f64::consts::PI * d * d / 4.0
            } else {
                return Err(Error::Model(format!(
                    "{}: give throat_diameter or throat_area",
                    p.id
                )));
            };
            if !(at > 0.0) {
                return Err(Error::Model(format!("{}: throat area must be > 0", p.id)));
            }
            let vol = match p.q_opt("volume")? {
                Some(v) => v,
                None => p.q("l_star")? * at,
            };
            chamber_of_comp.insert(ci, chamber_defs.len());
            chamber_defs.push(ChamberDef {
                comp: ci,
                vol,
                at,
                dt: (4.0 * at / std::f64::consts::PI).sqrt(),
            });
        }

        // ---- nodes from port groups
        let mut nodes: Vec<Node> = Vec::new();
        let mut node_fluid_req: Vec<Option<String>> = Vec::new();
        let mut node_init: Vec<Option<InitSpec>> = Vec::new();
        let mut node_vol_extra: Vec<f64> = Vec::new();
        let mut group_node = vec![usize::MAX; group_ports.len()];
        // (chamber index, oxidizer side?, node)
        let mut chamber_ports: Vec<(usize, bool, usize)> = Vec::new();

        let read_init = |p: &Params| -> Result<Option<InitSpec>> {
            let p0 = p.q_opt("P0")?;
            let t0 = p.q_opt("T0")?;
            let x0 = p.q_opt("x0")?;
            Ok(match p0 {
                Some(pv) => {
                    if t0.is_none() && x0.is_none() {
                        return Err(Error::Model(format!("{}: P0 given without T0 or x0", p.id)));
                    }
                    Some(InitSpec {
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
                .filter(|&ci| NODE_LIKE.contains(&kind_of(ci)))
                .collect();
            let chamber_side: Vec<usize> = ports
                .iter()
                .copied()
                .filter(|&pid| kind_of(port_list[pid].0) == "combustion_chamber")
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
            if !chamber_side.is_empty() {
                let (cci, pn) = &port_list[chamber_side[0]];
                if chamber_side.len() > 1 || !owners.is_empty() || ports.len() != 2 {
                    return Err(Error::Model(format!(
                        "{}.{pn}: connect each chamber inlet to exactly one flow element (an injector orifice, valve or pipe)",
                        defs[*cci].id
                    )));
                }
                let ch = chamber_of_comp[cci];
                nodes.push(Node {
                    name: format!("{}.{}", defs[*cci].id, pn),
                    comp: Some(*cci),
                    fluid: usize::MAX,
                    kind: NodeKind::ChamberPort {
                        ch,
                        neighbor: usize::MAX,
                    },
                    heat: vec![],
                    conn: vec![],
                });
                node_init.push(None);
                node_fluid_req.push(None);
                chamber_ports.push((ch, pn == "ox", idx));
            } else if let Some(&ci) = owners.first() {
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
                        for k in ["P", "T", "x"] {
                            if p.has(k) {
                                breakpoints.extend(p.sched(k)?.breakpoints());
                            }
                        }
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
                    _ => {
                        let vol = p.q("volume")?;
                        if !(vol > 0.0) {
                            return Err(Error::Model(format!("{}: volume must be > 0", p.id)));
                        }
                        let mut heat = vec![];
                        if p.spec.kind == "tank" {
                            let q = p.sched("Q")?;
                            breakpoints.extend(q.breakpoints());
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
                }
                node_fluid_req.push(fluid);
                comps[ci].nodes.push(idx);
            } else {
                // implicit junction; name after the first non-pipe port, else first port
                let pick = ports
                    .iter()
                    .find(|&&pid| kind_of(port_list[pid].0) != "pipe")
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

        // ---- branches, pipe-internal nodes, walls
        let mut branches: Vec<Branch> = Vec::new();
        let mut walls: Vec<Wall> = Vec::new();
        let mut wall_inits: Vec<WallInit> = Vec::new();
        let mut pump_designs: Vec<PumpDesign> = Vec::new();
        let port_node = |ci: usize, pn: &str| -> usize {
            group_node[group_of_port[port_ids[&(ci, pn.to_string())]]]
        };

        for (ci, p) in params.iter().enumerate() {
            let kind = p.spec.kind;
            if NODE_LIKE.contains(&kind) || kind == "shaft" || kind == "combustion_chamber" {
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
            let rmodel = |p: &Params| -> Result<RestrictionModel> {
                let s = p.str_opt("model").unwrap_or_else(|| "auto".into());
                RestrictionModel::parse(&s)
                    .ok_or_else(|| Error::Model(format!("{id}: unknown flow model '{s}'")))
            };
            // machine drive: a named shaft, or a prescribed speed (implicit shaft)
            let drive_of = |p: &Params,
                            shafts: &mut Vec<Shaft>,
                            breakpoints: &mut Vec<f64>|
             -> Result<Drive> {
                let ratio = p.q("ratio")?;
                if let Some(sci) = lookup_ref(p, "shaft", "shaft")? {
                    return Ok(Drive {
                        shaft: shaft_of_comp[&sci],
                        ratio,
                    });
                }
                let speed = p.sched_opt("speed")?.ok_or_else(|| {
                    Error::Model(format!("{id}: name a shaft or give a fixed speed"))
                })?;
                breakpoints.extend(speed.breakpoints());
                shafts.push(Shaft {
                    name: format!("{id}.drive"),
                    comp: None,
                    state: None,
                    speed: Some(speed),
                    inertia: 0.0,
                    bearing: 0.0,
                    viscous: 0.0,
                    w0: 0.0,
                    w_guess: 0.0,
                    hold: None,
                });
                Ok(Drive {
                    shaft: shafts.len() - 1,
                    ratio: 1.0,
                })
            };
            match kind {
                "pipe" => {
                    let l = p.q("length")?;
                    let d = p.q("diameter")?;
                    let rough = p.q("roughness")?;
                    let n = p.int("segments")?.clamp(1, 1000) as usize;
                    let npar = p.int("parallel")?.max(1) as f64;
                    let dz = p.q("dz")?;
                    let k = p.q("K")?;
                    if !(l > 0.0 && d > 0.0) {
                        return Err(Error::Model(format!(
                            "{id}: length and diameter must be > 0"
                        )));
                    }
                    let area = npar * std::f64::consts::PI * d * d / 4.0;
                    let seg_l = l / n as f64;
                    let seg_v = area * seg_l;
                    let init = read_init(p)?;
                    let fluid = p.str_opt("fluid");
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
                    for &e in [na, nb].iter() {
                        if !matches!(nodes[e].kind, NodeKind::Volume { .. }) {
                            continue;
                        }
                        node_vol_extra[e] += 0.5 * seg_v;
                        if let Some(f) = &fluid {
                            if node_fluid_req[e].is_none() {
                                node_fluid_req[e] = Some(f.clone());
                            }
                        }
                        if let Some(i) = &init {
                            if node_init[e].is_none() && nodes[e].comp.is_none() {
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
                    // share of the line per chain node (half segments at the ends) and its branches
                    let share = |pos: usize| -> (f64, Vec<usize>) {
                        if n == 1 {
                            (0.5, vec![first_branch])
                        } else if pos == 0 {
                            (0.5 / n as f64, vec![first_branch])
                        } else if pos == n {
                            (0.5 / n as f64, vec![first_branch + n - 1])
                        } else {
                            (
                                1.0 / n as f64,
                                vec![first_branch + pos - 1, first_branch + pos],
                            )
                        }
                    };
                    let wet_total = npar * std::f64::consts::PI * d * l;
                    if p.has("wall_thickness") {
                        if p.has("T_wall") {
                            return Err(Error::Model(format!("{id}: T_wall (prescribed wall temperature) cannot be combined with wall_thickness")));
                        }
                        let thick = p.q("wall_thickness")?;
                        let mname = p
                            .str_opt("wall_material")
                            .unwrap_or_else(|| "cucrzr".into());
                        let mat = material(&mname).ok_or_else(|| {
                            Error::Model(format!("{id}: unknown wall material '{mname}'"))
                        })?;
                        let mass_total = p.q("wall_mass_factor")?
                            * mat.rho
                            * npar
                            * std::f64::consts::PI
                            * (d + thick)
                            * thick
                            * l;
                        let hot_ch = lookup_ref(p, "hot_chamber", "combustion_chamber")?;
                        let hot_area_total = p.q_opt("hot_area")?;
                        let hot = if let Some(cci) = hot_ch {
                            HotSide::Chamber {
                                ch: chamber_of_comp[&cci],
                                contraction: p.q("hot_contraction")?,
                                factor: p.q("hot_htc_factor")?,
                            }
                        } else if p.has("hot_T") {
                            let t = p.sched("hot_T")?;
                            let htc = p.sched_opt("hot_htc")?.ok_or_else(|| {
                                Error::Model(format!("{id}: hot_T needs hot_htc"))
                            })?;
                            breakpoints.extend(t.breakpoints());
                            breakpoints.extend(htc.breakpoints());
                            HotSide::Gas { t, htc }
                        } else if p.has("q_wall") {
                            let q = p.sched("q_wall")?;
                            breakpoints.extend(q.breakpoints());
                            HotSide::Flux(q)
                        } else {
                            HotSide::None
                        };
                        let hot_total = match (&hot, hot_area_total) {
                            (HotSide::None, _) => 0.0,
                            (_, Some(a)) => a,
                            (HotSide::Flux(_), None) => wet_total,
                            (_, None) => {
                                return Err(Error::Model(format!(
                                    "{id}: give hot_area (hot-gas side area of the cooled wall)"
                                )))
                            }
                        };
                        let cool_factor = p.q("cool_area_factor")?;
                        let corr = p.q("cool_correction")?;
                        let t0 = p.q_opt("wall_T0")?;
                        for (pos, &nd) in chain.iter().enumerate() {
                            let (f, brs) = share(pos);
                            wall_inits.push(WallInit {
                                wall: walls.len(),
                                t0,
                            });
                            walls.push(Wall {
                                name: format!("{id}.w{pos}"),
                                comp: ci,
                                node: nd,
                                state: 0,
                                mat,
                                mass: mass_total * f,
                                area_cool: wet_total * f * cool_factor,
                                area_hot: (hot_total * f).max(1e-12),
                                thick,
                                d,
                                flow_area: area,
                                branches: brs,
                                hot: hot.clone(),
                                corr,
                                t0: 0.0,
                            });
                        }
                    } else {
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
                            for (pos, &nd) in chain.iter().enumerate() {
                                let (f, brs) = share(pos);
                                nodes[nd].heat.push(Heat::Wall {
                                    area: wet_total * f,
                                    d,
                                    flow_area: area,
                                    mode: mode.clone(),
                                    branches: brs,
                                });
                            }
                        }
                    }
                }
                "orifice" | "venturi" | "loss" | "valve" | "check_valve" | "regulator" => {
                    let recovery = if matches!(kind, "orifice" | "venturi") {
                        p.q("recovery")?.clamp(0.0, 0.98)
                    } else {
                        0.0
                    };
                    let (cda, ctrl, model) = match kind {
                        "orifice" | "venturi" => (p.cda()?, Ctrl::Fixed, rmodel(p)?),
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
                                rmodel(p)?,
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
                                rmodel(p)?,
                            )
                        }
                        _ => {
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
                                rmodel(p)?,
                            )
                        }
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
                            model,
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
                "pump" => {
                    let drive = drive_of(p, &mut shafts, &mut breakpoints)?;
                    let w_d = p.q("design_speed")?;
                    let mdot_d = p.q("design_mdot")?;
                    let dp_d = p.q("design_dp")?;
                    let eta_d = p.q("design_eff")?;
                    if !(w_d > 0.0 && mdot_d > 0.0 && dp_d > 0.0 && eta_d > 0.0 && eta_d <= 1.0) {
                        return Err(Error::Model(format!("{id}: design speed, flow, pressure rise must be > 0 and efficiency in (0, 1]")));
                    }
                    let inertance = p.q("inertance")?;
                    if !(inertance > 0.0) {
                        return Err(Error::Model(format!("{id}: inertance must be > 0")));
                    }
                    pump_designs.push(PumpDesign {
                        branch: branches.len(),
                        w_d,
                        mdot_d,
                        dp_d,
                        rho_d: p.q_opt("design_density")?,
                    });
                    let map = PumpMap {
                        w_d,
                        q_d: 0.0,
                        h_d: 0.0,
                        eta_d,
                        a: p.q("shutoff_head")?,
                        p0: p.q("shutoff_power")?,
                    };
                    comps[ci].branches.push(branches.len());
                    branches.push(Branch {
                        name: id.clone(),
                        comp: ci,
                        a: na,
                        b: nb,
                        kind: BranchKind::Pump {
                            sm: 0,
                            drive,
                            map,
                            inertance,
                        },
                    });
                }
                "turbine" => {
                    let drive = drive_of(p, &mut shafts, &mut breakpoints)?;
                    let cda = p.cda()?;
                    let dm = p.q("mean_diameter")?;
                    let eta_d = p.q("design_eff")?;
                    let x_d = p.q("design_velocity_ratio")?;
                    if !(cda > 0.0 && dm > 0.0 && eta_d > 0.0 && x_d > 0.0) {
                        return Err(Error::Model(format!("{id}: nozzle area, mean diameter, efficiency and velocity ratio must be > 0")));
                    }
                    comps[ci].branches.push(branches.len());
                    branches.push(Branch {
                        name: id.clone(),
                        comp: ci,
                        a: na,
                        b: nb,
                        kind: BranchKind::Turbine {
                            drive,
                            cda,
                            map: TurbineMap {
                                r_m: 0.5 * dm,
                                eta_d,
                                x_d,
                            },
                        },
                    });
                }
                k => {
                    return Err(Error::Model(format!(
                        "{id}: component type '{k}' not supported in this build"
                    )))
                }
            }
        }

        // adjacency
        for (k, b) in branches.iter().enumerate() {
            nodes[b.a].conn.push((k, -1.0));
            nodes[b.b].conn.push((k, 1.0));
        }

        // volumes of implicit junctions / pipe-end contributions; where elements
        // meet without a pipe, use a stub of five bore diameters per element
        for (i, n) in nodes.iter_mut().enumerate() {
            if let NodeKind::Volume { vol, .. } = &mut n.kind {
                *vol += node_vol_extra[i];
                if *vol <= 0.0 {
                    let stub: f64 = n
                        .conn
                        .iter()
                        .map(|&(k, _)| {
                            let a = match &branches[k].kind {
                                BranchKind::Restriction { cda, .. }
                                | BranchKind::Turbine { cda, .. } => cda / 0.8,
                                BranchKind::Pump { map, .. } => map.q_d.max(1e-4) / 10.0,
                                _ => 0.0,
                            };
                            let d = (4.0 * a / std::f64::consts::PI).sqrt();
                            a * 5.0 * d
                        })
                        .sum();
                    *vol = stub.max(DEFAULT_JUNCTION_VOLUME);
                }
            }
        }
        for &(_, _, nd) in &chamber_ports {
            let (k, _) = nodes[nd].conn[0];
            let b = &branches[k];
            let nb = if b.a == nd { b.b } else { b.a };
            if let NodeKind::ChamberPort { neighbor, .. } = &mut nodes[nd].kind {
                *neighbor = nb;
            }
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

        // ---- chambers: propellant pair and combustion table
        let mut chambers: Vec<Chamber> = Vec::new();
        for (c, cd) in chamber_defs.iter().enumerate() {
            let p = &params[cd.comp];
            let port = |ox: bool| {
                chamber_ports
                    .iter()
                    .find(|x| x.0 == c && x.1 == ox)
                    .map(|x| x.2)
                    .unwrap()
            };
            let (pf, po) = (port(false), port(true));
            let ff = &fluids[nodes[pf].fluid];
            let fo = &fluids[nodes[po].fluid];
            let table = combustion::find(&ff.key, &fo.key).ok_or_else(|| {
                let avail: Vec<String> = combustion::pairs()
                    .iter()
                    .map(|(a, b)| format!("{a}/{b}"))
                    .collect();
                Error::Model(format!(
                    "{}: no combustion table for fuel {} with oxidizer {} (available: {})",
                    p.id,
                    ff.key,
                    fo.key,
                    avail.join(", ")
                ))
            })?;
            let pamb = p.sched("p_ambient")?;
            breakpoints.extend(pamb.breakpoints());
            let ignition = match p.q_opt("ignition_time")? {
                Some(t0) => {
                    let ramp = p.q("ignition_ramp")?.max(1e-4);
                    breakpoints.extend([t0, t0 + ramp]);
                    Some((t0, ramp))
                }
                None => None,
            };
            chambers.push(Chamber {
                name: defs[cd.comp].id.clone(),
                comp: cd.comp,
                sp: 0,
                vol: cd.vol,
                at: cd.at,
                dt: cd.dt,
                eps: p.q("area_ratio")?.max(1.0),
                pamb,
                eta_c: p.q("cstar_eff")?,
                eta_f: p.q("cf_eff")?,
                ignition,
                table,
                port_f: pf,
                port_o: po,
                mw_f: ff.molar_mass,
                mw_o: fo.molar_mass,
                p0: p.q("P0")?.max(100.0),
            });
            comps[cd.comp].nodes.extend([pf, po]);
        }

        // ---- boundary props (fixed ones precomputed)
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

        // ---- initial (P, h) per node; BFS from specified nodes and boundaries,
        // first through elements open at t = 0, so closed valves separate regions
        let mut ph: Vec<Option<(f64, f64)>> = vec![None; nodes.len()];
        let mut queue = VecDeque::new();
        for (i, n) in nodes.iter().enumerate() {
            let fl = &fluids[n.fluid];
            let v = match (&n.kind, &node_init[i]) {
                (NodeKind::Boundary { p, spec, .. }, _) => {
                    let pr = boundary_props(fl, p, spec, 0.0)?;
                    Some((pr.p, pr.h))
                }
                (_, Some(InitSpec { p, t, x })) => {
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
        let is_port = |i: usize| matches!(nodes[i].kind, NodeKind::ChamberPort { .. });
        let mut deferred = VecDeque::new();
        loop {
            while let Some(i) = queue.pop_front() {
                let val = ph[i];
                for &(k, _) in &nodes[i].conn {
                    let b = &branches[k];
                    let j = if b.a == i { b.b } else { b.a };
                    if ph[j].is_none() && !is_port(j) {
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
                    let val = ph[i];
                    for &(k, _) in &nodes[i].conn {
                        let b = &branches[k];
                        let j = if b.a == i { b.b } else { b.a };
                        if ph[j].is_none() && !is_port(j) {
                            ph[j] = val;
                            queue.push_back(j);
                        }
                    }
                }
                None => break,
            }
        }

        // pump design points: design density from the initial inlet state unless given
        for pd in &pump_designs {
            let b = &branches[pd.branch];
            let rho = match pd.rho_d {
                Some(r) => r,
                None => {
                    let (p, h) = ph[b.a].ok_or_else(|| {
                        Error::Model(format!(
                            "{}: no initial inlet state for the design density",
                            b.name
                        ))
                    })?;
                    fluids[nodes[b.a].fluid].props_ph(p, h).rho
                }
            };
            if let BranchKind::Pump { map, .. } = &mut branches[pd.branch].kind {
                map.q_d = pd.mdot_d / rho;
                map.h_d = pd.dp_d / (rho * combustion::G0);
                map.w_d = pd.w_d;
            }
        }
        // speed guesses for steady-state continuation
        for (s, sh) in shafts.iter_mut().enumerate() {
            sh.w_guess = if sh.w0 > 0.0 {
                sh.w0
            } else {
                branches
                    .iter()
                    .find_map(|b| match &b.kind {
                        BranchKind::Pump { drive, map, .. } if drive.shaft == s => {
                            Some(map.w_d / drive.ratio)
                        }
                        _ => None,
                    })
                    .unwrap_or(0.0)
            };
        }
        for wi in &wall_inits {
            let w = &mut walls[wi.wall];
            w.t0 = match wi.t0 {
                Some(t) => t,
                None => match ph[w.node] {
                    Some((p, h)) => fluids[nodes[w.node].fluid].props_ph(p, h).t,
                    None => 290.0,
                },
            };
        }

        // ---- state vector
        let mut n_states = 0;
        let mut state_kind = Vec::new();
        let mut state_names = Vec::new();
        let mut y0 = Vec::new();
        let mut push_state =
            |kind: StateKind, name: String, v: f64, n_states: &mut usize| -> usize {
                y0.push(v);
                state_kind.push(kind);
                state_names.push(name);
                *n_states += 1;
                *n_states - 1
            };
        for (i, n) in nodes.iter_mut().enumerate() {
            if let NodeKind::Volume { sp, .. } = &mut n.kind {
                let (p, h) = ph[i].ok_or_else(|| {
                    Error::Model(format!(
                        "{}: no initial state (set P0/T0 on it or on a connected tank/boundary)",
                        n.name
                    ))
                })?;
                *sp = push_state(
                    StateKind::Pressure,
                    format!("{}.P", n.name),
                    p,
                    &mut n_states,
                );
                push_state(
                    StateKind::Enthalpy,
                    format!("{}.h", n.name),
                    h,
                    &mut n_states,
                );
            }
        }
        for b in branches.iter_mut() {
            if let BranchKind::Pipe { sm, .. } | BranchKind::Pump { sm, .. } = &mut b.kind {
                *sm = push_state(
                    StateKind::MassFlow,
                    format!("{}.mdot", b.name),
                    0.0,
                    &mut n_states,
                );
            }
        }
        for sh in shafts.iter_mut() {
            if sh.speed.is_none() {
                sh.state = Some(push_state(
                    StateKind::Speed,
                    format!("{}.speed", sh.name),
                    sh.w0,
                    &mut n_states,
                ));
            }
        }
        for w in walls.iter_mut() {
            w.state = push_state(
                StateKind::WallTemp,
                format!("{}.T", w.name),
                w.t0,
                &mut n_states,
            );
        }
        for ch in chambers.iter_mut() {
            ch.sp = push_state(
                StateKind::Pressure,
                format!("{}.Pc", ch.name),
                ch.p0,
                &mut n_states,
            );
            push_state(
                StateKind::MixFrac,
                format!("{}.Z", ch.name),
                0.5,
                &mut n_states,
            );
        }

        breakpoints.retain(|t| *t > 0.0 && t.is_finite());
        breakpoints.sort_by(|a, b| a.partial_cmp(b).unwrap());
        breakpoints.dedup_by(|a, b| (*a - *b).abs() < 1e-12);

        let mut net = Network {
            name: model.name.clone(),
            fluids,
            nodes,
            branches,
            shafts,
            walls,
            chambers,
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
            if matches!(n.kind, NodeKind::ChamberPort { .. }) {
                continue;
            }
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
        for (ci, c) in self.comps.iter().enumerate() {
            if c.kind == "pipe" && c.branches.len() > 1 {
                let first = c.branches[0];
                let last = *c.branches.last().unwrap();
                push(format!("{}.mdot_in", c.id), "kg/s", Probe::Mdot(first));
                push(format!("{}.mdot_out", c.id), "kg/s", Probe::Mdot(last));
                push(format!("{}.u_in", c.id), "m/s", Probe::Velocity(first));
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
                BranchKind::MassFlow { .. } => push(format!("{}.dp", b.name), "Pa", Probe::Dp(k)),
                BranchKind::Pump { .. } => {
                    push(format!("{}.dp", b.name), "Pa", Probe::Br(k, BrF::Rise));
                    push(format!("{}.head", b.name), "m", Probe::Br(k, BrF::Aux1));
                    push(format!("{}.eff", b.name), "-", Probe::Br(k, BrF::Aux2));
                    push(format!("{}.npsh", b.name), "m", Probe::Br(k, BrF::Aux3));
                    push(format!("{}.power", b.name), "W", Probe::Br(k, BrF::Power));
                    push(
                        format!("{}.torque", b.name),
                        "N m",
                        Probe::Br(k, BrF::Torque),
                    );
                    push(
                        format!("{}.speed", b.name),
                        "rad/s",
                        Probe::Br(k, BrF::Speed),
                    );
                }
                BranchKind::Turbine { .. } => {
                    push(format!("{}.dp", b.name), "Pa", Probe::Dp(k));
                    push(format!("{}.pr", b.name), "-", Probe::Br(k, BrF::Aux3));
                    push(format!("{}.eff", b.name), "-", Probe::Br(k, BrF::Aux1));
                    push(format!("{}.u_c0", b.name), "-", Probe::Br(k, BrF::Aux2));
                    push(format!("{}.power", b.name), "W", Probe::Br(k, BrF::Power));
                    push(
                        format!("{}.torque", b.name),
                        "N m",
                        Probe::Br(k, BrF::Torque),
                    );
                    push(
                        format!("{}.speed", b.name),
                        "rad/s",
                        Probe::Br(k, BrF::Speed),
                    );
                    push(format!("{}.choked", b.name), "-", Probe::Choked(k));
                }
            }
        }
        for (s, sh) in self.shafts.iter().enumerate() {
            if sh.comp.is_some() {
                push(format!("{}.speed", sh.name), "rad/s", Probe::ShaftSpeed(s));
                push(
                    format!("{}.turbine_torque", sh.name),
                    "N m",
                    Probe::ShaftTorque(s),
                );
            }
        }
        for (c, chm) in self.chambers.iter().enumerate() {
            let n = &chm.name;
            push(format!("{n}.Pc"), "Pa", Probe::Ch(c, ChF::Pc));
            push(format!("{n}.OF"), "-", Probe::Ch(c, ChF::Of));
            push(format!("{n}.thrust"), "N", Probe::Ch(c, ChF::Thrust));
            push(format!("{n}.Isp"), "s", Probe::Ch(c, ChF::Isp));
            push(format!("{n}.cstar"), "m/s", Probe::Ch(c, ChF::Cstar));
            push(format!("{n}.Tc"), "K", Probe::Ch(c, ChF::Tc));
            push(format!("{n}.mdot"), "kg/s", Probe::Ch(c, ChF::Mdot));
            push(format!("{n}.mdot_fuel"), "kg/s", Probe::Ch(c, ChF::MdotF));
            push(format!("{n}.mdot_ox"), "kg/s", Probe::Ch(c, ChF::MdotO));
            push(format!("{n}.ignition"), "-", Probe::Ch(c, ChF::Ignition));
        }
        for c in &self.comps {
            if c.kind != "pipe" {
                continue;
            }
            let chain: Vec<usize> =
                std::iter::once(c.ports.iter().find(|p| p.0 == "in").map(|p| p.1))
                    .flatten()
                    .chain(c.nodes.iter().copied())
                    .chain(c.ports.iter().find(|p| p.0 == "out").map(|p| p.1))
                    .collect();
            let ws: Vec<usize> = (0..self.walls.len())
                .filter(|&w| self.comps[self.walls[w].comp].id == c.id)
                .collect();
            if !ws.is_empty() {
                push(
                    format!("{}.Q", c.id),
                    "W",
                    Probe::WallSum(ws.clone(), WallF::QCool),
                );
                push(
                    format!("{}.Q_hot", c.id),
                    "W",
                    Probe::WallSum(ws.clone(), WallF::QHot),
                );
                push(format!("{}.Tw_max", c.id), "K", Probe::WallMax(ws.clone()));
                for &w in &ws {
                    push(
                        format!("{}.T", self.walls[w].name),
                        "K",
                        Probe::Wall(w, WallF::T),
                    );
                    push(
                        format!("{}.T_hot", self.walls[w].name),
                        "K",
                        Probe::Wall(w, WallF::THot),
                    );
                    push(
                        format!("{}.Q", self.walls[w].name),
                        "W",
                        Probe::Wall(w, WallF::QCool),
                    );
                }
            } else {
                let heated: Vec<usize> = chain
                    .into_iter()
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
}
