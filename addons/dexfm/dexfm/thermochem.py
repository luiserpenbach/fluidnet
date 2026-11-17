"""Thermochemistry provider interface.

`ThermochemProvider` is the seam where the in-house `h2cea` toolchain plugs in.
Until that package is wired, `CanteraProvider` supplies shifting-equilibrium
chamber and nozzle states for LOX/LH2 from first principles (HP equilibrium in
the chamber, isentropic equilibrium expansion to throat and exit).
"""
from __future__ import annotations
from dataclasses import dataclass
from functools import lru_cache
from typing import Protocol
import math
import warnings
import numpy as np
from scipy.optimize import brentq, minimize_scalar

G0 = 9.80665
# Enthalpies of the tank liquids relative to the 298.15 K gas reference (NASA CEA thermo data)
H_LH2 = -9012.0 / 2.01588e-3    # J/kg, H2(L) at 20.27 K
H_LOX = -12979.0 / 31.9988e-3   # J/kg, O2(L) at 90.17 K


@dataclass(frozen=True)
class ThermoState:
    of: float; pc: float; eps: float
    Tc: float; cstar: float; gamma: float; Mw: float
    cp: float; mu: float; Pr: float
    isp_vac: float; cf_vac: float; pe: float


class ThermochemProvider(Protocol):
    def solve(self, of: float, pc: float, eps: float) -> ThermoState: ...


class CanteraProvider:
    name = "cantera-h2o2-shifting-equilibrium"

    def __init__(self):
        import cantera as ct
        self.ct = ct
        self.gas = ct.Solution("h2o2.yaml")

    def solve(self, of: float, pc: float, eps: float) -> ThermoState:
        return _solve_cached(self, round(of, 4), round(pc, -3), round(eps, 2))

    def _solve(self, of, pc, eps) -> ThermoState:
        ct, g = self.ct, self.gas
        yf, yo = 1.0 / (1.0 + of), of / (1.0 + of)
        h0 = yf * H_LH2 + yo * H_LOX
        g.TPY = 3000.0, pc, {"H2": yf, "O2": yo}
        with warnings.catch_warnings():
            warnings.simplefilter("ignore")           # h2o2.yaml fits end at 3500 K; chamber can sit a few K above
            g.equilibrate("TP")
            g.HP = h0, pc
            g.equilibrate("HP")
        Tc, s0, rho_c = g.T, g.entropy_mass, g.density
        cp = g.cp_mass; mu = g.viscosity; k = g.thermal_conductivity
        Mw = g.mean_molecular_weight * 1e-3
        X0 = g.X.copy()

        def at(p):
            g.SPX = s0, p, X0
            with warnings.catch_warnings():
                warnings.simplefilter("ignore")
                g.equilibrate("SP")
            h = g.enthalpy_mass
            u = math.sqrt(max(2.0 * (h0 - h), 0.0))
            return g.density * u, u, g.density, g.T

        res = minimize_scalar(lambda x: -at(x * pc)[0], bounds=(0.45, 0.68), method="bounded",
                              options={"xatol": 1e-5})
        pt = res.x * pc
        Gt, ut, rhot, Tt = at(pt)
        cstar = pc / Gt
        gamma_s = math.log(pt / pc) / math.log(rhot / rho_c)     # isentropic exponent chamber->throat
        if eps <= 1.0:                                  # chamber and throat only (used by the start transient)
            return ThermoState(of, pc, eps, Tc, cstar, gamma_s, Mw, cp, mu, cp * mu / k, 0.0, 0.0, pt)
        f = lambda lp: math.log(Gt / at(math.exp(lp))[0]) - math.log(eps)
        lpe = brentq(f, math.log(pc * 1e-6), math.log(pt * 0.95), xtol=1e-6)
        pe = math.exp(lpe)
        Ge, ue, rhoe, Te = at(pe)
        isp = (ue + pe / Ge) / G0
        cf = isp * G0 / cstar
        return ThermoState(of, pc, eps, Tc, cstar, gamma_s, Mw, cp, mu, cp * mu / k, isp, cf, pe)


@lru_cache(maxsize=4096)
def _solve_cached(prov, of, pc, eps):
    return prov._solve(of, pc, eps)


class H2CeaAdapter:
    """Thermochemistry from the in-house `h2cea` package (RocketCEA front end, `h2cea.cea_si.point`).

    `h2cea` lives in the e2-propulsion-tools repository (folder `h2cea/`, next to `baseline/`) and is not
    pip-installable, so its folder is put on the import path: pass `path` or set the environment
    variable H2CEA_PATH to `<e2-propulsion-tools>/h2cea`. Importing it also loads the E2 baseline file
    of that repository; none of the E2 design values are used here, only `point()` with the RocketCEA
    built-in LOX (90.2 K) and LH2 (20.3 K) cards and zero ambient pressure.
    """
    name = "h2cea-rocketcea"

    def __init__(self, path: str | None = None, ox: str = "LOX", fuel: str = "LH2"):
        import os, sys
        path = path or os.environ.get("H2CEA_PATH")
        if path and path not in sys.path:
            sys.path.insert(0, path)
        try:
            from h2cea import cea_si
        except Exception as e:                       # missing package, RocketCEA or the baseline file
            raise RuntimeError(f"h2cea is not importable ({e}); set H2CEA_PATH or use the Cantera provider") from e
        self._point = cea_si.point
        self.ox, self.fuel = ox, fuel

    def solve(self, of: float, pc: float, eps: float) -> ThermoState:
        throat_only = eps <= 1.0
        p = self._point(round(pc / 1e5, 3), round(of, 4), 2.0 if throat_only else round(eps, 2), 0.0, self.ox, self.fuel)
        return ThermoState(of, pc, eps, p.Tc, p.cstar, p.gam_eq_c, p.mw_c * 1e-3, p.cp_fr_c, p.mu_c, p.pr_fr_c,
                           0.0 if throat_only else p.isp_vac, 0.0 if throat_only else p.cf_vac, p.pe * 1e5)


@lru_cache(maxsize=1)
def default_provider() -> ThermochemProvider:
    """Provider selected by the environment variable DEXFM_THERMOCHEM: "cantera" (default) or "h2cea".

    The choice is explicit so that results do not change silently between machines; the provider
    name is written into every summary file.
    """
    import os
    which = os.environ.get("DEXFM_THERMOCHEM", "cantera").lower()
    if which == "h2cea":
        return H2CeaAdapter()
    if which != "cantera":
        raise ValueError(f"DEXFM_THERMOCHEM must be 'cantera' or 'h2cea', not {which!r}")
    return CanteraProvider()
