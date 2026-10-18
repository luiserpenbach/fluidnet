"""RL10A-3-3A validation against the NASA CR-190786 test points.

Steps (python validation/rl10/rl10.py [calibrate|predict|all]):

1. Calibrate at test point 1 (TP1). Eight unknowns are solved together:
   - per-point controls: OCV opening (sets O/F), TCV opening (sets thrust);
   - model factors: hot-gas HTC factor, jacket throat-region loss K, turbine
     nozzle area, fuel and LOX injector discharge coefficients, fuel pump
     head factor, turbine peak efficiency.
   Targets: O/F and thrust (test inputs), turbine inlet temperature, the
   pressure drops pump discharge -> venturi inlet -> fuel manifold -> chamber
   and LOX plenum -> chamber, and the LOX pump speed. The turbine efficiency
   and TCV bypass that result are compared with the TM-107318 typical point
   (0.63 and 1.39 %). The nozzle thrust efficiency is set
   from the TM-107318 typical point (482 psia, O/F 5.26, 16,452 lbf).
2. Predict TP1..TP5: with the factors frozen, only the OCV and TCV openings
   are solved per point to match the test O/F and thrust (as done for ROCETS
   in CR-190786), and the ten measured parameters are compared.

Writes results.json next to this file and the calibrated factors into
examples/rl10a33a.yaml.
"""
from __future__ import annotations

import json
import math
import re
import sys
import time
from pathlib import Path

import numpy as np
from scipy.optimize import least_squares

import fluidnet as fn

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
MODEL = ROOT / "examples" / "rl10a33a.yaml"

PSI = 6894.757293
RANK = 5.0 / 9.0
LBF = 4.4482216152605
LBM = 0.45359237
GPM = 3.785411784e-3 / 60.0
RPM = 2.0 * math.pi / 60.0
IN2 = 0.0254**2

DATA = json.loads((HERE / "test_data.json").read_text())
PARAMS = list(DATA["params"])
TYP = DATA["typical_point"]
BYPASS_TARGET = (TYP["jacket_flow_lbs"] - TYP["turbine_flow_lbs"]) / TYP["jacket_flow_lbs"]
CF_TARGET = TYP["thrust"] * LBF / (TYP["chamber_P"] * PSI * 18.85 * IN2)

JACKET = ("jacket_down", "jacket_up_nozzle", "jacket_up_chamber")
FUEL_DP0 = {"fuel_pump_1": 522.65 * PSI, "fuel_pump_2": 545.88 * PSI}

# calibration vector: name, initial value, lower, upper
CAL = [
    ("htc_factor", 1.0, 0.3, 3.0),
    ("jacket_K", 5.0, 0.0, 60.0),
    ("turbine_area", 1.2 * IN2, 0.3 * IN2, 4.0 * IN2),
    ("fuel_inj_cd", 0.95, 0.3, 1.5),
    ("lox_inj_cd", 0.75, 0.2, 1.5),
    ("head_factor", 1.0, 0.7, 1.3),
    ("turbine_eff", 0.633, 0.3, 0.95),
    ("cf_eff", 0.93, 0.8, 1.05),
]
# TM-107318 typical point: turbine PR 1.39, dT 24.5 R at about 420 R gives
# efficiency 0.626 at u/c0 0.314 (peak 0.633 at u/c0 0.35). Reported for comparison.
TURBINE_EFF_PUBLISHED = 0.633
# TCV position at TP1 (not measured): fully open. The published typical-point
# bypass (1.39 % of jacket flow) is about the capacity of the published TCV
# area (0.0158 in2), so the valve runs near full open.
TCV_TP1 = 1.0
CTRL = [("ocv", 0.5, 0.0, 1.0), ("tcv", 0.6, 0.0, 1.0)]


_REF = None
# sensitivity option: fuel pump head-curve shape (shutoff head / design head);
# None keeps the model default (1.15, generic)
SHUTOFF_HEAD = None


