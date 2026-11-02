# Flow elements

FNT-CMP-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes the flow laws of the passive network elements as they are
implemented: pipes (friction, minor losses, elevation, parallel channels,
inertia), the restriction mass flux used by orifices, venturis, valves, check
valves, regulators and losses (liquid with cavitation and flashing,
two-phase, real-gas nozzle flow, pressure recovery, smoothing near zero
pressure difference), the valve characteristics and pressure-actuated valves,
and the node-like elements (boundary, tank, junction) and mass-flow sources.
Pumps and turbines are in [FNT-TRB-DOC-001](turbomachinery.md); pipe wall heat
transfer is in [FNT-HTX-DOC-001](heat-transfer.md). How elements become nodes
and branches is in [FNT-SLV-DOC-001](network-formulation.md).

## Model and equations

### Pipe

A pipe is discretised into N momentum segments of length ΔL = L/N with N − 1
internal volumes and half cells at the ends
([FNT-SLV-DOC-001](network-formulation.md)). Each segment obeys

```
(ΔL/A) dṁ/dt = P_a − P_b − ΔP_f − ρ̄ g Δz/N
ΔP_f = (f ΔL/d + K/N) ṁ |ṁ| / (2 ρ̄ A²)
A    = n_p π d²/4
Re   = |ṁ| d / (A μ̄)
ρ̄    = 2/(v_a + v_b),   μ̄ = (μ_a + μ_b)/2
```

with d the diameter of one channel, n_p the number of identical parallel
channels (`parallel`), K the sum of the minor-loss coefficients of the whole
pipe, Δz the outlet height minus the inlet height and g = 9.80665 m/s². Since
ṁ/A is the mass flux of one channel, Re and the friction loss are those of a
single channel; the inertia term (ΔL/A) is that of all channels together.

The Darcy friction factor is Churchill's (1977) correlation, which is smooth
and valid from laminar through transitional to fully rough flow
(`flow::churchill_f`):

```
A_c = (2.457 ln(1 / ((7/Re)^0.9 + 0.27 ε/d)))^16
B_c = (37530/Re)^16
f   = 8 ((8/Re)^12 + (A_c + B_c)^(−1.5))^(1/12)
```

with ε the absolute roughness and Re limited to at least 1×10⁻³. In the
laminar limit f = 64/Re.

### Restriction flux

All restrictions compute the mass flow from the effective area and a mass flux
G (`flow::restriction_flux`):

```
ṁ = sign · CdA · u · G(state_up, P_t)
```

The upstream state is that of the node with the higher pressure; sign = +1
for flow from `in` to `out`, −1 otherwise. CdA is the effective area (see
below), u the opening (1 for fixed restrictions), and P_t the throat pressure,
equal to the downstream pressure unless pressure recovery is set. With
P₀ = P_up and ΔP = P₀ − P_t, the flux model is chosen from the upstream state.
The parameter `model` selects `auto` (default, below) or `incompressible`,
which always uses G = √(2 ρ₀ ΔP) without a cavitation limit.

**Subcooled liquid.** The upstream state is treated as liquid if T₀ < T_c and
either its equilibrium quality is negative or it is at or above the critical
pressure (supercritical pressure below the critical temperature). With
P_sat = P_sat(T₀) (limited to P₀), η_s = P_sat/P₀ and η = P_t/P₀:

- If η ≥ η_s (the throat stays above the vapour pressure), or no saturation
  pressure exists, the flow is incompressible: G = √(2 ρ₀ ΔP).
- Otherwise Leung's omega method with the subcooled-inlet extension is used.
  The saturation omega ω_s is the two-point value of API 520 Annex C: an
  isentropic flash of saturated liquid at P_sat to 0.9 P_sat gives
  ω_s = 9 (v₉/v_l − 1), with v₉ the specific volume at 0.9 P_sat and the
  saturated-liquid entropy, found by `h_ps_near` (ω_s ≥ 1×10⁻⁶; ω_s = 1 if the
  flash cannot be evaluated). The normalised flux with ω = ω_s is given below.
- The critical (choking) ratio η_c is the η that maximises the flux on
  [min(1×10⁻⁴, 0.5 η_s), η_s], found by golden-section search (at most 40
  iterations, interval below 1×10⁻⁷) and compared with the value at the end
  point η_s (`omega_choke`). If η ≤ η_c the flow is choked at the maximum flux.
  At high subcooling the maximum is at η_s, which is the cavitating limit
  G = √(2 ρ₀ (P₀ − P_sat)); near saturation the flow chokes like
  homogeneous-equilibrium flashing flow.

