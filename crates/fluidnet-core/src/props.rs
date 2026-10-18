//! Real-fluid properties from pre-computed CoolProp tables on a (log10 P, h) grid.
//!
//! Interpolation: specific volume uses cubic Hermite along log P (with the stored
//! dv/dP|h) and linear along h, which is exact inside the two-phase dome where v is
//! linear in h. All other fields are bilinear. Derivatives needed by the volume
//! energy equation (dv/dP|h, dv/dh|P) are tabulated, so the right-hand side is
//! continuous everywhere, including across saturation lines.

use std::sync::Arc;

use crate::error::{Error, Result};

const NF: usize = 8;
const F_V: usize = 0;
const F_T: usize = 1;
const F_S: usize = 2;
const F_MU: usize = 3;
const F_K: usize = 4;
const F_CP: usize = 5;
const F_DVDP: usize = 6;
const F_DVDH: usize = 7;
const LN10: f64 = std::f64::consts::LN_10;

#[derive(Debug, Clone, serde::Deserialize)]
struct Header {
    key: String,
    label: String,
    molar_mass: f64,
    #[serde(rename = "R")]
    r: f64,
    #[serde(rename = "Tcrit")]
    tcrit: f64,
    #[serde(rename = "Pcrit")]
    pcrit: f64,
    #[serde(rename = "Ttriple")]
    ttriple: f64,
    #[serde(rename = "Ptriple")]
    ptriple: f64,
    #[serde(rename = "Tmin")]
    tmin: f64,
    #[serde(rename = "Tmax")]
    tmax: f64,
    #[serde(rename = "Pmin")]
    pmin: f64,
    #[serde(rename = "Pmax")]
    pmax: f64,
    #[serde(rename = "nP")]
    n_p: usize,
    #[serde(rename = "nH")]
    n_h: usize,
    #[serde(rename = "nSat")]
    n_sat: usize,
}

/// Thermodynamic and transport state at a point.
#[derive(Debug, Clone, Copy, Default)]
pub struct Props {
    pub p: f64,
    pub h: f64,
    pub v: f64,
    pub rho: f64,
    pub t: f64,
    pub s: f64,
    pub mu: f64,
    pub k: f64,
    pub cp: f64,
    /// (dv/dP)_h
    pub dvdp: f64,
    /// (dv/dh)_P
    pub dvdh: f64,
    /// Homogeneous-equilibrium speed of sound.
    pub a: f64,
    /// Equilibrium quality; < 0 subcooled, > 1 superheated, NaN above Pcrit.
    pub x: f64,
}

impl Props {
    /// (drho/dP)_h
    #[inline]
    pub fn drho_dp(&self) -> f64 {
        -self.rho * self.rho * self.dvdp
    }
    /// (drho/dh)_P
    #[inline]
    pub fn drho_dh(&self) -> f64 {
        -self.rho * self.rho * self.dvdh
    }
    /// Isentropic compressibility parameter omega = -(P/v)(dv/dP)_s (1/gamma for ideal gas).
    #[inline]
    pub fn omega(&self) -> f64 {
        let dvdp_s = self.dvdp + self.dvdh * self.v;
        (-(self.p / self.v) * dvdp_s).max(1e-9)
    }
}

pub struct FluidTable {
    pub key: String,
    pub label: String,
    pub molar_mass: f64,
    pub r_gas: f64,
    pub tcrit: f64,
    pub pcrit: f64,
    pub ttriple: f64,
    pub ptriple: f64,
    pub tmin: f64,
    pub tmax: f64,
    pub pmin: f64,
    pub pmax: f64,
    lp: Vec<f64>,
    /// pressure grid values 10^lp
    pg: Vec<f64>,
    hg: Vec<f64>,
    hmin: Vec<f64>,
    n_p: usize,
    n_h: usize,
    data: Vec<f32>,
    // saturation table, log10 P ascending
    sat_lp: Vec<f64>,
    sat_t: Vec<f64>,
    sat_hl: Vec<f64>,
    sat_hv: Vec<f64>,
    sat_vl: Vec<f64>,
    sat_vv: Vec<f64>,
    sat_sl: Vec<f64>,
    sat_sv: Vec<f64>,
    /// saturation state at each table pressure row (None above Pcrit / below Ptriple)
    row_sat: Vec<Option<Sat>>,
}

