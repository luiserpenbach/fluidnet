"""Size the expander-cycle example: find injector and turbine nozzle diameters
that give the target chamber pressure, mixture ratio and a balanced shaft at
the design speed. Uses the fluidnet CLI steady solver with the shaft speed
prescribed, so each evaluation is a well-conditioned steady state.

Usage: python tools/size_expander.py examples/expander_cycle.yaml
"""
import json
import re
import subprocess
import sys
import tempfile

import numpy as np
from scipy.optimize import least_squares

SRC = open(sys.argv[1]).read()
BIN = 'target/release/fluidnet'
TARGET = dict(Pc=40e5, OF=5.8)
SPEED = '50000 rpm'
RATIO_LOX = 0.3


def set_param(text, comp, key, value):
    line = next(l for l in text.splitlines() if f'{{id: {comp},' in l)
    if re.search(rf'\b{key}: [^,}}]+', line):
        new = re.sub(rf'\b{key}: [^,}}]+', f'{key}: {value}', line)
    else:
        new = line.replace(f'{{id: {comp},', f'{{id: {comp}, {key}: {value},')
    return text.replace(line, new)


def model(x, prescribed=True):
    d_f, d_o, d_t = x
    t = SRC
    t = set_param(t, 'fuel_injector', 'diameter', f'{d_f:.4f} mm')
    t = set_param(t, 'lox_injector', 'diameter', f'{d_o:.4f} mm')
    t = set_param(t, 'turbine', 'diameter', f'{d_t:.4f} mm')
    if prescribed:
        t = t.replace('N0: 0 rpm', f'speed: {SPEED}')
    return t


def solve(text):
    with tempfile.NamedTemporaryFile('w', suffix='.yaml', delete=False) as f:
        f.write(text)
    out = f.name + '.json'
    r = subprocess.run([BIN, 'run', f.name, '--steady', '--out', out], capture_output=True, text=True)
    if r.returncode != 0:
        raise RuntimeError(r.stderr.strip().splitlines()[-1])
    res = json.load(open(out))
    return {c['name']: c['data'][-1] for c in res['channels']}


def residual(x):
    try:
        v = solve(model(x))
    except RuntimeError as e:
        print('  failed:', e)
        return [10.0, 10.0, 10.0]
    load = v['fuel_pump.torque'] + RATIO_LOX * v['lox_pump.torque']
    drive = v['turbine.torque']
    r = [v['tca.Pc'] / TARGET['Pc'] - 1, v['tca.OF'] / TARGET['OF'] - 1, (drive - load) / load]
    print(f'  d_f {x[0]:6.2f} d_o {x[1]:6.2f} d_t {x[2]:6.2f} mm -> Pc {v["tca.Pc"]/1e5:6.2f} bar  O/F {v["tca.OF"]:5.2f}  '
          f'turbine {drive:6.2f} / pumps {load:6.2f} N m  T_in {v["turbine.in.T"]:6.1f} K  PR {v["turbine.pr"]:.3f}', flush=True)
    return r


x0 = [14.0, 14.0, 12.0]
sol = least_squares(residual, x0, bounds=([4, 4, 4], [40, 40, 40]), diff_step=1e-3, xtol=1e-6, ftol=1e-8)
print('solution (mm):', np.round(sol.x, 3), 'residual', np.round(sol.fun, 5))
v = solve(model(sol.x))
for k in ['tca.Pc', 'tca.OF', 'tca.thrust', 'tca.Isp', 'tca.mdot', 'turbine.in.T', 'turbine.in.P', 'turbine.pr', 'turbine.eff',
          'turbine.u_c0', 'turbine.power', 'fuel_pump.power', 'lox_pump.power', 'fuel_pump.dp', 'lox_pump.dp', 'jacket.Q', 'jacket.Tw_max', 'tbv.mdot', 'turbine.mdot']:
    print(f'  {k:18s} {v[k]:.5g}')
open(sys.argv[1] + '.sized', 'w').write(model(sol.x, prescribed=False))
