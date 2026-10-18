//! Combustion gas properties and thrust-chamber relations.
//!
//! Equilibrium tables (Cantera, `tools/gen_combustion_tables.py`) give c*, Tc,
//! molar mass and isentropic exponents vs O/F and Pc for reference injection
//! states, plus first-order sensitivities to the reactant mixture enthalpy.

use std::sync::{Arc, OnceLock};

use serde::Deserialize;

pub const RU: f64 = 8.314_462_618;
pub const G0: f64 = 9.806_65;

#[derive(Deserialize)]
struct RawTable {
    fuel: String,
    oxidizer: String,
    fuel_ref: String,
    ox_ref: String,
    h_ref_fuel: f64,
    h_ref_ox: f64,
    of: Vec<f64>,
    pc: Vec<f64>,
    cstar: Vec<Vec<f64>>,
    #[serde(rename = "Tc")]
    tc: Vec<Vec<f64>>,
    #[serde(rename = "M")]
    m: Vec<Vec<f64>>,
    gamma_s: Vec<Vec<f64>>,
    gamma_f: Vec<Vec<f64>>,
    dcstar_dh: Vec<Vec<f64>>,
    #[serde(rename = "dTc_dh")]
    dtc_dh: Vec<Vec<f64>>,
    #[serde(default)]
    dh_wall: Vec<Vec<f64>>,
    #[serde(default)]
    cp_wall: Vec<Vec<f64>>,
}

#[derive(Deserialize)]
struct RawFile {
    tables: Vec<RawTable>,
}

/// Equilibrium combustion table for one propellant pair.
#[derive(Debug)]
pub struct CombTable {
    pub fuel: String,
    pub oxidizer: String,
    pub fuel_ref: String,
    pub ox_ref: String,
    /// Reactant reference enthalpies in the fluid-table (CoolProp) convention, J/kg.
    pub h_ref_fuel: f64,
    pub h_ref_ox: f64,
    of: Vec<f64>,
    lpc: Vec<f64>,
    // row-major [pc][of]
    f: [Vec<f64>; 9],
    /// Table carries the equilibrium wall enthalpy potential.
    pub has_wall: bool,
}

/// Wall temperature at which `dh_wall` is tabulated, K.
pub const T_WALL_REF: f64 = 600.0;

/// Interpolated combustion state.
#[derive(Debug, Clone, Copy, Default)]
pub struct CombState {
    pub cstar: f64,
    pub tc: f64,
    pub molar_mass: f64,
    pub gamma_s: f64,
    pub gamma_f: f64,
    pub dcstar_dh: f64,
    pub dtc_dh: f64,
    /// Chamber enthalpy minus the enthalpy of the same gas in equilibrium at
    /// T_WALL_REF (hot-gas heat transfer driving potential), J/kg.
    pub dh_wall: f64,
    /// Equilibrium specific heat of the gas near the wall, J/kg/K.
    pub cp_wall: f64,
}

fn bracket(g: &[f64], x: f64) -> (usize, f64) {
    let n = g.len();
    let i = g.partition_point(|&v| v <= x).saturating_sub(1).min(n - 2);
    let t = ((x - g[i]) / (g[i + 1] - g[i])).clamp(0.0, 1.0);
    (i, t)
}

impl CombTable {
    pub fn of_range(&self) -> (f64, f64) {
        (self.of[0], *self.of.last().unwrap())
    }

    /// Bilinear in (O/F, log Pc), clamped to the table.
    pub fn eval(&self, of: f64, pc: f64) -> CombState {
        let no = self.of.len();
        let (j, tj) = bracket(&self.of, of);
        let (i, ti) = bracket(&self.lpc, pc.max(1.0).log10());
        let at = |k: usize| -> f64 {
            let a = &self.f[k];
            let v00 = a[i * no + j];
            let v01 = a[i * no + j + 1];
            let v10 = a[(i + 1) * no + j];
            let v11 = a[(i + 1) * no + j + 1];
            (1.0 - ti) * ((1.0 - tj) * v00 + tj * v01) + ti * ((1.0 - tj) * v10 + tj * v11)
        };
        CombState {
            cstar: at(0),
            tc: at(1),
            molar_mass: at(2),
            gamma_s: at(3),
            gamma_f: at(4),
            dcstar_dh: at(5),
            dtc_dh: at(6),
            dh_wall: if self.has_wall { at(7) } else { 0.0 },
            cp_wall: if self.has_wall { at(8) } else { 0.0 },
        }
    }
}

static TABLES: OnceLock<Vec<Arc<CombTable>>> = OnceLock::new();

pub fn tables() -> &'static [Arc<CombTable>] {
    TABLES.get_or_init(|| {
        let raw: RawFile = serde_json::from_str(include_str!("../data/combustion.json"))
            .expect("embedded combustion tables");
        raw.tables
            .into_iter()
            .map(|t| {
                let flat = |v: Vec<Vec<f64>>| -> Vec<f64> { v.into_iter().flatten().collect() };
                let has_wall = !t.dh_wall.is_empty() && !t.cp_wall.is_empty();
                Arc::new(CombTable {
                    has_wall,
                    lpc: t.pc.iter().map(|p| p.log10()).collect(),
                    of: t.of,
                    f: [
                        flat(t.cstar),
                        flat(t.tc),
                        flat(t.m),
                        flat(t.gamma_s),
                        flat(t.gamma_f),
                        flat(t.dcstar_dh),
                        flat(t.dtc_dh),
                        flat(t.dh_wall),
                        flat(t.cp_wall),
                    ],
                    fuel: t.fuel,
                    oxidizer: t.oxidizer,
                    fuel_ref: t.fuel_ref,
                    ox_ref: t.ox_ref,
                    h_ref_fuel: t.h_ref_fuel,
                    h_ref_ox: t.h_ref_ox,
                })
            })
            .collect()
    })
}

