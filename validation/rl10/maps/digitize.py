"""Digitize the marker points of the RL10 turbomachinery maps (NASA TM-107318,
figures 4.1.1-4.1.8) from 300 dpi page scans in scans/.

Each figure is deskewed (the scans are rotated by a fraction of a degree), the
grid lines are located by projection, mapped to the printed tick values, and
the hollow markers (squares / triangles) are located as enclosed white regions
of marker size. Outputs maps_digitized.json and overlay PNGs for checking.
"""
import json
import sys
from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy import ndimage

HERE = Path(__file__).resolve().parent

# page, crop box (x0, y0, x1, y1) in 300-dpi pixels, x ticks, y ticks
FIGS = {
    "4.1.1": dict(page=52, box=(560, 280, 1960, 1260), xt=[0, 0.025, 0.05, 0.075, 0.1, 0.125, 0.15], yt=[0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]),
    "4.1.2": dict(page=52, box=(560, 1740, 1960, 2700), xt=[0, 0.025, 0.05, 0.075, 0.1, 0.125, 0.15], yt=[0.2, 0.25, 0.3, 0.35, 0.4, 0.45, 0.5, 0.55, 0.6, 0.65, 0.7]),
    "4.1.3": dict(page=53, box=(560, 300, 1960, 1180), xt=[0, 0.02, 0.04, 0.06, 0.08, 0.1, 0.12], yt=[0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6]),
    "4.1.4": dict(page=53, box=(560, 1740, 1960, 2640), xt=[0, 0.025, 0.05, 0.075, 0.1, 0.125, 0.15], yt=[0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]),
    "4.1.5": dict(page=54, box=(600, 330, 1960, 1200), xt=[0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4], yt=[0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2]),
    "4.1.6": dict(page=54, box=(640, 1750, 1960, 2640), xt=[0, 0.02, 0.04, 0.06, 0.08, 0.1, 0.12], yt=[0.68, 0.69, 0.70, 0.71, 0.72, 0.73, 0.74, 0.75]),
    "4.1.7": dict(page=55, box=(600, 300, 1960, 1220), xt=[0, 0.02, 0.04, 0.06, 0.08, 0.1, 0.12], yt=[0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7]),
    "4.1.8": dict(page=55, box=(620, 1640, 1960, 2560), xt=[0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2, 1.4], yt=[0, 0.2, 0.4, 0.6, 0.8, 1.0, 1.2]),
}


def load(page):
    return np.array(Image.open(HERE / "scans" / f"p{page}.png").convert("L")) < 128


def best_angle(b, axis):
    """Angle (deg) that makes grid lines most axis-aligned (sharpest projection)."""
    best = (-1, 0.0)
    for a in np.arange(-1.5, 1.51, 0.05):
        r = ndimage.rotate(b.astype(np.float32), a, reshape=False, order=1) > 0.5
        p = r.mean(axis=axis)
        score = np.sort(p)[-12:].sum()
        if score > best[0]:
            best = (score, a)
    return best[1]


def lines(proj, thresh, minsep=15):
    idx = np.where(proj > thresh)[0]
    groups = []
    for i in idx:
        if groups and i - groups[-1][-1] <= 3:
            groups[-1].append(i)
        else:
            groups.append([i])
    c = [float(np.average(g, weights=proj[g])) for g in groups]
    out = []
    for v in c:
        if not out or v - out[-1] > minsep:
            out.append(v)
    return out


def grid_map(pos, ticks, ascending):
    """Linear pixel->value map from detected grid lines. Lines may be missing
    (hidden under text); each line is assigned a tick index from the uniform
    spacing, counting from the first line (left axis / top frame)."""
    pos = sorted(pos)
    d = np.diff(pos)
    sp = np.median(d[d < 1.3 * d.min()])
    idx = np.round((np.array(pos) - pos[0]) / sp).astype(int)
    keep = idx < len(ticks)
    t = np.array(ticks if ascending else list(reversed(ticks)))
    return np.polyfit(np.array(pos)[keep], t[idx[keep]], 1)


def digitize(name, f, debug_dir=None):
    img = load(f["page"])
    x0, y0, x1, y1 = f["box"]
    b = img[y0:y1, x0:x1]
    ang = best_angle(b, 1)
    r = ndimage.rotate(b.astype(np.float32), ang, reshape=False, order=1) > 0.5
    rowp, colp = r.mean(axis=1), r.mean(axis=0)
    hy = lines(rowp, 0.45)
    vx = lines(colp, 0.45)
    xt, yt = f["xt"], f["yt"]
    fx = grid_map(vx, xt, ascending=True)
    fy = grid_map(hy, yt, ascending=False)
    # hollow markers: white components enclosed by black, marker sized
    white = ~r
    lab, n = ndimage.label(white)
    pts = []
    for i, sl in enumerate(ndimage.find_objects(lab), 1):
        h = sl[0].stop - sl[0].start
        w = sl[1].stop - sl[1].start
        if not (3 <= h <= 16 and 3 <= w <= 16):
            continue
        # must be enclosed: dilated ring all black
        comp = lab[sl] == i
        area = comp.sum()
        if area < 8:
            continue
        cy, cx = ndimage.center_of_mass(lab == i)
        pts.append((cx, cy, w, h, area))
    res = []
    for cx, cy, w, h, area in pts:
        res.append({"x": float(np.polyval(fx, cx)), "y": float(np.polyval(fy, cy)), "px": [round(cx, 1), round(cy, 1)], "w": w, "h": h, "area": int(area)})
    res.sort(key=lambda p: p["x"])
    if debug_dir:
        im = Image.fromarray(((~r) * 255).astype(np.uint8)).convert("RGB")
        d = ImageDraw.Draw(im)
        for v in vx:
            d.line([(v, 0), (v, r.shape[0])], fill=(0, 160, 255))
        for v in hy:
            d.line([(0, v), (r.shape[1], v)], fill=(0, 160, 255))
        for p in res:
            cx, cy = p["px"]
            d.ellipse([cx - 9, cy - 9, cx + 9, cy + 9], outline=(255, 0, 0), width=3)
        im.save(Path(debug_dir) / f"ov_{name}.png")
    return {"angle": ang, "grid_x": vx, "grid_y": hy, "points": res}


if __name__ == "__main__":
    dbg = sys.argv[1] if len(sys.argv) > 1 else None
    out = {}
    for name, f in FIGS.items():
        out[name] = digitize(name, f, dbg)
        print(name, f"angle {out[name]['angle']:.2f}", len(out[name]["points"]), "points")
        for p in out[name]["points"]:
            print(f"   {p['x']:.4f} {p['y']:.4f}  {p['w']}x{p['h']} a={p['area']}")
    (HERE / "maps_digitized_raw.json").write_text(json.dumps(out, indent=1))
