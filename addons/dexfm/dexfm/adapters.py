"""Export seams toward the in-house tools.

`fluidnet_netlist` writes the powerpack as a neutral node/branch netlist (JSON) with the
design-point state and the calibrated component parameters of this model. The schema is this
package's own: the fluidnet repository could not be read when this was written, so the mapping
onto fluidnet component classes is the open step (see README, section "Wiring h2cea and fluidnet").
"""
from __future__ import annotations
from .cycle import CycleResult
from .offdesign import build_pack


def fluidnet_netlist(r: CycleResult) -> dict:
    cfg = r.setup.cfg; f, o = r.fuel, r.ox; pm = build_pack(r)
    x = o.heated_fraction
    nodes = {
        "tank_f": {"p": cfg.p_tank_f, "T": cfg.T_tank_f, "fluid": "ParaHydrogen", "boundary": True},
        "fp_out": {"p": f.pump.p_out, "T": f.pump.T_out}, "jf_in": {"p": f.p_zone_in, "T": f.pump.T_out},
        "ft_in": {"p": f.p_turb_in, "T": f.T_turb_in}, "f_inj": {"p": f.p_turb_out, "T": f.T_inj},
        "tank_o": {"p": cfg.p_tank_o, "T": cfg.T_tank_o, "fluid": "Oxygen", "boundary": True},
        "op_out": {"p": o.pump.p_out, "T": o.pump.T_out}, "o_split": {"p": o.p_zone_in, "T": o.pump.T_out},
        "ot_in": {"p": o.p_turb_in, "T": o.T_turb_in}, "o_inj": {"p": o.p_turb_out, "T": o.T_inj},
        "chamber": {"p": cfg.pc, "boundary": True},
    }
    br = [
        {"id": "FP", "type": "pump", "from": "tank_f", "to": "fp_out", "mdot": f.mdot_pack, "map": pm.fpump.__dict__, "shaft": "F"},
        {"id": "MFV", "type": "valve", "from": "fp_out", "to": "jf_in", "mdot": f.mdot_pack, "k_dp_rho_over_m2": pm.k_fl},
        {"id": "JKT_H2", "type": "heated_duct", "from": "jf_in", "to": "ft_in", "mdot": f.mdot_pack,
         "parallel": pm.npk, "Q_each": f.zone.Q, "dp": f.p_zone_in - f.p_turb_in},
        {"id": "FT", "type": "turbine", "from": "ft_in", "to": "f_inj", "mdot": f.mdot_pack * (1 - f.bypass), "map": pm.fturb.__dict__, "shaft": "F"},
        {"id": "FTBV", "type": "valve", "from": "ft_in", "to": "f_inj", "mdot": f.mdot_pack * f.bypass, "cda": pm.a_bf_d, "control": "thrust"},
        {"id": "INJ_F", "type": "injector", "from": "f_inj", "to": "chamber", "mdot": f.mdot_pack, "cda": pm.a_if},
        {"id": "OP", "type": "pump", "from": "tank_o", "to": "op_out", "mdot": o.mdot_pack, "map": pm.opump.__dict__, "shaft": "O"},
        {"id": "MOV", "type": "valve", "from": "op_out", "to": "o_split", "mdot": o.mdot_pack, "k_dp_rho_over_m2": pm.k_ol},
        {"id": "JKT_O2", "type": "heated_duct", "from": "o_split", "to": "ot_in", "mdot": o.mdot_pack * x,
         "parallel": pm.npk, "Q_each": o.zone.Q, "dp": o.p_zone_in - o.p_turb_in},
        {"id": "OT", "type": "turbine", "from": "ot_in", "to": "o_inj", "mdot": o.mdot_pack * x * (1 - o.bypass), "map": pm.oturb.__dict__, "shaft": "O"},
        {"id": "OTBV", "type": "valve", "from": "ot_in", "to": "o_inj", "mdot": o.mdot_pack * x * o.bypass, "cda": pm.a_bo_d, "control": "mixture ratio"},
        {"id": "OCV", "type": "valve", "from": "o_split", "to": "o_inj", "mdot": o.mdot_pack * (1 - x), "cda": pm.a_ocv, "control": "heated fraction"},
        {"id": "INJ_O", "type": "injector", "from": "o_inj", "to": "chamber", "mdot": o.mdot_pack, "cda": pm.a_io},
    ]
    shafts = {"F": {"rpm": f.pump.rpm, "power": f.pump.power}, "O": {"rpm": o.pump.rpm, "power": o.pump.power}}
    return {"schema": "dexfm-netlist-0.1", "config_sha": cfg.sha(), "nodes": nodes, "branches": br, "shafts": shafts}
