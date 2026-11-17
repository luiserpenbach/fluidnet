"""Steady off-design (throttle) solution of one powerpack with frozen hardware.

The hardware is frozen from a design-point `CycleResult`: pump and turbine maps, line and
valve flow areas, injector flow areas, and the cooling-channel geometry. For a commanded
chamber pressure and mixture ratio the solver finds shaft speeds, flows and the two control
valve positions:

    fuel turbine bypass valve  -> thrust (chamber pressure)
    oxygen turbine bypass valve -> mixture ratio

The oxidizer control valve on the direct liquid path is a fixed trim orifice. An optional
back-pressure valve downstream of the oxygen turbine holds the oxygen cooling circuit above a
pressure floor (supercritical) during deep throttling.

The inner hydraulic and torque balance is solved with heat inputs frozen; an outer loop
re-evaluates the detailed regenerative cooling model on the frozen channel geometry.
"""
from __future__ import annotations
from dataclasses import dataclass
import math
import numpy as np
from scipy.optimize import root

from .cycle import CycleResult
from .fluids import H2, O2
from .thermochem import default_provider
from .chamber import cool_zone
from . import turbo as tb


@dataclass
class PackModel:
    des: CycleResult
    npk: int
    fpump: tb.PumpMap; opump: tb.PumpMap; fturb: tb.TurbineMap; oturb: tb.TurbineMap
    k_fl: float; k_ol: float
    a_bf_d: float; a_bo_d: float; a_ocv: float; a_if: float; a_io: float
    rho_lf: float; rho_lo: float; h_tank_f: float; h_tank_o: float


def _pv_inj(pc, T):
    s = H2().pT(1.15 * pc, max(0.95 * T, 30.0))
    return s.p / s.rho


def build_pack(des: CycleResult) -> PackModel:
    cfg = des.setup.cfg; f, o = des.fuel, des.ox
    if o.turbine is None:
        raise ValueError("off-design model needs a dual expander design (oxygen turbine present)")
    h2, o2 = H2(), O2()
    npk = cfg.n_chambers // cfg.n_packs
    rho_lf = h2.pT(cfg.p_tank_f, cfg.T_tank_f).rho; rho_lo = o2.pT(cfg.p_tank_o, cfg.T_tank_o).rho
    k_fl = (f.pump.p_out - f.p_zone_in) * rho_lf / f.mdot_pack ** 2
    k_ol = (o.pump.p_out - o.p_zone_in) * rho_lo / o.mdot_pack ** 2
    a_bf = f.turbine.cda * f.bypass / (1 - f.bypass)
    a_bo = o.turbine.cda * o.bypass / (1 - o.bypass)
    pv = _pv_inj(cfg.pc, f.T_turb_in)
    a_if = math.sqrt(f.mdot_pack ** 2 * pv / (2 * f.p_turb_out * (f.p_turb_out - cfg.pc)))
    x = o.heated_fraction; m_h, m_d = x * o.mdot_pack, (1 - x) * o.mdot_pack
    h_po = o2.h_pT(o.pump.p_out, o.pump.T_out)
    h_hot = o2.h_pT(o.p_turb_in, o.T_turb_in) - o.turbine.power / m_h
    rho_mix = o2.ph(cfg.pc * (1 + cfg.dp_inj_o), (m_h * h_hot + m_d * h_po) / o.mdot_pack).rho
    a_io = o.mdot_pack / math.sqrt(2 * rho_mix * (o.p_turb_out - cfg.pc))
    a_ocv = m_d / math.sqrt(2 * rho_lo * (o.p_zone_in - o.p_turb_out)) if m_d > 0 else 0.0
    return PackModel(des, npk, tb.pump_map(f.pump), tb.pump_map(o.pump), tb.turbine_map(f.turbine, tb.H2_TURB),
                     tb.turbine_map(o.turbine, tb.O2_TURB), k_fl, k_ol, a_bf, a_bo, a_ocv, a_if, a_io, rho_lf, rho_lo,
                     h2.h_pT(cfg.p_tank_f, cfg.T_tank_f), o2.h_pT(cfg.p_tank_o, cfg.T_tank_o))