class Engine:
    """Model plus warm-started steady solves."""

    def __init__(self, cal: dict, cf_eff: float):
        self.base = fn.load(MODEL)
        self.cal = dict(cal)
        self.cf_eff = cf_eff
        self.last = None
        self.evals = 0
        self.t_solve = 0.0

    def model(self, tp: dict, ctrl: dict) -> dict:
        m = json.loads(json.dumps(self.base))
        c = self.cal
        fn.set_param(m, "lh2_tank", "P", tp["fuel_tank_P"] * PSI)
        fn.set_param(m, "lh2_tank", "T", tp["fuel_tank_T"] * RANK)
        fn.set_param(m, "lox_tank", "P", tp["lox_tank_P"] * PSI)
        fn.set_param(m, "lox_tank", "T", tp["lox_tank_T"] * RANK)
        for j in JACKET:
            fn.set_param(m, j, "hot_htc_factor", c["htc_factor"])
        fn.set_param(m, "jacket_up_chamber", "K", c["jacket_K"])
        fn.set_param(m, "turbine", "area", c["turbine_area"])
        fn.set_param(m, "turbine", "design_eff", c["turbine_eff"])
        fn.set_param(m, "fuel_injector", "cd", c["fuel_inj_cd"])
        fn.set_param(m, "lox_injector", "cd", c["lox_inj_cd"])
        for p, dp in FUEL_DP0.items():
            fn.set_param(m, p, "design_dp", dp * c["head_factor"])
            if SHUTOFF_HEAD is not None:
                fn.set_param(m, p, "shutoff_head", SHUTOFF_HEAD)
        fn.set_param(m, "tca", "cf_eff", self.cf_eff)
        fn.set_param(m, "mr_valve", "opening", float(ctrl["ocv"]))
        fn.set_param(m, "tcv", "opening", float(ctrl["tcv"]))
        return m

    def reference(self) -> "fn.Results":
        """A converged state of the model file to start from (cold steady
        solves of this 74-state model are sensitive to the initial guess)."""
        global _REF
        if _REF is None:
            m = fn.load(MODEL)
            fn.set_param(m, "tcv", "opening", 0.6)
            fn.set_param(m, "mr_valve", "opening", 0.3)
            _REF = fn.run(m, mode="steady")
        return _REF

    def solve(self, tp: dict, ctrl: dict) -> dict:
        m = self.model(tp, ctrl)
        t0 = time.perf_counter()
        r = None
        for init in (self.last, self.reference()):
            try:
                r = fn.run(m, mode="steady", init=init)
                break
            except RuntimeError:
                r = None
        self.t_solve += time.perf_counter() - t0
        self.evals += 1
        if r is None:
            raise RuntimeError("steady state failed")
        self.last = r
        return outputs(r, self.cf_eff)


def outputs(r: "fn.Results", cf_eff: float) -> dict:
    g = r.last
    fuel_rho = g("lh2_tank.rho")
    lox_rho = g("lox_tank.rho")
    o = {
        "fuel_pump_discharge_P": g("fuel_pump_discharge.P") / PSI,
        "fuel_pump_inlet_flow": g("fuel_pump_1.mdot") / fuel_rho / GPM,
        "venturi_inlet_P": g("venturi_inlet.P") / PSI,
        "turbine_inlet_T": g("turbine_inlet.T") / RANK,
        "fuel_manifold_P": g("fuel_manifold.P") / PSI,
        "lox_pump_discharge_P": g("lox_discharge.P") / PSI,
        "lox_pump_speed": g("lox_pump.speed") / RPM,
        "lox_pump_inlet_flow": g("lox_pump.mdot") / lox_rho / GPM,
        "lox_plenum_P": g("lox_plenum.P") / PSI,
        "chamber_P": g("tca.Pc") / PSI,
        "thrust": g("tca.thrust") / LBF,
        "of": g("tca.OF"),
        "bypass": g("tcv.mdot") / g("venturi.mdot"),
        "fuel_pump_speed": g("turbopump.speed") / RPM,
        "jacket_Q": sum(g(f"{j}.Q") for j in JACKET),
        "jacket_dp": (g("jacket_inlet.P") - g("venturi_inlet.P")) / PSI,
        "turbine_pr": g("turbine.pr"),
        "turbine_eff": g("turbine.eff"),
        "tw_max": max(g(f"{j}.Tw_max") for j in JACKET) / RANK,
        "cf_ideal": g("tca.thrust") / (g("tca.Pc") * 18.85 * IN2) / cf_eff,
        "isp": g("tca.Isp"),
        "Tc": g("tca.Tc"),
        "cstar": g("tca.cstar"),
        "jacket_in_T": g("jacket_inlet.T") / RANK,
        "mdot": g("tca.mdot") / LBM,
    }
    return o


