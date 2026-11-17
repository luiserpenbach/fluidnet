"""h2cea coupling. The mapping test runs everywhere; the comparison needs RocketCEA and H2CEA_PATH."""
import os, sys, types
from dataclasses import dataclass
import pytest

from dexfm.thermochem import H2CeaAdapter, CanteraProvider


def test_adapter_maps_point_fields(monkeypatch):
    @dataclass
    class P:
        Tc: float = 3200.0; cstar: float = 2360.0; gam_eq_c: float = 1.14; mw_c: float = 11.8
        cp_fr_c: float = 4100.0; mu_c: float = 9.3e-5; pr_fr_c: float = 0.6; isp_vac: float = 461.0
        cf_vac: float = 1.915; pe: float = 0.02
    calls = []
    def point(pc, mr, eps, pamb, ox, fuel):
        calls.append((pc, mr, eps, pamb, ox, fuel)); return P()
    pkg = types.ModuleType("h2cea"); mod = types.ModuleType("h2cea.cea_si"); mod.point = point; pkg.cea_si = mod
    monkeypatch.setitem(sys.modules, "h2cea", pkg); monkeypatch.setitem(sys.modules, "h2cea.cea_si", mod)
    s = H2CeaAdapter().solve(5.0, 32.75e5, 61.0)
    assert calls[-1] == (32.75, 5.0, 61.0, 0.0, "LOX", "LH2")        # bar, O/F, area ratio, vacuum, built-in cards
    assert s.Mw == pytest.approx(11.8e-3) and s.pe == pytest.approx(2000.0)
    assert (s.Tc, s.cstar, s.gamma, s.cp, s.mu, s.Pr, s.isp_vac, s.cf_vac) == (3200.0, 2360.0, 1.14, 4100.0, 9.3e-5, 0.6, 461.0, 1.915)
    t = H2CeaAdapter().solve(6.0, 50e5, 0.0)                         # chamber and throat only
    assert t.isp_vac == 0.0 and calls[-1][2] == 2.0


@pytest.mark.parametrize("of,pc,eps", [(5.0, 32.75e5, 61.0), (6.0, 50e5, 164.6), (6.0, 12.5e5, 164.6), (5.5, 50e5, 100.0)])
def test_h2cea_agrees_with_cantera(of, pc, eps):
    pytest.importorskip("rocketcea")
    if not os.environ.get("H2CEA_PATH"):
        pytest.skip("set H2CEA_PATH to <e2-propulsion-tools>/h2cea")
    a = H2CeaAdapter().solve(of, pc, eps); b = CanteraProvider().solve(of, pc, eps)
    assert a.cstar == pytest.approx(b.cstar, rel=0.005)
    assert a.isp_vac == pytest.approx(b.isp_vac, rel=0.005)
    assert a.Tc == pytest.approx(b.Tc, rel=0.01)
    assert a.cp == pytest.approx(b.cp, rel=0.05) and a.mu == pytest.approx(b.mu, rel=0.10) and a.Pr == pytest.approx(b.Pr, rel=0.15)