```
G/√(P₀ ρ₀) = √(2 [(1 − η_s) + ω η_s ln(η_s/η) − (ω − 1)(η_s − η)]) / (ω (η_s/η − 1) + 1)
```

(`omega_g`; ln(η_s/η) is replaced by its third-order series when
η_s − η < 1×10⁻⁶ η_s). At η = η_s this equals √(2 ρ₀ (P₀ − P_sat)), so the
incompressible and omega branches join continuously.

**Two-phase inlet** (0 ≤ x₀ ≤ 0.97). The omega method with η_s = 1 and the
two-point ω from the inlet isentrope: ω = 9 (v(0.9 P₀, s₀)/v₀ − 1). Choking is
found as above with η_s = 1.

**Gas and supercritical fluid** (x₀ ≥ 1.03, or no defined quality and not
liquid-like). Isentropic nozzle flow with the real-gas isentropic exponent
κ = 1/ω, ω = −(P₀/v₀)(∂v/∂P)_s = −(P₀/v₀)((∂v/∂P)_h + v₀ (∂v/∂h)_P) from the
table derivatives at the inlet, κ limited to [1.001, 3]:

```
η_c = (2/(κ + 1))^(κ/(κ − 1))
η_e = max(η, η_c)                       choked if η < η_c
G/√(P₀ ρ₀) = √(2κ/(κ − 1) (η_e^(2/κ) − η_e^((κ+1)/κ)))
```

**Blending** for 0.97 < x₀ < 1.03: G = (1 − b) G_two-phase + b G_gas with
b = smoothstep((x₀ − 0.97)/0.06), smoothstep(t) = t²(3 − 2t). The choked flag
is taken from the model with weight ≥ 0.5.

**Pressure recovery.** For orifices and venturis with recovery r
(`recovery`, limited to [0, 0.98]), the fraction r of the throat pressure drop
is recovered downstream. The throat pressure used in the flux is

```
P_t = max(P₀ − (P₀ − P_down)/(1 − r), 1×10⁻³ P₀)
```

Without cavitation this gives ṁ = CdA √(2 ρ₀ (P₀ − P_down)/(1 − r)); the throat
reaches the vapour pressure, and the venturi cavitates, at a smaller overall
pressure drop than a sharp orifice. Once choked, the flow is independent of the
downstream pressure.

**Smoothing near ΔP = 0.** Every flux is multiplied by

```
F(ΔP) = √|ΔP| / (ΔP² + ε²)^(1/4),   ε = 10 Pa + 1×10⁻⁵ P₀
```

(`reg_factor`). For |ΔP| ≫ ε, F → 1; for |ΔP| ≪ ε the √ΔP law becomes linear
in ΔP with a finite slope, so the flow is smooth through reversal and the
Jacobian stays finite. G = 0 for ΔP ≤ 0.

### Effective area

`Params::cda` gives the effective area CdA of an element:

| Given | CdA |
|---|---|
| `Cv` (valves only) | Cv × 1.699×10⁻⁵ m² |
| `Kv` (valves only) | Kv × 1.9642×10⁻⁵ m² |
| `area` or `diameter` | cd × A, A = `area` or π d²/4 |

The Cv and Kv factors are the CdA that passes 1 US gal/min of water at 1 psi
and 1 m³/h at 1 bar respectively (ρ = 1000 kg/m³). A `loss` element with
coefficient K and reference area A is a restriction with CdA = A/√K, so that
ΔP = K ρ u²/2 for incompressible flow; it uses the `auto` flux model.

Default discharge coefficients: orifice 0.7, venturi 0.95, valve 0.8, check
valve 0.7, regulator 0.8.

### Valve

A valve has a stem position s(t) (`opening`, a constant or a schedule,
clamped to [0, 1]) and a characteristic:

```
linear            u = s
equal_percentage  u = (50^s − 1)/49
quick_opening     u = √s
```

The area fraction is max(u, leak), with `leak` the closed-position area
fraction (default 1×10⁻⁶). Opening times enter the schedule breakpoints, so
the transient solver steps exactly onto the start and end of each ramp.

### Pressure-actuated valve

A valve with `sense` set has an actuator position state u_a instead of a
schedule. The command is linear in a sensed pressure signal:

```
σ     = P(sense) − P(sense_ref)          (sense_ref optional, else 0)
u_cmd = u_lo + (u_hi − u_lo) · clamp((σ − σ_lo)/(σ_hi − σ_lo), 0, 1)
du_a/dt = (u_cmd − u_a)/τ
```

