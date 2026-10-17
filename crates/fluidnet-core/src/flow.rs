//! Flow correlations: restriction mass flux (liquid, flashing, two-phase, gas),
//! pipe friction and convective heat transfer.

use crate::props::{FluidTable, Props};

/// Which mass-flux model a restriction uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestrictionModel {
    /// Choose from the upstream state: subcooled liquid -> omega (subcooled
    /// extension, reduces to the cavitating limit), two-phase -> omega (HEM),
    /// gas/supercritical -> real-gas isentropic nozzle with kappa = 1/omega.
    Auto,
    /// Single-phase incompressible, sqrt(2 rho dP), no cavitation limit.
    Incompressible,
}

impl RestrictionModel {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "auto" => Some(Self::Auto),
            "incompressible" | "spi" => Some(Self::Incompressible),
            _ => None,
        }
    }
}

/// Result of a restriction flux evaluation.
#[derive(Debug, Clone, Copy, Default)]
pub struct Flux {
    /// Mass flux through the effective area, kg/m2/s (>= 0).
    pub g: f64,
    pub choked: bool,
}

/// Linear-near-zero regularisation factor: sqrt(|dp|) / (dp^2 + eps^2)^(1/4).
/// Multiplying a sqrt(dp) law by this gives a smooth, finite-slope law at dp = 0.
#[inline]
fn reg_factor(dp: f64, eps: f64) -> f64 {
    let d = dp.abs();
    if d == 0.0 {
        return 0.0;
    }
    d.sqrt() / (d * d + eps * eps).powf(0.25)
}

/// Omega-model mass flux, normalised by sqrt(P0/v0), for inlet saturation
/// ratio eta_s = Psat/P0 (1 for a saturated / two-phase inlet) and throat
/// pressure ratio eta <= eta_s.
#[inline]
fn omega_g(omega: f64, eta_s: f64, eta: f64) -> f64 {
    let d = eta_s - eta;
    // (1 - eta_s) + w eta_s ln(eta_s/eta) - (w - 1)(eta_s - eta)
    let lnr = if d < 1e-6 * eta_s {
        // series of ln(eta_s/eta) for eta -> eta_s
        let u = d / eta_s;
        u + 0.5 * u * u + u * u * u / 3.0
    } else {
        (eta_s / eta).ln()
    };
    let num = (1.0 - eta_s) + omega * eta_s * lnr - (omega - 1.0) * d;
    let den = omega * (eta_s / eta - 1.0) + 1.0;
    (2.0 * num.max(0.0)).sqrt() / den
}

