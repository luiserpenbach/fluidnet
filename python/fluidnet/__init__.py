"""fluidnet: fluid network and engine-cycle simulation (Rust core).

Quick start::

    import fluidnet as fn

    res = fn.run("examples/expander_cycle.yaml", mode="steady")
    print(res["tca.Pc"][-1] / 1e5, "bar")

    model = fn.load("examples/expander_start.yaml")       # plain dict
    fn.set_param(model, "tbv", "opening", 0.1)            # edit
    res = fn.run(model)                                   # transient
    res.plot("tca.thrust", "turbopump.speed")             # matplotlib, optional

Models are YAML files or dicts with the same structure. Quantities may carry
units as strings ("300 bar", "6 mm"); results are SI (Pa, K, kg/s, rad/s, N).
"""
from __future__ import annotations

import json
import os
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
from typing import Any, Dict, Iterable, List, Mapping, Optional, Sequence, Union

import numpy as np

from . import _core

__version__ = _core.__version__
__all__ = ["run", "load", "save", "set_param", "get_param", "sweep", "props", "catalog", "check", "Results", "fluids_dir"]

ModelLike = Union[str, os.PathLike, Mapping[str, Any]]


def fluids_dir() -> str:
    """Directory with the fluid property tables: $FLUIDNET_FLUIDS, the copy
    shipped inside the package, or ./fluids."""
    env = os.environ.get("FLUIDNET_FLUIDS")
    if env:
        return env
    here = Path(__file__).parent / "fluids"
    if any(here.glob("*.fntb")):
        return str(here)
    for cand in (Path.cwd() / "fluids", Path(__file__).resolve().parents[2] / "fluids"):
        if any(cand.glob("*.fntb")):
            return str(cand)
    raise FileNotFoundError("fluid tables not found; set FLUIDNET_FLUIDS to the fluids/ directory")


def _text(model: ModelLike) -> str:
    if isinstance(model, Mapping):
        return json.dumps(model)
    p = Path(model)
    if p.suffix.lower() in (".yaml", ".yml", ".json") or p.exists():
        return p.read_text()
    return str(model)  # already YAML/JSON text


def load(path: Union[str, os.PathLike]) -> Dict[str, Any]:
    """Read a model file (YAML or JSON) into a dict."""
    return json.loads(_core.parse_model(Path(path).read_text()))


def save(model: Mapping[str, Any], path: Union[str, os.PathLike]) -> None:
    """Write a model dict as YAML (or JSON if the path ends in .json)."""
    p = Path(path)
    text = json.dumps(model, indent=2) if p.suffix.lower() == ".json" else _core.to_yaml(json.dumps(model))
    p.write_text(text)


def _component(model: Mapping[str, Any], cid: str) -> Dict[str, Any]:
    for c in model["components"]:
        if c["id"] == cid:
            return c
    raise KeyError(f"no component '{cid}'")


def get_param(model: Mapping[str, Any], cid: str, key: str) -> Any:
    return _component(model, cid).get(key)


def set_param(model: Dict[str, Any], cid: str, key: str, value: Any) -> Dict[str, Any]:
    """Set a component parameter (or a simulation setting with cid='simulation')."""
    if cid == "simulation":
        model.setdefault("simulation", {})[key] = value
    else:
        _component(model, cid)[key] = value
    return model


class Results:
    """Time histories of all output channels.

    ``res["tca.Pc"]`` is a numpy array aligned with ``res.time``;
    ``res.last("tca.Pc")`` the final value; ``res.units[name]`` the SI unit.
    """

    def __init__(self, meta: Dict[str, Any], data: bytes):
        self.meta = meta
        nt = meta["n_time"]
        arr = np.frombuffer(data, dtype="<f8").reshape(len(meta["channels"]) + 1, nt) if nt else np.zeros((1, 0))
        self.time = arr[0]
        self.names: List[str] = [c["name"] for c in meta["channels"]]
        self.units: Dict[str, str] = {c["name"]: c["unit"] for c in meta["channels"]}
        self._data = {n: arr[i + 1] for i, n in enumerate(self.names)}
        self.error: Optional[str] = meta.get("error")
        self.warnings: List[str] = meta.get("warnings", [])
        self.stats = meta.get("stats") or meta.get("steady")
        self.elapsed: float = meta.get("elapsed_s", float("nan"))

    def __getitem__(self, name: str) -> np.ndarray:
        try:
            return self._data[name]
        except KeyError:
            near = [n for n in self.names if name.split(".")[0] in n][:8]
            raise KeyError(f"no channel '{name}'" + (f"; channels of that component: {near}" if near else "")) from None

    def __contains__(self, name: str) -> bool:
        return name in self._data

    def last(self, name: str) -> float:
        return float(self[name][-1])

    def find(self, pattern: str) -> List[str]:
        return [n for n in self.names if pattern in n]

    def to_dict(self) -> Dict[str, np.ndarray]:
        return {"time": self.time, **self._data}

    def to_dataframe(self, channels: Optional[Iterable[str]] = None):
        import pandas as pd

        cols = list(channels) if channels is not None else self.names
        return pd.DataFrame({c: self[c] for c in cols}, index=pd.Index(self.time, name="time [s]"))

    def plot(self, *channels: str, ax=None):
        import matplotlib.pyplot as plt

        ax = ax or plt.figure(figsize=(8, 4)).gca()
        for c in channels:
            ax.plot(self.time, self[c], label=f"{c} [{self.units[c]}]")
        ax.set_xlabel("t [s]")
        ax.legend()
        ax.grid(alpha=0.3)
        return ax

    def __repr__(self) -> str:
        span = f"t = {self.time[0]:.4g}..{self.time[-1]:.4g} s, " if len(self.time) > 1 else ""
        return f"<Results {self.meta.get('name')!r} {self.meta.get('mode')}: {span}{len(self.names)} channels, {self.elapsed * 1e3:.1f} ms>"


def run(model: ModelLike, mode: str = "transient", steady_time: Optional[float] = None, fluids: Optional[str] = None) -> Results:
    """Run a model (path, YAML/JSON text, or dict). mode: 'transient' or 'steady'.

    A transient that fails part-way returns the results up to the failure with
    ``res.error`` set. The solver releases the GIL, so threads run in parallel.
    """
    meta, data = _core.run(_text(model), mode, fluids or fluids_dir(), steady_time)
    return Results(json.loads(meta), data)


def sweep(model: Mapping[str, Any], cid: str, key: str, values: Sequence[Any], mode: str = "steady", workers: Optional[int] = None) -> List[Results]:
    """Run one model per parameter value in parallel threads."""
    def one(v):
        m = json.loads(json.dumps(model))
        set_param(m, cid, key, v)
        return run(m, mode=mode)

    with ThreadPoolExecutor(max_workers=workers or os.cpu_count()) as ex:
        return list(ex.map(one, values))


def props(fluid: str, P: float, T: Optional[float] = None, h: Optional[float] = None, fluids: Optional[str] = None) -> Dict[str, float]:
    """Table properties at (P, T) or (P, h), SI units."""
    return json.loads(_core.props(fluid, P, fluids or fluids_dir(), h, T))


def catalog() -> Dict[str, Any]:
    """Component types with their ports and parameters."""
    return json.loads(_core.catalog_json())


def check(model: ModelLike, fluids: Optional[str] = None) -> Dict[str, Any]:
    """Compile a model without running it: states, nodes, branches, output channels."""
    return json.loads(_core.check(_text(model), fluids or fluids_dir()))
