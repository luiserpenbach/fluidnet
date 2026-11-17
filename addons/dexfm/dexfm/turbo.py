"""Turbopump preliminary sizing: centrifugal pumps and axial turbines.

Fidelity level L1 (specific-speed and velocity-ratio correlations). The pump efficiency
uses the Guelich best-efficiency envelope for radial pumps (function of specific speed nq
and volume flow, so it carries the size effect that separates the architectures), derated
for rocket-pump clearances. Turbine efficiency is a velocity-ratio parabola with tip
clearance and partial-admission penalties.
"""
from __future__ import annotations
from dataclasses import dataclass
import math
from .fluids import Fluid, G0

M3S_TO_GPM = 15850.32
M_TO_FT = 3.28084


@dataclass(frozen=True)
class PumpLimits:
    u_tip_max: float        # impeller tip speed limit [m/s]
    nss_max: float          # suction specific speed limit, US units (rpm, gpm, ft)
    dn_max: float           # bearing bore [mm] x speed [rpm]
    psi: float = 0.50       # stage head coefficient g*H/U2^2
    derate: float = 0.92    # rocket-pump derate on the industrial efficiency envelope
    nq_min: float = 7.0
    max_stages: int = 3


LH2_LIMITS = PumpLimits(u_tip_max=600.0, nss_max=60000.0, dn_max=2.7e6)
LOX_LIMITS = PumpLimits(u_tip_max=260.0, nss_max=30000.0, dn_max=1.8e6, max_stages=1)


@dataclass(frozen=True)
class PumpDesign:
    fluid: str; mdot: float; p_in: float; p_out: float; T_in: float; T_out: float
    rho: float; Q: float; head: float; stages: int; rpm: float; nq: float
    eta: float; power: float; u_tip: float; d2: float; nss: float; npsh: float
    d_shaft: float; dn: float; d_inducer: float; limit: str


def gulich_eta(nq: float, Q: float) -> float:
    """Best-efficiency envelope for single-stage radial pumps (Guelich, Centrifugal Pumps, eq. T3.9)."""
    q = max(1.0 / max(Q, 1e-4), 1.0)                      # Q_ref / Q with Q_ref = 1 m3/s
    m = 0.1 * q ** 0.15 * (45.0 / nq) ** 0.06
    return 1.0 - 0.095 * q ** m - 0.3 * (0.35 - math.log10(nq / 23.0)) ** 2 * q ** 0.05


def _shaft_d(power: float, omega: float, tau: float = 1.0e8) -> float:
    return max((16.0 * power / omega / (math.pi * tau)) ** (1.0 / 3.0), 0.0167)   # 25 mm minimum bearing bore


def _pump_core(rho, mdot, dp, rpm, stages, lim, npsh):
    Q = mdot / rho
    Hs = dp / (rho * G0) / stages
    nq = rpm * math.sqrt(Q) / Hs ** 0.75
    eta = max(gulich_eta(max(nq, 3.0), Q) * lim.derate, 0.05)
    power = mdot * dp / (rho * eta)
    omega = rpm * math.pi / 30.0
    u = math.sqrt(G0 * Hs / lim.psi)
    nss = rpm * math.sqrt(Q * M3S_TO_GPM) / (npsh * M_TO_FT) ** 0.75
    d_sh = _shaft_d(power, omega)
    return nq, eta, power, u, 2 * u / omega, nss, d_sh, 1.5 * d_sh * 1e3 * rpm


