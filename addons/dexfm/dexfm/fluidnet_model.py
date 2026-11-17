"""Build, calibrate and run a fluidnet model of one DEX powerpack.

The dexfm design point (`CycleResult`) supplies geometry, machine design points and target
states. This module turns it into a fluidnet model file (dict / YAML) with two shafts, two
cooled lines heated by the same chamber, the turbine bypass valves, the oxidizer control valve
on the direct liquid path and one equivalent chamber for the chambers the pack feeds
(throat area, channel counts and hot-gas areas multiplied by the number of chambers).

`calibrate()` adjusts the quantities fluidnet models differently from dexfm (flow areas,
hot-gas heat transfer factors, channel hydraulic diameters, turbine peak efficiencies) until
the fluidnet steady state reproduces the dexfm design point with prescribed shaft speeds.
The calibrated turbine efficiencies are then a cross-check between the two tools.
"""
from __future__ import annotations
import copy
import math
import numpy as np
from scipy.optimize import least_squares

from .cycle import CycleResult
from .offdesign import build_pack


def _d(area: float) -> float:
    return 2e3 * math.sqrt(area / math.pi)          # mm


def _contour(ct, idx, n=14):
    ds = ct.ds[idx]; s = np.cumsum(ds) - 0.5 * ds; s /= s[-1] + 0.5 * ds[-1]
    xs = np.linspace(0.0, 1.0, n)
    return [[round(float(x), 3), round(float(np.interp(x, s, ct.ar[idx])), 3)] for x in xs], float(np.sum(ds))


