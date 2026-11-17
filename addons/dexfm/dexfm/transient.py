"""Lumped start-transient model of one powerpack (fidelity level L0).

States: fuel and oxygen turbopump speeds and one lumped liner temperature per cooling zone.
Flows are quasi-steady (no line inertance, no manifold filling): at every time step the three
flows (fuel, heated oxygen, direct oxygen) are solved from the hydraulic balance with the pump
maps evaluated at the current speeds. Heat exchange with the liner uses an effectiveness
(NTU) form calibrated on the design-point solution of the detailed cooling model.

This reproduces the bootstrap logic of a closed expander start from tank head (stored heat in
the warm liner spins the turbines before and during ignition). It does not resolve valve
sequencing margins, priming, water hammer, pump stall or injector chug: those belong to the
fluid-network transient tool.
"""
from __future__ import annotations
from dataclasses import dataclass
import math
import numpy as np
from scipy.optimize import root

from .offdesign import PackModel, _pv_inj
from .fluids import H2, O2
from .thermochem import default_provider


@dataclass
class StartSpec:
    T_metal0: float = 250.0      # liner temperature at start [K]
    t_ox_open: float = 0.05      # main oxidizer valve opening command [s]
    t_ign: float = 0.30          # ignition [s]
    t_valve: float = 0.10        # valve stroke time [s]
    t_end: float = 5.0
    dt: float = 2.0e-3
    a_bf_start: float = 10.0     # fuel turbine bypass area during the start phase, relative to design (wide open)
    a_bo_start: float = 0.0      # oxygen turbine bypass during the start phase (closed: all heated oxygen to the turbine)
    t_ramp: float = 1.0          # time to move both bypass valves to the design position after ignition [s]
    a_ocv_start: float = 0.0     # oxidizer control valve (direct liquid path) during the start phase
    t_hold: float = 0.1          # delay between ignition and the start of the valve ramp [s]
    closed_loop: bool = False    # speed-tracking start: oxygen shaft is the master, fuel shaft follows it
    k_ctrl: float = 150.0        # integral gain of the bypass valve speed controllers [1/s]


def rotor_inertia(pump, turb, rho_imp: float) -> float:
    j_imp = pump.stages * 0.5 * math.pi * rho_imp * (pump.d2 / 2) ** 4 * (0.25 * pump.d2) * 0.6
    j_tur = turb.stages * 0.5 * math.pi * 8200.0 * (turb.dm / 2) ** 4 * (0.15 * turb.dm)
    return 1.3 * (j_imp + j_tur)


class _Zone:
    """Effectiveness model of one cooling zone, calibrated at the design point."""
    def __init__(self, z, m_d, Taw):
        self.m_d = m_d; self.T_in_d = float(z.Tb[0]); self.Tw_d = z.Tw_mean
        self.cp = (z.h_out - z.h_in) / (z.T_out - self.T_in_d)
        eff = (z.T_out - self.T_in_d) / (self.Tw_d - self.T_in_d)
        self.ua_d = -math.log(1 - min(eff, 0.98)) * m_d * self.cp
        self.uag_d = z.Q / (Taw - self.Tw_d)
        self.C = z.metal_mass * 390.0
        self.Taw = Taw

    def q_cool(self, m, T_in, Tw):
        m = max(m, 1e-6)
        ntu = self.ua_d * (m / self.m_d) ** 0.8 / (m * self.cp)
        return m * self.cp * (Tw - T_in) * (1 - math.exp(-ntu))

    def q_gas(self, pc_rel, Tw, lit):
        return self.uag_d * pc_rel ** 0.8 * (self.Taw - Tw) if lit else 0.0


