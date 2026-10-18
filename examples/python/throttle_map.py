"""Throttle map of the closed-expander example: steady states over the turbine
bypass valve opening, solved in parallel threads, plus the throttling transient.

    pip install fluidnet[plot]   (or build with maturin, see README)
    python examples/python/throttle_map.py
"""
from pathlib import Path

import matplotlib.pyplot as plt
import numpy as np

import fluidnet as fn

here = Path(__file__).resolve().parents[1]
model = fn.load(here / "expander_cycle.yaml")
fn.set_param(model, "tbv", "opening", 0.0586)  # constant opening for steady states

openings = np.linspace(0.0586, 1.0, 12)
runs = fn.sweep(model, "tbv", "opening", list(openings), mode="steady")
thrust = [r.last("tca.thrust") / 1e3 for r in runs]
of = [r.last("tca.OF") for r in runs]
speed = [r.last("turbopump.speed") * 30 / np.pi / 1e3 for r in runs]

fig, ax = plt.subplots(1, 2, figsize=(11, 4))
ax[0].plot(openings, thrust, "o-", label="thrust [kN]")
ax[0].plot(openings, speed, "s-", label="shaft speed [krpm]")
ax[0].plot(openings, of, "^-", label="O/F [-]")
ax[0].set_xlabel("turbine bypass valve opening [-]")
ax[0].legend()
ax[0].grid(alpha=0.3)

tr = fn.run(here / "expander_cycle.yaml")  # throttling transient from the design point
tr.plot("tca.Pc", ax=ax[1])
ax[1].set_title(f"TBV transient ({tr.elapsed * 1e3:.0f} ms to solve)")
plt.tight_layout()
plt.savefig("throttle_map.png", dpi=150)
print("wrote throttle_map.png")
