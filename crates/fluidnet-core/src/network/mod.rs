//! Compiled fluid network and its governing equations.
//!
//! States
//!   * volume nodes: pressure P and specific enthalpy h
//!   * pipe segments and pumps: mass flow (momentum equation with inertia)
//!   * shafts: angular speed
//!   * walls with heat capacity: temperature
//!   * combustion chambers: pressure and oxidizer mass fraction of the gas
//!
//! Algebraic (quasi-steady) elements: orifices, valves, check valves,
//! regulators, fittings, mass-flow sources, turbines.
//!
//! Volume node (rigid volume V, homogeneous equilibrium):
//!   V (drho/dP)_h dP/dt + V (drho/dh)_P dh/dt = sum(mdot_in) - sum(mdot_out)            (mass)
//!   rho V dh/dt - V dP/dt = sum(mdot_in (h_in - h)) + Q + W_machines                    (energy)
//! Pipe segment (length L, flow area A):
//!   (L/A) dmdot/dt = P_a - P_b - dP_friction(mdot) - rho g dz
//! Pump (inertance I):  I dmdot/dt = P_a - P_b + dP_pump(w, mdot)
//! Shaft:               J dw/dt = sum(turbine torques) - sum(pump torques) - losses
//! Wall:                m c(T) dT/dt = Q_hot - Q_coolant
//! Chamber:             dPc/dt = (R T / V)(mdot_f + mdot_o - mdot_throat),
//!                      m dZ/dt = mdot_o (1 - Z) - Z mdot_f

mod compile;

use std::sync::Arc;