def build_model(r: CycleResult, p: dict | None = None, init: str = "design") -> dict:
    """fluidnet model of one powerpack. `p` overrides the calibration parameters.

    init = "design" starts all states near the design point (steady solves); "cold" starts with
    shafts at rest, tank-head pressures and a warm liner (start transients).
    """
    su = r.setup; cfg = su.cfg; f, o = r.fuel, r.ox; pm = build_pack(r)
    npk = pm.npk
    q = dict(d_inj_f=_d(pm.a_if / 0.8), d_inj_o=_d(pm.a_io / 0.7), d_turb_f=_d(pm.fturb.cda / 0.95),
             d_turb_o=_d(pm.oturb.cda / 0.95), htc_f=0.8, htc_o=0.8, d_ch_f=2.2, d_ch_o=3.0,
             d_ocv=_d(pm.a_ocv / 0.8), eff_tf=f.turbine.eta, eff_to=o.turbine.eta,
             d_tbv_f=_d(pm.a_bf_d / 0.8), d_tbv_o=_d(pm.a_bo_d / 0.8))
    q.update(p or {})
    cf, Lf = _contour(su.ct, f.zone.idx); co, Lo = _contour(su.ct, o.zone.idx)
    nf = int(round(float(np.mean(f.zone.n_ch)))) * npk; no = int(round(float(np.mean(o.zone.n_ch)))) * npk
    rho_f, rho_o = pm.rho_lf, pm.rho_lo
    a_mfv = f.mdot_pack / math.sqrt(2 * rho_f * (f.pump.p_out - f.p_zone_in)) / 0.8
    a_mov = o.mdot_pack / math.sqrt(2 * rho_o * (o.pump.p_out - o.p_zone_in)) / 0.8
    T0 = 250.0
    nu_f = f.turbine.u / math.sqrt(2 * f.turbine.dh_s); nu_o = o.turbine.u / math.sqrt(2 * o.turbine.dh_s)
    comps = [
        {"id": "lh2_tank", "type": "boundary", "fluid": "H2", "P": f"{cfg.p_tank_f / 1e5} bar", "T": f"{cfg.T_tank_f} K"},
        {"id": "fuel_pump", "type": "pump", "shaft": "fuel_shaft", "design_speed": f"{f.pump.rpm:.0f} rpm",
         "design_mdot": f"{f.mdot_pack:.4f} kg/s", "design_dp": f"{(f.pump.p_out - f.pump.p_in) / 1e5:.2f} bar",
         "design_eff": round(f.pump.eta, 4), "design_density": round(rho_f, 2)},
        {"id": "fuel_line", "type": "pipe", "length": "0.5 m", "diameter": "51 mm", "segments": 2},
        {"id": "mfv", "type": "valve", "diameter": f"{_d(a_mfv):.3f} mm", "cd": 0.8, "opening": 1.0},
        {"id": "jacket_h2_in", "type": "junction", "volume": "0.3 L", "P0": "2.5 bar", "T0": f"{T0} K"},
        {"id": "jacket_h2", "type": "pipe", "length": f"{Lf:.4f} m", "diameter": f"{q['d_ch_f']:.4f} mm", "parallel": nf,
         "segments": 10, "roughness": "6 um", "wall_thickness": f"{su.wall.t_wall * 1e3} mm", "wall_material": "grcop42",
         "wall_mass_factor": 3.0, "wall_T0": f"{T0} K", "hot_chamber": "tca", "hot_area": f"{f.zone.area * npk * 1e4:.1f} cm2",
         "hot_contour": cf, "hot_htc_factor": round(q["htc_f"], 5), "cool_correction": 0.55, "cool_area_factor": 1.6,
         "P0": "2.5 bar", "T0": f"{T0} K"},
        {"id": "fturb_in", "type": "junction", "volume": "0.5 L", "P0": "2.5 bar", "T0": f"{T0} K"},
        {"id": "fuel_turbine", "type": "turbine", "shaft": "fuel_shaft", "diameter": f"{q['d_turb_f']:.4f} mm", "cd": 0.95,
         "mean_diameter": f"{f.turbine.dm * 1e3:.2f} mm", "design_eff": round(q["eff_tf"], 4),
         "design_velocity_ratio": round(nu_f, 4)},
        {"id": "ftbv", "type": "valve", "diameter": f"{q['d_tbv_f']:.4f} mm", "cd": 0.8, "opening": 1.0},
        {"id": "fturb_out", "type": "junction", "volume": "0.8 L", "P0": "1 bar", "T0": f"{T0} K"},
        {"id": "fuel_injector", "type": "orifice", "diameter": f"{q['d_inj_f']:.4f} mm", "cd": 0.8},
        {"id": "tca", "type": "combustion_chamber", "throat_area": f"{su.At * npk * 1e4:.4f} cm2", "area_ratio": round(su.eps, 1),
        
         "l_star": f"{cfg.lstar} m", "cstar_eff": cfg.eta_cstar, "cf_eff": cfg.eta_cf},
        {"id": "fuel_shaft", "type": "shaft", "inertia": "5.0e-4 kg m2", "N0": "0 rpm", "bearing_torque": "0.05 N m",
         "viscous_loss": round((1 / cfg.eta_mech - 1) * f.pump.power / (f.pump.rpm * math.pi / 30) ** 2, 8)},
        {"id": "lox_tank", "type": "boundary", "fluid": "O2", "P": f"{cfg.p_tank_o / 1e5} bar", "T": f"{cfg.T_tank_o} K"},
        {"id": "lox_pump", "type": "pump", "shaft": "lox_shaft", "design_speed": f"{o.pump.rpm:.0f} rpm",
         "design_mdot": f"{o.mdot_pack:.4f} kg/s", "design_dp": f"{(o.pump.p_out - o.pump.p_in) / 1e5:.2f} bar",
         "design_eff": round(o.pump.eta, 4), "design_density": round(rho_o, 1)},
        {"id": "lox_line", "type": "pipe", "length": "0.5 m", "diameter": "40 mm", "segments": 2},
        {"id": "mov", "type": "valve", "diameter": f"{_d(a_mov):.3f} mm", "cd": 0.8, "opening": 1.0},
        {"id": "lox_split", "type": "junction", "volume": "0.3 L", "P0": "1 bar", "T0": f"{T0} K"},
        {"id": "jacket_o2", "type": "pipe", "length": f"{Lo:.4f} m", "diameter": f"{q['d_ch_o']:.4f} mm", "parallel": no,
         "segments": 8, "roughness": "6 um", "wall_thickness": f"{su.wall.t_wall * 1e3} mm", "wall_material": "grcop42",
         "wall_mass_factor": 3.0, "wall_T0": f"{T0} K", "hot_chamber": "tca", "hot_area": f"{o.zone.area * npk * 1e4:.1f} cm2",
         "hot_contour": co, "hot_htc_factor": round(q["htc_o"], 5), "cool_correction": 0.3, "cool_area_factor": 1.6,
         "P0": "1 bar", "T0": f"{T0} K"},
        {"id": "oturb_in", "type": "junction", "volume": "0.5 L", "P0": "1 bar", "T0": f"{T0} K"},
        {"id": "lox_turbine", "type": "turbine", "shaft": "lox_shaft", "diameter": f"{q['d_turb_o']:.4f} mm", "cd": 0.95,
         "mean_diameter": f"{o.turbine.dm * 1e3:.2f} mm", "design_eff": round(q["eff_to"], 4),
         "design_velocity_ratio": round(nu_o, 4)},
        {"id": "otbv", "type": "valve", "diameter": f"{q['d_tbv_o']:.4f} mm", "cd": 0.8, "opening": 1.0},
        {"id": "ocv", "type": "valve", "diameter": f"{q['d_ocv']:.4f} mm", "cd": 0.8, "opening": 1.0},
        {"id": "ox_manifold", "type": "junction", "volume": "0.8 L", "P0": "1 bar", "T0": f"{T0} K"},
        {"id": "lox_injector", "type": "orifice", "diameter": f"{q['d_inj_o']:.4f} mm", "cd": 0.7},
        {"id": "lox_shaft", "type": "shaft", "inertia": "2.1e-4 kg m2", "N0": "0 rpm", "bearing_torque": "0.05 N m",
         "viscous_loss": round((1 / cfg.eta_mech - 1) * o.pump.power / (o.pump.rpm * math.pi / 30) ** 2, 8)},
    ]
    conns = [["lh2_tank", "fuel_pump.in"], ["fuel_pump.out", "fuel_line.in"], ["fuel_line.out", "mfv.in"], ["mfv.out", "jacket_h2_in"],
             ["jacket_h2_in", "jacket_h2.in"], ["jacket_h2.out", "fturb_in"], ["fturb_in", "fuel_turbine.in"],
             ["fturb_in", "ftbv.in"], ["fuel_turbine.out", "fturb_out"], ["ftbv.out", "fturb_out"],
             ["fturb_out", "fuel_injector.in"], ["fuel_injector.out", "tca.fuel"],
             ["lox_tank", "lox_pump.in"], ["lox_pump.out", "lox_line.in"], ["lox_line.out", "mov.in"], ["mov.out", "lox_split"],
             ["lox_split", "jacket_o2.in"], ["jacket_o2.out", "oturb_in"], ["oturb_in", "lox_turbine.in"],
             ["oturb_in", "otbv.in"], ["lox_turbine.out", "ox_manifold"], ["otbv.out", "ox_manifold"],
             ["lox_split", "ocv.in"], ["ocv.out", "ox_manifold"], ["ox_manifold", "lox_injector.in"],
             ["lox_injector.out", "tca.ox"]]
    if init == "design":       # initial states near the design point, so a steady solve converges without a warm start
        bar = lambda v: f"{v / 1e5:.1f} bar"
        guess = {"jacket_h2_in": (f.p_zone_in, 60.0), "jacket_h2": (f.p_turb_in, 0.6 * f.T_turb_in + 20.0),
                 "fturb_in": (f.p_turb_in, f.T_turb_in), "fturb_out": (f.p_turb_out, f.T_inj),
                 "lox_split": (o.p_zone_in, o.pump.T_out + 5.0), "jacket_o2": (o.p_turb_in, 0.6 * o.T_turb_in + 30.0),
                 "oturb_in": (o.p_turb_in, o.T_turb_in), "ox_manifold": (o.p_turb_out, o.T_inj)}
        for c in comps:
            if c["id"] in guess:
                c["P0"], c["T0"] = bar(guess[c["id"]][0]), f"{guess[c['id']][1]:.0f} K"
            if "wall_T0" in c:
                c["wall_T0"] = "400 K"
            if c["id"] == "tca":
                c["P0"] = bar(cfg.pc)
            if c["id"] == "fuel_shaft":
                c["N0"] = f"{f.pump.rpm:.0f} rpm"
            if c["id"] == "lox_shaft":
                c["N0"] = f"{o.pump.rpm:.0f} rpm"
    return {"name": f"{cfg.name} powerpack ({npk} chambers lumped)",
            "description": f"Generated by dexfm from configuration {cfg.sha()}. One powerpack of the dual expander engine.",
            "components": comps, "connections": conns, "simulation": {"t_end": 6, "output_dt": 0.005, "rtol": 1e-4}}


