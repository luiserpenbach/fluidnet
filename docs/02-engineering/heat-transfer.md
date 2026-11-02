# Heat transfer and walls

FNT-HTX-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes how heat enters the fluid network: tank heat input, pipe
walls with a prescribed temperature or heat flux, and pipe walls with heat
capacity between a hot side and the coolant. For walls with heat capacity it
covers the material model, conduction, the coolant-side correlation and the
three hot-side options (combustion chamber gas with the Bartz correlation,
prescribed gas temperature and coefficient, prescribed heat flux), including
the enthalpy-based driving potential for chamber gas. The chamber state that
the hot side uses is described in [FNT-CMB-DOC-001](combustion.md).

## Model and equations

All heat enters the energy equation of a `Volume` node as the term Q
([FNT-SLV-DOC-001](network-formulation.md)). A pipe's heat is distributed over
its chain of nodes (inlet node, internal nodes, outlet node) in proportion to
the length each node represents: the share f = 1/N for internal nodes and
f = 1/(2N) for the two end nodes (½ each for a single-segment pipe). The
wetted area of the whole pipe is A_wet = n_p π d L, with n_p parallel channels
of diameter d.

### Tank heat input

A tank adds its scheduled heat input `Q` (W, default 0) to its node.

### Coolant-side heat transfer coefficient

All pipe walls use the same coolant-side coefficient (`coolant_htc` in
`network/mod.rs`), evaluated with the mixture properties of the node and the
mean absolute mass flow of the pipe segments adjacent to that node (one at the
ends, two inside):

```
Re  = ṁ̄ d / (A μ)               A = n_p π d²/4, so Re is that of one channel
Pr  = c_p μ / k                  clamped to [0.1, 1000]
f   = (0.79 ln Re − 1.64)^(−2)
Nu_t = (f/8)(Re − 1000) Pr / (1 + 12.7 √(f/8) (Pr^(2/3) − 1))   for Re > 1000, else 0
Nu  = (3.66^10 + Nu_t^10)^(1/10)
h_c = Nu k / d
```

Nu_t is Gnielinski's correlation with Petukhov's friction factor; the
power-10 blend with the laminar constant-wall-temperature value 3.66 makes the
transition smooth and gives 3.66 at low Reynolds number (`flow::nusselt`).

For walls with heat capacity an optional property-ratio correction is applied:

```
Nu ← Nu · (T_b / T_w)^n          T_b/T_w clamped to [0.05, 20]
```

with n = `cool_correction` (default 0; 0.55 is a common value for
supercritical hydrogen).

### Pipe wall with prescribed temperature or heat flux

Without `wall_thickness`, a pipe may have a wall at a prescribed temperature
`T_wall` (schedule) or a prescribed heat flux `q_wall` (W/m², schedule). For
each chain node with share f:

```
T_wall given:  Q = h_c · f A_wet · (T_wall − T_b)
q_wall given:  Q = q_wall · f A_wet
```

The prescribed-temperature form uses h_c without the property-ratio
correction. The wall has no heat capacity.

### Wall elements with heat capacity

With `wall_thickness` t given, the pipe gets one wall element per chain node,
named `<pipe>.w0 … <pipe>.wN` from inlet to outlet, each with its own
temperature state T_w ([FNT-SLV-DOC-001](network-formulation.md)). Each element
lies between a hot side and the coolant in its node:

```
m c(T_w) dT_w/dt = Q_hot − Q_cool
```

**Mass.** The total wall mass is

```
m_total = F_m ρ_w n_p π (d + t) t L
```

with ρ_w the material density and F_m = `wall_mass_factor` (default 1) for
ribs, closeouts and flanges. Element i receives f_i m_total.

**Material and heat capacity.** The heat capacity follows the Debye model,
scaled to the room-temperature value of the material:

```
c(T) = c_300 · D(θ_D/T) / D(θ_D/300 K)
D(x) = (3/x³) ∫₀ˣ t⁴ eᵗ/(eᵗ − 1)² dt          (D = 1 for x < 1×10⁻³)
```

