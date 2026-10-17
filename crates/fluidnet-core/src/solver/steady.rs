//! Steady state: f(y) = 0 for the same equations used by the transient solver.
//!
//! 1. Damped Newton with backtracking line search on a tolerance-scaled
//!    residual norm (fast, converges from the usual initial guesses).
//! 2. Fallback: pseudo-transient continuation (Kelley & Keyes 1998) —
//!    implicit-Euler steps in pseudo time tau, grown geometrically and by
//!    switched evolution relaxation, so stiff and oscillatory (acoustic) modes
//!    are damped and the iteration becomes Newton's method as tau -> infinity.
//! Dead-end volumes simply keep their initial state.

use crate::error::{Error, Result};
use crate::network::{Eval, Network, StateKind};
use crate::solver::jacobian::JacobianBuilder;
use crate::solver::linsys::LinSys;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct SteadyStats {
    pub iterations: usize,
    pub rhs_evals: usize,
    pub jacobians: usize,
    pub residual_history: Vec<f64>,
    pub converged: bool,
    pub method: String,
}

fn steady_weights(net: &Network, y: &[f64], w: &mut [f64]) {
    for i in 0..net.n_states {
        w[i] = match net.state_kind[i] {
            StateKind::Pressure => 1e-2 + 1e-7 * y[i].abs(),
            StateKind::Enthalpy => 1e-1 + 1e-7 * y[i].abs().max(2e5),
            StateKind::MassFlow => 1e-9 + 1e-7 * y[i].abs(),
        };
    }
}

fn wnorm(v: &[f64], w: &[f64]) -> f64 {
    (v.iter().zip(w).map(|(a, b)| (a / b) * (a / b)).sum::<f64>() / v.len().max(1) as f64).sqrt()
}

fn valid(net: &Network, y: &[f64]) -> bool {
    y.iter()
        .enumerate()
        .all(|(i, v)| v.is_finite() && (net.state_kind[i] != StateKind::Pressure || *v > 0.0))
}

struct Ctx<'a> {
    net: &'a Network,
    t: f64,
    n: usize,
    ev: Eval,
    jb: JacobianBuilder,
    ls: LinSys,
    st: SteadyStats,
    debug: bool,
}

impl<'a> Ctx<'a> {
    fn f(&mut self, y: &[f64], out: &mut [f64]) -> bool {
        self.st.rhs_evals += 1;
        self.net.rhs(self.t, y, out, &mut self.ev).is_ok() && out.iter().all(|v| v.is_finite())
    }
    fn jac(&mut self, y: &[f64], _f: &[f64]) -> Result<()> {
        self.st.rhs_evals += self.jb.build(self.net, self.t, y, &mut self.ev)?;
        self.st.jacobians += 1;
        Ok(())
    }
    /// Solve (I/tau - J) d = f; tau = inf gives the Newton step.
    fn step(&mut self, tau: f64, f: &[f64]) -> Option<Vec<f64>> {
        let a = if tau.is_finite() { 1.0 / tau } else { 0.0 };
        if !self.ls.factor(a, 1.0, &self.jb.vals) {
            return None;
        }
        let mut d = f.to_vec();
        self.ls.solve(&mut d);
        if d.iter().all(|v| v.is_finite()) {
            Some(d)
        } else {
            None
        }
    }
}

/// Newton with line search. Returns Some(y) on convergence.
fn newton(cx: &mut Ctx, y0: &[f64], max_iter: usize) -> Result<Option<Vec<f64>>> {
    let n = cx.n;
    let mut y = y0.to_vec();
    let mut f = vec![0.0; n];
    let mut fnew = vec![0.0; n];
    let mut w = vec![1.0; n];
    if !cx.f(&y, &mut f) {
        return Ok(None);
    }
    steady_weights(cx.net, &y, &mut w);
    let mut phi = wnorm(&f, &w);
    for it in 0..max_iter {
        cx.st.residual_history.push(phi);
        cx.jac(&y, &f)?;
        let d = match cx.step(f64::INFINITY, &f) {
            Some(d) => d,
            None => return Ok(None),
        };
        let mut lam = 1.0;
        let mut ok = false;
        let mut ytry = vec![0.0; n];
        for _ in 0..10 {
            for i in 0..n {
                ytry[i] = y[i] + lam * d[i];
            }
            if valid(cx.net, &ytry) && cx.f(&ytry, &mut fnew) {
                let phin = wnorm(&fnew, &w);
                if phin <= (1.0 - 1e-4 * lam) * phi || phin < 1e-10 {
                    ok = true;
                    let step = lam * wnorm(&d, &w);
                    y.copy_from_slice(&ytry);
                    std::mem::swap(&mut f, &mut fnew);
                    steady_weights(cx.net, &y, &mut w);
                    phi = wnorm(&f, &w);
                    if cx.debug {
                        eprintln!(
                            "newton it {it:3} lambda {lam:.3} step {step:.3e} resid {phi:.3e}"
                        );
                    }
                    if step < 1.0 && lam == 1.0 {
                        cx.st.iterations += it + 1;
                        cx.st.residual_history.push(phi);
                        return Ok(Some(y));
                    }
                    break;
                }
            }
            lam *= 0.5;
        }
        if !ok {
            cx.st.iterations += it + 1;
            return Ok(None);
        }
    }
    cx.st.iterations += max_iter;
    Ok(None)
}