def design_pump(fl: Fluid, mdot: float, p_in: float, T_in: float, p_out: float, rpm: float, stages: int,
                lim: PumpLimits, p_vap: float) -> PumpDesign:
    s_in = fl.pT(p_in, T_in)
    rho = s_in.rho
    npsh = max(p_in - p_vap, 1e3) / (rho * G0)
    nq, eta, power, u, d2, nss, d_sh, dn = _pump_core(rho, mdot, p_out - p_in, rpm, stages, lim, npsh)
    Q = mdot / rho
    omega = rpm * math.pi / 30.0
    # inducer inlet tip diameter from inlet flow coefficient 0.10 and hub ratio 0.35
    d_ind = (4 * Q / (math.pi * (1 - 0.35 ** 2) * 0.10 * omega / 2)) ** (1.0 / 3.0)
    T_out = fl.ph(p_out, s_in.h + (p_out - p_in) / (rho * eta)).T
    return PumpDesign(fl.name, mdot, p_in, p_out, T_in, T_out, rho, Q, (p_out - p_in) / (rho * G0), stages, rpm, nq, eta,
                      power, u, d2, nss, npsh, d_sh, dn, d_ind, "")


def select_pump(fl: Fluid, mdot: float, p_in: float, T_in: float, p_out: float, lim: PumpLimits, p_vap: float,
                rpm_fixed: float | None = None) -> PumpDesign:
    """Highest-efficiency speed and stage count inside the tip-speed, suction, bearing and nq limits."""
    rho = fl.pT(p_in, T_in).rho
    npsh = max(p_in - p_vap, 1e3) / (rho * G0)
    best = None
    rpms = [rpm_fixed] if rpm_fixed else [4000.0 * 1.03 ** k for k in range(140)]
    for st in range(1, lim.max_stages + 1):
        for rpm in rpms:
            nq, eta, power, u, d2, nss, d_sh, dn = _pump_core(rho, mdot, p_out - p_in, rpm, st, lim, npsh)
            if u > lim.u_tip_max:
                break                                   # tip speed is set by head per stage: add a stage
            if (nss > lim.nss_max or dn > lim.dn_max) and not rpm_fixed:
                break                                   # both grow with speed
            if nq < lim.nq_min and not rpm_fixed:
                continue
            score = eta - 0.03 * (st - 1)               # a stage must buy at least 3 points
            if best is None or score > best[0]:
                best = (score, rpm, st)
    if best is None:
        raise RuntimeError(f"no feasible pump for {fl.name}: mdot={mdot:.2f} dp={(p_out - p_in) / 1e5:.0f} bar")
    _, rpm, st = best
    d = design_pump(fl, mdot, p_in, T_in, p_out, rpm, st, lim, p_vap)
    n2 = _pump_core(rho, mdot, p_out - p_in, rpm * 1.03, st, lim, npsh)
    limit = "suction" if n2[5] > lim.nss_max else "bearing DN" if n2[7] > lim.dn_max else "efficiency optimum"
    return PumpDesign(**{**d.__dict__, "limit": limit})


# --------------------------------------------------------------------------- turbine
@dataclass(frozen=True)
class TurbineLimits:
    u_max: float            # mean blade speed limit [m/s]
    eta_ref: float = 0.82
    nu_opt: float = 0.55
    clearance: float = 0.25e-3
    h_min: float = 5.0e-3
    cax_ratio: float = 0.35


H2_TURB = TurbineLimits(u_max=480.0)
O2_TURB = TurbineLimits(u_max=200.0)      # low blade speed on purpose: rub and particle-impact ignition margin in hot oxygen


@dataclass(frozen=True)
class TurbineDesign:
    fluid: str; mdot: float; p_in: float; T_in: float; p_out: float; T_out: float; rpm: float
    stages: int; dh_s: float; c0: float; u: float; nu: float; dm: float; blade_h: float
    admission: float; eta: float; power: float; cda: float; rho_in: float


