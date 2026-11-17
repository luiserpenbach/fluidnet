"""Study runners. Each writes a JSON file to the output directory; the report module plots them."""
from __future__ import annotations
import json, sys, time
from dataclasses import asdict
from pathlib import Path
from .config import EngineConfig, load
from .architecture import evaluate, best_split, engine_mass
from .cycle import solve, max_pc

ARCHS = [("1 chamber / 1 pack", 1, 1), ("2 chambers / 1 pack", 2, 1), ("2 chambers / 2 packs", 2, 2),
         ("3 chambers / 3 packs", 3, 3), ("4 chambers / 1 pack", 4, 1), ("4 chambers / 2 packs", 4, 2),
         ("4 chambers / 4 packs", 4, 4), ("6 chambers / 2 packs", 6, 2), ("6 chambers / 3 packs", 6, 3)]


def arch_sweep(base: EngineConfig, out: Path):
    rows = []
    for label, nc, npk in ARCHS:
        t = time.time()
        try:
            row, _ = evaluate(base.model_copy(update={"n_chambers": nc, "n_packs": npk}), label)
            rows.append(asdict(row))
            print(label, f"pc_max {row.pc_max / 1e5:.1f} bar", f"{time.time() - t:.0f}s", flush=True)
        except Exception as e:                       # keep the sweep alive, record the failure
            rows.append({"label": label, "n_chambers": nc, "n_packs": npk, "error": str(e)})
            print(label, "FAILED", e, flush=True)
        (out / "arch_sweep.json").write_text(json.dumps(rows, indent=1))


def ox_zone_trade(base: EngineConfig, out: Path):
    rows = []
    for lstar in (1.2, 1.6, 2.0):
        for zone, grid, extra in (("none", (0.0,), {}), ("barrel", (0.4, 0.55, 0.7, 0.85), {}),
                                  ("nozzle", (3.0, 5.0, 8.0), {"eps_regen": 40.0})):
            cfg = base.model_copy(update={"lstar": lstar, "ox_zone": zone, **extra})
            pc, split, r = best_split(cfg, grid)
            row = {"lstar": lstar, "ox_zone": zone, "pc_max": pc, "ox_param": split}
            if r is not None and pc is not None:
                row.update(isp=r.setup.isp, T_h2=r.fuel.T_turb_in, T_o2=r.ox.T_turb_in if r.ox.zone else None,
                           x=r.ox.heated_fraction, L_chamber=r.setup.ct.L_chamber,
                           q_peak=float(r.fuel.zone.q.max()), q_ox_peak=float(r.ox.zone.q.max()) if r.ox.zone else None)
            rows.append(row); print(row, flush=True)
            (out / "ox_zone_trade.json").write_text(json.dumps(rows, indent=1))


def sensitivity(base: EngineConfig, out: Path):
    rows = []
    cases = [("baseline", {}), ("heat transfer -20 %", {"c_bartz": base.c_bartz * 0.8}),
             ("heat transfer +20 %", {"c_bartz": base.c_bartz * 1.2}),
             ("pump efficiency -5 pts", {"pump_derate": base.pump_derate - 0.065}),
             ("turbine efficiency -5 pts", {"turb_eta_ref": base.turb_eta_ref - 0.05}),
             ("wall limit 650 K", {"wall": base.wall.model_copy(update={"T_limit": 650.0})}),
             ("wall limit 850 K", {"wall": base.wall.model_copy(update={"T_limit": 850.0})}),
             ("bypass margin 5 %", {"bypass_f": 0.05, "bypass_o": 0.05}),
             ("bypass margin 20 %", {"bypass_f": 0.20, "bypass_o": 0.20}),
             ("mixture ratio 5.5", {"of": 5.5}), ("mixture ratio 6.5", {"of": 6.5})]
    for name, upd in cases:
        pc, r = max_pc(base.model_copy(update=upd), tol=1.0e5)
        rows.append({"case": name, "pc_max": pc, "isp": r.setup.isp if pc else None})
        print(rows[-1], flush=True)
        (out / "sensitivity.json").write_text(json.dumps(rows, indent=1))


if __name__ == "__main__":
    what, cfg_path, out = sys.argv[1], sys.argv[2], Path(sys.argv[3])
    out.mkdir(parents=True, exist_ok=True)
    base = load(cfg_path)
    {"arch": arch_sweep, "ox": ox_zone_trade, "sens": sensitivity}[what](base, out)
