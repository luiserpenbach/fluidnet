"""Compare the h2cea (RocketCEA) and Cantera providers and the effect on the DEX1-A design point.

Run on a machine with RocketCEA:
    set H2CEA_PATH=C:\\Users\\<you>\\Documents\\GitHub-local\\e2-propulsion-tools\\h2cea     (Windows)
    python tools/compare_thermochem.py configs/dex1_baseline.yaml
"""
import sys
from dexfm.config import load
from dexfm.cycle import solve
from dexfm.thermochem import H2CeaAdapter, CanteraProvider

cfg = load(sys.argv[1] if len(sys.argv) > 1 else "configs/dex1_baseline.yaml")
provs = {"cantera": CanteraProvider(), "h2cea": H2CeaAdapter()}
print(f"{'':10s} {'Tc K':>8s} {'c* m/s':>8s} {'gamma':>7s} {'cp':>7s} {'mu':>10s} {'Pr':>6s} {'Isp s':>7s} | {'Isp del.':>8s} {'mdot':>7s} {'Q MW':>6s} {'T_H2':>6s} {'T_O2':>6s} closed")
for name, pv in provs.items():
    r = solve(cfg, pv); t = r.setup.th
    print(f"{name:10s} {t.Tc:8.0f} {t.cstar:8.1f} {t.gamma:7.4f} {t.cp:7.0f} {t.mu:10.3e} {t.Pr:6.3f} {t.isp_vac:7.2f} | "
          f"{r.setup.isp:8.2f} {r.setup.mdot_total:7.3f} {r.Q_total_chamber / 1e6:6.3f} {r.fuel.T_turb_in:6.1f} {r.ox.T_turb_in:6.1f} {r.closed}")