def match_controls(eng: Engine, tp: dict, ctrl0: dict, of: float, thrust: float) -> tuple[dict, dict]:
    """Solve OCV and TCV openings for the given O/F and thrust."""

    def res(x):
        try:
            o = eng.solve(tp, {"ocv": x[0], "tcv": x[1]})
        except RuntimeError:
            return np.array([10.0, 10.0])
        return np.array([(o["of"] - of) / of * 100, (o["thrust"] - thrust) / thrust * 100])

    x0 = np.array([ctrl0["ocv"], ctrl0["tcv"]])
    sol = least_squares(res, x0, bounds=([0.0, 0.0], [1.0, 1.0]), diff_step=2e-3, xtol=1e-10, ftol=1e-10)
    ctrl = {"ocv": sol.x[0], "tcv": sol.x[1]}
    return ctrl, eng.solve(tp, ctrl)


def calibrate(verbose=True) -> dict:
    tp = DATA["points"][0]
    t = tp["test"]
    cal = {n: v for n, v, _, _ in CAL}
    ctrl = {n: v for n, v, _, _ in CTRL}
    ctrl["tcv"] = TCV_TP1
    names = ["ocv"] + [c[0] for c in CAL]
    lo = [0.0] + [c[2] for c in CAL]
    hi = [1.0] + [c[3] for c in CAL]
    scale = np.array([0.1, 0.1, 1.0, 0.1 * IN2, 0.05, 0.05, 0.02, 0.02, 0.01])
    targets = {
        "of": tp["of"],
        "thrust": tp["thrust"],
        "turbine_inlet_T": t["turbine_inlet_T"],
        "dp_jacket": t["fuel_pump_discharge_P"] - t["venturi_inlet_P"],
        "dp_turbine": t["venturi_inlet_P"] - t["fuel_manifold_P"],
        "dp_fuel_inj": t["fuel_manifold_P"] - t["chamber_P"],
        "dp_lox_inj": t["lox_plenum_P"] - t["chamber_P"],
        "lox_pump_speed": t["lox_pump_speed"],
        "chamber_P": t["chamber_P"],
    }
    eng = Engine(cal, cal["cf_eff"])

    def res(x):
        c = dict(zip(names, x))
        eng.cal = {k: c[k] for k, *_ in CAL}
        eng.cf_eff = c["cf_eff"]
        try:
            o = eng.solve(tp, {"ocv": c["ocv"], "tcv": TCV_TP1})
        except RuntimeError:
            return np.full(len(targets), 10.0)
        got = {
            "of": o["of"],
            "thrust": o["thrust"],
            "turbine_inlet_T": o["turbine_inlet_T"],
            "dp_jacket": o["fuel_pump_discharge_P"] - o["venturi_inlet_P"],
            "dp_turbine": o["venturi_inlet_P"] - o["fuel_manifold_P"],
            "dp_fuel_inj": o["fuel_manifold_P"] - o["chamber_P"],
            "dp_lox_inj": o["lox_plenum_P"] - o["chamber_P"],
            "lox_pump_speed": o["lox_pump_speed"],
            "chamber_P": o["chamber_P"],
        }
        return np.array([(got[k] - v) / abs(v) * 100 for k, v in targets.items()])

    x0 = np.array([ctrl[n] if n in ctrl else cal[n] for n in names])
    sol = least_squares(res, x0, bounds=(lo, hi), x_scale=scale, diff_step=1e-3, xtol=1e-12, ftol=1e-12, max_nfev=600)
    c = dict(zip(names, sol.x))
    cal = {k: c[k] for k, *_ in CAL}
    ctrl = {"ocv": c["ocv"], "tcv": TCV_TP1}
    if verbose:
        print(f"calibration residual % {np.round(sol.fun, 4)}  ({eng.evals} solves, {eng.t_solve:.1f} s)")
        print("  " + ", ".join(f"{n} {v:.4g}" for n, v in zip(names, sol.x)))
    # cross-check against the TM-107318 typical point (O/F 5.26, 16,452 lbf)
    _, o = match_controls(eng, tp, dict(ctrl), TYP["of"], TYP["thrust"])
    typical = {"chamber_P": o["chamber_P"], "cf": o["thrust"] * LBF / (o["chamber_P"] * PSI * 18.85 * IN2),
               "bypass": o["bypass"], "turbine_pr": o["turbine_pr"], "jacket_Q_btus": o["jacket_Q"] / 1055.056,
               "jacket_dp": o["jacket_dp"], "mdot": o["mdot"]}
    if verbose:
        print("  at typical-point O/F and thrust: " + ", ".join(f"{k} {v:.4g}" for k, v in typical.items()))
    return {"cal": cal, "ctrl_tp1": ctrl, "cf_eff": cal["cf_eff"], "residual_pct": dict(zip(targets, sol.fun.tolist())),
            "targets": targets, "typical_check": typical}