def set_speeds(model: dict, r: CycleResult) -> dict:
    m = copy.deepcopy(model)
    for c in m["components"]:
        if c["id"] == "fuel_shaft": c["speed"] = f"{r.fuel.pump.rpm:.0f} rpm"
        if c["id"] == "lox_shaft": c["speed"] = f"{r.ox.pump.rpm:.0f} rpm"
    return m


TARGET_KEYS = ["tca.Pc", "tca.OF", "byp_f", "byp_o", "jacket_h2.Q", "jacket_o2.Q", "fturb_in.P", "oturb_in.P", "x_heated",
               "torque_f", "torque_o", "fturb_out.P", "ox_manifold.P"]
PARAM_KEYS = ["d_inj_f", "d_inj_o", "d_turb_f", "d_turb_o", "htc_f", "htc_o", "d_ch_f", "d_ch_o", "d_ocv", "eff_tf", "eff_to",
              "d_tbv_f", "d_tbv_o"]


def observe(res) -> dict:
    g = res.last
    mf = g("fuel_injector.mdot"); mo = g("lox_injector.mdot")
    return {"tca.Pc": g("tca.Pc"), "tca.OF": g("tca.OF"), "byp_f": g("ftbv.mdot") / mf,
            "byp_o": g("otbv.mdot") / max(g("jacket_o2.mdot") if "jacket_o2.mdot" in res else g("otbv.mdot") + g("lox_turbine.mdot"), 1e-9),
            "jacket_h2.Q": g("jacket_h2.Q"), "jacket_o2.Q": g("jacket_o2.Q"), "fturb_in.P": g("fturb_in.P"),
            "oturb_in.P": g("oturb_in.P"), "x_heated": (g("otbv.mdot") + g("lox_turbine.mdot")) / mo,
            "torque_f": g("fuel_turbine.torque") / g("fuel_pump.torque"), "torque_o": g("lox_turbine.torque") / g("lox_pump.torque"),
            "fturb_out.P": g("fturb_out.P"), "ox_manifold.P": g("ox_manifold.P")}