D is integrated with Simpson's rule on 64 intervals; T is floored at 4 K.
Copper's heat capacity falls by a factor of about 4 between 300 K and 50 K,
which governs the chill-down time of cryogenic hardware.

| `wall_material` | ρ_w [kg/m³] | k_w [W/(m K)] | c_300 [J/(kg K)] | θ_D [K] |
|---|---|---|---|---|
| `cucrzr` (default) | 8890 | 320 | 390 | 343 |
| `copper` | 8940 | 390 | 385 | 343 |
| `grcop42` | 8790 | 300 | 385 | 343 |
| `nickel` | 8890 | 70 | 456 | 450 |
| `stainless` | 8000 | 16 | 500 | 470 |
| `inconel718` | 8190 | 11.4 | 435 | 450 |
| `aluminum` | 2700 | 167 | 896 | 428 |

The conductivity k_w is constant with temperature. The values are those in
`machines::material`; their sources are not recorded in the code (see open
items).

**Conduction.** The state T_w is the temperature at mid-thickness. Conduction
through half the wall thickness, with the resistance t/(2 k_w A) per side, is
in series with the convective resistance on each side.

**Coolant side.**

```
A_c    = f_i A_wet · F_c
Q_cool = (T_w − T_b) / (1/(h_c A_c) + t/(2 k_w A_c))          (0 if h_c ≤ 0)
```

with F_c = `cool_area_factor` (default 1), the fin effectiveness times the
extra wetted area of the channel walls, and h_c including the property-ratio
correction at T_w. Q_cool is added to the node's heat.

**Hot-side area.** The hot-gas-side area of the whole line is `hot_area`
(required for chamber and gas hot sides; for a heat-flux hot side it defaults
to A_wet). It is distributed over the elements in proportion to the length
share and, if a contour is given, to the local radius:

```
w_i = f_i · √(A/A_t)_i     (with hot_contour)     or  w_i = f_i
A_h,i = hot_area · w_i / Σ w_j
```

`hot_contour` [[x/L, A/A_t], …] gives the local area ratio of the hot-gas flow
along the line from inlet (x/L = 0) to outlet (x/L = 1); element i sits at
x/L = i/N and takes the interpolated value, at least 1. Without a contour,
chamber-heated elements use the single value `hot_contraction` (default 3).

**Hot side, prescribed flux** (`q_wall` with `wall_thickness`):

```
Q_hot = q_wall · A_h
```

**Hot side, prescribed gas** (`hot_T`, `hot_htc`, both schedules):

```
Q_hot = (T_g − T_w) / (1/(h_g A_h) + t/(2 k_w A_h))           (0 if h_g ≤ 0)
```

**Hot side, combustion chamber** (`hot_chamber` names a chamber):

```
Q_hot = (T_drive − T_w) / (1/(h_g A_h) + t/(2 k_w A_h))
h_g   = F_h · h_Bartz
```

with F_h = `hot_htc_factor` (default 1). The Bartz coefficient
(`combustion::bartz_htc`) in SI form, with the chamber's current state:

```
h_Bartz = 0.026 / D_t^0.2 · μ^0.2 c_p / Pr^0.6 · (P_c / c*)^0.8 · (A_t/A)^0.9 · σ
R    = R_u / M
c_p  = γ_f R / (γ_f − 1)                     frozen
Pr   = 4 γ_f / (9 γ_f − 5)                   Eucken estimate
μ    = 46.6×10⁻¹⁰ M^0.5 (1.8 T_c)^0.6 × 17.858 Pa s     (Bartz estimate in lb/(in s) with T in °R, converted)
σ    = (½ T_w/T_c + ½)^(−0.68)               T_w/T_c clamped to [0.02, 1]
```

