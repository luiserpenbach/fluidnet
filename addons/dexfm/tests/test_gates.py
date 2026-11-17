"""Physics gates. These must pass before any document that quotes toolchain numbers is issued."""
import math
from pathlib import Path
import numpy as np
import pytest

from dexfm.config import load, EngineConfig
from dexfm.thermochem import CanteraProvider
from dexfm.fluids import H2, O2
from dexfm import turbo as tb
from dexfm.cycle import solve
from dexfm.offdesign import build_pack, solve_point

CFG = Path(__file__).resolve().parents[1] / "configs"


@pytest.fixture(scope="module")
def rl10():
    return solve(load(CFG / "rl10a33a.yaml"))


@pytest.fixture(scope="module")
def dex():
    return solve(load(CFG / "dex1_baseline.yaml"))


def test_thermochem_rl10_point():
    s = CanteraProvider().solve(5.0, 32.75e5, 61.0)
    assert 2330 < s.cstar < 2400          # m/s, CEA class value for O/F 5
    assert 455 < s.isp_vac < 468          # ideal shifting-equilibrium vacuum Isp; delivered RL10A-3-3A is 444.4 s
    assert 3150 < s.Tc < 3300


def test_rl10_delivered_performance(rl10):
    assert abs(rl10.setup.isp - 444.4) < 2.0
    assert abs(rl10.setup.mdot_total - 16.8) / 16.8 < 0.03


def test_rl10_jacket_heat_pickup(rl10):
    """Reference: 2.79 kg/s hydrogen from 26.5 K / 75.8 bar to 213 K / 56.1 bar (NASA RL10A-3-3A model data)."""
    h = H2()
    q_ref = 2.79 * (h.h_pT(56.1e5, 213.0) - h.h_pT(75.8e5, 26.5))
    z = rl10.fuel.zone
    assert abs(z.Q - q_ref) / q_ref < 0.15
    assert abs(z.T_out - 213.0) < 25.0
    assert 12e5 < z.dp < 26e5             # measured pump discharge to turbine inlet is about 19.7 bar


def test_rl10_turbomachinery_at_flight_speeds():
    h, o = H2(), O2()
    fp = tb.design_pump(h, 2.79, 1.85e5, 21.4, 75.8e5, 32000, 2, tb.LH2_LIMITS, 1.2e5)
    op = tb.design_pump(o, 13.6, 3.0e5, 97.0, 41.0e5, 12800, 1, tb.LOX_LIMITS, 1.5e5)
    tu = tb.design_turbine(h, 2.79, 56.1e5, 213.0, 38.76e5, 32000, tb.H2_TURB)
    assert 0.52 < fp.eta < 0.66           # published first-stage efficiency 0.585
    assert 0.68 < tu.eta < 0.80           # published about 0.74
    assert 0.85 < tu.power / (fp.power + op.power) < 1.35     # the real engine balances at this state


def test_vinci_class_fuel_pump_power():
    p = tb.design_pump(H2(), 5.8, 3.0e5, 21.0, 224e5, 90000, 2, tb.LH2_LIMITS, 1.2e5)
    assert 2.0e6 < p.power < 3.0e6        # about 2.5 MW quoted for the Vinci hydrogen turbopump


def test_baseline_closes(dex):
    assert dex.closed
    for s in (dex.fuel, dex.ox):
        assert s.turbine.power * dex.setup.cfg.eta_mech >= s.pump.power * 0.999
        assert s.zone.Twg_max <= dex.setup.wall.T_limit + 5.0
        assert s.zone.mach_max < 0.35
    assert dex.ox.T_turb_in <= dex.setup.cfg.T_ox_hot_max


def test_baseline_energy_bookkeeping(dex):
    for s, m in ((dex.fuel, dex.setup.mf_total / dex.setup.cfg.n_chambers),
                 (dex.ox, dex.ox.heated_fraction * dex.setup.mo_total / dex.setup.cfg.n_chambers)):
        z = s.zone
        assert abs(z.Q - m * (z.h_out - z.h_in)) / z.Q < 1e-6
    su = dex.setup
    assert abs(su.cfg.pc * su.At * su.th.cf_vac * su.cfg.eta_cf - su.thrust_chamber) / su.thrust_chamber < 2e-3


def test_offdesign_reproduces_design(dex):
    pm = build_pack(dex)
    pt, _ = solve_point(pm, 1.0)
    assert pt.converged
    assert abs(pt.n_f / dex.fuel.pump.rpm - 1) < 0.01 and abs(pt.n_o / dex.ox.pump.rpm - 1) < 0.01
    assert abs(pt.byp_f - dex.setup.cfg.bypass_f) < 0.02 and abs(pt.byp_o - dex.setup.cfg.bypass_o) < 0.02


def test_throttle_to_half(dex):
    pm = build_pack(dex)
    w = None
    for lv in (1.0, 0.9, 0.8, 0.7, 0.6, 0.5):
        pt, w2 = solve_point(pm, lv, u0=w)
        assert pt is not None and pt.converged
        w = w2
    assert 0.0 < pt.byp_f < 0.9 and 0.0 < pt.byp_o < 0.9
    assert abs(pt.of - dex.setup.cfg.of) < 0.01
    assert pt.phi_f > 0.5 and pt.phi_o > 0.5


def test_config_hash_is_stable():
    a = EngineConfig(); b = EngineConfig()
    assert a.sha() == b.sha() and a.sha() != EngineConfig(pc=51e5).sha()


def test_solution_does_not_depend_on_call_history(dex):
    """Regression: a stale CoolProp phase guess once made results depend on earlier solves."""
    base = load(CFG / "dex1_baseline.yaml")
    solve(base.model_copy(update={"pc": 35e5, "c_bartz": 0.48}))       # leaves sub-critical oxygen states behind
    again = solve(base)
    assert again.closed
    assert abs(again.ox.pump.p_out - dex.ox.pump.p_out) / dex.ox.pump.p_out < 1e-6
    assert abs(again.fuel.zone.Q - dex.fuel.zone.Q) / dex.fuel.zone.Q < 1e-6


def test_fluidnet_reproduces_design_and_starts(dex):
    """Cross-check with the fluidnet network model: same design point, and the default start reaches mainstage."""
    fn = pytest.importorskip("fluidnet")
    from dexfm import fluidnet_model as fm
    params, err = fm.calibrate(dex)
    assert err < 1e-3
    assert abs(params["eff_tf"] - dex.fuel.turbine.eta) < 0.04      # independent turbine models agree
    assert abs(params["eff_to"] - dex.ox.turbine.eta) < 0.04
    res = fm.run_start(dex, params)
    assert res.error is None
    assert abs(res.last("tca.Pc") / dex.setup.cfg.pc - 1) < 0.02
    assert abs(res.last("tca.OF") - dex.setup.cfg.of) < 0.1