@dataclass
class OffPoint:
    pc: float; of: float; throttle: float; converged: bool
    n_f: float; n_o: float; m_f: float; m_h: float; m_d: float
    p_fd: float; p_od: float; p_zo_f: float; p_zo_o: float; T_ti_f: float; T_ti_o: float
    p_te_f: float; p_te_o: float; p_inj_o: float
    byp_f: float; byp_o: float; a_bf_rel: float; a_bo_rel: float; a_ocv_rel: float
    phi_f: float; phi_o: float; x_heated: float
    dpinj_f: float; dpinj_o: float; T_inj_o: float
    Q_f: float; Q_o: float; Twg_f: float; Twg_o: float
    thrust_pack: float; isp: float; bpv_dp: float; ox_supercritical: bool
    q_ox_exit: float


def _chain(pm: PackModel, u, pc_t, of_t, heat, a_bpv=None, full=False):
    """Residuals of the pack balance.

    u = [N_f, m_f, N_o, m_h, m_d, a_bf, a_bo, a_ocv] in design-relative units. The oxidizer control
    valve area a_ocv is solved so that the heated oxygen fraction stays at its design value.
    """
    des = pm.des; cfg = des.setup.cfg; f, o = des.fuel, des.ox
    h2, o2 = H2(), O2()
    u = np.maximum(np.asarray(u, float), [0.02, 0.02, 0.02, 0.02, 0.0, 0.0, 0.0, 1e-3])
    n_f = u[0] * f.pump.rpm; m_f = u[1] * f.mdot_pack
    n_o = u[2] * o.pump.rpm; m_h = u[3] * o.mdot_pack * o.heated_fraction
    m_d = max(u[4], 1e-4) * o.mdot_pack * (1 - o.heated_fraction) if o.heated_fraction < 1 else 0.0
    a_bf = u[5] * pm.a_bf_d; a_bo = u[6] * pm.a_bo_d
    m_o = m_h + m_d
    Qf, dpzf, mref_f, Qo, dpzo, mref_o, cstar = heat
    pc = cstar * (m_f + m_o) / (des.setup.At * pm.npk)
    of = m_o / m_f
    w_f = n_f * math.pi / 30; w_o = n_o * math.pi / 30
    # ---- fuel side
    p_fd = max(cfg.p_tank_f + pm.fpump.dp(n_f, m_f), 2e5); tq_pf = pm.fpump.torque(n_f, m_f)
    h_po = pm.h_tank_f + tq_pf * w_f / m_f
    p_zi = max(p_fd - pm.k_fl * m_f ** 2 / pm.rho_lf, 1.8e5)
    p_zo = max(p_zi - dpzf * (m_f / mref_f) ** 2, 1.5e5)
    s_ti = h2.ph(p_zo, h_po + Qf * pm.npk / m_f)
    pv = _pv_inj(pc, s_ti.T)
    p_te = 0.5 * (pc + math.sqrt(pc * pc + 2 * m_f ** 2 * pv / pm.a_if ** 2))
    dp_t = p_zo - p_te
    sq = math.copysign(math.sqrt(2 * s_ti.rho * abs(dp_t)), dp_t)
    r3 = (m_f - (pm.fturb.cda + a_bf) * sq) / f.mdot_pack
    m_t = pm.fturb.cda * max(sq, 0.0)
    dh_f = h2.expand_isentropic(p_zo, s_ti.T, p_te)[0] if dp_t > 0 else 0.0
    tq_tf = pm.fturb.torque(m_t, dh_f, w_f)
    r4 = (tq_tf * cfg.eta_mech - tq_pf) / pm.fpump.tq_d
    # ---- oxygen side
    p_od = max(cfg.p_tank_o + pm.opump.dp(n_o, m_o), 2e5); tq_po = pm.opump.torque(n_o, m_o)
    h_pox = pm.h_tank_o + tq_po * w_o / m_o
    p_sp = max(p_od - pm.k_ol * m_o ** 2 / pm.rho_lo, 1.8e5)
    p_zoo = max(p_sp - dpzo * (m_h / mref_o) ** 2, 1.5e5)
    s_to = o2.ph(p_zoo, h_pox + Qo * pm.npk / m_h)
    # injector: iterate once on mixed enthalpy (turbine work removed from the heated stream)
    h_hot = s_to.h; tq_to = 0.0; p_inj = pc * (1 + cfg.dp_inj_o); bpv_dp = 0.0; p_teo = p_inj; m_to = 0.0; dh_o = 0.0; sqo = 0.0
    for _ in range(3):
        rho_mix = o2.ph(pc * (1 + cfg.dp_inj_o), (m_h * h_hot + m_d * h_pox) / m_o).rho
        p_inj = pc + m_o ** 2 / (2 * rho_mix * pm.a_io ** 2)
        if a_bpv is not None:
            rho_te = o2.ph(max(p_inj, 2e5), h_hot).rho
            bpv_dp = m_h ** 2 / (2 * rho_te * (a_bpv * pm.oturb.cda) ** 2)
        p_teo = p_inj + bpv_dp
        dp_o = p_zoo - p_teo
        sqo = math.copysign(math.sqrt(2 * s_to.rho * abs(dp_o)), dp_o)
        m_to = pm.oturb.cda * max(sqo, 0.0)
        dh_o = o2.expand_isentropic(p_zoo, s_to.T, p_teo)[0] if dp_o > 0 else 0.0
        tq_to = pm.oturb.torque(m_to, dh_o, w_o)
        h_hot = s_to.h - tq_to * w_o / m_h
    r6 = (m_h - (pm.oturb.cda + a_bo) * sqo) / (o.mdot_pack * o.heated_fraction)
    r7 = (tq_to * cfg.eta_mech - tq_po) / pm.opump.tq_d
    if pm.a_ocv > 0:
        dpd = p_sp - p_inj
        r5 = (m_d - u[7] * pm.a_ocv * math.copysign(math.sqrt(2 * pm.rho_lo * abs(dpd)), dpd)) / o.mdot_pack
        r8 = m_h / m_o / o.heated_fraction - 1.0
    else:
        r5 = u[4] - 1.0; r8 = u[7] - 1.0
    res = np.array([pc / pc_t - 1, of / of_t - 1, r3, r4, r5, r6, r7, r8])
    if not full:
        return res
    T_inj_o = o2.ph(p_inj, (m_h * h_hot + m_d * h_pox) / m_o).T
    return dict(res=res, pc=pc, of=of, n_f=n_f, n_o=n_o, m_f=m_f, m_h=m_h, m_d=m_d, p_fd=p_fd, p_od=p_od, p_zi_f=p_zi,
                p_zi_o=p_sp, p_zo_f=p_zo, p_zo_o=p_zoo, T_ti_f=s_ti.T, T_ti_o=s_to.T, p_te_f=p_te, p_te_o=p_teo,
                p_inj_o=p_inj, m_tf=m_t, m_to=m_to, T_po_f=h2.ph(p_fd, h_po).T, T_po_o=o2.ph(p_od, h_pox).T,
                T_inj_o=T_inj_o, bpv_dp=bpv_dp, q_ox=s_to.q)


