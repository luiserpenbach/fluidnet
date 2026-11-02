# Fluid property tables

FNT-FLD-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes how fluidnet obtains real-fluid properties: the offline
generation of property tables from CoolProp (`tools/gen_fluid_tables.py`), the
binary table format, and the run-time lookup in `crates/fluidnet-core/src/props.rs`
with its treatment of the saturation lines, the two-phase region, pressures
below the table and the derivative blending near saturation. It ends with
accuracy, cost and the procedure for adding a fluid.

## Model and equations

### Approach

The network equations use pressure P and specific enthalpy h as state
variables ([FNT-SLV-DOC-001](network-formulation.md)). Every right-hand-side
evaluation needs, at each node, the specific volume v (density ρ = 1/v),
temperature T, entropy s, viscosity μ, thermal conductivity k, isobaric heat
capacity c_p, and the derivatives (∂v/∂P)_h and (∂v/∂h)_P for the mass and
energy balance. These are tabulated offline with CoolProp on a rectilinear grid
in (log₁₀ P, h), together with a one-dimensional saturation table, and
interpolated at run time. CoolProp is not needed at run time.

All fluids use the Helmholtz-energy equations of state of CoolProp (`HEOS`
backend), CoolProp version 8.0.0 for the tables shipped with v0.5.0 (recorded
in each table header).

### Fluids and ranges

| Key | CoolProp name | Label | T_min [K] | T_max [K] | P_min [Pa] | P_max [Pa] | Grid nP × nH |
|---|---|---|---|---|---|---|---|
| H2 | ParaHydrogen | Parahydrogen | 13.8 | 1000 | 100 | 6.0×10⁷ | 143 × 311 |
| O2 | Oxygen | Oxygen | 54.36 | 1000 | 100 | 6.0×10⁷ | 144 × 313 |
| N2 | Nitrogen | Nitrogen | 63.15 | 1000 | 100 | 6.0×10⁷ | 142 × 313 |
| He | Helium | Helium | 2.177 | 1000 | 100 | 6.0×10⁷ | 143 × 316 |
| CH4 | Methane | Methane | 90.69 | 1000 | 100 | 6.0×10⁷ | 143 × 312 |
| H2O | Water | Water | 273.16 | 1000 | 100 | 6.0×10⁷ | 143 × 320 |
| N2O | NitrousOxide | Nitrous oxide | 182.3 | 700 | 100 | 2.0×10⁷ | 143 × 320 |
| C2H5OH | Ethanol | Ethanol | 159.1 | 650 | 100 | 3.0×10⁷ | 143 × 318 |

T_min is max(T_triple, CoolProp T_min); T_max and P_max are set per fluid in
`FLUIDS` in the generator; P_min is 100 Pa for all fluids (`P_MIN`). The grid
sizes are those of the shipped files in `fluids/`. Each file is about 1.5 MB.

### Table generation

`tools/gen_fluid_tables.py OUT_DIR [KEY ...]` builds one table per fluid
(`generate`):

- **Pressure grid** (`build_p_grid`): 120 points logarithmically spaced from
  P_min to P_max, merged with 24 points logarithmically spaced from 0.75 P_c to
  1.35 P_c to resolve the critical region; points closer than 1.3×10⁻³ in
  log₁₀ P (0.3 % in P) to the previous point are dropped.
- **Enthalpy grid** (`build_h_grid`): 312 temperatures geometrically spaced
  from 1.001 T_min to T_max, converted to enthalpy at the reference pressure
  min(2 P_c, P_max); four points are added below to reach the lowest h_min(P)
  over all pressures and four above to reach h(T_max) at the lowest pressure.
  The grid is shared by all pressure rows, so its spacing in h is fine at low
  temperature and coarse at high temperature.

**Fields.** At every grid node (`compute_row`) CoolProp is evaluated at (h, P),
with h raised to h_min(P) = h(1.001 T_min, P) where it is lower:

| Field | Quantity | Unit |
|---|---|---|
| v | specific volume | m³/kg |
| T | temperature | K |
| s | specific entropy | J/(kg K) |
| mu | dynamic viscosity | Pa s |
| k | thermal conductivity | W/(m K) |
| cp | isobaric heat capacity | J/(kg K) |
| dvdP | (∂v/∂P)_h = −(∂ρ/∂P)_h / ρ² | m³/(kg Pa) |
| dvdh | (∂v/∂h)_P = −(∂ρ/∂h)_P / ρ² | m³/J |

