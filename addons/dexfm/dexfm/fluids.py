"""Real-fluid property access (CoolProp HEOS) for hydrogen and oxygen.

Stateless helpers around one cached AbstractState per fluid. Two-phase states
are handled with a homogeneous mixture model so that sub-critical oxygen in a
cooling circuit does not crash the transport-property calls.
"""
from __future__ import annotations
from dataclasses import dataclass
from functools import lru_cache
import math
import warnings
import numpy as np
import CoolProp.CoolProp as CP

warnings.filterwarnings("ignore", category=np.exceptions.ComplexWarning)
from scipy.optimize import brentq

G0 = 9.80665


@dataclass(frozen=True)
class FState:
    p: float; T: float; h: float; rho: float; cp: float; mu: float; k: float; q: float
    @property
    def Pr(self) -> float: return self.cp * self.mu / self.k


class Fluid:
    _rho_exp = 0.0
    _dirty = False

    def __init__(self, name: str):
        self.name = name
        self.AS = CP.AbstractState("HEOS", name)
        self.M = self.AS.molar_mass()
        self.pcrit = self.AS.p_critical()
        self.Tcrit = self.AS.T_critical()
        self.Tmin = self.AS.Tmin() * 1.3
        self.Tmax = min(self.AS.Tmax(), 1500.0)

    # ---- single-phase / general state from (p, T)
    def pT(self, p: float, T: float) -> FState:
        a = self.AS
        try:
            a.update(CP.PT_INPUTS, p, min(max(T, self.Tmin), self.Tmax))
        except ValueError:
            a = self._reset()
            a.update(CP.PT_INPUTS, p, min(max(T, self.Tmin), self.Tmax))
        return FState(p, a.T(), a.hmass(), a.rhomass(), a.cpmass(), a.viscosity(), a.conductivity(), -1.0)

    def h_pT(self, p, T): return self.pT(p, T).h

    def s_pT(self, p, T):
        self.AS.update(CP.PT_INPUTS, p, T); return self.AS.smass()

    def h_ps(self, p, s):
        self.AS.update(CP.PSmass_INPUTS, p, s); return self.AS.hmass()

    def sat_liquid(self, p: float) -> FState:
        a = self.AS; a.update(CP.PQ_INPUTS, p, 0.0)
        return FState(p, a.T(), a.hmass(), a.rhomass(), a.cpmass(), a.viscosity(), a.conductivity(), 0.0)

    def _sat(self, p, q):
        a = self._reset() if self._dirty else self.AS
        a.update(CP.PQ_INPUTS, p, q)
        return a.T(), a.hmass(), a.rhomass(), a.cpmass(), a.viscosity(), a.conductivity()

    def ph(self, p: float, h: float) -> FState:
        """State from pressure and specific enthalpy, robust to two-phase."""
        if p < self.pcrit * 0.999:
            Tl, hl, rl, cl, ml, kl = self._sat(p, 0.0)
            Tv, hv, rv, cv, mv, kv = self._sat(p, 1.0)
            if hl < h < hv:
                x = (h - hl) / (hv - hl)
                rho = 1.0 / (x / rv + (1 - x) / rl)
                mu = 1.0 / (x / mv + (1 - x) / ml)
                return FState(p, Tl, h, rho, (1 - x) * cl + x * cv, mu, (1 - x) * kl + x * kv, x)
        T = self._T_ph(p, h)
        s = self.pT(p, T)
        return FState(p, s.T, h, s.rho, s.cp, s.mu, s.k, -1.0)

    def _reset(self):
        """CoolProp can keep a stale phase guess after a failed flash; a fresh state object clears it."""
        self.AS = CP.AbstractState("HEOS", self.name)
        return self.AS

    def _T_ph(self, p, h):
        for attempt in (0, 1):
            a = self.AS
            try:
                a.update(CP.HmassP_INPUTS, h, p)
                T = a.T()
                if self.Tmin <= T <= self.Tmax:
                    return T
                break
            except Exception:
                self._reset()
        def f(T):
            try:
                return self.h_pT(p, T) - h
            except ValueError:               # exactly on the saturation line: step off it
                return self.h_pT(p, T * 1.002) - h
        lo, hi = self.Tmin, self.Tmax
        if f(lo) > 0: return lo
        if f(hi) < 0: return hi
        return brentq(f, lo, hi, xtol=1e-4)

    def expand_isentropic(self, p_in, T_in, p_out):
        """Ideal specific work [J/kg] and outlet temperature for expansion p_in -> p_out."""
        for attempt in (0, 1):
            a = self.AS
            try:
                a.update(CP.PT_INPUTS, p_in, T_in); h1, s1 = a.hmass(), a.smass()
                a.update(CP.PSmass_INPUTS, p_out, s1)
                self._rho_exp = a.rhomass()
                return h1 - a.hmass(), a.T()
            except ValueError:
                self._reset()
                if attempt:
                    raise


    def expand_ph(self, p_in, h_in, p_out) -> float:
        """Ideal specific work from a (p, h) state; tolerant of two-phase inlet states (returns 0 on failure)."""
        for attempt in (0, 1):
            a = self.AS
            try:
                a.update(CP.HmassP_INPUTS, h_in, p_in); s1 = a.smass()
                a.update(CP.PSmass_INPUTS, p_out, s1)
                return max(h_in - a.hmass(), 0.0)
            except ValueError:
                self._reset()
        return 0.0


@lru_cache(maxsize=None)
def fluid(name: str) -> Fluid:
    return Fluid(name)


def H2() -> Fluid: return fluid("ParaHydrogen")
def O2() -> Fluid: return fluid("Oxygen")
