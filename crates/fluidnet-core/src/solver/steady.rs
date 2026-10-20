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
            StateKind::Speed => 1e-3 + 1e-7 * y[i].abs(),
            StateKind::WallTemp => 1e-4 + 1e-7 * y[i].abs(),
            StateKind::MixFrac => 1e-8,
            StateKind::Position => 1e-7,
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
            if rn < 1e-3 {
                cx.st.method = "pseudo-transient (quasi-steady)".into();
                return Ok(Ok(y));
            }
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
    // rates this small (well below 1e-3 of the steady weights per second) are
    // quasi-steady: e.g. a tank draining through a leak over days
    if rn < 1e-3 {
        cx.st.method = "pseudo-transient (quasi-steady)".into();
        return Ok(Ok(y));
    }
    Ok(Err((rn, worst)))
}

/// March the transient (inputs frozen at t) until derivatives vanish.
fn march(net: &Network, y0: &[f64], t: f64) -> Option<Vec<f64>> {
    use crate::solver::transient::{integrate, TransientOptions};
    let opts = TransientOptions {
        t0: 0.0,
        // march toward equilibrium; Newton polishes the result
        t_end: 1e9,
        rtol: 1e-4,
        atol_scale: 1.0,
        max_step: 1e8,
        max_steps: 5_000,
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
        Err(e) => {
            if cfg!(not(target_arch = "wasm32")) && std::env::var("FLUIDNET_DEBUG").is_ok() {
                eprintln!("steady: time-marching failed: {e}");
            }
            None
        }
    }
}

/// Ordinary transient: first the actual input history from 0 to t (for a
/// steady state at the end of a start sequence), then with inputs held at t,
/// with limited step size, until the rates are small or 30 s have passed.
fn settle(net: &Network, y0: &[f64], t: f64) -> Option<Vec<f64>> {
    use crate::solver::transient::{integrate, TransientOptions};
    let debug = cfg!(not(target_arch = "wasm32")) && std::env::var("FLUIDNET_DEBUG").is_ok();
    let mut y = y0.to_vec();
    if t > 0.0 {
        let opts = TransientOptions {
            t0: 0.0,
            t_end: t,
            rtol: 1e-4,
            atol_scale: 1.0,
            max_step: (t / 20.0).min(0.25),
            max_steps: 50_000,
            out_times: vec![t],
            freeze_time: None,
            steady_tol: None,
        };
        let mut last: Option<Vec<f64>> = None;
        let mut sink = |_t: f64, yy: &[f64]| {
            last = Some(yy.to_vec());
            Ok(())
        };
        let r = integrate(net, y0, &opts, &mut sink, &mut |_| true);
        if debug {
            eprintln!("steady: transient 0..{t} s: {:?}", r.as_ref().err());
        }
        match (r, last) {
            (Ok(_), Some(yy)) => y = yy,
            _ => {}
        }
    }
    let opts = TransientOptions {
        t0: 0.0,
        t_end: 30.0,
        rtol: 1e-4,
        atol_scale: 1.0,
        max_step: 0.25,
        max_steps: 50_000,
        // outputs along the way keep the latest state when a small limit cycle
        // (e.g. a feed-line oscillation) never meets the steady tolerance; the
        // Newton polish below starts from it
        out_times: (1..=60).map(|i| 0.5 * i as f64).collect(),
        freeze_time: Some(t),
        steady_tol: Some(1e-3),
    };
    let mut last: Option<Vec<f64>> = None;
    let mut sink = |_t: f64, yy: &[f64]| {
        last = Some(yy.to_vec());
        Ok(())
    };
    let mut t_last = 0.0;
    let r = integrate(net, &y, &opts, &mut sink, &mut |tt| {
        t_last = tt;
        true
    });
    if debug {
        eprintln!("steady: transient settling ended at t = {t_last:.3} s: {:?}", r.as_ref().err());
    }
    last
}