pub fn find(fuel: &str, ox: &str) -> Option<Arc<CombTable>> {
    tables()
        .iter()
        .find(|t| t.fuel.eq_ignore_ascii_case(fuel) && t.oxidizer.eq_ignore_ascii_case(ox))
        .cloned()
}

pub fn pairs() -> Vec<(String, String)> {
    tables()
        .iter()
        .map(|t| (t.fuel.clone(), t.oxidizer.clone()))
        .collect()
}

/// Vandenkerckhove function Gamma(gamma).
#[inline]
pub fn vdk(g: f64) -> f64 {
    g.sqrt() * (2.0 / (g + 1.0)).powf((g + 1.0) / (2.0 * (g - 1.0)))
}

/// Supersonic exit Mach number for area ratio eps (>= 1).
pub fn exit_mach(g: f64, eps: f64) -> f64 {
    if eps <= 1.0 {
        return 1.0;
    }
    let e = (g + 1.0) / (2.0 * (g - 1.0));
    let ar = |m: f64| (1.0 / m) * ((2.0 / (g + 1.0)) * (1.0 + 0.5 * (g - 1.0) * m * m)).powf(e);
    // Newton on ln(area ratio) from a supersonic start
    let mut m = 1.0 + (eps - 1.0).sqrt().max(0.2) * 1.2;
    for _ in 0..50 {
        let f = ar(m).ln() - eps.ln();
        let dm = 1e-6 * m;
        let d = (ar(m + dm).ln() - ar(m - dm).ln()) / (2.0 * dm);
        let step = f / d;
        m = (m - step).max(1.0 + 1e-9);
        if step.abs() < 1e-12 * m {
            break;
        }
    }
    m
}

/// Ideal thrust coefficient for isentropic exponent g, area ratio eps and
/// ambient-to-chamber pressure ratio.
pub fn thrust_coefficient(g: f64, eps: f64, pa_pc: f64) -> f64 {
    let me = exit_mach(g, eps);
    let pe_pc = (1.0 + 0.5 * (g - 1.0) * me * me).powf(-g / (g - 1.0));
    let mom = vdk(g)
        * (2.0 * g / (g - 1.0) * (1.0 - pe_pc.powf((g - 1.0) / g)))
            .max(0.0)
            .sqrt();
    mom + eps.max(1.0) * (pe_pc - pa_pc)
}

/// Bartz hot-gas heat transfer coefficient (SI form), with the wall-temperature
/// correction sigma evaluated at low Mach number (chamber barrel and
/// convergent section).
#[allow(clippy::too_many_arguments)]
pub fn bartz_htc(
    dt: f64,
    pc: f64,
    cstar: f64,
    gamma_f: f64,
    molar_mass: f64,
    tc: f64,
    t_wall: f64,
    contraction: f64,
) -> f64 {
    if !(pc > 0.0 && cstar > 0.0 && tc > 0.0) {
        return 0.0;
    }
    let r = RU / (molar_mass * 1e-3);
    let cp = gamma_f * r / (gamma_f - 1.0);
    let pr = 4.0 * gamma_f / (9.0 * gamma_f - 5.0);
    // Bartz viscosity estimate 46.6e-10 M^0.5 T^0.6 (lb/in s, T in R), converted to Pa s
    let mu = 46.6e-10 * molar_mass.sqrt() * (1.8 * tc).powf(0.6) * 17.858;
    let sigma = (0.5 * (t_wall / tc).clamp(0.02, 1.0) + 0.5).powf(-0.68);
    0.026 / dt.powf(0.2) * mu.powf(0.2) * cp / pr.powf(0.6)
        * (pc / cstar).powf(0.8)
        * (1.0 / contraction.max(1.0)).powf(0.9)
        * sigma
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_load_and_match_cea() {
        let t = find("H2", "O2").unwrap();
        let s = t.eval(6.0, 50e5);
        assert!((s.cstar - 2328.7).abs() < 2.0, "{}", s.cstar);
        assert!((s.tc - 3508.0).abs() < 5.0);
        assert!(find("CH4", "O2").is_some() && find("C2H5OH", "O2").is_some());
    }

    #[test]
    fn nozzle_relations() {
        assert!((vdk(1.4) - 0.6847).abs() < 1e-4);
        // gamma 1.2, eps 10: Me = 3.2783, vacuum CF = 1.7423 (independent root solve)
        let me = exit_mach(1.2, 10.0);
        assert!((me - 3.27834).abs() < 1e-4, "{me}");
        let cf = thrust_coefficient(1.2, 10.0, 0.0);
        assert!((cf - 1.74231).abs() < 1e-4, "{cf}");
    }

    #[test]
    fn wall_enthalpy_potential_follows_mixture_ratio() {
        // LOX/LH2 near the RL10 point: the equilibrium potential (recombination at
        // the wall) falls 4 % from O/F 5.63 to 4.67 while frozen cp (Tc - Tw) rises
        let t = find("H2", "O2").unwrap();
        assert!(t.has_wall);
        let (a, b) = (t.eval(5.63, 32.6e5), t.eval(4.67, 32.6e5));
        let r = b.dh_wall / a.dh_wall;
        assert!((r - 0.959).abs() < 0.01, "{r}");
        let cp = |s: &CombState| s.gamma_f * RU / (s.molar_mass * 1e-3) / (s.gamma_f - 1.0);
        let rf = cp(&b) * (b.tc - T_WALL_REF) / (cp(&a) * (a.tc - T_WALL_REF));
        assert!(rf > 1.0, "{rf}");
        assert!(a.cp_wall > 1000.0 && a.cp_wall < 5000.0);
    }
}