with σ_lo = `sense_lo`, σ_hi = `sense_hi` (σ_hi > σ_lo), u_lo = `opening_lo`,
u_hi = `opening_hi` (both clamped to [0, 1]; u_hi < u_lo gives a valve that
closes on rising pressure) and τ = `actuator_time` > 0. The initial position is
`opening` at t = 0 s. The valve area fraction is max(characteristic(u_a),
leak), with u_a clamped to [0, 1].

`sense` and `sense_ref` name a node (a node-like component id, an implicit
junction such as `main_valve.in`, or `component.port`) or a combustion chamber
(its id, `chamber.Pc` or one of its ports), which senses the chamber pressure
P_c.

With `latch: true` the motion is one-way during transients: du_a/dt is limited
to ≥ 0 if u_hi ≥ u_lo, or ≤ 0 otherwise, so the valve stays where the signal
has driven it (pneumatic sequencing valves). The direct steady-state methods
switch the latch off; the transient settling path of the steady solver keeps
it on ([FNT-SLV-DOC-002](solvers.md)).

### Check valve

The opening follows the differential pressure ΔP = P_in − P_out:

```
u = leak + (1 − leak) · smoothstep((ΔP − ΔP_crack)/(ΔP_full − ΔP_crack))
```

with `cracking_dp` (default 0.2 bar), `full_open_dp` (default 0.7 bar, must
exceed the cracking pressure) and `leak` (default 1×10⁻⁶). Reverse flow sees
only the leakage area. There is no hysteresis and no discrete event.

### Pressure regulator

A proportional pressure-reducing regulator senses its own outlet pressure:

```
e = (P_set − P_out)/B + 0.5
u = leak + (1 − leak) · smoothstep(e)
```

with `set_pressure` P_set (constant or schedule), proportional band B
(`band`, default 2 bar) and `leak` (default 1×10⁻⁶). The valve is fully open
for P_out ≤ P_set − B/2 and closed for P_out ≥ P_set + B/2; the regulated
pressure droops across the band as the flow demand changes.

### Mass-flow source

`mass_flow` imposes ṁ(t) from `in` to `out` (constant or schedule), independent
of the pressures. The flow carries the enthalpy of the upstream node like any
other branch. It represents an ideal pump or flow controller; no power is added
to the fluid.

### Boundary

A `boundary` prescribes P(t) and either T(t) or a quality x(t)
([FNT-SLV-DOC-001](network-formulation.md)). It supplies fluid at its own state
and absorbs outflow at any state.

### Tank

A `tank` is a rigid `Volume` node of volume V with the mass and energy balance
of [FNT-SLV-DOC-001](network-formulation.md) and an optional heat input Q(t)
(`Q`, W, constant or schedule) added to the energy equation. Its contents are
homogeneous and in equilibrium. `P0` is required, with `T0` or `x0`.

### Junction

A `junction` is a `Volume` node of small volume (`volume`, default 1 mL) where
several elements meet. Pipe half cells at its ports add to its volume. Where
two elements are connected without a named junction, the compiler creates an
implicit junction ([FNT-SLV-DOC-001](network-formulation.md)).

## Numerical treatment

- Restrictions, check valves, regulators, losses and mass-flow sources are
  algebraic: they add no states and are evaluated in `Network::evaluate` from
  the current node states. Only pipe segments carry inertia.
- Valve, check-valve and regulator openings are smooth functions of time or
  pressure (smoothstep), so there are no discrete events and the right-hand
  side stays continuous.
- The flux functions depend on the upstream state through the table
  properties and derivatives; at the switch between the liquid, two-phase and
  gas branches the flux is continuous (liquid to two-phase at x = 0 through
  the omega formula, two-phase to gas through the blend), but its derivative
  is not smooth everywhere. The steady and transient Newton iterations accept
  iterates that straddle such kinks on the residual test
  ([FNT-SLV-DOC-002](solvers.md)).

## Parameters and outputs

Key parameters: pipe `length`, `diameter`, `roughness` (default 1.5 µm),
`segments` (default 4), `parallel` (default 1), `K` (default 0), `dz`
(default 0); restriction `diameter` or `area`, `cd`, `recovery` (orifice 0,
venturi 0.85), `model`; valve `opening`, `characteristic`, `Cv`, `Kv`, `leak`
and the actuator parameters; check valve `cracking_dp`, `full_open_dp`;
regulator `set_pressure`, `band`; loss `K` (default 0.3); `mass_flow` `mdot`.
The full list with defaults and units is in the
[Component reference](../03-user-guide/component-reference.md).