/// Pseudo-transient continuation.
fn ptc(
    cx: &mut Ctx,
    y0: &[f64],
    max_iter: usize,
) -> Result<std::result::Result<Vec<f64>, (f64, usize)>> {
    let n = cx.n;
    let mut y = y0.to_vec();
    let mut f = vec![0.0; n];
    let mut fnew = vec![0.0; n];
    let mut w = vec![1.0; n];
    if !cx.f(&y, &mut f) {
        return Err(Error::Solver(
            "steady state: cannot evaluate the initial state".into(),
        ));
    }
    steady_weights(cx.net, &y, &mut w);
    let mut rn = wnorm(&f, &w);
    let mut tau = 1e-5;
    let mut small = 0;
    for it in 0..max_iter {
        cx.st.residual_history.push(rn);
        cx.jac(&y, &f)?;
        let mut accepted = false;
        for _ in 0..16 {
            let d = match cx.step(tau, &f) {
                Some(d) => d,
                None => {
                    tau *= 0.25;
                    continue;
                }
            };
            let ytry: Vec<f64> = y.iter().zip(&d).map(|(a, b)| a + b).collect();
            if !valid(cx.net, &ytry) || !cx.f(&ytry, &mut fnew) {
                tau *= 0.25;
                continue;
            }
            let rn_new = wnorm(&fnew, &w);
            if rn_new > 20.0 * rn && tau > 1e-8 {
                tau *= 0.25;
                continue;
            }
            let step = wnorm(&d, &w);
            y = ytry;
            std::mem::swap(&mut f, &mut fnew);
            steady_weights(cx.net, &y, &mut w);
            let ser = if rn_new > 0.0 {
                (rn / rn_new).clamp(0.5, 10.0)
            } else {
                10.0
            };
            tau = (tau * ser * 1.5).min(1e14);
            rn = wnorm(&f, &w);
            if cx.debug {
                eprintln!("ptc it {it:3} tau {tau:.3e} step {step:.3e} resid {rn:.3e}");
            }
            small = if step < 1.0 && tau > 1e3 {
                small + 1
            } else {
                0
            };
            accepted = true;
            break;
        }
        if !accepted {
            cx.st.iterations += it + 1;
            return Ok(Err((rn, 0)));
        }
        if small >= 2 {
            cx.st.iterations += it + 1;
            cx.st.residual_history.push(rn);
            return Ok(Ok(y));
        }
        // after the transient phase, hand over to Newton
        if tau > 1e6 && it % 10 == 9 {
            let snapshot = y.clone();
            if let Some(yn) = newton(cx, &snapshot, 30)? {
                return Ok(Ok(yn));
            }
        }
    }
    let mut worst = 0;
    for i in 0..n {
        if (f[i] / w[i]).abs() > (f[worst] / w[worst]).abs() {
            worst = i;
        }
    }
    cx.st.iterations += max_iter;
    Ok(Err((rn, worst)))
}

/// March the transient (inputs frozen at t) until derivatives vanish.
fn march(net: &Network, y0: &[f64], t: f64) -> Option<Vec<f64>> {
    use crate::solver::transient::{integrate, TransientOptions};
    let opts = TransientOptions {
        t0: 0.0,
        t_end: 1e7,
        rtol: 1e-4,
        atol_scale: 1.0,
        max_step: 1e6,
        max_steps: 200_000,
        out_times: vec![],
        freeze_time: Some(t),
        steady_tol: Some(1e-3),
    };
    let mut last: Option<Vec<f64>> = None;
    let mut sink = |_t: f64, y: &[f64]| {
        last = Some(y.to_vec());
        Ok(())
    };
    match integrate(net, y0, &opts, &mut sink, &mut |_| true) {
        Ok(_) => last,
        Err(_) => None,
    }
}

pub fn solve(
    net: &Network,
    y0: &[f64],
    t: f64,
    max_iter: usize,
) -> Result<(Vec<f64>, SteadyStats)> {
    let n = net.n_states;
    if n == 0 {
        return Ok((
            vec![],
            SteadyStats {
                converged: true,
                method: "algebraic".into(),
                ..Default::default()
            },
        ));
    }
    let (jb, ls) = JacobianBuilder::new(net);
    let mut cx = Ctx {
        net,
        t,
        n,
        ev: Eval::new(net),
        jb,
        ls,
        st: SteadyStats::default(),
        debug: cfg!(not(target_arch = "wasm32")) && std::env::var("FLUIDNET_DEBUG").is_ok(),
    };
    if let Some(y) = newton(&mut cx, y0, 40)? {
        cx.st.converged = true;
        cx.st.method = "newton".into();
        return Ok((y, cx.st));
    }
    let ptc_res = ptc(&mut cx, y0, max_iter.min(150))?;
    if let Ok(y) = ptc_res {
        cx.st.converged = true;
        cx.st.method = "pseudo-transient".into();
        return Ok((y, cx.st));
    }
    // time-march with frozen inputs using the transient integrator, then polish
    if let Some(ym) = march(net, y0, t) {
        if let Some(y) = newton(&mut cx, &ym, 40)? {
            cx.st.converged = true;
            cx.st.method = "time-marching + newton".into();
            return Ok((y, cx.st));
        }
    }
    match ptc_res {
        Ok(_) => unreachable!(),
        Err((rn, worst)) => Err(Error::Solver(format!(
            "steady state did not converge (residual {rn:.3e}, largest in {}). The system may have no steady state (e.g. a closed tank that keeps filling) or may oscillate.",
            net.state_names.get(worst).map(|s| s.as_str()).unwrap_or("?")
        ))),
    }
}
