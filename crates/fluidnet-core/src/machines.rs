//! Turbomachinery and wall-material models.

/// Piecewise-linear curve with linear extrapolation from the end segments.
#[derive(Debug, Clone)]
pub struct Curve {
    pub x: Vec<f64>,
    pub y: Vec<f64>,
}

impl Curve {
    /// Points sorted by x; at least two, distinct x.
    pub fn new(mut pts: Vec<(f64, f64)>) -> Option<Curve> {
        pts.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
        pts.dedup_by(|a, b| (a.0 - b.0).abs() < 1e-12);
        if pts.len() < 2 || pts.iter().any(|p| !p.0.is_finite() || !p.1.is_finite()) {
            return None;
        }
        Some(Curve {
            x: pts.iter().map(|p| p.0).collect(),
            y: pts.iter().map(|p| p.1).collect(),
        })
    }

    pub fn at(&self, x: f64) -> f64 {
        let n = self.x.len();
        let i = self.x.partition_point(|&v| v <= x).clamp(1, n - 1) - 1;
        let (x0, x1, y0, y1) = (self.x[i], self.x[i + 1], self.y[i], self.y[i + 1]);
        y0 + (y1 - y0) * (x - x0) / (x1 - x0)
    }
}

/// Centrifugal pump scaled from one design point by the affinity laws.
///
/// With n = N/N_d and q = Q/Q_d:
///   head   H = H_d (a n|n| + (1 - a) q|q|)          a = shutoff head ratio
///   power  P = P_d (p0 n²|n| + (1 - p0) n|n| q)      p0 = shutoff power ratio
/// so the efficiency follows from the two maps and equals eta_d at the design
/// point. All shaft power ends up in the fluid.
///
/// Measured maps: `head` gives psi = (H/H_d)/n² and `power` gives
/// pi = (P/P_d)/n³ as functions of the flow coefficient phi = q/n (all relative
/// to the design point). They replace the generic curves above n = 0.2 and are
/// blended into them between n = 0.05 and 0.2, so starts from rest keep the
/// generic zero-speed behaviour. `speed` is an efficiency multiplier vs
/// n = N/N_d (Reynolds and clearance effects at part speed); the shaft torque is
/// divided by it, evaluated at n >= 0.2 and faded in between n = 0.05 and 0.2.
#[derive(Debug, Clone)]
pub struct PumpMap {
    pub w_d: f64,
    pub q_d: f64,
    pub h_d: f64,
    pub eta_d: f64,
    pub a: f64,
    pub p0: f64,
    pub head: Option<Curve>,
    pub power: Option<Curve>,
    pub speed: Option<Curve>,
}

pub struct PumpPoint {
    /// pressure rise, Pa
    pub dp: f64,
    /// head, m
    pub head: f64,
    /// shaft power (= power into the fluid), W
    pub power: f64,
    /// shaft torque (load on the shaft), N m
    pub torque: f64,
    pub eff: f64,
}

impl PumpMap {
    pub fn eval(&self, w: f64, mdot: f64, rho: f64) -> PumpPoint {
        let g0 = crate::combustion::G0;
        let n = w / self.w_d;
        let q = mdot / (rho * self.q_d);
        let mut head = self.h_d * (self.a * n * n.abs() + (1.0 - self.a) * q * q.abs());
        let p_d = rho * g0 * self.h_d * self.q_d / self.eta_d;
        // torque = P / w, written without dividing by w
        let mut torque = p_d / self.w_d * n.abs() * (self.p0 * n + (1.0 - self.p0) * q);
        if self.head.is_some() || self.power.is_some() {
            let wgt = smooth01((n - 0.05) / 0.15);
            if wgt > 0.0 {
                let phi = q / n.max(0.05);
                if let Some(c) = &self.head {
                    head = wgt * self.h_d * n * n * c.at(phi) + (1.0 - wgt) * head;
                }
                if let Some(c) = &self.power {
                    torque = wgt * p_d / self.w_d * n * n * c.at(phi) + (1.0 - wgt) * torque;
                }
            }
        }
        if let Some(c) = &self.speed {
            let wgt = smooth01((n.abs() - 0.05) / 0.15);
            if wgt > 0.0 {
                let f = c.at(n.abs().max(0.2)).max(0.1);
                torque *= 1.0 + wgt * (1.0 / f - 1.0);
            }
        }
        let dp = rho * g0 * head;
        let power = torque * w;
        let hyd = dp * mdot / rho;
        let eff = if power.abs() > 1e-9 * p_d {
            hyd / power
        } else {
            0.0
        };
        PumpPoint {
            dp,
            head,
            power,
            torque,
            eff,
        }
    }
}