def solve_point(pm: PackModel, throttle: float, of: float | None = None, u0=None, p_floor_ox: float | None = None,
                prov=None):
    """Solve one throttle point (throttle = Pc / Pc_design). Returns (OffPoint, u) for continuation."""
    des = pm.des; cfg = des.setup.cfg; su = des.setup; f, o = des.fuel, des.ox
    prov = prov or default_provider()
    of = of or cfg.of
    pc_t = throttle * cfg.pc
    th = prov.solve(of, pc_t, su.eps)
    cstar = th.cstar * cfg.eta_cstar
    mref_f, mref_o = f.mdot_pack, o.mdot_pack * o.heated_fraction
    if u0 is not None and len(u0) == 2:                  # warm start: (unknowns, heat references, throttle) of a neighbour
        (u_prev, heat_prev, thr_prev) = u0[0]
        u = np.array(u_prev, float)
        sc = (throttle / thr_prev) ** 0.8
        heat = [heat_prev[0] * sc, heat_prev[1], heat_prev[2], heat_prev[3] * sc, heat_prev[4], heat_prev[5], cstar]
    else:
        u = np.array([math.sqrt(throttle), throttle, math.sqrt(throttle), throttle, throttle, 1.0, 1.0, 1.0], float)
        heat = [f.zone.Q * throttle ** 0.8, f.p_zone_in - f.p_turb_in, mref_f, o.zone.Q * throttle ** 0.8,
                o.p_zone_in - o.p_turb_in, mref_o, cstar]
    a_bpv = u0[1] if (u0 is not None and len(u0) == 2) else None
    ok = False; d = None; zf = zo = None; change = 1.0
    for outer in range(14):
        def fun(v):
            try:
                return _chain(pm, v, pc_t, of, heat, a_bpv)
            except ValueError:
                return np.full(8, 10.0)
        sol = root(fun, u, method="hybr", options={"xtol": 1e-9})
        if not sol.success and np.abs(sol.fun).max() > 1e-5:
            sol2 = root(fun, u, method="lm")
            if np.abs(sol2.fun).max() < np.abs(sol.fun).max():
                sol = sol2
        u = sol.x
        try:
            d = _chain(pm, u, pc_t, of, heat, a_bpv, full=True)
        except ValueError:
            return None, None
        ok = bool(np.abs(d["res"]).max() < 1e-5)
        # detailed cooling on frozen geometry at the current state
        try:
            zf = cool_zone("H2", "H2", su.ct, th, d["pc"], su.idx_h2, d["m_f"] / pm.npk, d["p_zi_f"], d["T_po_f"], su.wall,
                           cfg.c_bartz, cfg.eta_cstar, h_fixed=f.zone.h_ch)
            zo = cool_zone("O2", "O2", su.ct, th, d["pc"], su.idx_o2, d["m_h"] / pm.npk, d["p_zi_o"], d["T_po_o"], su.wall,
                           cfg.c_bartz, cfg.eta_cstar, h_fixed=o.zone.h_ch)
        except (RuntimeError, ValueError):
            ok = False; break
        new = [zf.Q, zf.dp + 0.01 * d["p_zo_f"], d["m_f"], zo.Q, zo.dp + 0.01 * d["p_zo_o"], d["m_h"], cstar]
        change = max(abs(new[0] / heat[0] - 1), abs(new[3] / heat[3] - 1), abs(new[1] / (heat[1] * (d["m_f"] / heat[2]) ** 2) - 1) * 0.2)
        heat = [0.5 * (a + b) if i in (0, 3) else b for i, (a, b) in enumerate(zip(heat, new))]
        # optional supercritical floor on the oxygen cooling circuit
        if p_floor_ox is not None and outer >= 1:
            if a_bpv is None and d["p_zo_o"] < p_floor_ox:
                a_bpv = 3.0
            if a_bpv is not None:
                err = d["p_zo_o"] / p_floor_ox
                if abs(err - 1) > 0.01:
                    a_bpv = float(np.clip(a_bpv * err ** 1.5, 0.02, 50.0)); change = 1.0
        if change < 2e-3 and ok:
            break
    if d is None or zf is None or zo is None:
        return None, None
    thrust = d["pc"] * su.At * pm.npk * th.cf_vac * cfg.eta_cf
    isp = thrust / ((d["m_f"] + d["m_h"] + d["m_d"]) * 9.80665)
    pt = OffPoint(d["pc"], d["of"], throttle, ok and change < 2e-3, d["n_f"], d["n_o"], d["m_f"], d["m_h"], d["m_d"],
                  d["p_fd"], d["p_od"], d["p_zo_f"], d["p_zo_o"], d["T_ti_f"], d["T_ti_o"], d["p_te_f"], d["p_te_o"],
                  d["p_inj_o"], 1 - d["m_tf"] / d["m_f"], 1 - d["m_to"] / d["m_h"], u[5], u[6], u[7],
                  pm.fpump.phi_rel(d["n_f"], d["m_f"]), pm.opump.phi_rel(d["n_o"], d["m_h"] + d["m_d"]),
                  d["m_h"] / (d["m_h"] + d["m_d"]), (d["p_te_f"] - d["pc"]) / d["pc"], (d["p_inj_o"] - d["pc"]) / d["pc"],
                  d["T_inj_o"], zf.Q, zo.Q, zf.Twg_max, zo.Twg_max, thrust, isp, d["bpv_dp"],
                  bool(zo.p.min() > O2().pcrit), d["q_ox"])
    return pt, ((u.copy(), list(heat), throttle), a_bpv)


def throttle_sweep(pm: PackModel, levels, of=None, p_floor_ox=None):
    """Continuation from 100 % downward; intermediate steps are inserted so each solve starts close."""
    out = []; u = None; last = 1.0
    for lv in levels:
        steps = max(int(math.ceil(abs(last - lv) / 0.05)), 1)
        pt = None
        for k in range(1, steps + 1):
            x = last + (lv - last) * k / steps
            pt, u_new = solve_point(pm, x, of, u0=u, p_floor_ox=p_floor_ox)
            if pt is not None and pt.converged:
                u = u_new
        out.append(pt); last = lv
    return out
