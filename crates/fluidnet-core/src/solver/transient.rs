//! TR-BDF2 (Bank et al. 1985; Hosea & Shampine 1996): L-stable, second-order,
//! one-step implicit Runge-Kutta. Both implicit stages share the iteration
//! matrix (I - d h J), d = gamma/2, gamma = 2 - sqrt(2). Local error uses the
//! embedded third-order weights and is filtered through the same matrix
//! (Shampine), so stiff components do not inflate it. Steps land exactly on
//! schedule breakpoints. The iteration matrix is refactored only when h changes
//! materially or the Jacobian is refreshed.

use crate::error::{Error, Result};
use crate::network::{Eval, Network, StateKind};
use crate::solver::jacobian::JacobianBuilder;
use crate::solver::linsys::LinSys;

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct Stats {
    pub steps: usize,
    pub rejected: usize,
    pub newton_failures: usize,
    pub rhs_evals: usize,
    pub jacobians: usize,
    pub factorizations: usize,
    pub n_states: usize,
    pub jacobian_colors: usize,
    pub nonzeros: usize,
    pub sparse: bool,
    pub h_min_used: f64,
    pub h_max_used: f64,
}

pub struct TransientOptions {
    pub t0: f64,
    pub t_end: f64,
    pub rtol: f64,
    pub atol_scale: f64,
    pub max_step: f64,
    pub max_steps: usize,
    pub out_times: Vec<f64>,
    /// Evaluate inputs (schedules) at this fixed time instead of t (time-marching to steady state).
    pub freeze_time: Option<f64>,
    /// Stop early once max |dy/dt| / weight falls below this (1/s), for steady-state marching.
    pub steady_tol: Option<f64>,
}

const GAMMA: f64 = 2.0 - std::f64::consts::SQRT_2;
const D: f64 = GAMMA / 2.0;
const W: f64 = std::f64::consts::SQRT_2 / 4.0;

struct Ctx<'a> {
    net: &'a Network,
    n: usize,
    ev: Eval,
    jb: JacobianBuilder,
    ls: LinSys,
    lu_h: f64,
    stats: Stats,
    wt: Vec<f64>,
    f: Vec<f64>,
    g: Vec<f64>,
    rtol: f64,
    atol_scale: f64,
    debug: bool,
    freeze: Option<f64>,
}

impl<'a> Ctx<'a> {
    fn rhs(&mut self, t: f64, y: &[f64], out: &mut [f64]) -> Result<()> {
        self.stats.rhs_evals += 1;
        self.net.rhs(self.freeze.unwrap_or(t), y, out, &mut self.ev)
    }

    fn jacobian(&mut self, t: f64, y: &[f64]) -> Result<()> {
        let evals = self
            .jb
            .build(self.net, self.freeze.unwrap_or(t), y, &mut self.ev)?;
        self.stats.rhs_evals += evals;
        self.stats.jacobians += 1;
        self.lu_h = f64::NAN; // force refactorisation
        Ok(())
    }

    fn factor(&mut self, h: f64) -> Result<()> {
        self.stats.factorizations += 1;
        self.lu_h = h;
        if !self.ls.factor(1.0, D * h, &self.jb.vals) {
            self.lu_h = f64::NAN;
            return Err(Error::Solver("singular iteration matrix".into()));
        }
        Ok(())
    }

    fn valid(&self, y: &[f64]) -> bool {
        y.iter().enumerate().all(|(i, v)| {
            v.is_finite() && (self.net.state_kind[i] != StateKind::Pressure || *v > 0.0)
        })
    }

