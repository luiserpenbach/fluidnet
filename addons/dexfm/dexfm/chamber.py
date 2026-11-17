"""Thrust chamber geometry, hot-gas heat transfer and regenerative cooling.

Fidelity level L1: one-dimensional. Hot-gas side by the Bartz correlation with
the sigma property correction, coolant side by Nusselt correlations with a
wall-to-bulk temperature correction and rectangular-channel fin efficiency,
real-fluid coolant properties from CoolProp. The chamber can be split into
cooling zones served by different propellants (hydrogen and oxygen).
"""
from __future__ import annotations
from dataclasses import dataclass, field
import math
import numpy as np
from scipy.optimize import brentq

from .fluids import Fluid, H2, O2, FState
from .thermochem import ThermoState


# --------------------------------------------------------------------------- geometry
@dataclass
class Contour:
    x: np.ndarray          # axial coordinate from injector face [m]
    r: np.ndarray          # inner wall radius [m]
    ar: np.ndarray         # local area ratio A/At
    mach: np.ndarray
    i_throat: int
    Rt: float; Rc: float; Re: float
    L_cyl: float; L_conv: float; L_noz: float
    eps: float; eps_regen: float
    i_regen_end: int       # last regen-cooled station

    @property
    def At(self): return math.pi * self.Rt ** 2
    @property
    def ds(self):
        d = np.hypot(np.diff(self.x), np.diff(self.r))
        return np.concatenate([[d[0]], 0.5 * (d[1:] + d[:-1]), [d[-1]]])
    @property
    def L_chamber(self): return self.L_cyl + self.L_conv
    @property
    def L_total(self): return self.L_cyl + self.L_conv + self.L_noz


def mach_from_ar(ar: float, g: float, supersonic: bool) -> float:
    if ar <= 1.0 + 1e-9:
        return 1.0
    f = lambda M: (1.0 / M) * ((2.0 / (g + 1.0)) * (1.0 + 0.5 * (g - 1.0) * M * M)) ** ((g + 1.0) / (2.0 * (g - 1.0))) - ar
    return brentq(f, 1.0 + 1e-9, 40.0) if supersonic else brentq(f, 1e-5, 1.0 - 1e-9)


def build_contour(At: float, eps: float, cr: float, lstar: float, gamma: float, eps_regen: float,
                  theta_c_deg: float = 30.0, bell: float = 0.8, n_cyl: int = 36, n_conv: int = 20,
                  n_div: int = 90) -> Contour:
    Rt = math.sqrt(At / math.pi); Rc = Rt * math.sqrt(cr); Re = Rt * math.sqrt(eps)
    tc = math.radians(theta_c_deg)
    L_conv = (Rc - Rt) / math.tan(tc)
    V_conv = math.pi / 3.0 * L_conv * (Rc * Rc + Rc * Rt + Rt * Rt)
    L_cyl = max((lstar * At - V_conv) / (math.pi * Rc * Rc), 0.5 * Rc)
    L_noz = bell * (Re - Rt) / math.tan(math.radians(15.0))
    # parabolic (quadratic Bezier) bell between throat and exit
    tn, te = math.radians(32.0), math.radians(7.5)
    x1 = (Re - Rt - L_noz * math.tan(te)) / (math.tan(tn) - math.tan(te)); r1 = Rt + x1 * math.tan(tn)
    t = np.linspace(0.0, 1.0, n_div + 1)[1:] ** 2.2          # cluster stations near the throat
    xd = 2 * (1 - t) * t * x1 + t * t * L_noz
    rd = (1 - t) ** 2 * Rt + 2 * (1 - t) * t * r1 + t * t * Re
    xc = np.linspace(0.0, L_cyl, n_cyl, endpoint=False)
    s = np.linspace(0.0, 1.0, n_conv, endpoint=False)
    # cosine-blended convergent (smooth inlet and throat radii)
    rv = Rt + (Rc - Rt) * 0.5 * (1 + np.cos(math.pi * s))
    xv = L_cyl + s * L_conv * 1.25
    L_conv_eff = L_conv * 1.25
    x = np.concatenate([xc, xv, [L_cyl + L_conv_eff], L_cyl + L_conv_eff + xd])
    r = np.concatenate([np.full(n_cyl, Rc), rv, [Rt], rd])
    i_t = n_cyl + n_conv
    ar = (r / Rt) ** 2
    mach = np.array([mach_from_ar(a, gamma, i > i_t) for i, a in enumerate(ar)])
    idx = np.where((ar <= eps_regen) | (np.arange(len(x)) <= i_t))[0]
    return Contour(x, r, ar, mach, i_t, Rt, Rc, Re, L_cyl, L_conv_eff, L_noz, eps, min(eps_regen, eps), int(idx.max()))


