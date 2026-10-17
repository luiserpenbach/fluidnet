"""Generate real-fluid property tables for fluidnet from CoolProp.

Each table is a rectilinear grid in (log10 P, h). Stored per node:
    v       specific volume            m3/kg
    T       temperature                K
    s       specific entropy           J/kg/K
    mu      dynamic viscosity          Pa s
    k       thermal conductivity       W/m/K
    cp      isobaric heat capacity     J/kg/K   (quality-weighted inside the dome)
    dvdP    (dv/dP) at constant h      m3/kg/Pa
    dvdh    (dv/dh) at constant P      m3/J
Inside the two-phase dome v is linear in h at fixed P, so bilinear interpolation
of v (not rho) is exact along h there. Derivatives use CoolProp's analytic
single-phase and two-phase (homogeneous equilibrium) partials.

A 1-D saturation table (P, T, h_l, h_v, v_l, v_v, s_l, s_v) is appended for
P_triple..P_crit.

File layout (little endian):
    b"FNTB" | u32 version | u32 header_len | header JSON (utf-8)
    f64[nP] log10 P grid | f64[nH] h grid | f64[nP] hmin(P) | f64[nP] hmax(P)
    f32[nF][nP][nH] fields | f64[nSat][8] saturation table

Usage:  python gen_fluid_tables.py OUT_DIR [FLUID_KEY ...]
"""
from __future__ import annotations

import json
import math
import struct
import sys
import time
from multiprocessing import Pool
from pathlib import Path

import numpy as np
import CoolProp
import CoolProp.CoolProp as CP

FIELDS = ["v", "T", "s", "mu", "k", "cp", "dvdP", "dvdh"]

FLUIDS = {
    "H2": dict(cp="ParaHydrogen", label="Parahydrogen", Tmax=1000.0, Pmax=600e5),
    "O2": dict(cp="Oxygen", label="Oxygen", Tmax=1000.0, Pmax=600e5),
    "N2": dict(cp="Nitrogen", label="Nitrogen", Tmax=1000.0, Pmax=600e5),
    "He": dict(cp="Helium", label="Helium", Tmax=1000.0, Pmax=600e5),
    "CH4": dict(cp="Methane", label="Methane", Tmax=1000.0, Pmax=600e5),
    "H2O": dict(cp="Water", label="Water", Tmax=1000.0, Pmax=600e5),
    "N2O": dict(cp="NitrousOxide", label="Nitrous oxide", Tmax=700.0, Pmax=200e5),
    "C2H5OH": dict(cp="Ethanol", label="Ethanol", Tmax=650.0, Pmax=300e5),
}

N_P_BASE = 120
N_P_CRIT = 24
N_H = 320
N_SAT = 200
P_MIN = 100.0


def build_p_grid(Pc: float, Pmax: float) -> np.ndarray:
    base = np.logspace(math.log10(P_MIN), math.log10(Pmax), N_P_BASE)
    crit = np.logspace(math.log10(0.75 * Pc), math.log10(1.35 * Pc), N_P_CRIT)
    grid = np.unique(np.concatenate([base, crit]))
    # drop points closer than 0.3 % in log space to avoid degenerate cells
    keep = [grid[0]]
    for p in grid[1:]:
        if math.log10(p) - math.log10(keep[-1]) > 1.3e-3:
            keep.append(p)
    return np.array(keep)


def h_at(AS, T, P):
    AS.update(CP.PT_INPUTS, P, T)
    return AS.hmass()


def build_h_grid(name, Tmin, Tmax, Pc, Pmax, P_grid):
    AS = CoolProp.AbstractState("HEOS", name)
    Pref = min(2.0 * Pc, Pmax)
    Ts = np.geomspace(Tmin * 1.001, Tmax, N_H - 8)
    hs = []
    for T in Ts:
        try:
            hs.append(h_at(AS, T, Pref))
        except ValueError:
            pass
    hs = np.array(sorted(hs))
    # extend to cover hmin over all pressures and hmax at lowest pressure
    lo = min(hs[0], min(_safe_h(AS, Tmin * 1.001, p) for p in P_grid))
    hi = max(hs[-1], _safe_h(AS, Tmax, P_grid[0]))
    pad_lo = np.linspace(lo, hs[0], 4, endpoint=False)
    pad_hi = np.linspace(hs[-1], hi, 5)[1:]
    grid = np.unique(np.concatenate([pad_lo, hs, pad_hi]))
    return grid