    /// Solve Y - d h f(t, Y) = c by modified Newton. Convergence is judged on
    /// the residual itself (not only the correction), so a stale iteration
    /// matrix cannot produce a falsely converged stage. Returns (converged, slow).
    fn newton(
        &mut self,
        t: f64,
        h: f64,
        c: &[f64],
        y: &mut [f64],
        yref: &[f64],
    ) -> Result<(bool, bool)> {
        let n = self.n;
        self.net
            .weights(yref, self.rtol, self.atol_scale, &mut self.wt);
        let mut f = std::mem::take(&mut self.f);
        let mut g = std::mem::take(&mut self.g);
        let mut result = (false, false);
        let mut prev_r = f64::INFINITY;
        let mut prev_d = f64::INFINITY;
        for it in 0..10 {
            if let Err(e) = self.rhs(t, y, &mut f) {
                self.f = f;
                self.g = g;
                return Err(e);
            }
            let mut rs = 0.0;
            for i in 0..n {
                g[i] = -(y[i] - D * h * f[i] - c[i]);
                let r = g[i] / self.wt[i];
                rs += r * r;
            }
            let rnorm = (rs / n.max(1) as f64).sqrt();
            if !rnorm.is_finite() {
                break;
            }
            if rnorm < 0.02 || (prev_d < 1e-3 && rnorm < 0.2) {
                result = (true, it >= 4);
                break;
            }
            if it >= 2 && rnorm > 0.8 * prev_r {
                break; // not contracting
            }
            prev_r = rnorm;
            self.ls.solve(&mut g);
            let mut s = 0.0;
            for i in 0..n {
                y[i] += g[i];
                let r = g[i] / self.wt[i];
                s += r * r;
            }
            prev_d = (s / n.max(1) as f64).sqrt();
            if !self.valid(y) || !prev_d.is_finite() {
                break;
            }
        }
        self.f = f;
        self.g = g;
        Ok(result)
    }
}

fn wrms(v: &[f64], w: &[f64]) -> f64 {
    let n = v.len().max(1);
    (v.iter().zip(w).map(|(a, b)| (a / b) * (a / b)).sum::<f64>() / n as f64).sqrt()
}