use crate::combustion::{bartz_htc, thrust_coefficient, vdk, CombTable, G0, RU, T_WALL_REF};
use crate::error::{Error, Result};
use crate::flow::{gas_flux, nusselt, pipe_dp, restriction_flux, RestrictionModel};
use crate::machines::{Material, PumpMap, TurbineMap};
use crate::model::SimSettings;
use crate::props::{FluidTable, Props};
use crate::units::Schedule;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateKind {
    Pressure,
    Enthalpy,
    MassFlow,
    Speed,
    WallTemp,
    MixFrac,
    /// Valve actuator position (0..1).
    Position,
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
    /// Injector side of a combustion chamber: pressure is the chamber state,
    /// enthalpy that of the node feeding it.
    ChamberPort {
        ch: usize,
        neighbor: usize,
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
    /// Opening set by an actuator state driven by a sensed pressure.
    Actuated {
        act: usize,
        charac: Charac,
        leak: f64,
    },
}

/// Pressure signal read by a valve actuator.
#[derive(Debug, Clone, Copy)]
pub enum Sense {
    Node(usize),
    Chamber(usize),
}

/// Valve actuator: first-order lag towards a position that is a linear
/// function of a sensed pressure (or pressure difference).
#[derive(Debug, Clone)]
pub struct Actuator {
    pub name: String,
    pub state: usize,
    pub sense: Sense,
    pub sense_ref: Option<Sense>,
    /// signal range [s_lo, s_hi] mapped to position [u_lo, u_hi]
    pub s_lo: f64,
    pub s_hi: f64,
    pub u_lo: f64,
    pub u_hi: f64,
    pub tau: f64,
    /// one-way motion (towards u_hi) in transients
    pub latch: bool,
    pub u0: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Drive {
    pub shaft: usize,
    pub ratio: f64,
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
    Pump {
        sm: usize,
        drive: Drive,
        map: PumpMap,
        inertance: f64,
    },
    Turbine {
        drive: Drive,
        cda: f64,
        map: TurbineMap,
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
pub struct Shaft {
    pub name: String,
    pub comp: Option<usize>,
    /// state index when the speed is solved, None when prescribed
    pub state: Option<usize>,
    pub speed: Option<Schedule>,
    pub inertia: f64,
    pub bearing: f64,
    pub viscous: f64,
    pub w0: f64,
    /// Speed guess for steady-state continuation (from N0 or the design speeds of its machines).
    pub w_guess: f64,
    /// Hold the speed at this value (steady-state continuation): dw/dt relaxes to it.
    pub hold: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum HotSide {
    None,
    Flux(Schedule),
    Gas {
        t: Schedule,
        htc: Schedule,
    },
    Chamber {
        ch: usize,
        /// local flow area over throat area at this wall element (Bartz)
        contraction: f64,
        factor: f64,
        /// drive with the equilibrium enthalpy difference (recombination at the
        /// wall) instead of cp (Tc - Tw)
        enthalpy: bool,
    },
}

/// Wall element with heat capacity between a hot side and the coolant in one node.
#[derive(Debug, Clone)]
pub struct Wall {
    pub name: String,
    pub comp: usize,
    pub node: usize,
    pub state: usize,
    pub mat: Material,
    pub mass: f64,
    /// effective coolant-side area (incl. fin factor)
    pub area_cool: f64,
    pub area_hot: f64,
    pub thick: f64,
    pub d: f64,
    pub flow_area: f64,
    pub branches: Vec<usize>,
    pub hot: HotSide,
    pub corr: f64,
    pub t0: f64,
}

#[derive(Debug, Clone)]
pub struct Chamber {
    pub name: String,
    pub comp: usize,
    pub sp: usize,
    pub vol: f64,
    pub at: f64,
    pub dt: f64,
    pub eps: f64,
    pub pamb: Schedule,
    pub eta_c: f64,
    pub eta_f: f64,
    /// (start, ramp)
    pub ignition: Option<(f64, f64)>,
    pub table: Arc<CombTable>,
    pub port_f: usize,
    pub port_o: usize,
    pub mw_f: f64,
    pub mw_o: f64,
    pub p0: f64,
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

#[derive(Debug, Clone, Copy)]
pub enum BrF {
    Power,
    Torque,
    Speed,
    /// pump head [m] / turbine efficiency
    Aux1,
    /// pump efficiency / turbine velocity ratio
    Aux2,
    /// pump NPSH available [m] / turbine pressure ratio
    Aux3,
    /// pressure rise P_out - P_in
    Rise,
}

#[derive(Debug, Clone, Copy)]
pub enum ChF {
    Pc,
    Of,
    Cstar,
    Tc,
    Mdot,
    MdotF,
    MdotO,
    Thrust,
    Isp,
    Ignition,
}

#[derive(Debug, Clone, Copy)]
pub enum WallF {
    T,
    THot,
    QHot,
    QCool,
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
    Br(usize, BrF),
    ShaftSpeed(usize),
    ShaftTorque(usize),
    Ch(usize, ChF),
    Wall(usize, WallF),
    WallMax(Vec<usize>),
    WallSum(Vec<usize>, WallF),
}

#[derive(Debug, Clone)]
pub struct Channel {
    pub name: String,
    pub unit: &'static str,
    pub probe: Probe,
}

#[derive(Clone)]
pub struct Network {
    pub name: String,
    pub fluids: Vec<Arc<FluidTable>>,
    pub nodes: Vec<Node>,
    pub branches: Vec<Branch>,
    pub shafts: Vec<Shaft>,
    pub walls: Vec<Wall>,
    pub chambers: Vec<Chamber>,
    pub actuators: Vec<Actuator>,
    /// Latching actuators move one way only (off for steady-state solves).
    pub latch_active: bool,
    /// States held at a value (steady-state fallback with tanks held at their
    /// initial state): their rate becomes (value - y) / 1 ms.
    pub held_states: Vec<(usize, f64)>,
    /// Components (tanks) held at their initial state in steady-state solves.
    pub steady_hold_comps: Vec<usize>,
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

#[derive(Debug, Clone, Copy, Default)]
pub struct ChEval {
    pub pc: f64,
    pub z: f64,
    pub of: f64,
    pub cstar: f64,
    pub tc: f64,
    pub gamma: f64,
    pub gamma_f: f64,
    pub molar_mass: f64,
    pub rt: f64,
    pub mdot: f64,
    pub mf: f64,
    pub mo: f64,
    pub thrust: f64,
    pub ign: f64,
    /// hot-gas driving potential to a wall at T_WALL_REF (burning gas), J/kg
    pub dh_wall: f64,
    pub cp_wall: f64,
    /// temperature of the unburnt propellant gas (before ignition)
    pub tc_cold: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WallEval {
    pub t: f64,
    pub t_hot: f64,
    pub q_hot: f64,
    pub q_cool: f64,
}

/// Evaluation workspace (all intermediate quantities at one (t, y)).
#[derive(Debug, Clone, Default)]
pub struct Eval {
    pub props: Vec<Props>,
    pub mdot: Vec<f64>,
    pub choked: Vec<bool>,
    pub opening: Vec<f64>,
    pub heat: Vec<f64>,
    /// power into the fluid per branch (pumps +, turbines -)
    pub power: Vec<f64>,
    /// torque applied to the shaft per branch, at machine speed (turbine +, pump -)
    pub torque: Vec<f64>,
    pub aux: Vec<[f64; 3]>,
    pub speed: Vec<f64>,
    pub ch: Vec<ChEval>,
    pub wall: Vec<WallEval>,
}

impl Eval {
    pub fn new(net: &Network) -> Self {
        let nb = net.branches.len();
        Eval {
            props: vec![Props::default(); net.nodes.len()],
            mdot: vec![0.0; nb],
            choked: vec![false; nb],
            opening: vec![1.0; nb],
            heat: vec![0.0; net.nodes.len()],
            power: vec![0.0; nb],
            torque: vec![0.0; nb],
            aux: vec![[f64::NAN; 3]; nb],
            speed: vec![0.0; net.shafts.len()],
            ch: vec![ChEval::default(); net.chambers.len()],
            wall: vec![WallEval::default(); net.walls.len()],
        }
    }
}

#[inline]
pub(crate) fn charac_eval(c: Charac, s: f64) -> f64 {
    match c {
        Charac::Linear => s,
        Charac::EqualPct => (50f64.powf(s) - 1.0) / 49.0,
        Charac::Quick => s.sqrt(),
    }
}

#[inline]
pub(crate) fn smoothstep(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

impl Network {
    /// Mass-flow guess for a cold steady-state solve: pumps at their design
    /// flow (and imposed mass flows), propagated through the network by mass
    /// balance, split at branch points in proportion to the effective flow
    /// areas. Returns (state index, value) for pipe and pump flow states.
    pub fn flow_guess(&self, t: f64, y: &[f64]) -> Vec<(usize, f64)> {
        let nb = self.branches.len();
        let mut ev = Eval::new(self);
        if self.evaluate(t, y, &mut ev).is_err() {
            return vec![];
        }
        let mut g: Vec<Option<f64>> = vec![None; nb];
        let mut weight = vec![0.0; nb];
        let mut seeded = false;
        for (k, b) in self.branches.iter().enumerate() {
            weight[k] = match &b.kind {
                BranchKind::Pipe { area, .. } => *area,
                BranchKind::Pump { map, .. } => {
                    let rho = ev.props[b.a].rho;
                    if rho > 0.0 && map.q_d > 0.0 {
                        g[k] = Some(map.q_d * rho);
                        seeded = true;
                    }
                    1.0
                }
                BranchKind::MassFlow { mdot } => {
                    g[k] = Some(mdot.at(t));
                    seeded = true;
                    1.0
                }
                BranchKind::Turbine { cda, .. } => *cda,
                BranchKind::Restriction { cda, ctrl, .. } => {
                    let u = match ctrl {
                        Ctrl::Fixed | Ctrl::Check { .. } | Ctrl::Regulator { .. } => 1.0,
                        Ctrl::Valve {
                            opening, charac, ..
                        } => charac_eval(*charac, opening.at(t).clamp(0.0, 1.0)),
                        Ctrl::Actuated { act, charac, .. } => {
                            charac_eval(*charac, y[self.actuators[*act].state].clamp(0.0, 1.0))
                        }
                    };
                    if u <= 1e-4 {
                        g[k] = Some(0.0);
                    }
                    cda * u
                }
            };
        }
        if !seeded {
            return vec![];
        }
        loop {
            let mut changed = false;
            for n in &self.nodes {
                if !matches!(n.kind, NodeKind::Volume { .. }) {
                    continue;
                }
                let mut sum_in = 0.0;
                let mut unknown: Vec<(usize, f64)> = Vec::new();
                for &(k, sgn) in &n.conn {
                    match g[k] {
                        Some(v) => sum_in += sgn * v,
                        None => unknown.push((k, sgn)),
                    }
                }
                if unknown.is_empty() || (unknown.len() > 1 && sum_in.abs() < 1e-12) {
                    continue;
                }
                let wsum: f64 = unknown.iter().map(|&(k, _)| weight[k].max(1e-12)).sum();
                for &(k, sgn) in &unknown {
                    // outflow share of the known net inflow
                    let share = sum_in * weight[k].max(1e-12) / wsum;
                    g[k] = Some(-share / sgn);
                }
                changed = true;
            }
            if !changed {
                break;
            }
        }
        let mut out = Vec::new();
        for (k, b) in self.branches.iter().enumerate() {
            if let (BranchKind::Pipe { sm, .. } | BranchKind::Pump { sm, .. }, Some(v)) =
                (&b.kind, g[k])
            {
                if v.is_finite() {
                    out.push((*sm, v));
                }
            }
        }
        out
    }

    /// Overwrite initial state values by state name (warm start from an
    /// earlier run of the same model). Returns how many names matched.
    pub fn apply_init(&mut self, init: &[(String, f64)]) -> usize {
        let idx: std::collections::HashMap<&str, usize> = self
            .state_names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let mut hit = 0;
        for (name, v) in init {
            if let Some(&i) = idx.get(name.as_str()) {
                if v.is_finite() {
                    self.y0[i] = *v;
                    hit += 1;
                }
            }
        }
        hit
    }

    pub fn node_index(&self, name: &str) -> Option<usize> {
        self.nodes.iter().position(|n| n.name == name)
    }

    fn drive_speed(&self, d: &Drive, ev: &Eval) -> f64 {
        d.ratio * ev.speed[d.shaft]
    }

    /// Evaluate all intermediate quantities at (t, y).
    pub fn evaluate(&self, t: f64, y: &[f64], ev: &mut Eval) -> Result<()> {
        // 1. volume and boundary nodes
        for (i, n) in self.nodes.iter().enumerate() {
            let fl = &self.fluids[n.fluid];
            match &n.kind {
                NodeKind::Volume { sp, .. } => ev.props[i] = fl.props_ph(y[*sp], y[*sp + 1]),
                NodeKind::Boundary {
                    fixed: Some(pr), ..
                } => ev.props[i] = *pr,
                NodeKind::Boundary { p, spec, .. } => ev.props[i] = boundary_props(fl, p, spec, t)?,
                NodeKind::ChamberPort { .. } => {}
            }
        }
        // 2. chamber ports: chamber pressure, enthalpy of the feeding node
        for (i, n) in self.nodes.iter().enumerate() {
            if let NodeKind::ChamberPort { ch, neighbor } = n.kind {
                let pc = y[self.chambers[ch].sp].max(1.0);
                ev.props[i] = self.fluids[n.fluid].props_ph(pc, ev.props[neighbor].h);
            }
        }
        // 3. shaft speeds
        for (s, sh) in self.shafts.iter().enumerate() {
            ev.speed[s] = match (&sh.state, &sh.speed) {
                (Some(st), _) => y[*st],
                (None, Some(sc)) => sc.at(t),
                (None, None) => 0.0,
            };
        }
        // 4. branch flows, machine power and torque
        for (k, b) in self.branches.iter().enumerate() {
            ev.choked[k] = false;
            ev.power[k] = 0.0;
            ev.torque[k] = 0.0;
            ev.mdot[k] = match &b.kind {
                BranchKind::Pipe { sm, .. } => y[*sm],
                BranchKind::MassFlow { mdot } => mdot.at(t),
                BranchKind::Pump { sm, drive, map, .. } => {
                    let m = y[*sm];
                    let pa = &ev.props[b.a];
                    let w = self.drive_speed(drive, ev);
                    let pt = map.eval(w, m, pa.rho);
                    ev.power[k] = pt.power;
                    ev.torque[k] = -pt.torque;
                    let fl = &self.fluids[self.nodes[b.a].fluid];
                    let npsh = match fl.psat_t(pa.t) {
                        Some(ps) => (pa.p - ps) / (pa.rho * G0),
                        None => f64::NAN,
                    };
                    ev.aux[k] = [pt.head, pt.eff, npsh];
                    m
                }
                BranchKind::Turbine { drive, cda, map } => {
                    let pa = &ev.props[b.a];
                    let pb = &ev.props[b.b];
                    let fl = &self.fluids[self.nodes[b.a].fluid];
                    let (up, pdn, sgn) = if pa.p >= pb.p {
                        (pa, pb.p, 1.0)
                    } else {
                        (pb, pa.p, -1.0)
                    };
                    let eps = 10.0 + 1e-5 * up.p;
                    let fx = restriction_flux(fl, up, pdn, eps, RestrictionModel::Auto);
                    ev.choked[k] = fx.choked;
                    let dh_s = match fl.h_ps_near(pdn, up.s, up.h - up.v * (up.p - pdn)) {
                        Some(h2) => (up.h - h2).max(0.0),
                        None => 0.0,
                    };
                    let w = self.drive_speed(drive, ev);
                    let pr = if pdn > 0.0 { up.p / pdn } else { 1.0 };
                    let m = sgn * cda * map.area_factor(w, dh_s, pr) * fx.g;
                    let (tq, pw, eff, x) = map.eval(w, m, dh_s);
                    ev.torque[k] = tq;
                    ev.power[k] = -pw;
                    ev.aux[k] = [eff, x, if pdn > 0.0 { up.p / pdn } else { f64::NAN }];
                    m
                }
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
                            charac_eval(*charac, s).max(*leak)
                        }
                        Ctrl::Check { crack, full, leak } => {
                            let dp = pa.p - pb.p;
                            leak + (1.0 - leak) * smoothstep((dp - crack) / (full - crack))
                        }
                        Ctrl::Regulator { set, band, leak } => {
                            let e = (set.at(t) - pb.p) / band + 0.5;
                            leak + (1.0 - leak) * smoothstep(e)
                        }
                        Ctrl::Actuated { act, charac, leak } => {
                            let s = y[self.actuators[*act].state].clamp(0.0, 1.0);
                            charac_eval(*charac, s).max(*leak)
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
        // 5. combustion chambers
        for (c, ch) in self.chambers.iter().enumerate() {
            ev.ch[c] = self.eval_chamber(ch, t, y, ev);
        }
        // 6. heat: prescribed and walls with heat capacity
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
                            let hc = coolant_htc(pr, ev, branches, *d, *flow_area, 1.0, 0.0);
                            hc * area * (s.at(t) - pr.t)
                        }
                    },
                };
            }
            ev.heat[i] = q;
        }
        for (wi, w) in self.walls.iter().enumerate() {
            let tw = y[w.state];
            let pr = ev.props[w.node];
            let hc = coolant_htc(&pr, ev, &w.branches, w.d, w.flow_area, tw, w.corr);
            let half = 0.5 * w.thick / w.mat.k;
            let q_cool = if hc > 0.0 {
                (tw - pr.t) / (1.0 / (hc * w.area_cool) + half / w.area_cool)
            } else {
                0.0
            };
            let (q_hot, t_hot) = match &w.hot {
                HotSide::None => (0.0, tw),
                HotSide::Flux(s) => {
                    let q = s.at(t) * w.area_hot;
                    (q, tw + q * half / w.area_hot.max(1e-12))
                }
                HotSide::Gas { t: tg, htc } => {
                    let hg = htc.at(t);
                    let q = if hg > 0.0 {
                        (tg.at(t) - tw) / (1.0 / (hg * w.area_hot) + half / w.area_hot)
                    } else {
                        0.0
                    };
                    (q, tw + q * half / w.area_hot.max(1e-12))
                }
                HotSide::Chamber {
                    ch,
                    contraction,
                    factor,
                    enthalpy,
                } => {
                    let ce = &ev.ch[*ch];
                    let chm = &self.chambers[*ch];
                    // driving temperature: Tc, or (enthalpy mode) the wall temperature plus
                    // the equilibrium enthalpy difference over the frozen cp used by Bartz
                    let t_drive = if *enthalpy && chm.table.has_wall && ce.ign > 0.0 {
                        let cp_g = ce.gamma_f * RU / (ce.molar_mass * 1e-3) / (ce.gamma_f - 1.0);
                        let dh = (ce.dh_wall - ce.cp_wall * (tw - T_WALL_REF)).max(0.0);
                        (1.0 - ce.ign) * ce.tc_cold + ce.ign * (tw + dh / cp_g)
                    } else {
                        ce.tc
                    };
                    // hot-gas-side surface temperature estimate for the Bartz sigma factor
                    let hg = factor
                        * bartz_htc(
                            chm.dt,
                            ce.pc,
                            ce.cstar,
                            ce.gamma_f,
                            ce.molar_mass,
                            ce.tc,
                            tw,
                            *contraction,
                        );
                    let q = if hg > 0.0 {
                        (t_drive - tw) / (1.0 / (hg * w.area_hot) + half / w.area_hot)
                    } else {
                        0.0
                    };
                    (q, tw + q * half / w.area_hot.max(1e-12))
                }
            };
            ev.heat[w.node] += q_cool;
            ev.wall[wi] = WallEval {
                t: tw,
                t_hot,
                q_hot,
                q_cool,
            };
        }
        Ok(())
    }

    fn eval_chamber(&self, ch: &Chamber, t: f64, y: &[f64], ev: &Eval) -> ChEval {
        let pc = y[ch.sp].max(1.0);
        let z = y[ch.sp + 1].clamp(1e-6, 1.0 - 1e-6);
        let inflow = |port: usize| -> f64 {
            self.nodes[port]
                .conn
                .iter()
                .map(|&(k, s)| s * ev.mdot[k])
                .sum()
        };
        let mf = inflow(ch.port_f);
        let mo = inflow(ch.port_o);
        let (of_lo, of_hi) = ch.table.of_range();
        let of = z / (1.0 - z);
        let cs = ch.table.eval(of.clamp(of_lo, of_hi), pc);
        // correction for injected enthalpy differing from the table reference
        let hf = ev.props[ch.port_f].h;
        let ho = ev.props[ch.port_o].h;
        let dh = (1.0 - z) * (hf - ch.table.h_ref_fuel) + z * (ho - ch.table.h_ref_ox);
        let cstar_hot = ch.eta_c * (cs.cstar + cs.dcstar_dh * dh).max(0.5 * cs.cstar);
        let tc_hot = ch.eta_c * ch.eta_c * (cs.tc + cs.dtc_dh * dh).max(0.5 * cs.tc);
        let rt_hot = (cstar_hot * vdk(cs.gamma_s)).powi(2);
        // cold propellant gas before ignition
        let mw_cold = 1.0 / ((1.0 - z) / ch.mw_f + z / ch.mw_o);
        let tf = ev.props[ch.port_f].t;
        let to = ev.props[ch.port_o].t;
        let t_cold = ((1.0 - z) * tf + z * to).max(100.0);
        let g_cold = 1.4;
        let rt_cold = RU / mw_cold * t_cold;
        let ign = match ch.ignition {
            None => 1.0,
            Some((t0, ramp)) => smoothstep((t - t0) / ramp.max(1e-6)),
        };
        let rt = (1.0 - ign) * rt_cold + ign * rt_hot;
        let gamma = (1.0 - ign) * g_cold + ign * cs.gamma_s;
        let gamma_f = (1.0 - ign) * g_cold + ign * cs.gamma_f;
        let tc = (1.0 - ign) * t_cold + ign * tc_hot;
        let molar_mass = 1e3 * ((1.0 - ign) * mw_cold + ign * cs.molar_mass * 1e-3);
        let cstar = rt.sqrt() / vdk(gamma);
        let pa = ch.pamb.at(t).max(0.0);
        // throat flow (choked or subsonic), no reverse flow from ambient
        let mdot = if pa < pc {
            let eps_reg = 10.0 + 1e-5 * pc;
            gas_flux(gamma, pa / pc, pc - pa, eps_reg) * pc / rt.sqrt() * ch.at
        } else {
            0.0
        };
        let crit = (2.0 / (gamma + 1.0)).powf(gamma / (gamma - 1.0));
        let thrust = if pa / pc < crit {
            (ch.eta_f * thrust_coefficient(gamma, ch.eps, pa / pc) * ch.at * pc).max(0.0)
        } else {
            // unchoked: ideal expansion to ambient pressure
            let v =
                (2.0 * gamma / (gamma - 1.0) * rt * (1.0 - (pa / pc).powf((gamma - 1.0) / gamma)))
                    .max(0.0)
                    .sqrt();
            ch.eta_f * mdot * v
        };
        ChEval {
            pc,
            z,
            of,
            cstar,
            tc,
            gamma,
            gamma_f,
            molar_mass,
            rt,
            mdot,
            mf,
            mo,
            thrust,
            ign,
            dh_wall: ch.eta_c * ch.eta_c * (cs.dh_wall + dh).max(0.0),
            cp_wall: cs.cp_wall,
            tc_cold: t_cold,
        }
    }

    /// dy/dt = f(t, y). Returns an error only for invalid boundary states.
    pub fn rhs(&self, t: f64, y: &[f64], dy: &mut [f64], ev: &mut Eval) -> Result<()> {
        self.evaluate(t, y, ev)?;
        for (k, b) in self.branches.iter().enumerate() {
            match &b.kind {
                BranchKind::Pipe {
                    sm,
                    l,
                    d,
                    area,
                    rough,
                    k: kl,
                    dz,
                } => {
                    let pa = &ev.props[b.a];
                    let pb = &ev.props[b.b];
                    let rho = 2.0 / (pa.v + pb.v);
                    let mu = 0.5 * (pa.mu + pb.mu);
                    let m = y[*sm];
                    let dpf = pipe_dp(m, rho, mu, *d, *area, *l, *rough, *kl);
                    dy[*sm] = (area / l) * (pa.p - pb.p - dpf - rho * G0 * dz);
                }
                BranchKind::Pump {
                    sm,
                    drive,
                    map,
                    inertance,
                } => {
                    let pa = &ev.props[b.a];
                    let pb = &ev.props[b.b];
                    let pt = map.eval(self.drive_speed(drive, ev), y[*sm], pa.rho);
                    dy[*sm] = (pa.p - pb.p + pt.dp) / inertance;
                    let _ = k;
                }
                _ => {}
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
                    let b = &self.branches[k];
                    if f > 0.0 {
                        let other = if b.a == i { b.b } else { b.a };
                        esum += f * (ev.props[other].h - pr.h);
                    }
                    if ev.power[k] != 0.0 {
                        // machine power goes with the flow; split smoothly near zero flow
                        let wb = 0.5 * (1.0 + (ev.mdot[k] / 1e-4).tanh());
                        esum += ev.power[k] * if s > 0.0 { wb } else { 1.0 - wb };
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
        for &(i, v) in &self.held_states {
            dy[i] = (v - y[i]) / 1e-3;
        }
        for (s, sh) in self.shafts.iter().enumerate() {
            if let (Some(st), Some(wh)) = (sh.state, sh.hold) {
                dy[st] = (wh - ev.speed[s]) / 1e-3;
                continue;
            }
            if let Some(st) = sh.state {
                let w = ev.speed[s];
                let mut tq = -sh.bearing * (w / 1.0).tanh() - sh.viscous * w;
                for (k, b) in self.branches.iter().enumerate() {
                    if let BranchKind::Pump { drive, .. } | BranchKind::Turbine { drive, .. } =
                        &b.kind
                    {
                        if drive.shaft == s {
                            tq += drive.ratio * ev.torque[k];
                        }
                    }
                }
                dy[st] = tq / sh.inertia;
            }
        }
        for (wi, w) in self.walls.iter().enumerate() {
            let we = &ev.wall[wi];
            dy[w.state] = (we.q_hot - we.q_cool) / (w.mass * w.mat.cp(we.t));
        }
        for (c, ch) in self.chambers.iter().enumerate() {
            let e = &ev.ch[c];
            let m = (e.pc * ch.vol / e.rt).max(1e-9);
            dy[ch.sp] = e.rt / ch.vol * (e.mf + e.mo - e.mdot);
            dy[ch.sp + 1] = (e.mo.max(0.0) * (1.0 - e.z) - e.z * e.mf.max(0.0)) / m;
        }
        for a in &self.actuators {
            let u = y[a.state];
            let target = self.actuator_target(a, ev);
            let mut d = (target - u) / a.tau;
            if a.latch && self.latch_active {
                d = if a.u_hi >= a.u_lo {
                    d.max(0.0)
                } else {
                    d.min(0.0)
                };
            }
            dy[a.state] = d;
        }
        Ok(())
    }

    fn sense_value(&self, s: Sense, ev: &Eval) -> f64 {
        match s {
            Sense::Node(i) => ev.props[i].p,
            Sense::Chamber(c) => ev.ch[c].pc,
        }
    }

    fn sense_states(&self, s: Sense) -> Vec<usize> {
        match s {
            Sense::Node(i) => self.node_states(i),
            Sense::Chamber(c) => vec![self.chambers[c].sp],
        }
    }

    /// Commanded actuator position from the sensed signal.
    pub fn actuator_target(&self, a: &Actuator, ev: &Eval) -> f64 {
        let sig = self.sense_value(a.sense, ev)
            - a.sense_ref.map(|r| self.sense_value(r, ev)).unwrap_or(0.0);
        let f = ((sig - a.s_lo) / (a.s_hi - a.s_lo)).clamp(0.0, 1.0);
        a.u_lo + (a.u_hi - a.u_lo) * f
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
                    x.clamp(0.0, 1.0)
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
            Probe::Br(k, f) => match f {
                BrF::Power => ev.power[*k].abs(),
                BrF::Torque => ev.torque[*k].abs(),
                BrF::Speed => match &self.branches[*k].kind {
                    BranchKind::Pump { drive, .. } | BranchKind::Turbine { drive, .. } => {
                        self.drive_speed(drive, ev)
                    }
                    _ => f64::NAN,
                },
                BrF::Aux1 => ev.aux[*k][0],
                BrF::Aux2 => ev.aux[*k][1],
                BrF::Aux3 => ev.aux[*k][2],
                BrF::Rise => {
                    let b = &self.branches[*k];
                    ev.props[b.b].p - ev.props[b.a].p
                }
            },
            Probe::ShaftSpeed(s) => ev.speed[*s],
            Probe::ShaftTorque(s) => {
                let mut tq = 0.0;
                for (k, b) in self.branches.iter().enumerate() {
                    if let BranchKind::Turbine { drive, .. } = &b.kind {
                        if drive.shaft == *s {
                            tq += drive.ratio * ev.torque[k];
                        }
                    }
                }
                tq
            }
            Probe::Ch(c, f) => {
                let e = &ev.ch[*c];
                match f {
                    ChF::Pc => e.pc,
                    ChF::Of => e.of,
                    ChF::Cstar => e.cstar,
                    ChF::Tc => e.tc,
                    ChF::Mdot => e.mdot,
                    ChF::MdotF => e.mf,
                    ChF::MdotO => e.mo,
                    ChF::Thrust => e.thrust,
                    ChF::Isp => {
                        if e.mdot > 1e-9 {
                            e.thrust / (e.mdot * G0)
                        } else {
                            f64::NAN
                        }
                    }
                    ChF::Ignition => e.ign,
                }
            }
            Probe::Wall(w, f) => wall_field(&ev.wall[*w], *f),
            Probe::WallMax(ws) => ws
                .iter()
                .map(|&w| ev.wall[w].t_hot)
                .fold(f64::NEG_INFINITY, f64::max),
            Probe::WallSum(ws, f) => ws.iter().map(|&w| wall_field(&ev.wall[w], *f)).sum(),
        }
    }

    /// Absolute tolerance weights per state for error and convergence norms.
    pub fn weights(&self, y: &[f64], rtol: f64, atol_scale: f64, w: &mut [f64]) {
        for i in 0..self.n_states {
            w[i] = match self.state_kind[i] {
                StateKind::Pressure => atol_scale * 1.0 + rtol * y[i].abs(),
                StateKind::Enthalpy => atol_scale * 10.0 + rtol * y[i].abs().max(2e5),
                StateKind::MassFlow => atol_scale * 1e-6 + rtol * y[i].abs(),
                StateKind::Speed => atol_scale * 0.1 + rtol * y[i].abs(),
                StateKind::WallTemp => atol_scale * 0.01 + rtol * y[i].abs(),
                StateKind::MixFrac => atol_scale * 1e-5 + rtol * y[i].abs(),
                StateKind::Position => atol_scale * 1e-4 + rtol * y[i].abs(),
            };
        }
    }

    /// Typical magnitudes for finite-difference perturbations.
    pub fn typical(&self, i: usize) -> f64 {
        match self.state_kind[i] {
            StateKind::Pressure => 1e5,
            StateKind::Enthalpy => 1e5,
            StateKind::MassFlow => 1e-2,
            StateKind::Speed => 100.0,
            StateKind::WallTemp => 100.0,
            StateKind::MixFrac => 0.1,
            StateKind::Position => 0.1,
        }
    }

    /// States that determine the thermodynamic state of node i.
    fn node_states(&self, i: usize) -> Vec<usize> {
        match self.nodes[i].kind {
            NodeKind::Volume { sp, .. } => vec![sp, sp + 1],
            NodeKind::Boundary { .. } => vec![],
            NodeKind::ChamberPort { ch, neighbor } => {
                let sp = self.chambers[ch].sp;
                let mut v = vec![sp, sp + 1];
                v.extend(self.node_states(neighbor));
                v
            }
        }
    }

    /// Equations that receive the flows of a branch ending at node i.
    fn node_rows(&self, i: usize) -> Vec<usize> {
        match self.nodes[i].kind {
            NodeKind::Volume { sp, .. } => vec![sp, sp + 1],
            NodeKind::Boundary { .. } => vec![],
            NodeKind::ChamberPort { ch, .. } => {
                let sp = self.chambers[ch].sp;
                vec![sp, sp + 1]
            }
        }
    }

    fn branch_reads(&self, k: usize) -> Vec<usize> {
        let b = &self.branches[k];
        let mut r = self.node_states(b.a);
        r.extend(self.node_states(b.b));
        match &b.kind {
            BranchKind::Pipe { sm, .. } => r.push(*sm),
            BranchKind::Pump { sm, drive, .. } => {
                r.push(*sm);
                r.extend(self.shafts[drive.shaft].state);
            }
            BranchKind::Turbine { drive, .. } => r.extend(self.shafts[drive.shaft].state),
            BranchKind::Restriction {
                ctrl: Ctrl::Actuated { act, .. },
                ..
            } => r.push(self.actuators[*act].state),
            _ => {}
        }
        r
    }

    /// Sparsity: for each state column, the rows of dy that depend on it.
    /// Built from (reads, writes) terms of every equation contribution.
    pub fn sparsity(&self) -> Vec<Vec<usize>> {
        let n = self.n_states;
        let mut terms: Vec<(Vec<usize>, Vec<usize>)> = Vec::new();
        for (i, nd) in self.nodes.iter().enumerate() {
            if let NodeKind::Volume { .. } = nd.kind {
                let mut reads = self.node_states(i);
                // prescribed-temperature wall heat depends on the flows of its branches
                for h in &nd.heat {
                    if let Heat::Wall { branches, .. } = h {
                        for &k in branches {
                            reads.extend(self.branch_reads(k));
                        }
                    }
                }
                terms.push((reads, self.node_rows(i)));
            }
        }
        for (k, b) in self.branches.iter().enumerate() {
            let reads = self.branch_reads(k);
            let mut writes = self.node_rows(b.a);
            writes.extend(self.node_rows(b.b));
            match &b.kind {
                BranchKind::Pipe { sm, .. } => writes.push(*sm),
                BranchKind::Pump { sm, drive, .. } => {
                    writes.push(*sm);
                    writes.extend(self.shafts[drive.shaft].state);
                }
                BranchKind::Turbine { drive, .. } => writes.extend(self.shafts[drive.shaft].state),
                _ => {}
            }
            terms.push((reads, writes));
        }
        for sh in &self.shafts {
            if let Some(s) = sh.state {
                terms.push((vec![s], vec![s]));
            }
        }
        for ch in &self.chambers {
            let mut reads = vec![ch.sp, ch.sp + 1];
            reads.extend(self.node_states(ch.port_f));
            reads.extend(self.node_states(ch.port_o));
            terms.push((reads, vec![ch.sp, ch.sp + 1]));
        }
        for a in &self.actuators {
            let mut reads = vec![a.state];
            reads.extend(self.sense_states(a.sense));
            if let Some(r) = a.sense_ref {
                reads.extend(self.sense_states(r));
            }
            terms.push((reads, vec![a.state]));
        }
        for w in &self.walls {
            let mut reads = vec![w.state];
            reads.extend(self.node_states(w.node));
            for &k in &w.branches {
                reads.extend(self.branch_reads(k));
            }
            if let HotSide::Chamber { ch, .. } = w.hot {
                let c = &self.chambers[ch];
                reads.extend([c.sp, c.sp + 1]);
                reads.extend(self.node_states(c.port_f));
                reads.extend(self.node_states(c.port_o));
                for &(k, _) in self.nodes[c.port_f]
                    .conn
                    .iter()
                    .chain(self.nodes[c.port_o].conn.iter())
                {
                    reads.extend(self.branch_reads(k));
                }
            }
            let mut writes = vec![w.state];
            writes.extend(self.node_rows(w.node));
            terms.push((reads, writes));
        }
        // chamber flows feed the chamber equations through the branches at its ports
        let mut cols: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (reads, writes) in terms {
            for &c in &reads {
                cols[c].extend(writes.iter().copied());
            }
        }
        for c in cols.iter_mut() {
            c.sort_unstable();
            c.dedup();
        }
        cols
    }
}

fn wall_field(w: &WallEval, f: WallF) -> f64 {
    match f {
        WallF::T => w.t,
        WallF::THot => w.t_hot,
        WallF::QHot => w.q_hot,
        WallF::QCool => w.q_cool,
    }
}

/// Coolant-side heat transfer coefficient: Gnielinski / laminar blend, with an
/// optional (T_bulk / T_wall)^n property correction.
fn coolant_htc(
    pr: &Props,
    ev: &Eval,
    branches: &[usize],
    d: f64,
    flow_area: f64,
    t_wall: f64,
    corr: f64,
) -> f64 {
    let m = branches.iter().map(|&k| ev.mdot[k].abs()).sum::<f64>() / branches.len().max(1) as f64;
    let re = m * d / (flow_area * pr.mu);
    let prn = (pr.cp * pr.mu / pr.k).clamp(0.1, 1000.0);
    let mut nu = nusselt(re, prn);
    if corr != 0.0 && t_wall > 0.0 && pr.t > 0.0 {
        nu *= (pr.t / t_wall).clamp(0.05, 20.0).powf(corr);
    }
    nu * pr.k / d
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

impl Network {
    /// (pressure state index, node index) of every tank.
    pub fn tank_states(&self) -> Vec<(usize, usize)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(i, n)| match (&n.kind, n.comp) {
                (NodeKind::Volume { sp, .. }, Some(ci)) if self.comps[ci].kind == "tank" => {
                    Some((*sp, i))
                }
                _ => None,
            })
            .collect()
    }
}
