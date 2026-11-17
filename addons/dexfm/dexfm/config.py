"""Engine configuration: YAML is the single source of truth, hashed for traceability."""
from __future__ import annotations
import hashlib
import json
from pathlib import Path
from typing import Literal
import yaml
from pydantic import BaseModel, Field


class WallCfg(BaseModel):
    k: float = 320.0
    t_wall: float = 0.8e-3
    rib_min: float = 0.8e-3
    w_throat: float = 1.0e-3
    w_max: float = 2.5e-3
    h_min: float = 0.8e-3
    h_max: float = 6.0e-3
    aspect_max: float = 8.0
    T_limit: float = 750.0
    roughness: float = 6e-6


class EngineConfig(BaseModel):
    name: str = "DEX1"
    # --- system level
    thrust_total: float = Field(180e3, description="total vacuum thrust of the propulsion system [N]")
    n_chambers: int = 4
    n_packs: int = 2
    of: float = 6.0
    pc: float = 50e5
    exit_area_total: float | None = Field(3.0, description="sum of nozzle exit areas [m2]; sets the area ratio")
    eps: float | None = None
    # --- chamber
    cr: float = 4.0
    lstar: float = 1.2
    eps_regen: float = 12.0
    ox_zone: Literal["none", "barrel", "nozzle"] = "barrel"
    ox_param: float = 0.5
    wall: WallCfg = WallCfg()
    c_bartz: float = Field(0.80, description="Bartz scale factor, calibrated on RL10A-3-3A total jacket heat pickup")
    eta_cstar: float = 0.985
    eta_cf: float = 0.975
    # --- feed boundary
    p_tank_f: float = 2.5e5
    T_tank_f: float = 21.5
    p_vap_f: float = 1.3e5
    p_tank_o: float = 3.0e5
    T_tank_o: float = 91.0
    p_vap_o: float = 1.1e5
    # --- pressure budget (fractions of local pressure)
    dp_inj_f: float = 0.15
    dp_inj_o: float = 0.18
    dp_line: float = 0.03
    dp_valve: float = 0.04
    # --- power balance
    bypass_f: float = Field(0.10, description="fuel turbine bypass fraction held as control margin at design")
    bypass_o: float = 0.10
    eta_mech: float = 0.97
    T_ox_hot_max: float = 450.0
    ox_heated_fraction: float | None = None      # None: solved for minimum oxidizer pump discharge pressure
    pump_derate: float = 0.92
    turb_eta_ref: float = 0.82

    def sha(self) -> str:
        return hashlib.sha256(json.dumps(self.model_dump(), sort_keys=True).encode()).hexdigest()[:12]


def load(path: str | Path) -> EngineConfig:
    return EngineConfig(**yaml.safe_load(Path(path).read_text()))


def dump(cfg: EngineConfig, path: str | Path) -> None:
    Path(path).write_text(yaml.safe_dump(cfg.model_dump(), sort_keys=False))
