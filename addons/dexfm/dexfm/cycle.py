"""Design-point power balance of the dual expander cycle for one powerpack.

One powerpack is one hydrogen turbopump plus one oxygen turbopump feeding
`n_chambers / n_packs` thrust chambers in parallel.

Fuel side  : tank -> LH2 pump -> main fuel valve -> hydrogen cooling zone of each chamber
             -> collector -> fuel turbine (with bypass) -> injector.
Oxygen side: tank -> LOX pump -> split. A fraction x is heated in the oxygen cooling zone and
             drives the oxygen turbine (with bypass) before injection; the rest goes to the
             injector directly through the oxidizer control valve.
With ox_zone = "none" the model degrades to a single (hydrogen) expander in which the fuel
turbine drive gas also carries the oxygen pump power; this is the RL10 validation mode.
"""
from __future__ import annotations
from dataclasses import dataclass, field, replace
import math
import numpy as np
from scipy.optimize import brentq

from .config import EngineConfig
from .fluids import H2, O2, G0
from .thermochem import ThermochemProvider, ThermoState, default_provider
from .chamber import build_contour, cool_zone, zone_indices, WallSpec, Contour, ZoneResult
from . import turbo as tb


@dataclass
class Setup:
    cfg: EngineConfig; th: ThermoState; isp: float; eps: float
    mdot_total: float; mf_total: float; mo_total: float
    thrust_chamber: float; At: float; ct: Contour; wall: WallSpec
    idx_h2: np.ndarray; idx_o2: np.ndarray


@dataclass
class Side:
    name: str
    closed: bool
    closure_ratio: float          # best achievable (turbine shaft power)/(pump power) at design bypass
    pump: tb.PumpDesign | None = None
    turbine: tb.TurbineDesign | None = None
    zone: ZoneResult | None = None
    p_zone_in: float = 0.0
    p_turb_in: float = 0.0; T_turb_in: float = 0.0; p_turb_out: float = 0.0
    T_inj: float = 0.0; bypass: float = 0.0; heated_fraction: float = 1.0
    mdot_pack: float = 0.0; extra_load: float = 0.0
    note: str = ""


@dataclass
class CycleResult:
    setup: Setup; fuel: Side; ox: Side
    @property
    def closed(self): return self.fuel.closed and self.ox.closed
    @property
    def Q_total_chamber(self):
        return sum(s.zone.Q for s in (self.fuel, self.ox) if s.zone is not None)


def make_setup(cfg: EngineConfig, prov: ThermochemProvider | None = None) -> Setup:
    prov = prov or default_provider()
    Fc = cfg.thrust_total / cfg.n_chambers
    eps = cfg.eps or 80.0
    for _ in range(6):
        th = prov.solve(cfg.of, cfg.pc, eps)
        isp = th.isp_vac * cfg.eta_cstar * cfg.eta_cf
        mc = Fc / (isp * G0)
        At = mc * th.cstar * cfg.eta_cstar / cfg.pc
        if cfg.eps is not None:
            break
        eps_new = cfg.exit_area_total / cfg.n_chambers / At
        if abs(eps_new - eps) < 0.05:
            eps = eps_new; break
        eps = eps_new
    th = prov.solve(cfg.of, cfg.pc, eps)
    isp = th.isp_vac * cfg.eta_cstar * cfg.eta_cf
    mc = Fc / (isp * G0); At = mc * th.cstar * cfg.eta_cstar / cfg.pc
    ct = build_contour(At, eps, cfg.cr, cfg.lstar, th.gamma, cfg.eps_regen)
    wall = WallSpec(**cfg.wall.model_dump())
    ih, io = zone_indices(ct, cfg.ox_zone, cfg.ox_param)
    mt = mc * cfg.n_chambers
    return Setup(cfg, th, isp, eps, mt, mt / (1 + cfg.of), mt * cfg.of / (1 + cfg.of), Fc, At, ct, wall, ih, io)


def _min_closing_pressure(ratio, p_lo, p_hi, n=36):
    """Lowest pump discharge pressure with ratio >= 1; returns (p, closed, best_ratio)."""
    ps = np.geomspace(p_lo, p_hi, n)
    best = (-1.0, ps[0]); prev = None
    for p in ps:
        r = ratio(p)
        if r > best[0]:
            best = (r, p)
        if r >= 1.0:
            if prev is None:
                return p, True, r
            return brentq(lambda q: ratio(q) - 1.0, prev, p, xtol=2e3), True, best[0]
        prev = p
    return best[1], False, best[0]