D_t is the throat diameter, P_c, c\*, T_c, M and γ_f are the chamber pressure,
characteristic velocity (including the c\* efficiency), temperature, molar mass
(g/mol) and frozen isentropic exponent ([FNT-CMB-DOC-001](combustion.md)), and
A/A_t ≥ 1 the local area ratio of the element. σ is the Bartz wall-temperature
correction evaluated at low Mach number (chamber barrel and convergent
section), with the wall element temperature T_w. The curvature factor
(D_t/r_c)^0.1 of the full Bartz equation is not included (equivalent to a
factor of 1). h_Bartz = 0 if P_c, c\* or T_c is not positive.

**Driving potential** (`hot_driving`). With `temperature`, T_drive = T_c, so the
flux is driven by c_p,frozen (T_c − T_w). With `enthalpy` (default, and only if
the combustion table carries the wall data and the chamber has started
ignition), the driving potential is the equilibrium enthalpy difference between
the chamber gas and the same gas in equilibrium at the wall temperature:

```
Δh(T_w) = max(Δh_wall − c_p,wall (T_w − 600 K), 0)
T_drive = (1 − ι) T_cold + ι (T_w + Δh(T_w)/c_p,frozen)
```

Δh_wall is the tabulated difference h_c − h_eq(600 K) at the chamber's O/F and
P_c, corrected for the injection enthalpy and scaled by η_c\*², and c_p,wall is
the tabulated equilibrium heat capacity near the wall
([FNT-CMB-DOC-001](combustion.md)); ι is the ignition progress and T_cold the
pre-ignition gas temperature. With negligible wall resistance the flux becomes

```
q = (h_g / c_p,frozen) · (h_c − h_eq(T_w))
```

This includes the heat released when dissociated species recombine at the cold
wall. For LOX/LH2 it makes the heat flux fall with mixture ratio on the
fuel-rich side: the tabulated Δh_wall falls by about 4 % from O/F 5.63 to 4.67
at 32.6 bar, while c_p,frozen (T_c − T_w) rises (unit test
`wall_enthalpy_potential_follows_mixture_ratio`). On the RL10A-3-3A model the
predicted turbine inlet temperature is within 0.84 % of the test data at all
test points with this potential ([RL10A-3-3A validation](../04-verification/rl10.md)).

**Hot-side surface temperature.** For output, the hot-side surface temperature
is T_hot = T_w + Q_hot · t/(2 k_w A_h).

## Numerical treatment

- The wall temperature is a state, so the wall acts as a thermal capacitance
  between the hot gas and the coolant. Its time constant is m c / (h A), which
  for thin copper walls can be much shorter than the flow time constants; the
  implicit integrator handles this stiffness.
- The coolant-side coefficient depends on the flows of the adjacent pipe
  segments; the sparsity pattern of a wall element therefore includes the
  reads of those branches, and for a chamber-heated wall the chamber states,
  the port node states and the reads of the injector branches
  ([FNT-SLV-DOC-001](network-formulation.md)).
- c(T_w) is evaluated with 64-interval Simpson integration on every
  right-hand-side call for every wall element.

## Parameters and outputs