# --------------------------------------------------------------------------- hot gas side
def bartz_h(ct: Contour, th: ThermoState, pc: float, Twg: np.ndarray | float, i, c_bartz: float, eta_cstar: float):
    g = th.gamma; M = ct.mach[i]
    T0 = th.Tc * eta_cstar ** 2
    fac = 1.0 + 0.5 * (g - 1.0) * M * M
    sigma = (0.5 * (Twg / T0) * fac + 0.5) ** -0.68 * fac ** -0.12
    Dt = 2 * ct.Rt
    h = c_bartz * 0.026 / Dt ** 0.2 * (th.mu ** 0.2 * th.cp / th.Pr ** 0.6) * (pc / (th.cstar * eta_cstar)) ** 0.8 \
        * 2.0 ** 0.1 * (1.0 / ct.ar[i]) ** 0.9 * sigma
    Taw = T0 / fac * (1.0 + th.Pr ** (1.0 / 3.0) * 0.5 * (g - 1.0) * M * M)
    return h, Taw


# --------------------------------------------------------------------------- cooling
@dataclass
class WallSpec:
    k: float = 320.0            # liner thermal conductivity [W/m/K]  (copper alloy, GRCop-42 class)
    t_wall: float = 0.8e-3      # hot wall thickness [m]
    rib_min: float = 0.8e-3
    w_throat: float = 1.0e-3
    w_max: float = 2.5e-3
    h_min: float = 0.8e-3
    h_max: float = 6.0e-3
    aspect_max: float = 8.0
    T_limit: float = 750.0      # allowable hot-gas-side wall temperature [K]
    roughness: float = 6e-6
    rho_metal: float = 8800.0
    cp_metal: float = 390.0
    close_out: float = 2.0e-3


@dataclass
class ZoneResult:
    name: str; coolant: str
    idx: np.ndarray             # station indices in flow order
    q: np.ndarray; Twg: np.ndarray; Twc: np.ndarray; Tb: np.ndarray; p: np.ndarray
    h_ch: np.ndarray; w_ch: np.ndarray; n_ch: np.ndarray; vel: np.ndarray
    Q: float; dp: float; T_out: float; p_out: float; h_out: float; h_in: float
    Twg_max: float; Tw_mean: float; area: float; metal_mass: float; overtemp: bool
    mach_max: float


def _nusselt(coolant: str, Re: float, Pr: float, Tw: float, Tb: float) -> float:
    ratio = max(Tw / Tb, 1.0)
    if coolant == "H2":
        return 0.025 * Re ** 0.8 * Pr ** 0.4 * ratio ** -0.55
    return 0.023 * Re ** 0.8 * Pr ** 0.4 * ratio ** -0.30


def _friction(Re: float, rel: float) -> float:
    Re = max(Re, 2300.0)
    return 1.0 / (1.8 * math.log10((rel / 3.7) ** 1.11 + 6.9 / Re)) ** 2   # Haaland (Darcy)


