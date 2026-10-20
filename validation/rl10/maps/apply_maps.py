"""Write the digitized maps (rl10_maps.json) into examples/rl10a33a.yaml."""
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
MODEL = HERE.parents[2] / "examples" / "rl10a33a.yaml"
maps = json.loads((HERE / "rl10_maps.json").read_text())


def fmt(curve):
    return "[" + ", ".join(f"[{x:g}, {y:g}]" for x, y in curve) + "]"


def put(s, comp, key, val):
    line = re.compile(r"(  - \{id: %s, [^\n]*?)(, ui: \{[^}]*\}\})" % re.escape(comp))
    m = line.search(s)
    assert m, comp
    body = m.group(1)
    kv = re.compile(r", %s: (\[\[.*?\]\]|[^,}]+)" % re.escape(key))
    body = kv.sub("", body) + f", {key}: {val}"
    return s[: m.start()] + body + m.group(2) + s[m.end():]


s = MODEL.read_text()
for pid, p in maps["pumps"].items():
    for key in ("head_curve", "eff_curve", "speed_eff_curve"):
        s = put(s, pid, key, fmt(p[key]))
t = maps["turbine"]
s = put(s, "turbine", "design_velocity_ratio", f"{t['X_peak'] / 2 ** 0.5:.3f}")
s = put(s, "turbine", "eff_curve", fmt([c for c in t["eff_curve"] if c[0] > 0]))
s = put(s, "turbine", "area_map", "[" + ", ".join(f"[{p:g}, {x:g}, {f:g}]" for p, x, f in t["area_map"]) + "]")
MODEL.write_text(s)
print("maps written to", MODEL)