def simulate_start(pm: PackModel, spec: StartSpec = StartSpec(), prov=None) -> dict:
    des = pm.des; cfg = des.setup.cfg; su = des.setup; f, o = des.fuel, des.ox
    prov = prov or default_provider()
    h2, o2 = H2(), O2()
    Taw = su.th.Tc * cfg.eta_cstar ** 2 * 0.97
    zf = _Zone(f.zone, f.mdot_pack / pm.npk, Taw); zo = _Zone(o.zone, o.mdot_pack * o.heated_fraction / pm.npk, Taw)
    Jf = rotor_inertia(f.pump, f.turbine, 4430.0); Jo = rotor_inertia(o.pump, o.turbine, 8200.0)
    dpzf_d = f.p_zone_in - f.p_turb_in; dpzo_d = o.p_zone_in - o.p_turb_in
    rho_zf_d = 0.5 * (h2.pT(f.p_zone_in, zf.T_in_d).rho + h2.pT(f.p_turb_in, f.T_turb_in).rho)
    rho_zo_d = 0.5 * (o2.pT(o.p_zone_in, zo.T_in_d).rho + o2.pT(o.p_turb_in, o.T_turb_in).rho)
    m_hd = o.mdot_pack * o.heated_fraction; m_dd = o.mdot_pack - m_hd
    cs_cache = {}

    def cstar_hot(of):
        k = round(min(max(of, 0.8), 12.0), 1)
        if k not in cs_cache:
            cs_cache[k] = prov.solve(k, cfg.pc, 0.0).cstar * cfg.eta_cstar
        return cs_cache[k]

    def ramp(t, t0):
        return float(np.clip((t - t0) / spec.t_valve, 0.0, 1.0))

    ctrl = {"a_bf": spec.a_bf_start, "a_bo": spec.a_bo_start}

    def flows(t, y, guess):
        n_f, n_o, Twf, Two = y
        fr = float(np.clip((t - spec.t_ign - spec.t_hold) / spec.t_ramp, 0.0, 1.0))
        a_ocv = spec.a_ocv_start + (1.0 - spec.a_ocv_start) * fr
        if spec.closed_loop:
            a_ocv = min(n_o / o.pump.rpm, 1.0)
        a_bf = spec.a_bf_start + (1.0 - spec.a_bf_start) * fr
        a_bo = spec.a_bo_start + (1.0 - spec.a_bo_start) * fr
        if spec.closed_loop:
            a_bf, a_bo = ctrl["a_bf"], ctrl["a_bo"]
        th_f = max(ramp(t, 0.0), 0.03); th_o = ramp(t, spec.t_ox_open)
        lit = t >= spec.t_ign and th_o > 0
        w_f, w_o = n_f * math.pi / 30, n_o * math.pi / 30

        def chain(v, full=False):
            v = [min(max(float(x), 0.0), 3.0) for x in v]
            m_f = max(v[0], 1e-4) * f.mdot_pack
            m_h = max(v[1], 0.0) * m_hd if th_o > 0 else 0.0
            m_d = max(v[2], 0.0) * m_dd if th_o > 0 else 0.0
            m_o = m_h + m_d
            # chamber
            Tq_pf = pm.fpump.torque(n_f, m_f)
            p_fd = max(cfg.p_tank_f + pm.fpump.dp(n_f, m_f), 0.3e5)
            h_po = pm.h_tank_f + max(Tq_pf, 0) * w_f / m_f
            p_zi = max(p_fd - pm.k_fl * m_f ** 2 / pm.rho_lf / th_f ** 2, 0.25e5)
            T_in = h2.ph(max(p_zi, 0.5e5), h_po).T
            Qf = zf.q_cool(m_f / pm.npk, T_in, Twf)
            s_avg_rho = None
            h_out = h_po + Qf * pm.npk / m_f
            # zone pressure drop scaled with mass flow squared over mean density
            s_out = h2.ph(max(p_zi * 0.8, 0.3e5), h_out)
            rho_m = 0.5 * (h2.ph(max(p_zi, 0.5e5), h_po).rho + s_out.rho)
            p_zo = max(p_zi - dpzf_d * (m_f / f.mdot_pack) ** 2 * rho_zf_d / rho_m, 0.2e5)
            s_ti = h2.ph(p_zo, h_out)
            if lit and m_o > 0:
                pc = cstar_hot(m_o / m_f) * (m_f + m_o) / (su.At * pm.npk)
            else:
                pc = math.sqrt(4124.0 * max(s_ti.T, 30.0)) / 0.685 * m_f / (su.At * pm.npk)
            pv = _pv_inj(max(pc, 0.3e5), s_ti.T)
            p_te = 0.5 * (pc + math.sqrt(pc * pc + 2 * m_f ** 2 * pv / pm.a_if ** 2))
            dp_t = p_zo - p_te
            sq = math.copysign(math.sqrt(2 * s_ti.rho * abs(dp_t)), dp_t)
            r1 = (m_f - (pm.fturb.cda + a_bf * pm.a_bf_d) * sq) / f.mdot_pack
            r2 = r3 = 0.0; p_od = cfg.p_tank_o; Qo = 0.0; tq_to = 0.0; tq_po = 0.0; T_to = Two; p_inj = pc
            if th_o > 0:
                tq_po = pm.opump.torque(n_o, m_o)
                p_od = max(cfg.p_tank_o + pm.opump.dp(n_o, m_o), 0.3e5)
                h_pox = pm.h_tank_o + max(tq_po, 0) * w_o / max(m_o, 1e-6)
                p_sp = max(p_od - pm.k_ol * m_o ** 2 / pm.rho_lo / th_o ** 2, 0.25e5)
                Qo = zo.q_cool(m_h / pm.npk, cfg.T_tank_o, Two) if m_h > 0 else 0.0
                h_hot = h_pox + (Qo * pm.npk / m_h if m_h > 1e-6 else 0.0)
                s_hot = o2.ph(max(p_sp * 0.9, 0.3e5), h_hot)
                rho_mo = 0.5 * (pm.rho_lo + s_hot.rho)
                p_zoo = max(p_sp - dpzo_d * (m_h / m_hd) ** 2 * rho_zo_d / rho_mo, 0.2e5)
                s_to = o2.ph(p_zoo, h_hot); T_to = s_to.T
                rho_mix = o2.ph(max(pc * 1.15, 0.5e5), (m_h * h_hot + m_d * h_pox) / max(m_o, 1e-6)).rho
                p_inj = pc + m_o ** 2 / (2 * rho_mix * pm.a_io ** 2)
                dpo = p_zoo - p_inj
                sqo = math.copysign(math.sqrt(2 * s_to.rho * abs(dpo)), dpo)
                r2 = (m_h - (pm.oturb.cda + a_bo * pm.a_bo_d) * sqo) / m_hd
                dpd = p_sp - p_inj
                r3 = (m_d - a_ocv * pm.a_ocv * math.copysign(math.sqrt(2 * pm.rho_lo * abs(dpd)), dpd)) / max(m_dd, 1e-6)
                if a_ocv <= 0.0:
                    r3 = v[2]
                if full:
                    m_to = pm.oturb.cda * max(sqo, 0.0)
                    dh_o = o2.expand_ph(p_zoo, s_to.h, p_inj) if dpo > 1e3 else 0.0
                    tq_to = pm.oturb.torque(m_to, dh_o, w_o)
            else:
                r2, r3 = v[1], v[2]
            if not full:
                return [r1, r2, r3]
            m_t = pm.fturb.cda * max(sq, 0.0)
            dh_f = h2.expand_ph(p_zo, s_ti.h, p_te) if dp_t > 1e3 else 0.0
            tq_tf = pm.fturb.torque(m_t, dh_f, w_f)
            return dict(m_f=m_f, m_h=m_h, m_d=m_d, pc=pc, p_fd=p_fd, p_od=p_od, T_tf=s_ti.T, T_to=T_to, Qf=Qf, Qo=Qo,
                        tq_tf=tq_tf, tq_pf=Tq_pf, tq_to=tq_to, tq_po=tq_po, lit=lit, res=max(abs(r1), abs(r2), abs(r3)))

        def safe(v):
            try:
                return chain(v)
            except ValueError:
                return [5.0, 5.0, 5.0]
        sol = root(safe, guess, method="hybr", options={"xtol": 1e-8})
        v = sol.x
        if max(abs(x) for x in sol.fun) > 1e-4:
            sol2 = root(safe, [max(guess[0], 0.02), 0.05 if th_o > 0 else 0.0, 0.05 if th_o > 0 else 0.0], method="lm")
            if max(abs(x) for x in sol2.fun) < max(abs(x) for x in sol.fun):
                v = sol2.x
        v = [min(max(float(x), 0.0), 3.0) for x in v]
        try:
            return chain(v, full=True), [max(v[0], 1e-4), v[1], v[2]]
        except ValueError:
            return chain(guess, full=True), list(guess)

    y = np.array([0.0, 0.0, spec.T_metal0, spec.T_metal0])
    guess = [0.03, 0.0, 0.0]; bad = 0
    n = int(spec.t_end / spec.dt)
    keys = ("t", "n_f", "n_o", "Twf", "Two", "pc", "m_f", "m_o", "of", "p_fd", "p_od", "T_tf", "T_to", "res")
    out = {k: [] for k in keys}
    for i in range(n + 1):
        t = i * spec.dt
        if abs(t - spec.t_ox_open) < spec.dt / 2:
            guess = [guess[0], 0.02, 0.02]
        d, guess = flows(t, y, guess)
        m_o = d["m_h"] + d["m_d"]
        if i % 5 == 0:
            for k, val in zip(keys, (t, y[0], y[1], y[2], y[3], d["pc"], d["m_f"], m_o, m_o / d["m_f"], d["p_fd"], d["p_od"],
                                     d["T_tf"], d["T_to"], d["res"])):
                out[k].append(val)
        pc_rel = d["pc"] / cfg.pc
        dn_f = (d["tq_tf"] * cfg.eta_mech - d["tq_pf"]) / Jf * 30 / math.pi
        dn_o = (d["tq_to"] * cfg.eta_mech - d["tq_po"]) / Jo * 30 / math.pi
        dTf = (zf.q_gas(pc_rel, y[2], d["lit"]) - d["Qf"]) / zf.C
        dTo = (zo.q_gas(pc_rel, y[3], d["lit"]) - d["Qo"]) / zo.C
        y = y + spec.dt * np.array([dn_f, dn_o, dTf, dTo])
        y[0] = max(y[0], 0.0); y[1] = max(y[1], 0.0)
        if spec.closed_loop:
            xo = y[1] / o.pump.rpm; xf = y[0] / f.pump.rpm
            ref_o = min(t / spec.t_ramp, 1.0)
            ctrl["a_bo"] = float(np.clip(ctrl["a_bo"] + spec.k_ctrl * (xo - ref_o) * spec.dt, 0.0, 60.0))
            ctrl["a_bf"] = float(np.clip(ctrl["a_bf"] + spec.k_ctrl * (xf - max(xo, 0.10)) * spec.dt, 0.0, 60.0))
        if y[0] > 1.4 * f.pump.rpm or y[1] > 1.4 * o.pump.rpm:
            res_event = "overspeed"; break
        bad = bad + 1 if d["res"] > 1e-3 else 0
        if bad > 25:                                   # hydraulic balance lost: one propellant cannot enter the chamber
            res_event = "fuel starvation" if d["m_f"] < 0.02 * f.mdot_pack else "oxidizer starvation"; break
    else:
        res_event = "completed"
    res = {k: np.array(v) for k, v in out.items()}
    res["J_f"] = Jf; res["J_o"] = Jo; res["event"] = res_event
    return res