def channel_layout(ct: Contour, wall: WallSpec):
    """Channel count and width at every station (count doubles when width exceeds w_max)."""
    circ = 2 * math.pi * (ct.r + wall.t_wall)
    n0 = int(circ[ct.i_throat] / (wall.w_throat + wall.rib_min))
    n = np.empty_like(circ); w = np.empty_like(circ)
    for i, c in enumerate(circ):
        k = 0
        while c / (n0 * 2 ** k) - wall.rib_min > wall.w_max:
            k += 1
        n[i] = n0 * 2 ** k
        w[i] = min(c / n[i] - wall.rib_min, wall.w_max)
    return n, w


def _local(coolant, b: FState, mdot_ch, w, h, pitch_hot, hg_fun, wall: WallSpec):
    """Solve the local wall energy balance for one station. Returns q [W/m2 hot side], Twg, Twc, hc, vel."""
    A = w * h; G = mdot_ch / A; Dh = 2 * w * h / (w + h)
    Re = G * Dh / b.mu
    Twg = 600.0; Twc = 400.0
    for _ in range(12):
        hg, Taw = hg_fun(Twg)
        hc = _nusselt(coolant, Re, b.Pr, max(Twc, b.T + 1.0), b.T) * b.k / Dh
        rib = pitch_hot - w
        m = math.sqrt(2.0 * hc / (wall.k * max(rib, 1e-5)))
        eta_f = math.tanh(m * h) / (m * h)
        Rtot = 1.0 / hg + wall.t_wall / wall.k + pitch_hot / (hc * (w + 2 * eta_f * h))
        q = (Taw - b.T) / Rtot
        Twg_n = Taw - q / hg
        Twc = Twg_n - q * wall.t_wall / wall.k
        if abs(Twg_n - Twg) < 0.2:
            Twg = Twg_n; break
        Twg = 0.5 * (Twg + Twg_n)
    return q, Twg, Twc, hc, G / b.rho, Re, Dh


