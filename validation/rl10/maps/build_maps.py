"""Convert the digitized TM-107318 maps into fluidnet map tables.

Input: maps_digitized_raw.json (digitize.py). Output: rl10_maps.json with
  fuel_pump_1 / fuel_pump_2 / lox_pump: head_curve [[phi/phi_d, psi/psi_d]],
      eff_curve [[phi/phi_d, eta/eta_d]], speed_eff_curve [[N/N_d, factor]]
  turbine: eff_curve [[u/c0, eta/eta_peak]], eta_peak
The design point of each pump is the P&W typical operating point (TM-107318
tables 2.2.1 and 2.3.1); its flow coefficient follows from the definitions in
section 10 of the report.
"""
import json
from pathlib import Path

import numpy as np

HERE = Path(__file__).resolve().parent
LB, FT3, GAL = 0.45359237, 0.3048 ** 3, 3.785411784e-3

raw = json.loads((HERE / "maps_digitized_raw.json").read_text())


def pts(fig, drop=(), xmin=0.005):
    out = []
    for p in raw[fig]["points"]:
        if p["w"] < 8 or p["h"] < 6 or p["x"] < xmin:
            continue
        if any(abs(p["x"] - dx) < 0.0008 and abs(p["y"] - dy) < 0.003 for dx, dy in drop):
            continue
        out.append((round(p["x"], 4), round(p["y"], 4)))
    return sorted(out)


# markers to discard: legend symbols, letters in the printed formulas, axis labels and
# split halves of markers that sit on a grid line
DROP = {
    "4.1.1": [(0.1470, -0.0494)],
    "4.1.2": [(0.0757, 0.5031)],
    "4.1.3": [(0.0341, 0.1339)],
    "4.1.4": [(0.0415, 0.1930), (0.0652, 0.1888), (0.0679, 0.1295), (0.0861, 0.1278)],
    "4.1.5": [(0.4078, 0.7919)],
    "4.1.6": [(0.0234, 0.6937), (0.0405, 0.7295), (0.0400, 0.7282), (0.0610, 0.6870)],
    "4.1.7": [(0.0666, 0.1154)],
    "4.1.8": [(0.6134, 0.3207), (0.6778, 0.2661), (0.9358, 0.2702)],
}

# markers split in two by a grid line, entered as the mean of the two halves
ADD = {"4.1.6": [(0.0402, 0.7288)]}

maps = {k: sorted(pts(k, DROP.get(k, ())) + ADD.get(k, [])) for k in raw}
# speed correction curves pass through the origin (marker at 0,0)
for k in ("4.1.5", "4.1.8"):
    maps[k] = [(0.0, 0.0)] + [p for p in maps[k] if p[0] > 0.1]


def interp(curve, x):
    xs, ys = zip(*curve)
    return float(np.interp(x, xs, ys))


def pump_phi_d(Q_gpm, N, D, b):
    # flow coefficient phi = Q/N / (pi^2 D^2 b 0.004329), Q in gpm, N in rpm, D, b in inches
    return Q_gpm / N / (np.pi ** 2 * D ** 2 * b * 0.004329)


def psi_d(H_ft, N, D):
    return H_ft / (np.pi ** 2 * D ** 2 * N ** 2 * 5.99e-8)


def gpm(mdot_lbs, rho_kgm3):
    return mdot_lbs * LB / rho_kgm3 / GAL * 60.0


# typical operating point (TM-107318 tables 2.2.1, 2.3.1); densities as in examples/rl10a33a.yaml
DESIGN = {
    "fuel_pump_1": dict(head="4.1.1", eff="4.1.3", sc="4.1.5", Q=gpm(6.051, 69.656), N=31537, D=7.07, b=0.230, H=16969, eta=0.5810),
    "fuel_pump_2": dict(head="4.1.2", eff="4.1.4", sc="4.1.5", Q=gpm(6.051, 68.651), N=31537, D=7.07, b=0.220, H=17989, eta=0.5619),
    # LOX maps state phi = 5.495 Q/N directly
    "lox_pump": dict(head="4.1.6", eff="4.1.7", sc="4.1.8", Q=gpm(31.40, 1106.47), N=12615, D=4.20, b=0.251, H=1212, eta=0.6422, k=5.495),
}

out = {"source": "NASA TM-107318 (Binder, Tomsik, Veres 1997) figures 4.1.1-4.1.8 and 4.1.17, digitized by validation/rl10/maps/digitize.py",
       "pumps": {}}
for pid, d in DESIGN.items():
    phid = d["k"] * d["Q"] / d["N"] if "k" in d else pump_phi_d(d["Q"], d["N"], d["D"], d["b"])
    head, eff, sc = maps[d["head"]], maps[d["eff"]], maps[d["sc"]]
    psid_map, etad_map = interp(head, phid), interp(eff, phid)
    sc1 = interp(sc, 1.0)
    out["pumps"][pid] = {
        "phi_d": phid,
        "psi_d_map": psid_map,
        "psi_d_typical": psi_d(d["H"], d["N"], d["D"]),
        "eta_d_map": etad_map,
        "eta_typical": d["eta"],
        "head_points": head,
        "eff_points": eff,
        "speed_points": sc,
        "head_curve": [[round(x / phid, 4), round(y / psid_map, 4)] for x, y in head],
        "eff_curve": [[round(x / phid, 4), round(y / etad_map, 4)] for x, y in eff],
        "speed_eff_curve": [[round(x, 4), round(y / sc1, 4)] for x, y in sc],
    }

# turbine: eta(X) = 3.99912 X - 8.26206 X^2 + 8.41619 X^3 - 3.90371 X^4 (figure 4.1.17) with the
# RL10 map velocity ratio X = sqrt(2) u/c0, c0 = sqrt(2 dh_s) (report section 10)
TC = [0.0, 3.99912, -8.26206, 8.41619, -3.90371]
X = np.linspace(0.0, 0.8, 33)
eta = sum(c * X ** i for i, c in enumerate(TC))
peak = float(eta.max())
out["turbine"] = {
    "poly": TC,
    "eta_peak": peak,
    "X_peak": float(X[eta.argmax()]),
    "eff_curve": [[round(float(x / np.sqrt(2.0)), 4), round(float(e / peak), 4)] for x, e in zip(X, eta)],
}
# turbine effective flow area (figure 4.1.16), CdA in in2 vs X for four pressure
# ratios; stored as a factor on the area at X = 0.45 (u/c0 = 0.32), PR = 1.4, about
# the TP1 operating point
area = json.loads((HERE / "area_digitized_raw.json").read_text())["series"]
ref = float(np.interp(0.45, *zip(*area["1.4"])))
out["turbine"]["area_ref_in2"] = ref
out["turbine"]["area_points"] = area
out["turbine"]["area_map"] = [[float(pr), round(x / np.sqrt(2.0), 4), round(y / ref, 4)]
                              for pr, pts in area.items() for x, y in pts]

(HERE / "rl10_maps.json").write_text(json.dumps(out, indent=1))
for pid, p in out["pumps"].items():
    print(f"{pid}: phi_d {p['phi_d']:.4f}  psi_d map {p['psi_d_map']:.4f} (typical {p['psi_d_typical']:.4f})  "
          f"eta_d map {p['eta_d_map']:.4f} (typical {p['eta_typical']:.4f})  points {len(p['head_points'])}/{len(p['eff_points'])}/{len(p['speed_points'])}")
print(f"turbine: peak eta {peak:.4f} at X {out['turbine']['X_peak']:.3f} (u/c0 {out['turbine']['X_peak'] / np.sqrt(2):.3f})")