/// Try the direct methods (Newton, then pseudo-transient continuation, then
/// time-marching) on one network.
fn solve_direct(
    net: &Network,
    y0: &[f64],
    t: f64,
    max_iter: usize,
    st_out: &mut SteadyStats,
) -> Result<std::result::Result<Vec<f64>, (f64, usize)>> {
    let n = net.n_states;
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
    fn finish(cx: &mut Ctx, st_out: &mut SteadyStats, method: &str) {
        st_out.iterations += cx.st.iterations;
        st_out.rhs_evals += cx.st.rhs_evals;
        st_out.jacobians += cx.st.jacobians;
        st_out.residual_history.append(&mut cx.st.residual_history);
        cx.st.iterations = 0;
        cx.st.rhs_evals = 0;
        cx.st.jacobians = 0;
        if !method.is_empty() {
            st_out.method = if st_out.method.is_empty() {
                method.to_string()
            } else {
                format!("{} + {method}", st_out.method)
            };
        }
    }
    if let Some(y) = newton(&mut cx, y0, 40)? {
        finish(&mut cx, st_out, "newton");
        return Ok(Ok(y));
    }
    let ptc_res = ptc(&mut cx, y0, max_iter.min(150))?;
    if let Ok(y) = ptc_res {
        let m = if cx.st.method.is_empty() {
            "pseudo-transient".to_string()
        } else {
            cx.st.method.clone()
        };
        finish(&mut cx, st_out, &m);
        return Ok(Ok(y));
    }
    if let Some(ym) = march(net, y0, t) {
        if let Some(y) = newton(&mut cx, &ym, 40)? {
            finish(&mut cx, st_out, "time-marching + newton");
            return Ok(Ok(y));
        }
    }
    finish(&mut cx, st_out, "");
    Ok(ptc_res)
}

pub fn solve(
    net: &Network,
    y0: &[f64],
    t: f64,
    max_iter: usize,
) -> Result<(Vec<f64>, SteadyStats)> {
    if net.n_states == 0 {
        return Ok((
            vec![],
            SteadyStats {
                converged: true,
                method: "algebraic".into(),
                ..Default::default()
            },
        ));
    }
    // cold start: flow states left at zero get a guess from the pump design flows
    let y_user = y0;
    let mut y_guess = y0.to_vec();
    for (i, v) in net.flow_guess(t, y0) {
        if y_guess[i] == 0.0 {
            y_guess[i] = v;
        }
    }
    let y0 = &y_guess[..];
    // latching actuators settle at their commanded position in a steady state;
    // the settling transient below replays the input history and keeps them
    let latched = net;
    let unlatched;
    let net = if net.latch_active && net.actuators.iter().any(|a| a.latch) {
        let mut n = net.clone();
        n.latch_active = false;
        unlatched = n;
        &unlatched
    } else {
        net
    };
    let mut st = SteadyStats::default();
    let free: Vec<usize> = (0..net.shafts.len())
        .filter(|&s| net.shafts[s].state.is_some() && net.shafts[s].w_guess > 0.0)
        .collect();
    // turbomachinery: hold the shafts at their speed guesses first, then release
    if !free.is_empty() {
        let mut held = net.clone();
        let mut yh = y0.to_vec();
        for &s in &free {
            held.shafts[s].hold = Some(net.shafts[s].w_guess);
            yh[net.shafts[s].state.unwrap()] = net.shafts[s].w_guess;
        }
        if let Ok(Ok(y1)) = solve_direct(&held, &yh, t, max_iter, &mut st) {
            st.method = format!("shaft continuation ({})", st.method);
            if let Ok(y) = solve_direct(net, &y1, t, max_iter, &mut st)? {
                st.converged = true;
                return Ok((y, st));
            }
        }
        st.method.clear();
    }
    let direct = solve_direct(net, y0, t, max_iter, &mut st)?;
    let direct = match direct {
        Ok(y) => Ok(y),
        Err(e) => {
            // last resort: settle with an ordinary transient (bounded steps, free
            // shafts), then polish. Large engine models started from a crude
            // guess can surge or cavitate when marched with huge steps.
            match settle(latched, y_user, t) {
                Some(ys) => {
                    let mut st2 = SteadyStats::default();
                    match solve_direct(net, &ys, t, max_iter, &mut st2)? {
                        Ok(y) => {
                            st.iterations += st2.iterations;
                            st.rhs_evals += st2.rhs_evals;
                            st.jacobians += st2.jacobians;
                            st.method = format!("transient settling + {}", st2.method);
                            Ok(y)
                        }
                        Err(e2) => Err(e2),
                    }
                }
                None => Err(e),
            }
        }
    };
    match direct {
        Ok(y) => {
            st.converged = true;
            Ok((y, st))
        }
        Err((rn, worst)) => Err(Error::Solver(format!(
            "steady state did not converge (residual {rn:.3e}, largest in {}). The system may have no steady state (e.g. a closed tank that keeps filling) or may oscillate.",
            net.state_names.get(worst).map(|s| s.as_str()).unwrap_or("?")
        ))),
    }
}