/// Impulse-type turbine stage. With spouting velocity c0 = sqrt(2 dh_s) and
/// blade speed u, the efficiency is parabolic in x = u/c0 with peak eta_d at
/// x_d; the equivalent torque is linear in blade speed:
///   tau = mdot r (eta_d / x_d) (c0 - u / (2 x_d))
/// giving the stall torque at u = 0 and runaway at x = 2 x_d.
///
/// Measured map: `eff` gives eta/eta_d vs x (through x = 0, eta = 0); the
/// torque is then mdot r c0 eta(x) / (2 x), finite at stall.
#[derive(Debug, Clone)]
pub struct TurbineMap {
    pub r_m: f64,
    pub eta_d: f64,
    pub x_d: f64,
    pub eff: Option<Curve>,
    pub area: Option<AreaMap>,
}

/// Nozzle-area factor vs velocity ratio, one curve per pressure ratio;
/// linear between the pressure ratios, held at the table edges.
#[derive(Debug, Clone)]
pub struct AreaMap {
    pub pr: Vec<f64>,
    pub curves: Vec<Curve>,
}

impl AreaMap {
    /// From [PR, x, factor] triples.
    pub fn new(rows: &[(f64, f64, f64)]) -> Option<AreaMap> {
        let mut prs: Vec<f64> = rows.iter().map(|r| r.0).collect();
        prs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        prs.dedup_by(|a, b| (*a - *b).abs() < 1e-12);
        let mut curves = Vec::new();
        for &pr in &prs {
            let pts: Vec<(f64, f64)> = rows.iter().filter(|r| (r.0 - pr).abs() < 1e-12).map(|r| (r.1, r.2)).collect();
            curves.push(Curve::new(pts)?);
        }
        if curves.is_empty() {
            return None;
        }
        Some(AreaMap { pr: prs, curves })
    }

    pub fn at(&self, x: f64, pr: f64) -> f64 {
        let held = |c: &Curve| c.at(x.clamp(c.x[0], c.x[c.x.len() - 1]));
        let n = self.pr.len();
        if n == 1 || pr <= self.pr[0] {
            return held(&self.curves[0]);
        }
        if pr >= self.pr[n - 1] {
            return held(&self.curves[n - 1]);
        }
        let i = self.pr.partition_point(|&v| v <= pr).clamp(1, n - 1) - 1;
        let f = (pr - self.pr[i]) / (self.pr[i + 1] - self.pr[i]);
        (1.0 - f) * held(&self.curves[i]) + f * held(&self.curves[i + 1])
    }
}

impl TurbineMap {
    /// Nozzle-area factor at speed w, isentropic enthalpy drop dh_s and pressure ratio pr.
    pub fn area_factor(&self, w: f64, dh_s: f64, pr: f64) -> f64 {
        match &self.area {
            None => 1.0,
            Some(a) => {
                let c0 = (2.0 * dh_s.max(0.0)).sqrt();
                let x = if c0 > 1e-9 { (w * self.r_m / c0).abs() } else { 0.0 };
                a.at(x, pr)
            }
        }
    }

    /// (torque, power to shaft, efficiency, velocity ratio)
    pub fn eval(&self, w: f64, mdot: f64, dh_s: f64) -> (f64, f64, f64, f64) {
        let c0 = (2.0 * dh_s.max(0.0)).sqrt();
        let u = w * self.r_m;
        let torque = match &self.eff {
            None => mdot * self.r_m * (self.eta_d / self.x_d) * (c0 - u / (2.0 * self.x_d)),
            Some(c) => {
                if c0 < 1e-9 {
                    0.0
                } else {
                    let x = u / c0;
                    // eta / x, with the slope of the first segment near x = 0
                    let g = if x.abs() < 1e-6 {
                        (c.y[1] - c.y[0]) / (c.x[1] - c.x[0])
                    } else {
                        c.at(x) / x
                    };
                    0.5 * mdot * self.r_m * c0 * self.eta_d * g
                }
            }
        };
        let power = torque * w;
        let ideal = mdot * dh_s.max(0.0);
        let eff = if ideal > 1e-12 { power / ideal } else { 0.0 };
        let x = if c0 > 1e-9 { u / c0 } else { f64::NAN };
        (torque, power, eff, x)
    }
}

/// Wall material: density, conductivity, room-temperature heat capacity and
/// Debye temperature (heat capacity follows the Debye model, which matters for
/// chill-down of cryogenic hardware).
#[derive(Debug, Clone, Copy)]
pub struct Material {
    pub rho: f64,
    pub k: f64,
    pub cp300: f64,
    pub theta_d: f64,
}

