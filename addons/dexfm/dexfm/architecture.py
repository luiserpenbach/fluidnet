"""Propulsion-system architecture trade: number of thrust chambers and powerpacks.

All architectures deliver the same total vacuum thrust inside the same total nozzle exit
area, so chamber count changes throat size (heat pickup per unit propellant), turbopump
size (efficiency), engine length and failure tolerance at constant system performance budget.
"""
from __future__ import annotations
from dataclasses import dataclass, asdict
import math
import numpy as np
from .config import EngineConfig
from .cycle import solve, max_pc, CycleResult


def best_split(cfg: EngineConfig, grid=(0.4, 0.55, 0.7, 0.85), tol=1.5e5):
    """Oxygen-zone extent that maximises the closing chamber pressure. Returns (pc_max, ox_param, result)."""
    if cfg.ox_zone == "none":
        pc, r = max_pc(cfg, tol=tol)
        return pc, 0.0, r
    best = (None, None, None)
    for g in grid:
        pc, r = max_pc(cfg.model_copy(update={"ox_param": g}), tol=tol)
        if pc is not None and (best[0] is None or pc > best[0]):
            best = (pc, g, r)
    return best


# ----------------------------------------------------------------------------- mass and envelope
def turbopump_mass(shaft_power_w: float) -> float:
    """Two-point power-law fit (RL10 class and SSME high-pressure fuel turbopump class). Assumption."""
    return 1.29 * (shaft_power_w / 1e3) ** 0.516


def engine_mass(r: CycleResult) -> dict:
    su = r.setup; cfg = su.cfg; ct = su.ct
    ds = ct.ds
    ext = np.arange(ct.i_regen_end + 1, len(ct.x))
    a_ext = float(np.sum(2 * math.pi * ct.r[ext] * ds[ext]))
    regen = sum(s.zone.metal_mass for s in (r.fuel, r.ox) if s.zone is not None) * 1.25    # manifolds, flanges
    inj = 250.0 * math.pi * ct.Rc ** 2 + 2.0
    chamber = regen + inj + 8.0 * a_ext                 # radiatively cooled extension at 8 kg/m2
    tp = sum(turbopump_mass(s.pump.power) for s in (r.fuel, r.ox) if s.pump is not None)
    chambers = chamber * cfg.n_chambers; packs = tp * cfg.n_packs
    other = 0.20 * (chambers + packs) + 2.5 * cfg.n_chambers + 6.0 * cfg.n_packs    # valves, ducts, gimbals, harness
    return dict(chamber_each=chamber, turbopumps_per_pack=tp, total=chambers + packs + other,
                length=ct.L_total + 0.30, exit_diameter=2 * ct.Re, ext_area=a_ext)


@dataclass
class ArchRow:
    label: str; n_chambers: int; n_packs: int
    pc_max: float; ox_param: float; pc_design: float
    isp: float; eps: float; thrust_chamber: float
    q_per_kg: float                  # jacket heat pickup per kg of propellant [kJ/kg]
    eta_fp: float; rpm_fp: float; p_fp: float; pd_fp: float; stages_fp: int
    eta_op: float; rpm_op: float; p_op: float; pd_op: float
    T_h2: float; T_o2: float; x_o2: float
    mass: float; length: float; d_exit: float
    thrust_pack_out: float           # fraction of total thrust left after losing one powerpack
    turbopumps: int; closed: bool


def evaluate(cfg: EngineConfig, label: str, pc_margin: float = 0.80, grid=(0.4, 0.55, 0.7, 0.85)) -> tuple[ArchRow, CycleResult]:
    pc_max, split, _ = best_split(cfg, grid)
    if pc_max is None:
        raise RuntimeError(f"{label}: cycle does not close at any chamber pressure")
    cfg_d = cfg.model_copy(update={"pc": round(pc_margin * pc_max, -4), "ox_param": split or cfg.ox_param})
    r = solve(cfg_d)
    su = r.setup; m = engine_mass(r); f, o = r.fuel, r.ox
    row = ArchRow(label, cfg.n_chambers, cfg.n_packs, pc_max, split, cfg_d.pc, su.isp, su.eps, su.thrust_chamber,
                  r.Q_total_chamber * cfg.n_chambers / su.mdot_total / 1e3,
                  f.pump.eta, f.pump.rpm, f.pump.power, f.pump.p_out, f.pump.stages,
                  o.pump.eta, o.pump.rpm, o.pump.power, o.pump.p_out,
                  f.T_turb_in, o.T_turb_in, o.heated_fraction, m["total"], m["length"], m["exit_diameter"],
                  1.0 - 1.0 / cfg.n_packs, 2 * cfg.n_packs, r.closed)
    return row, r