/// Integrate from y0 over [t0, t_end]; `sink(t, y)` receives states at each
/// output time; `progress(t)` is called periodically and may return false to stop.
pub fn integrate(
    net: &Network,
    y0: &[f64],
    opt: &TransientOptions,
    sink: &mut dyn FnMut(f64, &[f64]) -> Result<()>,
    progress: &mut dyn FnMut(f64) -> bool,
) -> Result<Stats> {
    let n = net.n_states;
    let mut out_iter = opt.out_times.iter().copied().peekable();
    if n == 0 {
        // purely algebraic network (boundaries + restrictions): sample outputs
        for t in out_iter {
            sink(t, &[])?;
        }
        return Ok(Stats::default());
    }
    let (jb, ls) = JacobianBuilder::new(net);
    let mut cx = Ctx {
        net,
        n,
        ev: Eval::new(net),
        stats: Stats {
            n_states: n,
            h_min_used: f64::INFINITY,
            jacobian_colors: jb.n_colors(),
            nonzeros: ls.nnz(),
            sparse: ls.is_sparse(),
            ..Default::default()
        },
        jb,
        ls,
        lu_h: f64::NAN,
        wt: vec![1.0; n],
        f: vec![0.0; n],
        g: vec![0.0; n],
        rtol: opt.rtol,
        atol_scale: opt.atol_scale,
        debug: cfg!(not(target_arch = "wasm32")) && std::env::var("FLUIDNET_DEBUG").is_ok(),
        freeze: opt.freeze_time,
    };

    let mut t = opt.t0;
    let mut y = y0.to_vec();
    let mut f0 = vec![0.0; n];
    cx.rhs(t, &y, &mut f0)?;
    while let Some(&to) = out_iter.peek() {
        if to <= t + 1e-15 {
            sink(to, &y)?;
            out_iter.next();
        } else {
            break;
        }
    }
    cx.jacobian(t, &y)?;
    let mut jac_fresh = true;
    let mut jac_age = 0usize;

    // initial step from the scaled derivative norm
    net.weights(&y, opt.rtol, opt.atol_scale, &mut cx.wt);
    let d1 = wrms(&f0, &cx.wt);
    let span = opt.t_end - opt.t0;
    let mut h = if d1 > 1e-10 {
        (0.01 / d1).min(0.01 * span)
    } else {
        1e-3 * span
    };
    h = h.clamp(1e-10 * span.max(1e-6), opt.max_step);

    let (t_start, t_stop) = (t, opt.t_end);
    let mut bps = net
        .breakpoints
        .iter()
        .copied()
        .filter(move |&b| b > t_start && b < t_stop)
        .peekable();

    let mut y2 = vec![0.0; n];
    let mut y3 = vec![0.0; n];
    let mut f2 = vec![0.0; n];
    let mut f3 = vec![0.0; n];
    let mut c = vec![0.0; n];
    let mut err = vec![0.0; n];
    let mut ymax = vec![0.0; n];
    let mut yi = vec![0.0; n];
    // previous accepted step for predictor extrapolation
    let mut prev: Option<(f64, Vec<f64>, Vec<f64>)> = None; // (h_prev, y_prev, f_prev)
    let e1 = W - (1.0 - W) / 3.0;
    let e2 = W - (3.0 * W + 1.0) / 3.0;
    let e3 = D - D / 3.0;
    let mut steps_since_progress = 0;

    while t < opt.t_end - 1e-12 * opt.t_end.abs().max(1.0) {
        if cx.stats.steps + cx.stats.rejected >= opt.max_steps {
            return Err(Error::Solver(format!(
                "maximum number of steps ({}) reached at t = {t:.6e} s",
                opt.max_steps
            )));
        }
        while let Some(&b) = bps.peek() {
            if b <= t + 1e-12 * b.abs().max(1.0) {
                bps.next();
            } else {
                break;
            }
        }
        let stop = bps.peek().copied().unwrap_or(opt.t_end).min(opt.t_end);
        let mut landing = false;
        if t + h >= stop - 1e-12 * stop.abs().max(1.0) {
            h = stop - t;
            landing = true;
        } else if t + 1.5 * h > stop {
            h = 0.5 * (stop - t);
        }
        let hmin = 1e-13 * t.abs().max(1e-3);
        if h < hmin {
            return Err(Error::Solver(format!(
                "step size underflow at t = {t:.6e} s (h = {h:.3e}). Typical causes: a state left the fluid table (pressure or temperature out of range), trapped liquid being compressed, or an inconsistent model",
            )));
        }
        if !(cx.lu_h == h) {
            if let Err(e) = cx.factor(h) {
                if jac_fresh {
                    return Err(e);
                }
                cx.jacobian(t, &y)?;
                jac_fresh = true;
                continue;
            }
        }

        // ---- stage 1: trapezoidal rule to t + gamma h
        // predictor: extrapolate the previous step's cubic Hermite (y_prev,f_prev)-(y,f0)
        let hermite_extrap = |theta: f64, out: &mut [f64]| -> bool {
            if let Some((hp, yp, fp)) = &prev {
                let s = 1.0 + theta * h / hp;
                if s > 2.5 {
                    return false;
                }
                let (s2, s3) = (s * s, s * s * s);
                let h00 = 2.0 * s3 - 3.0 * s2 + 1.0;
                let h10 = s3 - 2.0 * s2 + s;
                let h01 = -2.0 * s3 + 3.0 * s2;
                let h11 = s3 - s2;
                for i in 0..out.len() {
                    out[i] = h00 * yp[i] + h10 * hp * fp[i] + h01 * y[i] + h11 * hp * f0[i];
                }
                true
            } else {
                false
            }
        };
        for i in 0..n {
            c[i] = y[i] + D * h * f0[i];
        }
        if !hermite_extrap(GAMMA, &mut y2) || !cx.valid(&y2) {
            for i in 0..n {
                y2[i] = y[i] + GAMMA * h * f0[i];
            }
        }
        let (ok1, slow1) = cx.newton(t + GAMMA * h, h, &c, &mut y2, &y)?;
        let mut ok = ok1;
        let mut slow = slow1;
        if ok {
            for i in 0..n {
                f2[i] = (y2[i] - c[i]) / (D * h);
            }
            // ---- stage 2: BDF2 to t + h
            for i in 0..n {
                c[i] = y[i] + W * h * (f0[i] + f2[i]);
            }
            // predictor: quadratic through y (t), y2 (t + gamma h) with slope f2 at y2
            for i in 0..n {
                y3[i] = y2[i] + (1.0 - GAMMA) * h * f2[i];
            }
            if prev.is_some() {
                let mut ye = vec![0.0; n];
                if hermite_extrap(1.0, &mut ye) && cx.valid(&ye) {
                    // blend: correct the extrapolation by the stage-1 mismatch
                    let mut ym = vec![0.0; n];
                    hermite_extrap(GAMMA, &mut ym);
                    for i in 0..n {
                        y3[i] = ye[i] + (y2[i] - ym[i]);
                    }
                }
            }
            let (ok2, slow2) = cx.newton(t + h, h, &c, &mut y3, &y)?;
            ok = ok2;
            slow |= slow2;
        }
        if !ok {
            if cx.debug {
                eprintln!("t {t:.6e} h {h:.3e} newton failure (jacobian fresh: {jac_fresh})");
            }
            cx.stats.newton_failures += 1;
            if !jac_fresh {
                cx.jacobian(t, &y)?;
                jac_fresh = true;
                jac_age = 0;
            } else {
                h *= 0.25;
            }
            continue;
        }
        for i in 0..n {
            f3[i] = (y3[i] - c[i]) / (D * h);
            err[i] = h * (e1 * f0[i] + e2 * f2[i] + e3 * f3[i]);
            ymax[i] = if y[i].abs() > y3[i].abs() {
                y[i]
            } else {
                y3[i]
            };
        }
        cx.ls.solve(&mut err);
        net.weights(&ymax, opt.rtol, opt.atol_scale, &mut cx.wt);
        let errn = wrms(&err, &cx.wt);

        if errn <= 1.0 {
            // ---- accept; dense output by cubic Hermite
            let t_new = if landing { stop } else { t + h };
            while let Some(&to) = out_iter.peek() {
                if to <= t_new + 1e-12 * t_new.abs().max(1.0) {
                    let th = ((to - t) / h).clamp(0.0, 1.0);
                    let (t2_, t3_) = (th * th, th * th * th);
                    let h00 = 2.0 * t3_ - 3.0 * t2_ + 1.0;
                    let h10 = t3_ - 2.0 * t2_ + th;
                    let h01 = -2.0 * t3_ + 3.0 * t2_;
                    let h11 = t3_ - t2_;
                    for i in 0..n {
                        yi[i] = h00 * y[i] + h10 * h * f0[i] + h01 * y3[i] + h11 * h * f3[i];
                    }
                    sink(to, &yi)?;
                    out_iter.next();
                } else {
                    break;
                }
            }
            prev = if landing {
                None
            } else {
                Some((h, y.clone(), f0.clone()))
            };
            t = t_new;
            std::mem::swap(&mut y, &mut y3);
            std::mem::swap(&mut f0, &mut f3);
            cx.stats.steps += 1;
            cx.stats.h_min_used = cx.stats.h_min_used.min(h);
            cx.stats.h_max_used = cx.stats.h_max_used.max(h);
            if landing {
                // inputs may have a kink or jump here: refresh f
                let yc = y.clone();
                cx.rhs(t, &yc, &mut f0)?;
            }
            jac_age += 1;
            jac_fresh = false;
            if let Some(tol) = opt.steady_tol {
                let mut m: f64 = 0.0;
                net.weights(&y, opt.rtol, opt.atol_scale, &mut cx.wt);
                for i in 0..n {
                    m = m.max((f0[i] / cx.wt[i]).abs());
                }
                if m < tol {
                    sink(t, &y)?;
                    return Ok(cx.stats);
                }
            }
            let fac = if errn > 0.0 {
                (0.9 * errn.powf(-1.0 / 3.0)).clamp(0.2, 4.0)
            } else {
                4.0
            };
            // hysteresis: keep h (and the factorisation) for small increases
            let mut h_next = if (1.0..1.3).contains(&fac) && !landing {
                h
            } else {
                h * fac
            };
            h_next = h_next.min(opt.max_step);
            if slow || jac_age >= 40 || landing {
                cx.jacobian(t, &y)?;
                jac_fresh = true;
                jac_age = 0;
            }
            h = h_next;
            steps_since_progress += 1;
            if steps_since_progress >= 100 {
                steps_since_progress = 0;
                if !progress(t) {
                    return Err(Error::Solver(format!("stopped by user at t = {t:.4e} s")));
                }
            }
        } else {
            if cx.debug {
                let mut wi = 0;
                for i in 0..n {
                    if (err[i] / cx.wt[i]).abs() > (err[wi] / cx.wt[wi]).abs() {
                        wi = i;
                    }
                }
                eprintln!(
                    "t {t:.6e} h {h:.3e} reject err {errn:.3e} (worst {})",
                    net.state_names[wi]
                );
            }
            cx.stats.rejected += 1;
            h *= (0.9 * errn.powf(-1.0 / 3.0)).clamp(0.1, 0.5);
            if !jac_fresh {
                cx.jacobian(t, &y)?;
                jac_fresh = true;
                jac_age = 0;
            }
        }
    }
    for to in out_iter {
        if to <= opt.t_end + 1e-9 * opt.t_end.abs().max(1.0) {
            sink(to, &y)?;
        }
    }
    progress(opt.t_end);
    Ok(cx.stats)
}