Single-phase derivatives use CoolProp's `first_partial_deriv`; inside the dome
(below 0.9999 P_c, 0 ≤ quality ≤ 1) the homogeneous-equilibrium derivatives
use `first_two_phase_deriv`. Inside the dome the transport properties and c_p
are quality-weighted from the saturated values: 1/μ = x/μ_v + (1 − x)/μ_l,
k = x k_v + (1 − x) k_l, c_p = x c_p,v + (1 − x) c_p,l.

- **Failed points** (`fill_row`): interior gaps in a row are filled by linear
  interpolation in h; points below the first valid node copy that node; points
  above the last valid node are extended as an ideal gas with constant c_p
  (T = T₀ + Δh/c_p, v = R T/P, s = s₀ + c_p ln(T/T₀), μ ∝ T^0.7, k ∝ T^0.8,
  (∂v/∂P)_h = −v/P, (∂v/∂h)_P = R/(P c_p)).
- **Saturation table** (`build_sat_table`): 200 pressures geometrically spaced
  from max(1.001 P_triple, 100 Pa) to 0.9995 P_c, with columns P, T, h_l, h_v,
  v_l, v_v, s_l, s_v.

### File format FNTB

Little-endian binary (`.fntb`), read by `FluidTable::from_bytes`:

```
b"FNTB" | u32 version (= 1) | u32 header length | header JSON (UTF-8)
f64[nP]  log10 P grid
f64[nH]  h grid
f64[nP]  h_min(P)
f64[nP]  h_max(P)            (read and not used)
f32[8][nP][nH]  fields in the order v, T, s, mu, k, cp, dvdP, dvdh
f64[nSat][8]    saturation table P, T, hl, hv, vl, vv, sl, sv
```

The header holds `key`, `label`, `coolprop_name`, `coolprop_version`,
`molar_mass` (kg/mol), `R` (J/(kg K)), `Tcrit`, `Pcrit`, `rhocrit`, `Ttriple`,
`Ptriple`, `Tmin`, `Tmax`, `Pmin`, `Pmax`, `nP`, `nH`, `nSat`, `fields` and
`sat_columns`. The fields are stored in single precision to halve the file
size; grids and the saturation table are double precision.

Native front ends load every `*.fntb` file of a directory
(`FluidLibrary::load_dir`). The web UI loads the same bytes, shipped as
gzip-compressed base64 text. `FluidLibrary::get` finds a table by key or label,
ignoring case.

### Lookup

`FluidTable::props_ph(P, h)` returns a `Props` record (P, h, v, ρ, T, s, μ, k,
c_p, (∂v/∂P)_h, (∂v/∂h)_P, homogeneous-equilibrium sound speed a, quality x).
It proceeds as follows.

**Below P_min** (100 Pa): the state at (P_min, h) is extrapolated as an ideal
gas at constant h. With r = P_min/P: v ← v r, ρ ← ρ/r,
(∂v/∂h)_P ← (∂v/∂h)_P · r, (∂v/∂P)_h = −v/P, and x is undefined. T, s, μ, k and
c_p keep their values at P_min. This keeps evacuated lines and chambers smooth
down to vacuum (P is floored at 1×10⁻⁹ Pa).

**Above P_max**: P is clamped to P_max; the returned state has P = P_max.

**Below the critical pressure**, with a saturation state available
(`sat_full`: the saturation table interpolated linearly in log₁₀ P, plus the
slopes dh_l/dP, dh_v/dP, dv_l/dP, dv_v/dP of the piecewise-linear table), h is
clamped to [h_min(P), h_max of the grid] and then:

*Inside the dome* (h_l < h < h_v), the exact lever rule on the saturation
table is used (`two_phase`):

```
x         = (h − h_l)/(h_v − h_l)
v         = v_l + x (v_v − v_l)
s         = s_l + x (s_v − s_l)
T         = T_sat(P)
(∂v/∂h)_P = (v_v − v_l)/(h_v − h_l)
dx/dP     = (−h_l' (h_v − h_l) − (h − h_l)(h_v' − h_l')) / (h_v − h_l)²
(∂v/∂P)_h = v_l' + x (v_v' − v_l') + (v_v − v_l) dx/dP
```

where the prime is d/dP along the saturation line. μ, k and c_p come from the
plain table interpolation (they are quality-weighted in the table).