/// Maximise omega_g over eta in [eta_lo, eta_s]; returns (eta*, g*).
fn omega_choke(omega: f64, eta_s: f64) -> (f64, f64) {
    let (mut a, mut b) = (1e-4_f64.min(eta_s * 0.5), eta_s);
    const R: f64 = 0.618_033_988_749_895;
    let mut c = b - R * (b - a);
    let mut d = a + R * (b - a);
    let mut fc = omega_g(omega, eta_s, c);
    let mut fd = omega_g(omega, eta_s, d);
    for _ in 0..40 {
        if fc > fd {
            b = d;
            d = c;
            fd = fc;
            c = b - R * (b - a);
            fc = omega_g(omega, eta_s, c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + R * (b - a);
            fd = omega_g(omega, eta_s, d);
        }
        if (b - a) < 1e-7 {
            break;
        }
    }
    let e = 0.5 * (a + b);
    let g = omega_g(omega, eta_s, e);
    // the end point eta_s may be the maximum (strongly subcooled: cavitation limit)
    let g_end = omega_g(omega, eta_s, eta_s * (1.0 - 1e-9));
    if g_end >= g {
        (eta_s, g_end)
    } else {
        (e, g)
    }
}

/// Real-gas isentropic nozzle flux with isentropic exponent kappa, normalised
/// by sqrt(P0 rho0). Returns (g, choked).
#[inline]
fn gas_g(kappa: f64, eta: f64) -> (f64, bool) {
    let k = kappa.clamp(1.001, 3.0);
    let eta_c = (2.0 / (k + 1.0)).powf(k / (k - 1.0));
    let (e, choked) = if eta < eta_c {
        (eta_c, true)
    } else {
        (eta, false)
    };
    let term = e.powf(2.0 / k) - e.powf((k + 1.0) / k);
    ((2.0 * k / (k - 1.0) * term.max(0.0)).sqrt(), choked)
}

#[inline]
fn smoothstep(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Mass flux from upstream state `up` to downstream pressure `p_dn` (<= up.p),
/// with regularisation pressure `eps` (Pa).
pub fn restriction_flux(
    fluid: &FluidTable,
    up: &Props,
    p_dn: f64,
    eps: f64,
    model: RestrictionModel,
) -> Flux {
    let p0 = up.p;
    let dp = (p0 - p_dn).max(0.0);
    let rf = reg_factor(dp, eps);
    if dp <= 0.0 {
        return Flux::default();
    }
    if model == RestrictionModel::Incompressible {
        return Flux {
            g: (2.0 * up.rho).sqrt() * dp.sqrt() * rf,
            choked: false,
        };
    }
    let eta2 = (p_dn / p0).clamp(1e-6, 1.0);
    let sq = (p0 * up.rho).sqrt(); // sqrt(P0/v0)
    let xq = if up.x.is_nan() {
        fluid.quality(up.p, up.h)
    } else {
        up.x
    };
    let up = &Props { x: xq, ..*up };

    let subcooled_like =
        up.t < fluid.tcrit && (up.x < 0.0 || (up.x.is_nan() && up.p >= fluid.pcrit));
    if subcooled_like {
        // liquid: no flashing if downstream stays above Psat
        let psat = fluid.psat_t(up.t).unwrap_or(0.0).min(p0);
        let eta_s = psat / p0;
        if eta2 >= eta_s || psat <= 0.0 {
            return Flux {
                g: (2.0 * up.rho).sqrt() * dp.sqrt() * rf,
                choked: false,
            };
        }
        // two-point omega at saturation (API 520 Annex C): isentropic flash
        // of saturated liquid from Psat to 0.9 Psat
        let w_s = match (fluid.sat_p(psat), fluid.sat_sl(psat)) {
            (Some((_, hl, _, vl, _)), Some(sl)) => {
                let p9 = 0.9 * psat;
                match fluid.h_ps_near(p9, sl, hl - vl * 0.1 * psat) {
                    Some(h9) => (9.0 * (fluid.props_ph(p9, h9).v / vl - 1.0)).max(1e-6),
                    None => 1.0,
                }
            }
            _ => 1.0,
        };
        let (eta_c, gc) = omega_choke(w_s, eta_s);
        let (g, choked) = if eta2 <= eta_c {
            (gc, true)
        } else {
            (omega_g(w_s, eta_s, eta2), false)
        };
        return Flux {
            g: g * sq * rf,
            choked,
        };
    }

    let w = up.omega();
    // two-phase inlet: two-point omega from the isentrope at 0.9 P0 (API 520)
    let w2p = || -> f64 {
        let p9 = 0.9 * p0;
        match fluid.h_ps_near(p9, up.s, up.h - up.v * 0.1 * p0) {
            Some(h9) => (9.0 * (fluid.props_ph(p9, h9).v / up.v - 1.0)).max(1e-6),
            None => w,
        }
    };
    let two_phase_g = || -> (f64, bool) {
        let w = w2p();
        let (eta_c, gc) = omega_choke(w, 1.0);
        if eta2 <= eta_c {
            (gc, true)
        } else {
            (omega_g(w, 1.0, eta2), false)
        }
    };
    let x = up.x;
    let (g, choked) = if x.is_finite() && x <= 0.97 {
        two_phase_g()
    } else if x.is_finite() && x < 1.03 {
        let (g1, c1) = two_phase_g();
        let (g2, c2) = gas_g(1.0 / w, eta2);
        let b = smoothstep((x - 0.97) / 0.06);
        ((1.0 - b) * g1 + b * g2, if b < 0.5 { c1 } else { c2 })
    } else {
        gas_g(1.0 / w, eta2)
    };
    Flux {
        g: g * sq * rf,
        choked,
    }
}

/// Churchill (1977) Darcy friction factor, valid and smooth from laminar to
/// fully rough turbulent flow.
pub fn churchill_f(re: f64, rel_rough: f64) -> f64 {
    let re = re.max(1e-3);
    let a = (2.457 * (1.0 / ((7.0 / re).powf(0.9) + 0.27 * rel_rough)).ln()).powi(16);
    let b = (37530.0 / re).powi(16);
    8.0 * ((8.0 / re).powi(12) + (a + b).powf(-1.5)).powf(1.0 / 12.0)
}

/// Frictional pressure drop (signed with mdot) over length `l`, plus K losses.
#[inline]
pub fn pipe_dp(mdot: f64, rho: f64, mu: f64, d: f64, area: f64, l: f64, rough: f64, k: f64) -> f64 {
    let re = mdot.abs() * d / (area * mu);
    let f = churchill_f(re, rough / d);
    (f * l / d + k) * mdot * mdot.abs() / (2.0 * rho * area * area)
}

/// Single-phase forced convection Nusselt number: Gnielinski turbulent with a
/// smooth blend to the laminar constant-wall-temperature value.
pub fn nusselt(re: f64, pr: f64) -> f64 {
    let nu_lam: f64 = 3.66;
    let nu_t = if re > 1000.0 {
        let f = (0.79 * re.ln() - 1.64).powi(-2);
        (f / 8.0) * (re - 1000.0) * pr
            / (1.0 + 12.7 * (f / 8.0).sqrt() * (pr.powf(2.0 / 3.0) - 1.0))
    } else {
        0.0
    };
    (nu_lam.powi(10) + nu_t.max(0.0).powi(10)).powf(0.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omega_ideal_gas_limit() {
        // saturated-inlet omega model for omega = 1/1.4: Leung's eta_c ~ 0.562
        let (eta_c, _) = omega_choke(1.0 / 1.4, 1.0);
        assert!((eta_c - 0.562).abs() < 0.003, "eta_c = {eta_c}");
    }

    #[test]
    fn gas_choke_ideal() {
        let (g, choked) = gas_g(1.4, 0.1);
        assert!(choked);
        assert!((g - 0.6847).abs() < 1e-3, "g = {g}");
    }

    #[test]
    fn churchill_limits() {
        assert!((churchill_f(100.0, 0.0) - 0.64).abs() < 0.01);
        // Blasius at Re=1e5 smooth: 0.316/Re^0.25 = 0.01777
        assert!((churchill_f(1e5, 0.0) - 0.018).abs() < 0.001);
    }
}
