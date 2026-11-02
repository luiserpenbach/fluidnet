# Digitization of the RL10A-3-3A turbomachinery maps

FNT-VAL-ANA-003 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Question

Can the Pratt & Whitney pump and turbine maps printed in NASA TM-107318 be
recovered from the report scans with enough accuracy to replace the generic
turbomachinery curves of the RL10 model, and how are they converted into the
normalized tables fluidnet uses?

## Inputs

Binder, Tomsik, Veres, *RL10A-3-3A Rocket Engine Modeling Project*, NASA
TM-107318 (1997), https://ntrs.nasa.gov/citations/19970010379. PDF pages 52–60
(report pages 46–54), figures 4.1.1–4.1.17, rendered at 300 dpi, 1 bit, stored
as `validation/rl10/maps/scans/p52.png` … `p60.png`.

| Figure | Content | Used as |
|---|---|---|
| 4.1.1–4.1.2 | Fuel pump stage 1 head and efficiency vs flow coefficient | `fuel_pump_1` `head_curve`, `eff_curve` |
| 4.1.3–4.1.4 | Fuel pump stage 2 head and efficiency | `fuel_pump_2` `head_curve`, `eff_curve` |
| 4.1.5–4.1.6 | LOX pump head and efficiency | `lox_pump` `head_curve`, `eff_curve` |
| 4.1.7–4.1.8 | Efficiency speed correction, fuel and LOX pumps | `speed_eff_curve` |
| 4.1.16 | Turbine effective flow area vs velocity ratio, lines of pressure ratio | `turbine` `area_map` |
| 4.1.17 | Turbine efficiency polynomial vs velocity ratio | `turbine` `eff_curve` |
| 4.1.10–4.1.15 | Extended low-speed pump maps | not yet used (open item) |

## Method

### Conventions of the report (TM-107318 section 10)

```
pump flow coefficient   φ = Q/N / (π² D² b · 0.004329)    Q in gpm, N in rpm, D and b in in
LOX maps as printed      φ = 5.495 Q/N
head coefficient        ψ = H / (π² D² N² · 5.99·10⁻⁸)      H in ft
turbine velocity ratio  X = √2 · u/c0,   c0 = √(2 Δh_s)
turbine efficiency      η = 3.99912 X − 8.26206 X² + 8.41619 X³ − 3.90371 X⁴
```

D is the impeller exit diameter and b the blade height at exit. The turbine
velocity ratio of figures 4.1.16 and 4.1.17 carries the √2 factor defined in
section 10; the figure axes only say U/Co. The polynomial peaks at η = 0.7436
at X = 0.525 (u/c0 = 0.371).

### Marker extraction (`digitize.py`, `digitize_area.py`)

1. Each scan is deskewed; the measured rotations are 0.35–1.35°.
2. Grid lines are detected and mapped to the printed tick values. Where grid
   lines are missing, uniform grid spacing assigns the tick indices.
3. Markers are located automatically. Spurious detections in legends and text
   are removed with per-figure exclusion lists; markers split by a grid line
   are merged by averaging. Small holes in markers are filled before detection
   (filling whole grid cells was an early error and is excluded).
4. Results: 12 points per pump map, 14 per speed correction, 45 of the 52
   turbine-area markers (the remaining 7 merge with grid lines or with each
   other).

Check plots: `check_digitized.png` (points over the scans) and `check_area.png`.

### Normalization (`build_maps.py`)

fluidnet's tabulated maps are relative to the design point of each machine
([FNT-TRB-DOC-001](../02-engineering/turbomachinery.md)):

- pumps: φ/φ_d, ψ/ψ_d and η/η_d, with the design point at the P&W typical
  operating point (TM-107318 tables 2.2.1 and 2.3.1);
- speed correction: factor against N/N_d, 1 at N_d;
- turbine efficiency: η/η_d against u/c0 = X/√2;
- turbine area: factor on the effective area at X = 0.45, PR = 1.4
  (1.152 in²), as `[[PR, u/c0, factor], ...]`.

`apply_maps.py` writes the tables into `examples/rl10a33a.yaml`;
`plot_maps.py` draws `check_curves.png` and `check_turbine.png`.

## Results

Check against the P&W typical operating point, using flow and head
coefficients computed from the definitions above:

| Check | Map value | Reference | Difference |
|---|---|---|---|
| Fuel pump stage 1 head at the typical flow | – | typical point | 0.3 % |
| Fuel pump stage 2 head at the typical flow | – | typical point | 1.1 % |
| LOX pump head at the typical flow | – | typical point | 0.8 % |
| LOX pump efficiency at the typical flow | 0.639 | 0.642, typical point | −0.5 % |
| Fuel pump efficiencies (without P&W's 1.03 factor) | – | typical point | maps 2–5 % below |
| Turbine efficiency at the TP1 operating point (u/c0 = 0.32) | 0.734 | 0.731, fluidnet 0.4 calibration with 1.2 % bypass | +0.4 % |

Observations on the map shapes in normalized form:

- At the design point the pumps run left of their best-efficiency flow; best
  efficiency lies at 1.2–1.4 times the design flow coefficient.
- The fuel-pump head falls less steeply with flow than the generic curve.
- The turbine area map holds its edge values below u/c0 = 0.07, where the
  lowest markers for PR 1.2 and 1.4 could not be separated; the area there is
  about 4 % low.

## Discussion

The head maps reproduce the typical-point heads within 1.1 %, which is within
the reading accuracy of the printed figures. The fuel-pump efficiency maps lie
2–5 % below the typical-point efficiencies; the model keeps the typical-point
efficiency at the design flow and uses only the map shape. The √2 factor in the
turbine velocity ratio is taken from the report's definition of terms; without
it, the polynomial would put peak efficiency at a blade speed far from the
measured operating range.

## Conclusions

The digitized maps are accurate to about 1 % in head and are used in the RL10
model of fluidnet 0.5 ([FNT-VAL-ANA-002](rl10.md)). The extended low-speed maps
(figures 4.1.10–4.1.15) are the remaining data needed for start and shutdown
transients.

## Run record

| Item | Value |
|---|---|
| Software | fluidnet 0.5.0, commit 36580d1 |
| Command | `cd validation/rl10/maps && python digitize.py && python digitize_area.py && python build_maps.py && python apply_maps.py && python plot_maps.py` |
| Outputs | `maps_digitized_raw.json`, `area_digitized_raw.json`, `rl10_maps.json`, check plots |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