def _safe_h(AS, T, P):
    try:
        return h_at(AS, T, P)
    except ValueError:
        return float("nan")


def sat_props(ASs, P):
    """Saturated liquid / vapour transport + cp at pressure P (P < Pc)."""
    out = {}
    for q, tag in ((0.0, "l"), (1.0, "v")):
        ASs.update(CP.PQ_INPUTS, P, q)
        out["h" + tag] = ASs.hmass()
        out["v" + tag] = 1.0 / ASs.rhomass()
        out["s" + tag] = ASs.smass()
        out["T"] = ASs.T()
        for key, fn in (("mu", ASs.viscosity), ("k", ASs.conductivity), ("cp", ASs.cpmass)):
            try:
                out[key + tag] = fn()
            except ValueError:
                out[key + tag] = float("nan")
    return out


def compute_row(args):
    name, P, hgrid, Tmin, Tmax, Pc = args
    AS = CoolProp.AbstractState("HEOS", name)
    ASs = CoolProp.AbstractState("HEOS", name)
    hmin = _safe_h(AS, Tmin * 1.001, P)
    hmax = _safe_h(AS, Tmax, P)
    sat = None
    if P < Pc * 0.9999:
        try:
            sat = sat_props(ASs, P)
        except ValueError:
            sat = None
    nH = len(hgrid)
    row = np.full((len(FIELDS), nH), np.nan)
    for j, h in enumerate(hgrid):
        hh = max(h, hmin)
        try:
            AS.update(CP.HmassP_INPUTS, hh, P)
            rho = AS.rhomass()
            T = AS.T()
            s = AS.smass()
            Q = AS.Q()
            two_phase = sat is not None and 0.0 <= Q <= 1.0
            if two_phase:
                drdP = AS.first_two_phase_deriv(CP.iDmass, CP.iP, CP.iHmass)
                drdh = AS.first_two_phase_deriv(CP.iDmass, CP.iHmass, CP.iP)
                x = Q
                mu = 1.0 / (x / sat["muv"] + (1 - x) / sat["mul"])
                k = x * sat["kv"] + (1 - x) * sat["kl"]
                cp = x * sat["cpv"] + (1 - x) * sat["cpl"]
            else:
                drdP = AS.first_partial_deriv(CP.iDmass, CP.iP, CP.iHmass)
                drdh = AS.first_partial_deriv(CP.iDmass, CP.iHmass, CP.iP)
                try:
                    mu = AS.viscosity()
                except ValueError:
                    mu = float("nan")
                try:
                    k = AS.conductivity()
                except ValueError:
                    k = float("nan")
                cp = AS.cpmass()
            v = 1.0 / rho
            row[:, j] = [v, T, s, mu, k, cp, -drdP / rho**2, -drdh / rho**2]
        except ValueError:
            pass
    return P, hmin, hmax, row


def fill_row(row, hgrid, P, R):
    """Fill failed points: ideal-gas extension above the last valid point,
    nearest valid copy below the first valid point, linear between."""
    nF, nH = row.shape
    valid = np.all(np.isfinite(row[[0, 1, 2, 5, 6, 7]]), axis=0)
    if not valid.any():
        raise RuntimeError(f"no valid points at P={P}")
    idx = np.where(valid)[0]
    first, last = idx[0], idx[-1]
    # interior gaps: linear interpolation in h
    for f in range(nF):
        good = np.isfinite(row[f])
        if good.sum() >= 2:
            row[f] = np.interp(hgrid, hgrid[good], row[f][good])
    row[:, :first] = row[:, [first]]
    if last < nH - 1:
        v0, T0, s0, mu0, k0, cp0 = row[:6, last]
        h0 = hgrid[last]
        for j in range(last + 1, nH):
            T = T0 + (hgrid[j] - h0) / cp0
            v = R * T / P
            row[0, j] = v
            row[1, j] = T
            row[2, j] = s0 + cp0 * math.log(T / T0)
            row[3, j] = mu0 * (T / T0) ** 0.7
            row[4, j] = k0 * (T / T0) ** 0.8
            row[5, j] = cp0
            row[6, j] = -v / P
            row[7, j] = R / (P * cp0)
    return row