def targets(r: CycleResult) -> dict:
    cfg = r.setup.cfg; npk = cfg.n_chambers // cfg.n_packs
    return {"tca.Pc": cfg.pc, "tca.OF": cfg.of, "byp_f": cfg.bypass_f, "byp_o": cfg.bypass_o,
            "jacket_h2.Q": r.fuel.zone.Q * npk, "jacket_o2.Q": r.ox.zone.Q * npk, "fturb_in.P": r.fuel.p_turb_in,
            "oturb_in.P": r.ox.p_turb_in, "x_heated": r.ox.heated_fraction, "torque_f": 1.0 / cfg.eta_mech, "torque_o": 1.0 / cfg.eta_mech,
            "fturb_out.P": r.fuel.p_turb_out, "ox_manifold.P": r.ox.p_turb_out}


def calibrate(r: CycleResult, verbose: bool = False):
    """Fit the calibration parameters so the fluidnet steady state matches the dexfm design point."""
    import fluidnet as fn
    su = r.setup; pm = build_pack(r); f, o = r.fuel, r.ox
    x0 = np.array([_d(pm.a_if / 0.8), _d(pm.a_io / 0.7), _d(pm.fturb.cda / 0.95), _d(pm.oturb.cda / 0.95), 0.8, 0.8, 2.2, 3.0,
                   _d(pm.a_ocv / 0.8), f.turbine.eta, o.turbine.eta, _d(pm.a_bf_d / 0.8), _d(pm.a_bo_d / 0.8)])
    tg = targets(r); state = {"init": None}

    def run(x):
        m = set_speeds(build_model(r, dict(zip(PARAM_KEYS, x))), r)
        res = fn.run(m, mode="steady", init=state["init"]) if state["init"] is not None else fn.run(m, mode="steady")
        return res

    def resid(xs):
        x = xs * x0
        try:
            res = run(x)
        except RuntimeError as e:
            if verbose: print("  failed:", str(e)[:100])
            return np.full(len(TARGET_KEYS), 5.0)
        ob = observe(res)
        out = np.array([ob[k] / tg[k] - 1 for k in TARGET_KEYS])
        if verbose: print("  |res|max %.4f" % np.abs(out).max(), flush=True)
        if np.abs(out).max() < 0.5:
            state["init"] = res
        return out

    sol = least_squares(resid, np.ones(len(x0)), bounds=(0.3, 3.0), diff_step=2e-3, xtol=1e-8, max_nfev=400)
    params = dict(zip(PARAM_KEYS, (sol.x * x0).tolist()))
    return params, float(np.abs(sol.fun).max())