def solve_fuel(su: Setup, extra_load: float = 0.0, T_pump_out_guess: float = 32.0) -> Side:
    cfg = su.cfg; fl = H2()
    npk = cfg.n_chambers // cfg.n_packs
    mf_pack = su.mf_total / cfg.n_packs; mf_ch = su.mf_total / cfg.n_chambers
    p_te = cfg.pc * (1 + cfg.dp_inj_f) * (1 + cfg.dp_line)
    lim_p = replace(tb.LH2_LIMITS, derate=cfg.pump_derate); lim_t = replace(tb.H2_TURB, eta_ref=cfg.turb_eta_ref)
    p_fd = 2.6 * cfg.pc; T_po = T_pump_out_guess; out = None
    for _ in range(6):
        p_zi = p_fd * (1 - cfg.dp_line - cfg.dp_valve)
        try:
            zone = cool_zone("H2", "H2", su.ct, su.th, cfg.pc, su.idx_h2, mf_ch, p_zi, T_po, su.wall, cfg.c_bartz, cfg.eta_cstar)
        except (RuntimeError, ValueError):
            p_fd *= 1.3; continue
        dpz, T_ti = zone.dp, zone.T_out

        def parts(p):
            pump = tb.select_pump(fl, mf_pack, cfg.p_tank_f, cfg.T_tank_f, p, lim_p, cfg.p_vap_f)
            p_ti = (p * (1 - cfg.dp_line - cfg.dp_valve) - dpz) * 0.99
            if p_ti <= p_te * 1.03:
                return pump, None, p_ti
            turb = tb.design_turbine(fl, mf_pack * (1 - cfg.bypass_f), p_ti, T_ti, p_te, pump.rpm, lim_t)
            return pump, turb, p_ti

        def ratio(p):
            try:
                pump, turb, _ = parts(p)
            except (RuntimeError, ValueError):
                return 0.0
            return 0.0 if turb is None else turb.power * cfg.eta_mech / (pump.power + extra_load)

        p_new, closed, best = _min_closing_pressure(ratio, (p_te * 1.06 + dpz) / (1 - cfg.dp_line - cfg.dp_valve), 8.0 * cfg.pc)
        try:
            pump, turb, p_ti = parts(p_new)
        except (RuntimeError, ValueError):
            return Side("fuel", False, 0.0, note="no feasible pump")
        T_po = pump.T_out
        h_ti = fl.h_pT(p_ti, T_ti)
        T_inj = T_ti
        if turb is not None:
            h_te = h_ti - turb.eta * turb.dh_s
            T_inj = fl.ph(p_te, (1 - cfg.bypass_f) * h_te + cfg.bypass_f * h_ti).T
        out = Side("fuel", closed and not zone.overtemp, best, pump, turb, zone, p_new * (1 - cfg.dp_line - cfg.dp_valve),
                   p_ti, T_ti, p_te, T_inj, cfg.bypass_f, 1.0, mf_pack, extra_load,
                   "wall over temperature limit" if zone.overtemp else "")
        if abs(p_new - p_fd) / p_fd < 0.004:
            break
        p_fd = p_new
    if out is None:
        return Side("fuel", False, 0.0, note="cooling circuit pressure collapse")
    return out