def predict(calres: dict, verbose=True) -> list:
    eng = Engine(calres["cal"], calres["cf_eff"])
    ctrl = dict(calres["ctrl_tp1"])
    out = []
    for tp in DATA["points"]:
        ctrl, o = match_controls(eng, tp, dict(calres["ctrl_tp1"]), tp["of"], tp["thrust"])
        row = {"id": tp["id"], "ctrl": ctrl, "model": o, "test": tp["test"], "rocets_pct": tp["rocets_pct"]}
        row["err_pct"] = {k: (o[k] - tp["test"][k]) / tp["test"][k] * 100 for k in PARAMS}
        out.append(row)
        if verbose:
            print(f"{tp['id']}: OCV {ctrl['ocv']:.3f} TCV {ctrl['tcv']:.3f} | O/F {o['of']:.3f} F {o['thrust']:.0f} lbf | "
                  + " ".join(f"{k.split('_')[0][:4]}{v:+.2f}" for k, v in row["err_pct"].items()))
    return out


def write_model(calres: dict):
    """Store the calibrated factors and the TP1 valve settings in the example model."""
    s = MODEL.read_text()
    c = calres["cal"]

    def setp(comp, key, val, fmt="{:.5g}"):
        nonlocal s
        pat = re.compile(r"(\{id: %s, [^\n]*?\b%s: )([^,}]+)" % (re.escape(comp), re.escape(key)))
        s, n = pat.subn(lambda mo: mo.group(1) + fmt.format(val), s)
        assert n == 1, (comp, key)

    for j in JACKET:
        setp(j, "hot_htc_factor", c["htc_factor"])
    setp("jacket_up_chamber", "K", c["jacket_K"])
    setp("turbine", "area", c["turbine_area"] / IN2, "{:.5g} in2")
    setp("fuel_injector", "cd", c["fuel_inj_cd"])
    setp("lox_injector", "cd", c["lox_inj_cd"])
    for p, dp in FUEL_DP0.items():
        setp(p, "design_dp", dp * c["head_factor"] / PSI, "{:.2f} psi")
    setp("turbine", "design_eff", c["turbine_eff"])
    setp("tca", "cf_eff", calres["cf_eff"])
    setp("mr_valve", "opening", calres["ctrl_tp1"]["ocv"], "{:.4f}")
    setp("tcv", "opening", calres["ctrl_tp1"]["tcv"], "{:.4f}")
    MODEL.write_text(s)


