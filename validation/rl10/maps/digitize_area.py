"""Digitize TM-107318 figure 4.1.16 (turbine effective CdA vs velocity ratio for
four pressure ratios). After deskewing, enclosed regions are filled and a
morphological opening removes the curve and grid lines, leaving the markers
(hollow or filled squares and circles). Marker fill (hollow/filled) and shape
(square/circle) identify the pressure-ratio series."""
import json
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

from digitize import best_angle, grid_map, lines

HERE = Path(__file__).resolve().parent
BOX = (700, 250, 2050, 1400)
XT = [0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]
YT = [1.0, 1.05, 1.1, 1.15, 1.2, 1.25, 1.3, 1.35, 1.4, 1.45, 1.5]
# legend box (in figure units) to ignore
LEGEND = (0.05, 0.27, 1.02, 1.135)


def run(debug_png=None):
    page = np.array(Image.open(HERE / "scans" / "p60.png").convert("L")) < 128
    x0, y0, x1, y1 = BOX
    b = page[y0:y1, x0:x1]
    ang = best_angle(b, 1)
    r = ndimage.rotate(b.astype(np.float32), ang, reshape=False, order=1) > 0.5
    hy = lines(r.mean(axis=1), 0.45)
    vx = lines(r.mean(axis=0), 0.45)
    fx = grid_map(vx, XT, True)
    fy = grid_map(hy, YT, False)
    holes = ndimage.binary_fill_holes(r) & ~r
    hl, nh = ndimage.label(holes)
    sizes = ndimage.sum(holes, hl, range(1, nh + 1))
    small = np.isin(hl, 1 + np.where(sizes < 300)[0])
    filled = r | small
    core = ndimage.binary_opening(filled, structure=np.ones((9, 9)))
    lab, n = ndimage.label(core)
    pts = []
    for i, sl in enumerate(ndimage.find_objects(lab), 1):
        h, w = sl[0].stop - sl[0].start, sl[1].stop - sl[1].start
        if not (10 <= h <= 30 and 10 <= w <= 30):
            continue
        cy, cx = ndimage.center_of_mass(lab == i)
        X, Y = float(np.polyval(fx, cx)), float(np.polyval(fy, cy))
        if LEGEND[0] <= X <= LEGEND[1] and LEGEND[2] <= Y <= LEGEND[3]:
            continue
        # hollow if the original (unfilled) marker has a white interior
        win = r[int(cy) - 3: int(cy) + 4, int(cx) - 3: int(cx) + 4]
        hollow = win.mean() < 0.5
        # square vs circle: corner pixels of the filled blob
        blob = (lab[sl] == i)
        corners = blob[0, 0] + blob[0, -1] + blob[-1, 0] + blob[-1, -1]
        shape = "square" if corners >= 3 else "circle"
        pts.append({"x": X, "y": Y, "hollow": bool(hollow), "shape": shape, "px": (cx, cy), "size": (w, h)})
    # series by rank: at every velocity ratio the curves are ordered PR >= 2.1 > 1.8 > 1.4 > 1.2
    cols = {}
    for p in pts:
        xr = round(p["x"] / 0.05) * 0.05
        if p["y"] > 1.0 and abs(p["x"] - xr) < 0.01 and 0.04 < xr < 0.66:
            cols.setdefault(round(xr, 2), []).append(p)
    out = {2.1: [], 1.8: [], 1.4: [], 1.2: []}
    for xr, ps in sorted(cols.items()):
        ps.sort(key=lambda q: -q["y"])
        if len(ps) == 4:
            for pr, q in zip((2.1, 1.8, 1.4, 1.2), ps):
                q["pr"] = pr
                out[pr].append((xr, round(q["y"], 4)))
        elif len(ps) > 4:
            # extra blobs below the curves (legend lettering): keep the top four
            for pr, q in zip((2.1, 1.8, 1.4, 1.2), ps[:4]):
                q["pr"] = pr
                out[pr].append((xr, round(q["y"], 4)))
    # leftmost column: the two upper markers are separate, the lower two merge
    if 0.05 in cols and len(cols[0.05]) == 2:
        for pr, q in zip((2.1, 1.8), sorted(cols[0.05], key=lambda q: -q["y"])):
            q["pr"] = pr
            out[pr].append((0.05, round(q["y"], 4)))
    # columns with markers merged into grid lines or into each other: assign each
    # marker found to the nearest series (interpolated from the complete columns)
    full = {pr: dict(v) for pr, v in out.items()}
    for xr, ps in sorted(cols.items()):
        if len(ps) >= 4 or xr < 0.075:
            continue
        for q in ps:
            est = {pr: np.interp(xr, sorted(f), [f[k] for k in sorted(f)]) for pr, f in full.items()}
            pr = min(est, key=lambda k: abs(est[k] - q["y"]))
            if abs(est[pr] - q["y"]) < 0.03:
                q["pr"] = pr
                out[pr].append((xr, round(q["y"], 4)))
        print(f"u/c0 {xr}: {len(ps)} of 4 markers found")
    for pr in out:
        out[pr].sort()
    if debug_png:
        im = Image.fromarray(((~r) * 255).astype(np.uint8)).convert("RGB")
        d = ImageDraw.Draw(im)
        col = {1.2: (255, 0, 0), 1.4: (0, 160, 0), 1.8: (0, 0, 255), 2.1: (255, 140, 0)}
        for p in pts:
            if "pr" not in p:
                continue
            pr = p["pr"]
            cx, cy = p["px"]
            d.ellipse([cx - 13, cy - 13, cx + 13, cy + 13], outline=col[pr], width=3)
        im.save(debug_png)
    return {"angle": ang, "series": {str(k): v for k, v in sorted(out.items())}}


if __name__ == "__main__":
    res = run(HERE / "check_area.png")
    for k, v in res["series"].items():
        print(k, len(v), v)
    (HERE / "area_digitized_raw.json").write_text(json.dumps(res, indent=1))