def design_turbine(fl: Fluid, mdot: float, p_in: float, T_in: float, p_out: float, rpm: float,
                   lim: TurbineLimits) -> TurbineDesign:
    dh_s, T2s = fl.expand_isentropic(p_in, T_in, p_out)
    rho_exp = fl._rho_exp
    s_in = fl.pT(p_in, T_in)
    omega = rpm * math.pi / 30.0
    best = None
    for st in (1, 2):
        c0 = math.sqrt(2.0 * dh_s / st)
        u = min(lim.nu_opt * c0, lim.u_max)
        dm = 2 * u / omega
        rho_out = rho_exp
        ann = mdot / (rho_out * lim.cax_ratio * c0)
        bh = ann / (math.pi * dm); adm = 1.0
        if bh < lim.h_min:                               # shrink the wheel first, then go partial admission
            dm_small = max(ann / (math.pi * lim.h_min), 0.6 * dm)
            dm, u = dm_small, dm_small * omega / 2
            bh = ann / (math.pi * dm)
            if bh < lim.h_min:
                adm = bh / lim.h_min; bh = lim.h_min
        x = (u / c0) / lim.nu_opt
        eta = lim.eta_ref * (2 * x - x * x) * (1 - 1.5 * lim.clearance / bh) * (1 - 0.25 * (1 - adm))
        eta -= 0.02 * (st - 1)                           # interstage loss
        if best is None or eta > best[0] + 0.03:         # a second stage must buy at least 3 points
            best = (eta, st, c0, u, dm, bh, adm)
    eta, st, c0, u, dm, bh, adm = best
    power = mdot * eta * dh_s
    T_out = fl.ph(p_out, s_in.h - eta * dh_s).T
    cda = mdot / math.sqrt(2.0 * s_in.rho * (p_in - p_out))
    return TurbineDesign(fl.name, mdot, p_in, T_in, p_out, T_out, rpm, st, dh_s, c0, u, u / c0, dm, bh, adm, eta, power,
                         cda, s_in.rho)


# --------------------------------------------------------------------------- off-design maps
@dataclass(frozen=True)
class PumpMap:
    """Homologous polynomial map, valid from locked rotor to overspeed.

    dp   = dp_d  * (1.20 n^2 + 0.10 n m - 0.30 m^2)
    torque = tq_d * (0.40 n^2 + 0.60 n m)
    with n = N/N_d and m = mdot/mdot_d.
    """
    dp_d: float; tq_d: float; n_d: float; m_d: float

    def dp(self, n_rpm, mdot):
        n, m = n_rpm / self.n_d, mdot / self.m_d
        return self.dp_d * (1.20 * n * n + 0.10 * n * m - 0.30 * m * m)

    def torque(self, n_rpm, mdot):
        n, m = n_rpm / self.n_d, mdot / self.m_d
        return self.tq_d * (0.40 * n * n + 0.60 * n * m)

    def phi_rel(self, n_rpm, mdot):
        return (mdot / self.m_d) / max(n_rpm / self.n_d, 1e-6)


def pump_map(d: PumpDesign) -> PumpMap:
    return PumpMap(d.p_out - d.p_in, d.power / (d.rpm * math.pi / 30.0), d.rpm, d.mdot)


@dataclass(frozen=True)
class TurbineMap:
    """Fixed-geometry turbine: orifice-form flow function and velocity-ratio efficiency parabola."""
    cda: float; eta_d: float; nu_d: float; nu_opt: float; r_m: float; stages: int

    def flow(self, rho_in, p_in, p_out):
        return self.cda * math.sqrt(max(2.0 * rho_in * (p_in - p_out), 0.0))

    def torque(self, mdot, dh_s, omega):
        if dh_s <= 0.0 or mdot <= 0.0:
            return 0.0
        c0 = math.sqrt(2.0 * dh_s / self.stages)
        xd = self.nu_d / self.nu_opt
        eta_pk = self.eta_d / (2 * xd - xd * xd)
        a = self.r_m / (c0 * self.nu_opt)
        return max(mdot * dh_s * eta_pk * (2 * a - omega * a * a), 0.0)


def turbine_map(d: TurbineDesign, lim: TurbineLimits) -> TurbineMap:
    return TurbineMap(d.cda, d.eta, d.nu, lim.nu_opt, d.dm / 2, d.stages)