Pipe parameters: `T_wall`, `q_wall`, `wall_thickness`, `wall_material`,
`wall_mass_factor`, `wall_T0` (default: initial fluid temperature of the
element's node), `hot_chamber`, `hot_area`, `hot_contraction`,
`hot_htc_factor`, `hot_contour`, `hot_driving`, `hot_T`, `hot_htc`,
`cool_correction`, `cool_area_factor`, `parallel`. Tank: `Q`. Units and
defaults: [Component reference](../03-user-guide/component-reference.md#pipe).
`T_wall` cannot be combined with `wall_thickness`.

| Channel | Unit | Meaning |
|---|---|---|
| `<pipe>.Q` | W | Heat into the coolant: sum of Q_cool over the wall elements, or of the prescribed-wall heat over the chain nodes |
| `<pipe>.Q_hot` | W | Sum of Q_hot over the wall elements |
| `<pipe>.Tw_max` | K | Largest hot-side surface temperature T_hot of the elements |
| `<pipe>.w<i>.T` | K | Mid-thickness wall temperature of element i |
| `<pipe>.w<i>.T_hot` | K | Hot-side surface temperature of element i |
| `<pipe>.w<i>.Q` | W | Q_cool of element i |

## Limits and assumptions

- Conduction is one-dimensional through the wall thickness; there is no axial
  conduction between wall elements and no circumferential variation (no fin
  model beyond the area factor).
- Constant thermal conductivity; heat capacity from the Debye model only,
  which underestimates metals at low temperature (open item OI-011 in the
  [register](../01-project/open-items.md)).
- The coolant-side correlation is single-phase forced convection applied with
  homogeneous mixture properties. Boiling and two-phase heat transfer (for
  example during chill-down) are not modelled (OI-012).
- The Bartz σ factor is the low-Mach form and is evaluated with the
  mid-thickness wall temperature; in the throat and divergent nozzle the
  coefficient relies on the local area ratio only. The curvature factor is
  omitted.
- Radiation from the hot gas and from nozzle extensions is not modelled.
- The enthalpy driving potential assumes local equilibrium at the wall
  temperature, linearised about 600 K with c_p,wall.
- The hot-side heat of all wall elements of one pipe comes from one chamber;
  film cooling and mixture-ratio gradients near the wall are represented only
  through `hot_htc_factor`.

## Verification

- `heated_pipe_closes_energy_balance` (`tests/validation.rs`): a pipe with a
  prescribed flux; Q = q π d L to 1×10⁻⁹ and ṁ Δh = Q to 5×10⁻³.
- `expander_cycle_conserves_energy_and_torque`: the hydrogen enthalpy rise
  across the cooling jacket equals the jacket heat `jacket.Q` to 1×10⁻⁶.
- `copper_heat_capacity` (`src/machines.rs`): c = 385 J/(kg K) at 300 K, within
  20 J/(kg K) of the tabulated 100 J/(kg K) at 50 K and 254 J/(kg K) at 100 K.
- `wall_enthalpy_potential_follows_mixture_ratio` (`src/combustion.rs`).
- System level: [RL10A-3-3A validation](../04-verification/rl10.md);
  analytic cases: [Analytic verification cases](../04-verification/analytic-cases.md).

## Code location

| Topic | Location |
|---|---|
| Coolant coefficient | `crates/fluidnet-core/src/network/mod.rs`: `coolant_htc`; `crates/fluidnet-core/src/flow.rs`: `nusselt` |
| Prescribed walls, tank heat, wall elements | `network/mod.rs`: `Network::evaluate`, step 6 ("heat"); `Network::rhs` (walls) |
| Wall construction, area distribution, contour | `crates/fluidnet-core/src/network/compile.rs`: match arm `"pipe"`, closures `share`, `eps_at`, `weight` |
| Materials, Debye heat capacity | `crates/fluidnet-core/src/machines.rs`: `material`, `Material::cp`, `debye` |
| Bartz coefficient | `crates/fluidnet-core/src/combustion.rs`: `bartz_htc` |
| Wall enthalpy potential | `combustion.rs`: `CombState::dh_wall`, `cp_wall`, `T_WALL_REF`; `network/mod.rs`: `eval_chamber` |

## Open items

- The sources of the material properties in `machines::material` (density,
  conductivity, c_300, θ_D) are not recorded.
- The catalog spells the material `aluminum`; `README.md` writes "aluminium"
  and uses display names (CuCrZr, GRCop-42, Inconel 718). Model files must use
  the lower-case keys of the table above; the lookup is case-sensitive.
- The code comment on σ calls T_w a hot-gas-side surface temperature
  estimate, but the mid-thickness state is passed. Using T_hot would need an
  iteration or the previous value; the effect has not been assessed.
- Wall elements at a pipe end attached to a boundary exchange heat with the
  boundary state, and that heat is not accounted in any balance
  ([FNT-SLV-DOC-001](network-formulation.md)).
- The former `docs/ARCHITECTURE.md` reported a turbine inlet temperature
  error of 5–7 % on the RL10A-3-3A with the temperature potential. That
  comparison has no recorded run; repeat it with `hot_driving: temperature`
  and record it in the RL10 analysis.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