pub fn material(name: &str) -> Option<Material> {
    Some(match name {
        "cucrzr" => Material {
            rho: 8890.0,
            k: 320.0,
            cp300: 390.0,
            theta_d: 343.0,
        },
        "copper" => Material {
            rho: 8940.0,
            k: 390.0,
            cp300: 385.0,
            theta_d: 343.0,
        },
        "grcop42" => Material {
            rho: 8790.0,
            k: 300.0,
            cp300: 385.0,
            theta_d: 343.0,
        },
        "nickel" => Material {
            rho: 8890.0,
            k: 70.0,
            cp300: 456.0,
            theta_d: 450.0,
        },
        "stainless" => Material {
            rho: 8000.0,
            k: 16.0,
            cp300: 500.0,
            theta_d: 470.0,
        },
        "inconel718" => Material {
            rho: 8190.0,
            k: 11.4,
            cp300: 435.0,
            theta_d: 450.0,
        },
        "aluminum" => Material {
            rho: 2700.0,
            k: 167.0,
            cp300: 896.0,
            theta_d: 428.0,
        },
        _ => return None,
    })
}

/// Debye heat-capacity function C/(3R) for x = theta_D / T.
fn debye(x: f64) -> f64 {
    if x < 1e-3 {
        return 1.0;
    }
    // 3/x^3 int_0^x t^4 e^t/(e^t-1)^2 dt, Simpson with 64 intervals
    let n = 64;
    let h = x / n as f64;
    let f = |t: f64| -> f64 {
        if t < 1e-8 {
            return 0.0;
        }
        let e = t.exp();
        t.powi(4) * e / ((e - 1.0) * (e - 1.0))
    };
    let mut s = f(0.0) + f(x);
    for i in 1..n {
        s += f(i as f64 * h) * if i % 2 == 1 { 4.0 } else { 2.0 };
    }
    3.0 / x.powi(3) * s * h / 3.0
}