Outputs: every flow element has `.mdot` (kg/s). Restrictions add `.dp` (Pa,
in minus out), `.choked` (0 or 1) and, for valves, check valves and
regulators, `.opening` (area fraction). Pipes with one segment add `.dp` and
`.u_in` (m/s); multi-segment pipes add `.mdot_in`, `.mdot_out`, `.u_in` and
`.dp` over the whole pipe, and `.s<i>.mdot` per segment. Mass-flow sources add
`.dp`. Tanks add `.m` (kg).

## Limits and assumptions

- Pipe momentum has no convective acceleration term and uses static node
  pressures; the energy equation carries no kinetic energy, so a restriction
  is isenthalpic between its nodes.
- Friction uses the single-phase Churchill factor with homogeneous mixture
  density and viscosity, also in two-phase flow.
- The restriction flux is quasi-steady; the liquid inertia inside an orifice
  or valve is not modelled.
- Omega-method choking assumes homogeneous equilibrium flashing; delayed
  (non-equilibrium) flashing in short orifices is not modelled. For short
  orifices with slightly subcooled inlets, equilibrium models are known to
  give lower flows than measured.
- The gas flux uses one isentropic exponent evaluated at the inlet state.
- The check valve and regulator have no inertia, damping or hysteresis; their
  dynamics come only from the volumes around them. The regulator is purely
  proportional (no integral action, no dome pressure).
- `incompressible` restrictions do not cavitate or choke.

## Verification

In `crates/fluidnet-core/tests/validation.rs`:

| Test | Checks |
|---|---|
| `liquid_orifice_matches_bernoulli` | ṁ = 0.7 A √(2 ρ ΔP) for water to 2×10⁻⁴ |
| `pipe_friction_matches_darcy_colebrook` | Pipe flow against Darcy–Colebrook, within 1 % (Churchill differs by about 0.4 % in that regime) |
| `cavitating_venturi_chokes_at_vapour_pressure` | ṁ = 0.95 A √(2 ρ (P₀ − P_v)) to 1×10⁻³ and choked flag set |
| `saturated_ln2_flashing_matches_hem` | Saturated LN₂ through an orifice against HEM by isentropic integration with CoolProp, within 3 %, choked |
| `n2_blowdown_matches_isentropic_ideal_gas` | Choked gas discharge from a tank against the ideal-gas solution, within 2×10⁻³ |
| `water_hammer_matches_joukowsky` | Pipe inertia and compressibility: Joukowsky surge within 2 %, period 4L/a within 1 % |
| `regulator_holds_set_pressure_within_band` | Regulated plenum pressure between 29 bar and 32 bar |
| `pressure_actuated_valves_follow_and_latch` | Actuated valve follows the command with its lag; latched valve stays open |

Unit tests in `src/flow.rs`: `omega_ideal_gas_limit` (saturated-inlet omega
with ω = 1/1.4 chokes at η_c = 0.562), `gas_choke_ideal` (choked flux
0.6847 √(P₀ρ₀) for κ = 1.4), `churchill_limits` (laminar f = 0.64 at Re = 100;
Blasius at Re = 1×10⁵).

Case descriptions: [Analytic verification cases](../04-verification/analytic-cases.md).

## Code location

| Topic | Location |
|---|---|
| Restriction flux, omega method, gas nozzle, smoothing | `crates/fluidnet-core/src/flow.rs`: `restriction_flux`, `omega_g`, `omega_choke`, `gas_g`, `gas_flux`, `reg_factor` |
| Friction | `flow.rs`: `churchill_f`, `pipe_dp` |
| Pipe momentum | `crates/fluidnet-core/src/network/mod.rs`: `Network::rhs` (`BranchKind::Pipe`) |
| Openings, throat pressure | `network/mod.rs`: `Network::evaluate` (`BranchKind::Restriction`), `charac_eval`, `smoothstep` |
| Actuators | `network/mod.rs`: `actuator_target`, `Network::rhs` (actuators); `compile.rs`: `resolve_sense` |
| Effective area | `crates/fluidnet-core/src/catalog.rs`: `Params::cda`, `Params::area` |
| Element compilation | `crates/fluidnet-core/src/network/compile.rs`: `Network::compile`, match arms for `pipe`, for the six restriction types (`orifice`, `venturi`, `loss`, `valve`, `check_valve`, `regulator`) and for `mass_flow` |

## Open items

- The restriction flux has no unit test of the subcooled omega branch between
  the incompressible and the fully choked limits (partly flashing flow at
  moderate subcooling); the verification covers only the two limits and the
  saturated inlet.
- The choked flag of the gas–two-phase blend switches at b = 0.5 while the
  flux blends smoothly; the flag is for output only.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