def cool_zone(name: str, coolant: str, ct: Contour, th: ThermoState, pc: float, idx: np.ndarray, mdot: float,
              p_in: float, T_in: float, wall: WallSpec, c_bartz: float, eta_cstar: float,
              h_fixed: np.ndarray | None = None, q_scale: float = 1.0) -> ZoneResult:
    """March one coolant through the stations `idx` (given in flow order).

    If `h_fixed` is None the channel height is designed station by station (largest height that
    keeps the hot wall under the limit); otherwise the frozen geometry is evaluated (off-design).
    """
    fl: Fluid = H2() if coolant == "H2" else O2()
    n_all, w_all = channel_layout(ct, wall)
    ds = ct.ds
    b = fl.pT(p_in, T_in); h_in = b.h
    p = p_in - 1.0 * 0.5 * b.rho * 10.0 ** 2            # inlet manifold loss (about one dynamic head at 10 m/s)
    hb = b.h
    N = len(idx)
    out = {k: np.zeros(N) for k in ("q", "Twg", "Twc", "Tb", "p", "h", "w", "n", "v")}
    Q = 0.0; area = 0.0; overtemp = False; mach_max = 0.0; metal = 0.0; G_prev = None; rho_prev = None
    for j, i in enumerate(idx):
        b = fl.ph(p, hb)
        w = w_all[i]; n = n_all[i]; mdot_ch = mdot / n
        pitch_hot = 2 * math.pi * ct.r[i] / n
        hg_fun = lambda Tw, i=i: tuple(v * (q_scale if kk == 0 else 1.0) for kk, v in enumerate(bartz_h(ct, th, pc, Tw, i, c_bartz, eta_cstar)))
        hmax = min(wall.h_max, wall.aspect_max * w)
        if h_fixed is not None:
            hh = h_fixed[j]
            sol = _local(coolant, b, mdot_ch, w, hh, pitch_hot, hg_fun, wall)
        else:
            sol = _local(coolant, b, mdot_ch, w, hmax, pitch_hot, hg_fun, wall); hh = hmax
            if sol[1] > wall.T_limit:
                lo_sol = _local(coolant, b, mdot_ch, w, wall.h_min, pitch_hot, hg_fun, wall)
                if lo_sol[1] > wall.T_limit:
                    sol, hh = lo_sol, wall.h_min
                else:
                    hh = brentq(lambda x: _local(coolant, b, mdot_ch, w, x, pitch_hot, hg_fun, wall)[1] - wall.T_limit,
                                wall.h_min, hmax, xtol=2e-5)
                    sol = _local(coolant, b, mdot_ch, w, hh, pitch_hot, hg_fun, wall)
        q, Twg, Twc, hc, vel, Re, Dh = sol
        overtemp = overtemp or Twg > wall.T_limit + 5.0
        dA = 2 * math.pi * ct.r[i] * ds[i]
        Q += q * dA; area += dA
        hb += q * dA / mdot
        G = mdot_ch / (w * hh)
        f = _friction(Re, wall.roughness / Dh)
        dpf = f * ds[i] / Dh * G * G / (2 * b.rho)
        dpm = 0.0 if G_prev is None else (G * G / b.rho - G_prev * G_prev / rho_prev)
        p -= dpf + max(dpm, -0.5 * dpf)
        G_prev, rho_prev = G, b.rho
        if p < 1e4:
            raise RuntimeError(f"coolant pressure collapsed in zone {name} at station {i}")
        a_snd = math.sqrt(1.3 * p / b.rho) if b.q < 0 else 1e9
        mach_max = max(mach_max, vel / a_snd)
        metal += (2 * math.pi * ct.r[i] * (wall.t_wall + wall.close_out) + n * (pitch_hot * 1.02 - w) * hh) * ds[i] * wall.rho_metal
        for k, v in zip(("q", "Twg", "Twc", "Tb", "p", "h", "w", "n", "v"), (q, Twg, Twc, b.T, p, hh, w, n, vel)):
            out[k][j] = v
    bo = fl.ph(p, hb)
    p -= 1.5 * 0.5 * bo.rho * out["v"][-1] ** 2          # exit collector loss
    bo = fl.ph(p, hb)
    dA_all = 2 * math.pi * ct.r[idx] * ds[idx]
    Tw_mean = float(np.sum(0.5 * (out["Twg"] + out["Twc"]) * dA_all) / np.sum(dA_all))
    return ZoneResult(name, coolant, np.asarray(idx), out["q"], out["Twg"], out["Twc"], out["Tb"], out["p"], out["h"],
                      out["w"], out["n"], out["v"], Q, p_in - p, bo.T, p, hb, h_in, float(out["Twg"].max()), Tw_mean,
                      area, metal, overtemp, mach_max)


def zone_indices(ct: Contour, ox_zone: str, ox_param: float):
    """Station index sets (in coolant flow order) for the hydrogen and oxygen circuits.

    ox_zone = "none"   : hydrogen cools everything, counterflow (nozzle end -> injector).
    ox_zone = "barrel" : oxygen cools the first `ox_param` fraction of the cylinder next to the injector,
                         flowing toward the injector; hydrogen cools the rest.
    ox_zone = "nozzle" : oxygen cools the nozzle from area ratio `ox_param` to the regen end, flowing
                         downstream; hydrogen cools throat and chamber.
    """
    allr = np.arange(0, ct.i_regen_end + 1)
    if ox_zone == "none":
        return allr[::-1], np.array([], dtype=int)
    if ox_zone == "barrel":
        ox = allr[ct.x[allr] <= ox_param * ct.L_cyl]
        h2 = allr[ct.x[allr] > ox_param * ct.L_cyl]
        return h2[::-1], ox[::-1]
    if ox_zone == "nozzle":
        sup = (allr > ct.i_throat) & (ct.ar[allr] >= ox_param)
        return allr[~sup][::-1], allr[sup]
    raise ValueError(ox_zone)
