"""Check plots: digitized points over the scans, and measured vs generic curves."""
import json
from pathlib import Path

import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
import numpy as np
from PIL import Image

from digitize import FIGS, digitize

HERE = Path(__file__).resolve().parent
maps = json.loads((HERE / "rl10_maps.json").read_text())

# 1. overlays: digitized points (red) on the deskewed scan crops
fig, axs = plt.subplots(2, 4, figsize=(20, 9))
used = {"4.1.1": maps["pumps"]["fuel_pump_1"]["head_points"], "4.1.3": maps["pumps"]["fuel_pump_1"]["eff_points"],
        "4.1.2": maps["pumps"]["fuel_pump_2"]["head_points"], "4.1.4": maps["pumps"]["fuel_pump_2"]["eff_points"],
        "4.1.5": maps["pumps"]["fuel_pump_1"]["speed_points"], "4.1.6": maps["pumps"]["lox_pump"]["head_points"],
        "4.1.7": maps["pumps"]["lox_pump"]["eff_points"], "4.1.8": maps["pumps"]["lox_pump"]["speed_points"]}
for ax, (name, f) in zip(axs.flat, FIGS.items()):
    d = digitize(name, f)
    from scipy import ndimage
    page = np.array(Image.open(HERE / "scans" / f"p{f['page']}.png").convert("L")) < 128
    x0, y0, x1, y1 = f["box"]
    r = ndimage.rotate(page[y0:y1, x0:x1].astype(np.float32), d["angle"], reshape=False, order=1) > 0.5
    ax.imshow(~r, cmap="gray")
    # pixel positions of the points that were kept
    keep = {(round(p["x"], 4), round(p["y"], 4)): p["px"] for p in d["points"]}
    pts = [keep[tuple(q)] for q in used[name] if tuple(q) in keep]
    if pts:
        px = np.array(pts)
        ax.plot(px[:, 0], px[:, 1], "o", mfc="none", mec="red", ms=9, mew=1.5)
    ax.set_title(f"Figure {name}: {len(used[name])} points")
    ax.axis("off")
fig.tight_layout()
fig.savefig(HERE / "check_digitized.png", dpi=80)

# 2. normalized curves vs the generic model curves
fig, axs = plt.subplots(1, 3, figsize=(16, 4.6))
phi = np.linspace(0.3, 3.0, 200)
a, p0 = 1.15, 0.45
for pid, p in maps["pumps"].items():
    h = np.array(p["head_curve"])
    e = np.array(p["eff_curve"])
    axs[0].plot(h[:, 0], h[:, 1], "o-", ms=4, label=pid)
    axs[1].plot(e[:, 0], e[:, 1], "o-", ms=4, label=pid)
    s = np.array(p["speed_eff_curve"])
    axs[2].plot(s[:, 0], s[:, 1], "o-", ms=4, label=pid)
axs[0].plot(phi, a + (1 - a) * phi * np.abs(phi), "k--", label="generic (shutoff head 1.15)")
axs[1].plot(phi, phi * (a + (1 - a) * phi * phi) / (p0 + (1 - p0) * phi), "k--", label="generic (shutoff power 0.45)")
axs[0].set(xlabel="phi / phi_d", ylabel="psi / psi_d", title="Head", ylim=(0, 1.4))
axs[1].set(xlabel="phi / phi_d", ylabel="eta / eta_d", title="Efficiency", ylim=(0, 1.2))
axs[2].set(xlabel="N / N_d", ylabel="factor", title="Efficiency speed correction")
for ax in axs:
    ax.grid(alpha=0.3)
    ax.legend(fontsize=8)
fig.tight_layout()
fig.savefig(HERE / "check_curves.png", dpi=90)

t = np.array(maps["turbine"]["eff_curve"])
fig, ax = plt.subplots(figsize=(6, 4))
x = np.linspace(0, 0.6, 100)
xd = 0.35
ax.plot(t[:, 0], t[:, 1] * maps["turbine"]["eta_peak"], "o-", ms=3, label="P&W map (fig. 4.1.17), x = X/sqrt(2)")
ax.plot(x, 0.73612 * 2 * x / xd * (1 - x / (2 * xd)), "k--", label="generic parabola, fluidnet 0.4 calibration")
ax.set(xlabel="u / c0, c0 = sqrt(2 dh_s)", ylabel="efficiency", ylim=(0, 0.85))
ax.grid(alpha=0.3)
ax.legend(fontsize=8)
fig.tight_layout()
fig.savefig(HERE / "check_turbine.png", dpi=90)
print("written")
