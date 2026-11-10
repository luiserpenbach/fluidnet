"""Reference-result baselines for the example and test models.

Every model in examples/ and tests_models/ is run steady and transient with the
CLI; the results are compared with stored baselines in validation/baselines/.
A change to the code that moves any channel beyond the tolerance fails the
check, so result changes are always deliberate and recorded.

    python tools/baselines.py check  [--cli PATH] [--only NAME]
    python tools/baselines.py update --reason "why the results changed" [--cli PATH] [--only NAME]

Stored per run: the final value of every channel (steady) or every channel
resampled at 41 equally spaced times (transient), as gzip-compressed JSON.
Tolerance per channel: |new - old| <= rtol * scale + atol, where scale is the
largest magnitude of that channel in the baseline. Non-finite values must stay
non-finite. `update` appends the reason, the date and the commit to
validation/baselines/CHANGELOG.md.
"""
from __future__ import annotations

import argparse
import gzip
import json
import math
import shutil
import subprocess
import sys
import tempfile
import time
from datetime import date
from pathlib import Path

import numpy as np

ROOT = Path(__file__).resolve().parents[1]
BASE = ROOT / "validation" / "baselines"
N_SAMPLES = 41
RTOL = {"steady": 1e-6, "transient": 1e-4}
ATOL = 1e-9

# Runs that are excluded, with the reason (see docs/03-user-guide/examples.md).
SKIP = {
    ("regulated_pressurization", "steady"): "no steady state at t = 0 by design",
    ("t_heated_pipe", "transient"): "model is meant for the steady state only (OI-029)",
    ("rl10a33a_start", "steady"): "45 s solve; covered by the transient and by cargo test",
}


def models() -> list[tuple[str, Path]]:
    out = []
    for d, prefix in ((ROOT / "examples", ""), (ROOT / "tests_models", "t_")):
        for p in sorted(d.glob("*.yaml")):
            out.append((prefix + p.stem, p))
    return out


def find_cli(explicit: str | None) -> str:
    cands = [explicit] if explicit else [
        str(ROOT / "target" / "release" / "fluidnet"),
        str(ROOT / "target" / "release" / "fluidnet.exe"),
        shutil.which("fluidnet") or "",
    ]
    for c in cands:
        if c and Path(c).exists():
            return c
    sys.exit("fluidnet CLI not found; build it with `cargo build --release` or pass --cli")


def run(cli: str, model: Path, mode: str) -> dict:
    with tempfile.TemporaryDirectory() as td:
        out = Path(td) / "r.json"
        cmd = [cli, "run", str(model), "--out", str(out), "--fluids", str(ROOT / "fluids")]
        if mode == "steady":
            cmd.insert(3, "--steady")
        r = subprocess.run(cmd, capture_output=True, text=True)
        if r.returncode != 0 or not out.exists():
            raise RuntimeError(f"{model.name} {mode}: exit {r.returncode}\n{r.stderr.strip()}")
        res = json.loads(out.read_text())
    if res.get("error"):
        raise RuntimeError(f"{model.name} {mode}: {res['error']}")
    return reduce(res, mode)


def _arr(x) -> np.ndarray:
    return np.array([np.nan if v is None else v for v in x], dtype=float)


def reduce(res: dict, mode: str) -> dict:
    t = _arr(res["time"])
    ch = {}
    if mode == "steady":
        for c in res["channels"]:
            ch[c["name"]] = [_arr(c["data"])[-1]]
        grid = [float(t[-1])]
    else:
        grid = np.linspace(t[0], t[-1], N_SAMPLES)
        for c in res["channels"]:
            y = _arr(c["data"])
            ok = np.isfinite(y)
            if ok.sum() >= 2:
                v = np.interp(grid, t[ok], y[ok])
                # keep non-finite where the nearest output was non-finite
                idx = np.clip(np.searchsorted(t, grid), 0, len(t) - 1)
                v[~ok[idx]] = np.nan
            else:
                v = np.full(len(grid), np.nan)
            ch[c["name"]] = v.tolist()
        grid = grid.tolist()
    return {"mode": mode, "time": grid, "channels": ch}


def path_of(name: str, mode: str) -> Path:
    return BASE / f"{name}.{mode}.json.gz"


def save(name: str, mode: str, data: dict) -> None:
    BASE.mkdir(parents=True, exist_ok=True)
    clean = json.loads(json.dumps(data, allow_nan=True).replace("NaN", "null"))
    with gzip.open(path_of(name, mode), "wt", encoding="utf-8") as f:
        json.dump(clean, f, separators=(",", ":"))