*In the liquid and vapour regions* (`props_ph_sat`, `sat_row`), the lookup
takes the two table rows bracketing log₁₀ P. On each row it evaluates the
fields at the same enthalpy distance from that row's saturation point,
Δh = h_l − h for liquid or h − h_v for vapour, interpolating linearly in h
between the grid nodes; if the bracketing grid node lies beyond the saturation
line, it is replaced by the saturation point (v, T and s from the saturation
table; the other fields copied from the single-phase node). Liquid and vapour
data are therefore never mixed across the saturation line. Between the two
rows, v is interpolated by a cubic Hermite polynomial in log₁₀ P whose end
slopes are the derivatives along the constant-Δh path,
dv/dP = (∂v/∂P)_h + (∂v/∂h)_P · dh_sat/dP (scaled by P ln 10 Δlog₁₀ P); all
other fields are linear in log₁₀ P. If either row has no saturation state
(rows at or above P_c or below the saturation table), the plain scheme is
used.

**Plain scheme** (`props_ph_plain`, used at and above P_c and as fallback):
bilinear interpolation in (log₁₀ P, h) for T, s, μ, k, c_p, (∂v/∂P)_h and
(∂v/∂h)_P; v by a cubic Hermite polynomial in log₁₀ P using the stored
(∂v/∂P)_h at both rows (exact for ideal-gas behaviour, v ∝ 1/P) and linear in
h. Inside the dome v is linear in h at fixed P, so the linear h interpolation
of v (not of ρ) is exact there. If the Hermite value is not positive, the
bilinear value is used.

All paths floor μ at 1×10⁻⁷ Pa s, k at 1×10⁻⁴ W/(m K) and c_p at
1 J/(kg K). The sound speed follows from the tabulated derivatives,

```
1/a² = (∂ρ/∂P)_s = −ρ² ((∂v/∂P)_h + v (∂v/∂h)_P)
```

and is 0 where this is not positive.

### Derivative blending near saturation

The equilibrium compressibility (∂v/∂P)_h jumps by orders of magnitude when a
liquid reaches x = 0. A cavitating liquid line would then force the integrator
into very small steps. `props_ph` therefore blends (∂v/∂P)_h and (∂v/∂h)_P,
and recomputes a, over a quality band of ±0.01 around each saturation line
(`BAND` = 0.01):

```
w = smoothstep((x − x_line + 0.01) / 0.02),   smoothstep(t) = t²(3 − 2t), t clamped to [0, 1]
∂ = (1 − w) ∂_low + w ∂_high
```

At the liquid line ∂_low is the single-phase liquid derivative (at the state
itself if x ≤ 0, else at h_l) and ∂_high the two-phase derivative (at the state
itself if x > 0, else just inside the dome at h_l). The vapour line is treated
the same way with two-phase below and vapour above. Density, enthalpy,
temperature and all other properties remain those of the unblended lookup;
only the two derivatives and the sound speed change, which acts like a short
relaxation zone in the mass balance.

### Other functions

| Function | Purpose |
|---|---|
| `props_ph_x` | `props_ph` plus the quality x = (h − h_l)/(h_v − h_l) (< 0 subcooled, > 1 superheated, NaN at or above P_c) |
| `h_pt` | h from (P, T) by Illinois regula falsi on [h_min, h_l] for T < T_sat, [h_v, h_max] otherwise; error if T is outside the table |
| `h_px` | h from (P, x); error at or above P_c |
| `h_ps`, `h_ps_near` | h from (P, s): Newton with (∂s/∂h)_P = 1/T from a guess (8 iterations, tolerance 1×10⁻⁷ relative), falling back to bracketing |
| `psat_t`, `sat_p`, `sat_sl` | saturation pressure from T; saturation state and liquid entropy from P |
| `quality` | equilibrium quality from (P, h) |

## Numerical treatment

- All grids are searched by binary search (`partition_point`); interpolation
  weights are clamped to [0, 1], so no lookup extrapolates beyond the grid
  except the explicit ideal-gas branch below P_min.
- Each saturation row state (`row_sat`) is precomputed at load time for every
  table pressure.
- The lookup is continuous across the saturation lines in v, h and T; the
  derivatives are continuous through the blending band. The right-hand side is
  therefore continuous everywhere the state stays inside the table, which the
  implicit integrator relies on.

## Parameters and outputs

A model selects a fluid with the `fluid` parameter of a boundary, tank,
junction or pipe ([FNT-SLV-DOC-001](network-formulation.md)). Every node reports
`.P`, `.T`, `.rho`, `.h` and `.x`. The CLI command `fluidnet props FLUID P T`
and the Python function `fluidnet.props(fluid, P, T=…, h=…)` give direct
access to the lookup.