def build_sat_table(name, Ptr, Pc):
    AS = CoolProp.AbstractState("HEOS", name)
    Ps = np.geomspace(max(Ptr * 1.001, P_MIN), Pc * 0.9995, N_SAT)
    rows = []
    for P in Ps:
        try:
            AS.update(CP.PQ_INPUTS, P, 0.0)
            T = AS.T(); hl = AS.hmass(); vl = 1 / AS.rhomass(); sl = AS.smass()
            AS.update(CP.PQ_INPUTS, P, 1.0)
            hv = AS.hmass(); vv = 1 / AS.rhomass(); sv = AS.smass()
            rows.append([P, T, hl, hv, vl, vv, sl, sv])
        except ValueError:
            pass
    return np.array(rows)


def generate(key, out_dir: Path):
    spec = FLUIDS[key]
    name = spec["cp"]
    t0 = time.time()
    Tc = CP.PropsSI("Tcrit", name)
    Pc = CP.PropsSI("pcrit", name)
    rhoc = CP.PropsSI("rhomass_critical", name)
    Ttr = CP.PropsSI("Ttriple", name)
    Ptr = CP.PropsSI("ptriple", name)
    M = CP.PropsSI("molarmass", name)
    R = 8.314462618 / M
    Tmin = max(Ttr, CP.PropsSI("Tmin", name))
    Tmax = spec["Tmax"]
    Pgrid = build_p_grid(Pc, spec["Pmax"])
    hgrid = build_h_grid(name, Tmin, Tmax, Pc, spec["Pmax"], Pgrid)
    nP, nH = len(Pgrid), len(hgrid)

    with Pool(2) as pool:
        results = pool.map(compute_row, [(name, P, hgrid, Tmin, Tmax, Pc) for P in Pgrid])
    data = np.zeros((len(FIELDS), nP, nH))
    hmin = np.zeros(nP)
    hmax = np.zeros(nP)
    for i, (P, hmn, hmx, row) in enumerate(results):
        data[:, i, :] = fill_row(row, hgrid, P, R)
        hmin[i] = hmn if math.isfinite(hmn) else hgrid[0]
        hmax[i] = hmx if math.isfinite(hmx) else hgrid[-1]
    sat = build_sat_table(name, Ptr, Pc)

    header = dict(
        key=key, label=spec["label"], coolprop_name=name, coolprop_version=CoolProp.__version__,
        molar_mass=M, R=R, Tcrit=Tc, Pcrit=Pc, rhocrit=rhoc, Ttriple=Ttr, Ptriple=Ptr,
        Tmin=Tmin, Tmax=Tmax, Pmin=float(Pgrid[0]), Pmax=float(Pgrid[-1]),
        nP=nP, nH=nH, nSat=len(sat), fields=FIELDS,
        sat_columns=["P", "T", "hl", "hv", "vl", "vv", "sl", "sv"],
    )
    hb = json.dumps(header).encode()
    out = out_dir / f"{key}.fntb"
    with open(out, "wb") as f:
        f.write(b"FNTB")
        f.write(struct.pack("<II", 1, len(hb)))
        f.write(hb)
        f.write(np.log10(Pgrid).astype("<f8").tobytes())
        f.write(hgrid.astype("<f8").tobytes())
        f.write(hmin.astype("<f8").tobytes())
        f.write(hmax.astype("<f8").tobytes())
        f.write(data.astype("<f4").tobytes())
        f.write(sat.astype("<f8").tobytes())

    # dome resolution diagnostics
    diag = []
    for P in (1e5, 0.5 * Pc):
        if P < Pc and P > Ptr:
            hl = CP.PropsSI("H", "P", P, "Q", 0, name)
            hv = CP.PropsSI("H", "P", P, "Q", 1, name)
            diag.append(f"{P/1e5:.2f} bar: {int(((hgrid > hl) & (hgrid < hv)).sum())} nodes in dome")
    print(f"{key:7s} nP={nP} nH={nH} size={out.stat().st_size/1e6:.2f} MB "
          f"t={time.time()-t0:.0f}s  " + "; ".join(diag), flush=True)


if __name__ == "__main__":
    out_dir = Path(sys.argv[1])
    out_dir.mkdir(parents=True, exist_ok=True)
    keys = sys.argv[2:] or list(FLUIDS)
    for k in keys:
        generate(k, out_dir)