def _ox_for_fraction(su: Setup, x: float) -> Side:
    cfg = su.cfg; fl = O2()
    mo_pack = su.mo_total / cfg.n_packs; mo_ch = su.mo_total / cfg.n_chambers
    p_te = cfg.pc * (1 + cfg.dp_inj_o) * (1 + cfg.dp_line)
    lim_p = replace(tb.LOX_LIMITS, derate=cfg.pump_derate); lim_t = replace(tb.O2_TURB, eta_ref=cfg.turb_eta_ref)
    p_od = 1.9 * cfg.pc; T_po = cfg.T_tank_o + 3.0; out = None
    for _ in range(6):
        p_zi = p_od * (1 - cfg.dp_line - cfg.dp_valve)
        try:
            zone = cool_zone("O2", "O2", su.ct, su.th, cfg.pc, su.idx_o2, x * mo_ch, p_zi, T_po, su.wall, cfg.c_bartz, cfg.eta_cstar)
        except (RuntimeError, ValueError):
            p_od *= 1.3; continue
        dpz, T_ti = zone.dp, zone.T_out

        def parts(p):
            pump = tb.select_pump(fl, mo_pack, cfg.p_tank_o, cfg.T_tank_o, p, lim_p, cfg.p_vap_o)
            p_ti = (p * (1 - cfg.dp_line - cfg.dp_valve) - dpz) * 0.99
            if p_ti <= p_te * 1.03:
                return pump, None, p_ti
            turb = tb.design_turbine(fl, x * mo_pack * (1 - cfg.bypass_o), p_ti, T_ti, p_te, pump.rpm, lim_t)
            return pump, turb, p_ti

        def ratio(p):
            try:
                pump, turb, _ = parts(p)
            except (RuntimeError, ValueError):
                return 0.0
            return 0.0 if turb is None else turb.power * cfg.eta_mech / pump.power

        p_new, closed, best = _min_closing_pressure(ratio, (p_te * 1.06 + dpz) / (1 - cfg.dp_line - cfg.dp_valve), 6.0 * cfg.pc)
        try:
            pump, turb, p_ti = parts(p_new)
        except (RuntimeError, ValueError):
            return Side("ox", False, 0.0, heated_fraction=x, note="no feasible pump")
        T_po = pump.T_out
        h_ti = fl.h_pT(p_ti, T_ti)
        h_hot = h_ti if turb is None else (1 - cfg.bypass_o) * (h_ti - turb.eta * turb.dh_s) + cfg.bypass_o * h_ti
        h_cold = fl.h_pT(pump.p_out, pump.T_out)
        T_inj = fl.ph(p_te, x * h_hot + (1 - x) * h_cold).T
        ok = closed and not zone.overtemp and T_ti <= cfg.T_ox_hot_max
        note = "wall over temperature limit" if zone.overtemp else ("hot oxygen above limit" if T_ti > cfg.T_ox_hot_max else "")
        out = Side("ox", ok, best, pump, turb, zone, p_new * (1 - cfg.dp_line - cfg.dp_valve), p_ti, T_ti, p_te, T_inj,
                   cfg.bypass_o, x, mo_pack, 0.0, note)
        if abs(p_new - p_od) / p_od < 0.004:
            break
        p_od = p_new
    return out or Side("ox", False, 0.0, heated_fraction=x, note="cooling circuit pressure collapse")


def solve_ox(su: Setup) -> Side:
    cfg = su.cfg
    if cfg.ox_zone == "none":
        fl = O2(); mo_pack = su.mo_total / cfg.n_packs
        p_od = cfg.pc * (1 + cfg.dp_inj_o) * (1 + cfg.dp_line) / (1 - cfg.dp_line - cfg.dp_valve)
        pump = tb.select_pump(fl, mo_pack, cfg.p_tank_o, cfg.T_tank_o, p_od, replace(tb.LOX_LIMITS, derate=cfg.pump_derate), cfg.p_vap_o)
        return Side("ox", True, 1.0, pump, None, None, T_inj=pump.T_out, heated_fraction=0.0, mdot_pack=mo_pack,
                    note="oxygen pump driven by hydrogen (single expander)")
    xs = [cfg.ox_heated_fraction] if cfg.ox_heated_fraction else [0.1, 0.15, 0.2, 0.3, 0.4, 0.5, 0.65, 0.8, 1.0]
    sides = [_ox_for_fraction(su, x) for x in xs]
    good = [s for s in sides if s.closed]
    if good:
        return min(good, key=lambda s: s.pump.power)
    return max(sides, key=lambda s: s.closure_ratio)


def solve(cfg: EngineConfig, prov: ThermochemProvider | None = None) -> CycleResult:
    su = make_setup(cfg, prov)
    ox = solve_ox(su)
    extra = ox.pump.power if (cfg.ox_zone == "none" and ox.pump) else 0.0
    fuel = solve_fuel(su, extra_load=extra)
    return CycleResult(su, fuel, ox)


def max_pc(cfg: EngineConfig, prov=None, lo: float = 20e5, hi: float = 110e5, tol: float = 1.0e5, step: float = 15e5):
    """Highest chamber pressure at which both sides close with the configured bypass margins.

    A coarse upward scan locates the highest closing pressure (closure can also fail at very low
    pressure, where the oxygen circuit is far below critical), then bisection refines it.
    """
    best = None; p = lo; last_ok = None; first_fail_above = None
    while p <= hi + 1:
        r = solve(cfg.model_copy(update={"pc": p}), prov)
        if r.closed:
            last_ok, best, first_fail_above = p, r, None
        elif last_ok is not None and first_fail_above is None:
            first_fail_above = p
            break
        p += step
    if last_ok is None:
        return None, r
    lo_p, hi_p = last_ok, (first_fail_above or hi)
    while hi_p - lo_p > tol:
        mid = 0.5 * (lo_p + hi_p)
        r = solve(cfg.model_copy(update={"pc": mid}), prov)
        if r.closed:
            lo_p, best = mid, r
        else:
            hi_p = mid
    return lo_p, best