def load(name: str, mode: str) -> dict | None:
    p = path_of(name, mode)
    if not p.exists():
        return None
    with gzip.open(p, "rt", encoding="utf-8") as f:
        return json.load(f)


def compare(old: dict, new: dict, mode: str) -> tuple[bool, str, float]:
    """Returns (ok, message, worst normalised deviation)."""
    oc, nc = old["channels"], new["channels"]
    missing = sorted(set(oc) - set(nc))
    added = sorted(set(nc) - set(oc))
    if missing or added:
        return False, f"channels changed: missing {missing[:5]}, added {added[:5]}", math.inf
    if len(old["time"]) != len(new["time"]) or not np.allclose(old["time"], new["time"], rtol=1e-9, atol=1e-12):
        return False, "output time grid changed", math.inf
    worst, worst_ch = 0.0, ""
    for k, ov in oc.items():
        a, b = _arr(ov), _arr(nc[k])
        fa, fb = np.isfinite(a), np.isfinite(b)
        if (fa != fb).any():
            return False, f"{k}: finite/non-finite pattern changed", math.inf
        if not fa.any():
            continue
        scale = np.max(np.abs(a[fa]))
        dev = np.max(np.abs(a[fa] - b[fa])) / (RTOL[mode] * scale + ATOL)
        if dev > worst:
            worst, worst_ch = dev, k
    ok = worst <= 1.0
    return ok, ("" if ok else f"worst channel {worst_ch}"), worst


def git_commit() -> str:
    try:
        return subprocess.run(["git", "rev-parse", "--short", "HEAD"], cwd=ROOT, capture_output=True,
                              text=True, check=True).stdout.strip()
    except Exception:  # noqa: BLE001
        return "unknown"


def main() -> None:
    ap = argparse.ArgumentParser(description="Reference-result baselines")
    ap.add_argument("action", choices=["check", "update"])
    ap.add_argument("--cli")
    ap.add_argument("--only", help="substring of model names to include")
    ap.add_argument("--reason", help="required for update: why the results changed")
    a = ap.parse_args()
    if a.action == "update" and not a.reason:
        sys.exit("update needs --reason")
    cli = find_cli(a.cli)

    failed, updated = [], []
    print(f"{'model':32s} {'mode':10s} {'time':>8s}  {'dev/tol':>9s}  result")
    for name, p in models():
        if a.only and a.only not in name:
            continue
        for mode in ("steady", "transient"):
            if (name, mode) in SKIP:
                continue
            t0 = time.perf_counter()
            try:
                new = run(cli, p, mode)
            except RuntimeError as e:
                print(f"{name:32s} {mode:10s} {'':>8s}  {'':>9s}  RUN FAILED")
                print("    " + str(e).replace("\n", "\n    "))
                failed.append((name, mode))
                continue
            dt = time.perf_counter() - t0
            old = load(name, mode)
            if a.action == "update":
                if old is None or not compare(old, new, mode)[0]:
                    save(name, mode, new)
                    updated.append(f"{name}.{mode}")
                    print(f"{name:32s} {mode:10s} {dt:7.2f}s  {'':>9s}  updated")
                else:
                    print(f"{name:32s} {mode:10s} {dt:7.2f}s  {'':>9s}  unchanged")
                continue
            if old is None:
                print(f"{name:32s} {mode:10s} {dt:7.2f}s  {'':>9s}  NO BASELINE")
                failed.append((name, mode))
                continue
            ok, msg, dev = compare(old, new, mode)
            print(f"{name:32s} {mode:10s} {dt:7.2f}s  {dev:9.3g}  {'ok' if ok else 'CHANGED: ' + msg}")
            if not ok:
                failed.append((name, mode))

    if a.action == "update":
        if updated:
            log = BASE / "CHANGELOG.md"
            head = "" if log.exists() else (
                "# Baseline changes\n\nEvery update of the reference-result baselines, with the reason.\n\n"
                "| Date | Parent commit | Runs updated | Reason |\n|---|---|---|---|\n")
            with open(log, "a", encoding="utf-8") as f:
                f.write(head + f"| {date.today().isoformat()} | {git_commit()} | {', '.join(updated)} | {a.reason} |\n")
        print(f"\n{len(updated)} baseline(s) written" + (f"; {len(failed)} run(s) failed" if failed else ""))
        sys.exit(1 if failed else 0)
    print(f"\n{'all baselines match' if not failed else f'{len(failed)} run(s) differ or failed'}")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