impl std::fmt::Debug for FluidTable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "FluidTable({})", self.key)
    }
}

fn rd_f64(b: &[u8], off: &mut usize, n: usize) -> Result<Vec<f64>> {
    let end = *off + 8 * n;
    if end > b.len() {
        return Err(Error::Fluid("truncated fluid table".into()));
    }
    let v = b[*off..end]
        .chunks_exact(8)
        .map(|c| f64::from_le_bytes(c.try_into().unwrap()))
        .collect();
    *off = end;
    Ok(v)
}

fn rd_f32(b: &[u8], off: &mut usize, n: usize) -> Result<Vec<f32>> {
    let end = *off + 4 * n;
    if end > b.len() {
        return Err(Error::Fluid("truncated fluid table".into()));
    }
    let v = b[*off..end]
        .chunks_exact(4)
        .map(|c| f32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    *off = end;
    Ok(v)
}

/// Index i such that g[i] <= x < g[i+1], clamped to [0, n-2].
#[inline]
fn bracket(g: &[f64], x: f64) -> usize {
    let n = g.len();
    let i = g.partition_point(|&v| v <= x);
    i.saturating_sub(1).min(n - 2)
}

#[inline]
fn lerp_tab(xs: &[f64], ys: &[f64], x: f64) -> f64 {
    let i = bracket(xs, x);
    let t = ((x - xs[i]) / (xs[i + 1] - xs[i])).clamp(0.0, 1.0);
    ys[i] + t * (ys[i + 1] - ys[i])
}

impl FluidTable {
    pub fn from_bytes(b: &[u8]) -> Result<Arc<FluidTable>> {
        if b.len() < 12 || &b[0..4] != b"FNTB" {
            return Err(Error::Fluid("not a fluidnet table (bad magic)".into()));
        }
        let ver = u32::from_le_bytes(b[4..8].try_into().unwrap());
        if ver != 1 {
            return Err(Error::Fluid(format!("unsupported table version {ver}")));
        }
        let hl = u32::from_le_bytes(b[8..12].try_into().unwrap()) as usize;
        let hdr: Header = serde_json::from_slice(&b[12..12 + hl])
            .map_err(|e| Error::Fluid(format!("bad table header: {e}")))?;
        let mut off = 12 + hl;
        let lp = rd_f64(b, &mut off, hdr.n_p)?;
        let hg = rd_f64(b, &mut off, hdr.n_h)?;
        let hmin = rd_f64(b, &mut off, hdr.n_p)?;
        let _hmax = rd_f64(b, &mut off, hdr.n_p)?;
        let data = rd_f32(b, &mut off, NF * hdr.n_p * hdr.n_h)?;
        let sat = rd_f64(b, &mut off, 8 * hdr.n_sat)?;
        let col = |c: usize| -> Vec<f64> { (0..hdr.n_sat).map(|i| sat[8 * i + c]).collect() };
        let sat_p = col(0);
        let mut tab = FluidTable {
            key: hdr.key,
            label: hdr.label,
            molar_mass: hdr.molar_mass,
            r_gas: hdr.r,
            tcrit: hdr.tcrit,
            pcrit: hdr.pcrit,
            ttriple: hdr.ttriple,
            ptriple: hdr.ptriple,
            tmin: hdr.tmin,
            tmax: hdr.tmax,
            pmin: hdr.pmin,
            pmax: hdr.pmax,
            pg: lp.iter().map(|x| 10f64.powf(*x)).collect(),
            lp,
            hg,
            hmin,
            n_p: hdr.n_p,
            n_h: hdr.n_h,
            data,
            sat_lp: sat_p.iter().map(|p| p.log10()).collect(),
            sat_t: col(1),
            sat_hl: col(2),
            sat_hv: col(3),
            sat_vl: col(4),
            sat_vv: col(5),
            sat_sl: col(6),
            sat_sv: col(7),
            row_sat: vec![],
        };
        tab.row_sat = tab.pg.iter().map(|&p| tab.sat_full(p)).collect();
        Ok(Arc::new(tab))
    }

    #[inline]
    fn at(&self, f: usize, i: usize, j: usize) -> f64 {
        self.data[(f * self.n_p + i) * self.n_h + j] as f64
    }

    pub fn h_lo(&self) -> f64 {
        self.hg[0]
    }
    pub fn h_hi(&self) -> f64 {
        self.hg[self.n_h - 1]
    }

    /// Lowest valid enthalpy at pressure P (fluid at Tmin).
    pub fn hmin_at(&self, p: f64) -> f64 {
        let x = p.clamp(self.pmin, self.pmax).log10();
        lerp_tab(&self.lp, &self.hmin, x)
    }

    /// Saturation-aware property lookup at (P, h); P and h are clamped to the
    /// table domain. Below Pcrit: exact lever rule inside the dome, and
    /// single-phase states interpolated at constant subcooling / superheat with
    /// the saturation point as an end point, so liquid and vapour data are never
    /// mixed across the saturation line.
    pub fn props_ph(&self, p: f64, h: f64) -> Props {
        if p < self.pmin {
            // below the table: ideal-gas extrapolation at constant h (rho ~ P),
            // keeping derivatives smooth down to vacuum
            let mut pr = self.props_ph(self.pmin, h);
            let p = p.max(1e-9);
            let r = self.pmin / p;
            pr.v *= r;
            pr.rho /= r;
            pr.dvdp = -pr.v / p;
            pr.dvdh *= r;
            pr.p = p;
            pr.x = f64::NAN;
            return pr;
        }
        let pc = p.min(self.pmax);
        let raw = self.props_ph_sat(pc, h);
        // Blend the compressibility derivatives across the saturation lines over
        // a narrow quality band. The equilibrium derivative (dv/dP)_h jumps by
        // orders of magnitude at x = 0; a smooth transition (a short relaxation
        // zone) keeps implicit integration efficient when liquid lines cavitate.
        const BAND: f64 = 0.01;
        if let Some(st) = self.sat_full(pc) {
            let dhv = st.hv - st.hl;
            let x = (raw.h - st.hl) / dhv;
            let blend = |line_h: f64, on_low: Props, on_high: Props, xr: f64| -> Props {
                let w = smoothstep_band((xr + BAND) / (2.0 * BAND));
                let mut pr = raw;
                pr.dvdp = (1.0 - w) * on_low.dvdp + w * on_high.dvdp;
                pr.dvdh = (1.0 - w) * on_low.dvdh + w * on_high.dvdh;
                let rho = pr.rho;
                let drdp_s = -rho * rho * (pr.dvdp + pr.dvdh * pr.v);
                pr.a = if drdp_s > 0.0 {
                    (1.0 / drdp_s).sqrt()
                } else {
                    0.0
                };
                let _ = line_h;
                pr
            };
            if x.abs() < BAND {
                let liq = if x <= 0.0 {
                    raw
                } else {
                    self.props_ph_sat(pc, st.hl)
                };
                let tp = if x > 0.0 {
                    raw
                } else {
                    self.two_phase(pc, st.hl + 1e-9 * dhv, &st)
                };
                return blend(st.hl, liq, tp, x);
            }
            if (x - 1.0).abs() < BAND {
                let tp = if x < 1.0 {
                    raw
                } else {
                    self.two_phase(pc, st.hv - 1e-9 * dhv, &st)
                };
                let vap = if x >= 1.0 {
                    raw
                } else {
                    self.props_ph_sat(pc, st.hv)
                };
                return blend(st.hv, tp, vap, x - 1.0);
            }
        }
        raw
    }

    /// Saturation-aware lookup without derivative blending.
    fn props_ph_sat(&self, pc: f64, h: f64) -> Props {
        if let Some(st) = self.sat_full(pc) {
            let hc = h.clamp(self.hmin_at(pc), self.h_hi());
            if hc > st.hl && hc < st.hv {
                return self.two_phase(pc, hc, &st);
            }
            let liquid = hc <= st.hl;
            let dh = if liquid { st.hl - hc } else { hc - st.hv };
            let x = pc.log10();
            let i = bracket(&self.lp, x);
            if let (Some(r0), Some(r1)) =
                (self.sat_row(i, liquid, dh), self.sat_row(i + 1, liquid, dh))
            {
                let dx = self.lp[i + 1] - self.lp[i];
                let tx = ((x - self.lp[i]) / dx).clamp(0.0, 1.0);
                let t2 = tx * tx;
                let t3 = t2 * tx;
                let m0 = r0.dv_dp_rel * self.pg[i] * LN10 * dx;
                let m1 = r1.dv_dp_rel * self.pg[i + 1] * LN10 * dx;
                let mut v = (2.0 * t3 - 3.0 * t2 + 1.0) * r0.f[F_V]
                    + (t3 - 2.0 * t2 + tx) * m0
                    + (-2.0 * t3 + 3.0 * t2) * r1.f[F_V]
                    + (t3 - t2) * m1;
                let lin = |k: usize| r0.f[k] + tx * (r1.f[k] - r0.f[k]);
                if !(v > 0.0) {
                    v = lin(F_V).max(1e-12);
                }
                return self.finish(
                    pc,
                    hc,
                    v,
                    lin(F_T),
                    lin(F_S),
                    lin(F_MU),
                    lin(F_K),
                    lin(F_CP),
                    lin(F_DVDP),
                    lin(F_DVDH),
                );
            }
            return self.props_ph_plain(pc, hc);
        }
        self.props_ph_plain(pc, h)
    }

    fn finish(
        &self,
        p: f64,
        h: f64,
        v: f64,
        t: f64,
        s: f64,
        mu: f64,
        k: f64,
        cp: f64,
        dvdp: f64,
        dvdh: f64,
    ) -> Props {
        let rho = 1.0 / v;
        let drdp_s = -rho * rho * (dvdp + dvdh * v);
        let a = if drdp_s > 0.0 {
            (1.0 / drdp_s).sqrt()
        } else {
            0.0
        };
        Props {
            p,
            h,
            v,
            rho,
            t,
            s,
            mu: mu.max(1e-7),
            k: k.max(1e-4),
            cp: cp.max(1.0),
            dvdp,
            dvdh,
            a,
            x: f64::NAN,
        }
    }

    /// Two-phase (homogeneous equilibrium) state from the saturation table.
    fn two_phase(&self, p: f64, h: f64, st: &Sat) -> Props {
        let dhs = st.hv - st.hl;
        let x = (h - st.hl) / dhs;
        let v = st.vl + x * (st.vv - st.vl);
        let s = st.sl + x * (st.sv - st.sl);
        let dvdh = (st.vv - st.vl) / dhs;
        let dxdp = (-st.dhl * dhs - (h - st.hl) * (st.dhv - st.dhl)) / (dhs * dhs);
        let dvdp = st.dvl + x * (st.dvv - st.dvl) + (st.vv - st.vl) * dxdp;
        let tr = self.props_ph_plain(p, h); // transport and cp (quality-weighted in the table)
        let mut pr = self.finish(p, h, v, st.t, s, tr.mu, tr.k, tr.cp, dvdp, dvdh);
        pr.x = x;
        pr
    }

    /// Values on table row r at constant subcooling (liquid) or superheat
    /// (vapour) dh from that row's saturation point.
    fn sat_row(&self, r: usize, liquid: bool, dh: f64) -> Option<SatRow> {
        let st = self.row_sat[r]?;
        let hsat = if liquid { st.hl } else { st.hv };
        let hr = if liquid { hsat - dh } else { hsat + dh }.clamp(self.hmin[r], self.h_hi());
        let j = bracket(&self.hg, hr);
        let node = |jj: usize| -> [f64; NF] {
            let mut a = [0.0; NF];
            for (f, v) in a.iter_mut().enumerate() {
                *v = self.at(f, r, jj);
            }
            a
        };
        let (mut a, mut b) = (node(j), node(j + 1));
        let (mut ha, mut hb) = (self.hg[j], self.hg[j + 1]);
        if liquid && hb > st.hl {
            b = a;
            b[F_V] = st.vl;
            b[F_T] = st.t;
            b[F_S] = st.sl;
            hb = st.hl;
        } else if !liquid && ha < st.hv {
            a = b;
            a[F_V] = st.vv;
            a[F_T] = st.t;
            a[F_S] = st.sv;
            ha = st.hv;
        }
        let ty = if hb - ha > 1e-12 {
            ((hr - ha) / (hb - ha)).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let mut f = [0.0; NF];
        for k in 0..NF {
            f[k] = a[k] + ty * (b[k] - a[k]);
        }
        let dsat = if liquid { st.dhl } else { st.dhv };
        Some(SatRow {
            f,
            dv_dp_rel: f[F_DVDP] + f[F_DVDH] * dsat,
        })
    }

    /// Full saturation state with d/dP slopes, for Ptriple < P < ~Pcrit.
    pub fn sat_full(&self, p: f64) -> Option<Sat> {
        let n = self.sat_lp.len();
        if n < 2 || p >= self.pcrit || p <= 0.0 {
            return None;
        }
        let x = p.log10();
        if x < self.sat_lp[0] || x > self.sat_lp[n - 1] {
            return None;
        }
        let i = bracket(&self.sat_lp, x);
        let dx = self.sat_lp[i + 1] - self.sat_lp[i];
        let t = ((x - self.sat_lp[i]) / dx).clamp(0.0, 1.0);
        let l = |v: &[f64]| v[i] + t * (v[i + 1] - v[i]);
        let d = |v: &[f64]| (v[i + 1] - v[i]) / dx / (p * LN10);
        Some(Sat {
            t: l(&self.sat_t),
            hl: l(&self.sat_hl),
            hv: l(&self.sat_hv),
            vl: l(&self.sat_vl),
            vv: l(&self.sat_vv),
            sl: l(&self.sat_sl),
            sv: l(&self.sat_sv),
            dhl: d(&self.sat_hl),
            dhv: d(&self.sat_hv),
            dvl: d(&self.sat_vl),
            dvv: d(&self.sat_vv),
        })
    }

    /// Plain (log P, h) table interpolation without saturation handling.
    pub fn props_ph_plain(&self, p: f64, h: f64) -> Props {
        let pc = p.clamp(self.pmin, self.pmax);
        let x = pc.log10();
        let i = bracket(&self.lp, x);
        let dx = self.lp[i + 1] - self.lp[i];
        let tx = ((x - self.lp[i]) / dx).clamp(0.0, 1.0);
        let hmin = self.hmin[i] + tx * (self.hmin[i + 1] - self.hmin[i]);
        let hc = h.clamp(hmin, self.h_hi());
        let j = bracket(&self.hg, hc);
        let ty = ((hc - self.hg[j]) / (self.hg[j + 1] - self.hg[j])).clamp(0.0, 1.0);

        let w00 = (1.0 - tx) * (1.0 - ty);
        let w10 = tx * (1.0 - ty);
        let w01 = (1.0 - tx) * ty;
        let w11 = tx * ty;
        let bil = |f: usize| -> f64 {
            w00 * self.at(f, i, j)
                + w10 * self.at(f, i + 1, j)
                + w01 * self.at(f, i, j + 1)
                + w11 * self.at(f, i + 1, j + 1)
        };
        // Hermite along log P for v
        let p0 = self.pg[i];
        let p1 = self.pg[i + 1];
        let t2 = tx * tx;
        let t3 = t2 * tx;
        let h00 = 2.0 * t3 - 3.0 * t2 + 1.0;
        let h10 = t3 - 2.0 * t2 + tx;
        let h01 = -2.0 * t3 + 3.0 * t2;
        let h11 = t3 - t2;
        let herm = |jj: usize| -> f64 {
            h00 * self.at(F_V, i, jj)
                + h10 * dx * self.at(F_DVDP, i, jj) * p0 * LN10
                + h01 * self.at(F_V, i + 1, jj)
                + h11 * dx * self.at(F_DVDP, i + 1, jj) * p1 * LN10
        };
        let mut v = (1.0 - ty) * herm(j) + ty * herm(j + 1);
        if !(v > 0.0) {
            v = bil(F_V).max(1e-12);
        }
        let dvdp = bil(F_DVDP);
        let dvdh = bil(F_DVDH);
        let rho = 1.0 / v;
        // HEM sound speed: 1/a^2 = (drho/dP)_s = (drho/dP)_h + (drho/dh)_P / rho
        let drdp_s = -rho * rho * (dvdp + dvdh * v);
        let a = if drdp_s > 0.0 {
            (1.0 / drdp_s).sqrt()
        } else {
            0.0
        };
        Props {
            p: pc,
            h: hc,
            v,
            rho,
            t: bil(F_T),
            s: bil(F_S),
            mu: bil(F_MU).max(1e-7),
            k: bil(F_K).max(1e-4),
            cp: bil(F_CP).max(1.0),
            dvdp,
            dvdh,
            a,
            x: f64::NAN,
        }
    }

    /// Properties at (P, h) including the equilibrium quality.
    pub fn props_ph_x(&self, p: f64, h: f64) -> Props {
        let mut pr = self.props_ph(p, h);
        pr.x = self.quality(pr.p, h);
        pr
    }

    /// Saturation properties at P (P < Pcrit): (T, h_l, h_v, v_l, v_v).
    pub fn sat_p(&self, p: f64) -> Option<(f64, f64, f64, f64, f64)> {
        if self.sat_lp.len() < 2 || p >= self.pcrit || p <= 0.0 {
            return None;
        }
        let x = p.log10();
        if x < self.sat_lp[0] {
            return None;
        }
        let i = bracket(&self.sat_lp, x);
        let t = ((x - self.sat_lp[i]) / (self.sat_lp[i + 1] - self.sat_lp[i])).clamp(0.0, 1.0);
        let l = |v: &[f64]| v[i] + t * (v[i + 1] - v[i]);
        Some((
            l(&self.sat_t),
            l(&self.sat_hl),
            l(&self.sat_hv),
            l(&self.sat_vl),
            l(&self.sat_vv),
        ))
    }

    /// Saturated-liquid entropy at P (P < Pcrit).
    pub fn sat_sl(&self, p: f64) -> Option<f64> {
        if self.sat_lp.len() < 2 || p >= self.pcrit || p <= 0.0 {
            return None;
        }
        let x = p.log10();
        if x < self.sat_lp[0] {
            return None;
        }
        Some(lerp_tab(&self.sat_lp, &self.sat_sl, x))
    }

    /// Enthalpy at (P, s) by Newton iteration with (ds/dh)_P = 1/T, starting
    /// from `h_guess`; falls back to bracketing.
    pub fn h_ps_near(&self, p: f64, s: f64, h_guess: f64) -> Option<f64> {
        let mut h = h_guess;
        for _ in 0..8 {
            let pr = self.props_ph(p, h);
            let ds = pr.s - s;
            let dh = -ds * pr.t;
            h = (h + dh).clamp(self.hmin_at(p), self.h_hi());
            if dh.abs() < 1e-7 * h.abs().max(1e3) {
                return Some(h);
            }
        }
        self.h_ps(p, s)
    }

    /// Saturation pressure at T, or None if T is outside [Ttriple, Tcrit).
    pub fn psat_t(&self, t: f64) -> Option<f64> {
        let n = self.sat_t.len();
        if n < 2 || t < self.sat_t[0] || t >= self.tcrit {
            return None;
        }
        if t >= self.sat_t[n - 1] {
            return Some(10f64.powf(self.sat_lp[n - 1]));
        }
        Some(10f64.powf(lerp_tab(&self.sat_t, &self.sat_lp, t)))
    }

    pub fn quality(&self, p: f64, h: f64) -> f64 {
        match self.sat_p(p) {
            Some((_, hl, hv, _, _)) => (h - hl) / (hv - hl),
            None => f64::NAN,
        }
    }

    /// Enthalpy from (P, T). Saturated states are ambiguous; use `h_px` for those.
    pub fn h_pt(&self, p: f64, t: f64) -> Result<f64> {
        let p = p.clamp(self.pmin, self.pmax);
        let (mut lo, mut hi) = (self.hmin_at(p), self.h_hi());
        if let Some((tsat, hl, hv, _, _)) = self.sat_p(p) {
            if t < tsat {
                hi = hl;
            } else {
                lo = hv;
            }
        }
        self.invert_h(p, t, lo, hi, |pr| pr.t).ok_or_else(|| {
            Error::Fluid(format!(
                "{}: T = {:.2} K at P = {:.4e} Pa outside table",
                self.key, t, p
            ))
        })
    }

    /// Enthalpy from (P, quality). Requires P < Pcrit.
    pub fn h_px(&self, p: f64, x: f64) -> Result<f64> {
        match self.sat_p(p) {
            Some((_, hl, hv, _, _)) => Ok(hl + x * (hv - hl)),
            None => Err(Error::Fluid(format!(
                "{}: quality specified at P = {:.4e} Pa, which is not below Pcrit",
                self.key, p
            ))),
        }
    }

    /// Enthalpy from (P, s).
    pub fn h_ps(&self, p: f64, s: f64) -> Option<f64> {
        let p = p.clamp(self.pmin, self.pmax);
        self.invert_h(p, s, self.hmin_at(p), self.h_hi(), |pr| pr.s)
    }

    /// Solve g(props_ph(p, h)) = target for h on [lo, hi] (g monotonic in h).
    fn invert_h(
        &self,
        p: f64,
        target: f64,
        mut lo: f64,
        mut hi: f64,
        g: impl Fn(&Props) -> f64,
    ) -> Option<f64> {
        let mut flo = g(&self.props_ph(p, lo)) - target;
        let mut fhi = g(&self.props_ph(p, hi)) - target;
        if flo > 0.0 {
            return if flo.abs() < 1e-6 * target.abs().max(1.0) {
                Some(lo)
            } else {
                None
            };
        }
        if fhi < 0.0 {
            return if fhi.abs() < 1e-6 * target.abs().max(1.0) {
                Some(hi)
            } else {
                None
            };
        }
        // Illinois regula falsi
        let mut side = 0i32;
        for _ in 0..100 {
            let mid = if (fhi - flo).abs() > 0.0 {
                (lo * fhi - hi * flo) / (fhi - flo)
            } else {
                0.5 * (lo + hi)
            };
            let fm = g(&self.props_ph(p, mid)) - target;
            if fm.abs() <= 1e-10 * target.abs().max(1.0)
                || (hi - lo).abs() < 1e-9 * hi.abs().max(1.0)
            {
                return Some(mid);
            }
            if fm * fhi > 0.0 {
                hi = mid;
                fhi = fm;
                if side == -1 {
                    flo *= 0.5;
                }
                side = -1;
            } else {
                lo = mid;
                flo = fm;
                if side == 1 {
                    fhi *= 0.5;
                }
                side = 1;
            }
        }
        Some(0.5 * (lo + hi))
    }
}

#[inline]
fn smoothstep_band(x: f64) -> f64 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Saturation state at one pressure, with d/dP slopes along the saturation line.
#[derive(Debug, Clone, Copy)]
pub struct Sat {
    pub t: f64,
    pub hl: f64,
    pub hv: f64,
    pub vl: f64,
    pub vv: f64,
    pub sl: f64,
    pub sv: f64,
    pub dhl: f64,
    pub dhv: f64,
    pub dvl: f64,
    pub dvv: f64,
}

struct SatRow {
    f: [f64; NF],
    dv_dp_rel: f64,
}

/// Registry of loaded fluid tables, looked up by key ("N2", "H2", ...).
#[derive(Default, Clone)]
pub struct FluidLibrary {
    tables: Vec<Arc<FluidTable>>,
}

impl FluidLibrary {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn add(&mut self, t: Arc<FluidTable>) {
        self.tables.retain(|x| x.key != t.key);
        self.tables.push(t);
    }
    pub fn get(&self, key: &str) -> Option<Arc<FluidTable>> {
        let k = key.trim();
        self.tables
            .iter()
            .find(|t| t.key.eq_ignore_ascii_case(k) || t.label.eq_ignore_ascii_case(k))
            .cloned()
    }
    pub fn keys(&self) -> Vec<String> {
        self.tables.iter().map(|t| t.key.clone()).collect()
    }
    /// Load every *.fntb file in a directory (native builds).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_dir(dir: &std::path::Path) -> Result<Self> {
        let mut lib = Self::new();
        let rd =
            std::fs::read_dir(dir).map_err(|e| Error::Fluid(format!("{}: {e}", dir.display())))?;
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|s| s.to_str()) == Some("fntb") {
                let b =
                    std::fs::read(&p).map_err(|e| Error::Fluid(format!("{}: {e}", p.display())))?;
                lib.add(FluidTable::from_bytes(&b)?);
            }
        }
        Ok(lib)
    }
}
