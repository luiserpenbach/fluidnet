# RL10A-3-3A turbomachinery maps

Source: Binder, Tomsik, Veres, *RL10A-3-3A Rocket Engine Modeling Project*,
NASA TM-107318 (1997), https://ntrs.nasa.gov/citations/19970010379.

| File | Content |
|---|---|
| `scans/p52.png` … `p60.png` | PDF pages 52–60 (report pages 46–54, figures 4.1.1–4.1.17) rendered at 300 dpi, 1 bit |
| `digitize.py` | Pump head, efficiency and speed-correction maps (figures 4.1.1–4.1.8) → `maps_digitized_raw.json` |
| `digitize_area.py` | Turbine effective-area map (figure 4.1.16) → `area_digitized_raw.json` |
| `build_maps.py` | Normalizes the maps to the P&W typical operating point and adds the turbine efficiency polynomial of figure 4.1.17 → `rl10_maps.json` |
| `apply_maps.py` | Writes the maps into `examples/rl10a33a.yaml` |
| `plot_maps.py` | Check plots: `check_digitized.png` (points over the scans), `check_curves.png`, `check_turbine.png`; `digitize_area.py` writes `check_area.png` |

Conventions (TM-107318 section 10): pump flow coefficient
φ = Q/N / (π² D² b · 0.004329) with Q in gpm, N in rpm, D and b (impeller exit
diameter and blade height) in inches; LOX maps use φ = 5.495 Q/N as printed.
Head coefficient ψ = H / (π² D² N² · 5.99·10⁻⁸), H in ft. The turbine velocity
ratio X of figures 4.1.16 and 4.1.17 carries a √2 factor: X = √2 u/c0 with
c0 = √(2 Δh_s). fluidnet tables use φ/φ_d, ψ/ψ_d, η/η_d at the typical point
and u/c0 = X/√2; the turbine area map is a factor on the area at X = 0.45,
PR = 1.4 (1.152 in²).

Rerun: `python digitize.py && python digitize_area.py && python build_maps.py && python apply_maps.py && python plot_maps.py`.