### Accuracy and cost

The former `docs/ARCHITECTURE.md` (v0.5.0, in the git history) reports the following accuracy of `props_ph` against
CoolProp at random states (99th percentile of the relative error):

| Region | Density | Temperature |
|---|---|---|
| Single phase | 2–4×10⁻⁴ | < 0.1 K |
| On the saturation lines | about 1×10⁻³ | < 0.1 K |
| Two-phase | about 2×10⁻⁴ | < 0.1 K |

The CLI command `fluidnet propsbatch FLUID` (reads "P h" lines, prints ρ, T, s
and a) supports such a comparison; the comparison script itself is not in the
repository (see open items).

One lookup costs about 0.25–0.3 µs natively. `fluidnet bench MODEL` prints the
cost of `props_ph` for the model's first fluid.

## Limits and assumptions

- Homogeneous equilibrium inside the dome: no metastable liquid or vapour, no
  slip. Metastability is represented only by the blending band.
- P above P_max is clamped, not extrapolated: a node above P_max reports
  P_max and uses its properties. h outside [h_min(P), h_max] is clamped
  likewise. A state that leaves the table usually shows up as a step-size
  underflow in the transient solver.
- Below P_min temperature and transport properties do not change with
  pressure.
- Near the critical point the saturation table ends at 0.9995 P_c and the
  saturation-aware scheme needs a saturation state on both bracketing rows;
  between the last subcritical row and P_c the plain bilinear scheme is used.
- The fields are single precision (about 7 significant digits).
- Parahydrogen is the only hydrogen modification available.

## Verification

- `liquid_orifice_matches_bernoulli` matches the analytic orifice flow
  computed with the CoolProp water density at 10 bar and 293.15 K
  (998.6247 kg/m³) to within 2×10⁻⁴, which bounds the table density error at
  that state.
- `n2_blowdown_matches_isentropic_ideal_gas` checks gas properties along an
  isentrope; `saturated_ln2_flashing_matches_hem` checks the two-phase lookup
  and isentropic flash against a CoolProp-integrated HEM flow;
  `water_hammer_matches_joukowsky` checks the liquid sound speed.
- See [Analytic verification cases](../04-verification/analytic-cases.md).

## Code location

| Topic | Location |
|---|---|
| Table generation | `tools/gen_fluid_tables.py`: `FLUIDS`, `build_p_grid`, `build_h_grid`, `compute_row`, `fill_row`, `build_sat_table`, `generate` |
| Reader | `crates/fluidnet-core/src/props.rs`: `FluidTable::from_bytes` |
| Lookup | `props.rs`: `props_ph`, `props_ph_sat`, `sat_row`, `two_phase`, `props_ph_plain`, `sat_full` |
| Inversions | `props.rs`: `h_pt`, `h_px`, `h_ps`, `h_ps_near`, `invert_h` |
| Library | `props.rs`: `FluidLibrary` |
| UI copies | `ui/scripts/copy-assets.mjs`, `ui/src/engine.ts` (`FLUIDS`) |

### Adding a fluid

1. Add an entry to `FLUIDS` in `tools/gen_fluid_tables.py` with the CoolProp
   fluid name, a label, T_max and P_max. Check that CoolProp's `HEOS` backend
   supports the fluid over that range, including transport properties.
2. Generate the table: `python tools/gen_fluid_tables.py fluids KEY`
   (requires CoolProp and numpy). The script prints the grid size, the file
   size and the number of h nodes inside the dome at 1 bar and 0.5 P_c; check
   that the dome is resolved.
3. Copy the file to `python/fluidnet/fluids/` for the Python wheel.
4. For the web UI, run `npm run wasm` in `ui/` (regenerates
   `ui/public/fluids/*.txt`) and add the key and label to `FLUIDS` in
   `ui/src/engine.ts`.
5. If the fluid is a propellant, add combustion tables
   ([FNT-CMB-DOC-001](combustion.md)).
6. Check a few states against CoolProp, for example with
   `fluidnet propsbatch KEY`.

## Open items

- The accuracy figures above come from the former `docs/ARCHITECTURE.md`; the comparison
  script and its results are not in the repository. Add a script (for example
  `tools/check_fluid_tables.py`) and a results file so that the numbers are
  traceable.
- The lookup cost has no stored benchmark record.
- The `h_max(P)` array is written to the file but not used by the reader.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