def summary(rows: list) -> dict:
    """Error statistics: all points, predictions only (TP2-5), and after removing the TP1 offset."""
    def stats(vals):
        a = np.abs(np.array(vals))
        return {"n": int(a.size), "mean_abs": float(a.mean()), "rms": float(np.sqrt((a**2).mean())), "max_abs": float(a.max()),
                "within_1": float((a <= 1.0).mean()), "within_2": float((a <= 2.0).mean()), "within_3": float((a <= 3.0).mean())}

    pred = [r for r in rows if r["id"] != "TP1"]
    tp1 = rows[0]
    out = {}
    out["fluidnet_tp2_5"] = stats([r["err_pct"][k] for r in pred for k in PARAMS])
    out["rocets_tp2_5"] = stats([r["rocets_pct"][k] for r in pred for k in PARAMS])
    out["fluidnet_tp1"] = stats([tp1["err_pct"][k] for k in PARAMS])
    out["rocets_all"] = stats([r["rocets_pct"][k] for r in rows for k in PARAMS])
    # offset-removed: both models anchored to the measured TP1 value
    fl, ro = [], []
    for r in pred:
        for k in PARAMS:
            t1, tk = tp1["test"][k], r["test"][k]
            m1, mk = tp1["model"][k], r["model"][k]
            r1 = t1 * (1 + tp1["rocets_pct"][k] / 100)
            rk = tk * (1 + r["rocets_pct"][k] / 100)
            fl.append(((mk - m1 + t1) - tk) / tk * 100)
            ro.append(((rk - r1 + t1) - tk) / tk * 100)
    out["fluidnet_anchored"] = stats(fl)
    out["rocets_anchored"] = stats(ro)
    per = {}
    for k in PARAMS:
        per[k] = {
            "fluidnet_mean_abs": float(np.mean([abs(r["err_pct"][k]) for r in pred])),
            "rocets_mean_abs": float(np.mean([abs(r["rocets_pct"][k]) for r in pred])),
            "fluidnet_max_abs": float(np.max([abs(r["err_pct"][k]) for r in pred])),
        }
    out["per_param"] = per
    return out


def main():
    global SHUTOFF_HEAD
    what = sys.argv[1] if len(sys.argv) > 1 else "all"
    t0 = time.time()
    res_file = HERE / "results.json"
    if what == "headcurve":
        # sensitivity: fuel pump head curve as flat as the measured TP1-TP4 head coefficients
        SHUTOFF_HEAD = 1.03
        res_file = HERE / "results_headcurve.json"
        calres = calibrate()
    elif what in ("calibrate", "all"):
        calres = calibrate()
        write_model(calres)
    else:
        calres = json.loads(res_file.read_text())["calibration"]
    rows = predict(calres)
    stats = summary(rows)
    res_file.write_text(json.dumps({"calibration": calres, "points": rows, "stats": stats, "shutoff_head": SHUTOFF_HEAD}, indent=1))
    s = stats
    print(f"\nTP2-TP5 predictions, {s['fluidnet_tp2_5']['n']} values:")
    for key, lab in (("fluidnet_tp2_5", "fluidnet"), ("rocets_tp2_5", "ROCETS  "), ("fluidnet_anchored", "fluidnet, TP1 offset removed"), ("rocets_anchored", "ROCETS,   TP1 offset removed")):
        v = s[key]
        print(f"  {lab:30s} mean |e| {v['mean_abs']:.2f} %  rms {v['rms']:.2f} %  max {v['max_abs']:.2f} %  within 1 % {v['within_1']*100:.0f} %")
    print(f"done in {time.time() - t0:.1f} s")


if __name__ == "__main__":
    main()
