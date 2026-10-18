"""Size the closed-expander example with the fluidnet Python API.

Finds the fuel injector, LOX injector and turbine nozzle diameters that give
the target chamber pressure, mixture ratio and a balanced shaft at the design
speed. Each evaluation is a steady state with the shaft speed prescribed (a
well-conditioned problem); the result is then checked with the shaft free.

Usage:  python tools/size_expander.py examples/expander_cycle.yaml
"""
import sys

import numpy as np
from scipy.optimize import least_squares

import fluidnet as fn

PC, OF, SPEED, RATIO_LOX = 40e5, 5.8, "50000 rpm", 0.3

base = fn.load(sys.argv[1])


def model(x, prescribed=True):
    m = fn.load(sys.argv[1])
    fn.set_param(m, "fuel_injector", "diameter", f"{x[0]:.4f} mm")
    fn.set_param(m, "lox_injector", "diameter", f"{x[1]:.4f} mm")
    fn.set_param(m, "turbine", "diameter", f"{x[2]:.4f} mm")
    if prescribed:
        fn.set_param(m, "turbopump", "speed", SPEED)
    return m


def residual(x):
    try:
        r = fn.run(model(x), mode="steady")
    except RuntimeError as e:
        print("  failed:", e)
        return [10.0, 10.0, 10.0]
    load = r.last("fuel_pump.torque") + RATIO_LOX * r.last("lox_pump.torque")
    drive = r.last("turbine.torque")
    print(f"  d_f {x[0]:6.2f} d_o {x[1]:6.2f} d_t {x[2]:6.2f} mm -> Pc {r.last('tca.Pc')/1e5:6.2f} bar  "
          f"O/F {r.last('tca.OF'):5.2f}  turbine {drive:6.2f} / pumps {load:6.2f} N m", flush=True)
    return [r.last("tca.Pc") / PC - 1, r.last("tca.OF") / OF - 1, (drive - load) / load]


x0 = [float(str(fn.get_param(base, c, "diameter")).split()[0]) for c in ("fuel_injector", "lox_injector", "turbine")]
sol = least_squares(residual, x0, bounds=([4, 4, 4], [40, 40, 40]), diff_step=1e-3, xtol=1e-6)
print("diameters [mm]:", np.round(sol.x, 3))
free = fn.run(model(sol.x, prescribed=False), mode="steady")
for k in ["tca.Pc", "tca.OF", "tca.thrust", "tca.Isp", "turbopump.speed", "turbine_inlet.T", "turbine.pr", "jacket.Q", "jacket.Tw_max"]:
    print(f"  {k:18s} {free.last(k):.5g} {free.units[k]}")