TBV_RANGE = 16.0      # bypass valves are built with this many times their design flow area


def start_model(r: CycleResult, params: dict, seq: dict, t_end: float = 6.0, T_wall0: float = 250.0) -> dict:
    """Transient model of a tank-head start. `seq` holds the valve schedules and the ignition time.

    Bypass valve schedules are given relative to the design flow area (1.0 = design position).
    """
    m = build_model(r, params, init="cold")
    k = math.sqrt(TBV_RANGE)
    for c in m["components"]:
        cid = c["id"]
        if cid in ("ftbv", "otbv"):
            c["diameter"] = f"{float(c['diameter'].split()[0]) * k:.4f} mm"
            if isinstance(seq[cid], dict):               # pressure-actuated (closed-loop) valve: pass fluidnet keys through
                c.update(seq[cid])
            else:
                c["opening"] = [[t, min(v, TBV_RANGE) / TBV_RANGE] for t, v in seq[cid]]
        elif cid in ("mfv", "mov", "ocv"):
            c["opening"] = seq[cid]
            if "char_" + cid in seq:
                c["characteristic"] = seq["char_" + cid]
        elif cid == "tca":
            c["ignition_time"] = seq["ignition"]; c["ignition_ramp"] = "50 ms"
        if "wall_T0" in c:
            c["wall_T0"] = f"{T_wall0} K"
    m["simulation"] = {"t_end": t_end, "output_dt": 0.005, "rtol": 1e-4}
    m["name"] += " start"
    return m


def summarize_start(res, r: CycleResult, dt_print: float = 0.25) -> list[dict]:
    t = res.time
    rows = []
    for tt in np.arange(0.0, t[-1] + 1e-9, dt_print):
        i = int(np.searchsorted(t, tt))
        i = min(i, len(t) - 1)
        g = lambda k: float(res[k][i])
        rows.append(dict(t=float(t[i]), n_f=g("fuel_shaft.speed") * 30 / math.pi / r.fuel.pump.rpm,
                         n_o=g("lox_shaft.speed") * 30 / math.pi / r.ox.pump.rpm, pc=g("tca.Pc") / 1e5, of=g("tca.OF"),
                         mf=g("fuel_injector.mdot"), mo=g("lox_injector.mdot"), T_tf=g("fturb_in.T"), T_to=g("oturb_in.T"),
                         Tw_f=g("jacket_h2.Tw_max"), Tw_o=g("jacket_o2.Tw_max")))
    return rows


# Start sequence that reaches mainstage in the fluidnet model (liner at 250 K). Bypass valve values are
# relative to the design flow area. The oxygen turbine bypass is closed-loop: it opens with the difference
# between oxygen and fuel pump discharge pressure, so the oxygen shaft follows the fuel shaft.
DEFAULT_START = dict(
    mfv=[[0, 0.02], [0.2, 1]], ignition=0.5,
    mov=[[0, 0.003], [0.35, 0.003], [0.6, 1]],
    ftbv=[[0, 3], [1.0, 3], [3.0, 1]],
    otbv=dict(sense="lox_pump.out", sense_ref="fuel_pump.out", sense_lo="-22 bar", sense_hi="10 bar",
              opening_lo=0.0, opening_hi=1.0, actuator_time="30 ms", opening=0.69),
    ocv=[[0, 0.003], [0.6, 0.003], [2.5, 1]])


def run_start(r: CycleResult, params: dict, seq: dict | None = None, **kw):
    import fluidnet as fn
    return fn.run(start_model(r, params, seq or DEFAULT_START, **kw))