impl Material {
    /// Heat capacity at temperature T (Debye shape scaled to cp at 300 K).
    pub fn cp(&self, t: f64) -> f64 {
        let t = t.max(4.0);
        self.cp300 * debye(self.theta_d / t) / debye(self.theta_d / 300.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pump_design_point() {
        let m = PumpMap {
            w_d: 3000.0,
            q_d: 0.01,
            h_d: 500.0,
            eta_d: 0.7,
            a: 1.15,
            p0: 0.45,
            head: None,
            power: None,
            speed: None,
        };
        let p = m.eval(3000.0, 10.0, 1000.0);
        assert!((p.head - 500.0).abs() < 1e-9);
        assert!((p.eff - 0.7).abs() < 1e-12);
        let s = m.eval(3000.0, 0.0, 1000.0);
        assert!((s.head - 575.0).abs() < 1e-9);
        // affinity: half speed, half flow -> quarter head
        let h = m.eval(1500.0, 5.0, 1000.0);
        assert!((h.head - 125.0).abs() < 1e-9 && (h.eff - 0.7).abs() < 1e-12);
    }

    #[test]
    fn turbine_peak_and_runaway() {
        let t = TurbineMap {
            r_m: 0.05,
            eta_d: 0.6,
            x_d: 0.4,
            eff: None,
            area: None,
        };
        let dh = 2e5;
        let c0 = (2.0 * dh as f64).sqrt();
        let (_, _, eff, _) = t.eval(0.4 * c0 / 0.05, 1.0, dh);
        assert!((eff - 0.6).abs() < 1e-12);
        let (tq, _, _, _) = t.eval(0.8 * c0 / 0.05, 1.0, dh);
        assert!(tq.abs() < 1e-9);
    }

    #[test]
    fn copper_heat_capacity() {
        let cu = material("copper").unwrap();
        assert!((cu.cp(300.0) - 385.0).abs() < 1e-9);
        // tabulated copper: ~ 100 J/kg/K at 50 K, ~ 254 at 100 K
        assert!((cu.cp(50.0) - 100.0).abs() < 20.0, "{}", cu.cp(50.0));
        assert!((cu.cp(100.0) - 254.0).abs() < 20.0, "{}", cu.cp(100.0));
    }

    #[test]
    fn tabulated_maps_reproduce_generic_curves() {
        let a = 1.15;
        let p0 = 0.45;
        let phis: Vec<f64> = (0..=30).map(|i| i as f64 * 0.05).collect();
        let head = Curve::new(phis.iter().map(|&f| (f, a + (1.0 - a) * f * f)).collect()).unwrap();
        let power = Curve::new(phis.iter().map(|&f| (f, p0 + (1.0 - p0) * f)).collect()).unwrap();
        let base = PumpMap { w_d: 3000.0, q_d: 0.01, h_d: 500.0, eta_d: 0.6, a, p0, head: None, power: None, speed: None };
        let tab = PumpMap { head: Some(head), power: Some(power), ..base.clone() };
        for &(w, m) in &[(3000.0, 10.0), (2400.0, 7.0), (3300.0, 13.0)] {
            let (g, t) = (base.eval(w, m, 1000.0), tab.eval(w, m, 1000.0));
            assert!((g.dp - t.dp).abs() < 1e-3 * g.dp.abs(), "{} {}", g.dp, t.dp);
            assert!((g.power - t.power).abs() < 2e-3 * g.power.abs(), "{} {}", g.power, t.power);
        }
        let xd = 0.35;
        let xs: Vec<f64> = (1..=80).map(|i| i as f64 * 0.01).collect();
        let eff = Curve::new(xs.iter().map(|&x| (x, 2.0 * x / xd * (1.0 - x / (2.0 * xd)))).collect());
        let tb = TurbineMap { r_m: 0.07, eta_d: 0.62, x_d: xd, eff: None, area: None };
        let tt = TurbineMap { eff: eff.map(|c| {
            let mut pts: Vec<(f64, f64)> = c.x.iter().copied().zip(c.y.iter().copied()).collect();
            pts.push((0.0, 0.0));
            Curve::new(pts).unwrap()
        }), ..tb.clone() };
        for &w in &[0.0, 1000.0, 3000.0, 5000.0] {
            let (a1, _, _, _) = tb.eval(w, 2.0, 2e5);
            let (a2, _, _, _) = tt.eval(w, 2.0, 2e5);
            // piecewise-linear table: the stall torque uses the first segment's slope
            let tol = if w == 0.0 { 0.02 } else { 1e-3 };
            assert!((a1 - a2).abs() < tol * a1.abs().max(1.0), "w {w}: {a1} {a2}");
        }
    }

    #[test]
    fn speed_correction_divides_torque() {
        let sc = Curve::new(vec![(0.0, 0.0), (0.2, 0.5), (0.5, 0.8), (1.0, 1.0), (1.5, 1.0)]).unwrap();
        let base = PumpMap { w_d: 3000.0, q_d: 0.01, h_d: 500.0, eta_d: 0.6, a: 1.15, p0: 0.45, head: None, power: None, speed: None };
        let cor = PumpMap { speed: Some(sc), ..base.clone() };
        // design speed: unchanged; half speed: efficiency x 0.8, torque / 0.8, head unchanged
        let (b1, c1) = (base.eval(3000.0, 10.0, 1000.0), cor.eval(3000.0, 10.0, 1000.0));
        assert!((b1.torque - c1.torque).abs() < 1e-9 * b1.torque);
        let (b2, c2) = (base.eval(1500.0, 5.0, 1000.0), cor.eval(1500.0, 5.0, 1000.0));
        assert!((c2.torque - b2.torque / 0.8).abs() < 1e-9 * b2.torque);
        assert!((c2.eff - 0.8 * b2.eff).abs() < 1e-9);
        assert_eq!(b2.dp, c2.dp);
        // below 20 % speed the factor is held at its 20 % value and faded out towards 5 %
        let (b3, c3) = (base.eval(300.0, 1.0, 1000.0), cor.eval(300.0, 1.0, 1000.0));
        let r = c3.torque / b3.torque;
        assert!(r > 1.0 && r < 2.0, "{r}");
        let (b4, c4) = (base.eval(100.0, 1.0, 1000.0), cor.eval(100.0, 1.0, 1000.0));
        assert!((c4.torque - b4.torque).abs() < 1e-12 * b4.torque.abs().max(1.0));
    }

    #[test]
    fn turbine_area_map_bilinear_and_held() {
        let rows = [(1.2, 0.1, 1.2), (1.2, 0.5, 1.0), (2.0, 0.1, 1.4), (2.0, 0.5, 1.2)];
        let a = AreaMap::new(&rows).unwrap();
        assert!((a.at(0.3, 1.2) - 1.1).abs() < 1e-12);
        assert!((a.at(0.3, 1.6) - 1.2).abs() < 1e-12);
        assert!((a.at(0.0, 1.0) - 1.2).abs() < 1e-12);
        assert!((a.at(0.9, 3.0) - 1.2).abs() < 1e-12);
    }
}

fn smooth01(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
