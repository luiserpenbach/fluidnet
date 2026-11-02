# Pumps, turbines and shafts

FNT-TRB-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes the turbomachinery models: the centrifugal pump (generic
affinity-law map from one design point, tabulated head and efficiency maps,
efficiency speed correction, fluid inertance, NPSH), the impulse turbine
(real-gas nozzle flow, generic and tabulated efficiency, nozzle area map) and
the shaft torque balance with gear ratios. It gives the equations as
implemented in `crates/fluidnet-core/src/machines.rs` and in the pump, turbine
and shaft parts of `network/mod.rs` and `network/compile.rs`.

## Model and equations

### Drives and gear ratio

A pump or turbine either names a `shaft` or takes a fixed `speed` (constant or
schedule). The machine speed is ω_m = r · ω_shaft with r the gear `ratio`
(machine speed over shaft speed, default 1). A machine with a fixed speed gets
an implicit, prescribed-speed shaft named `<machine>.drive`; for such a machine
the ratio is 1 and the `ratio` parameter is not used. All machine equations
below use ω_m.

### Pump: generic map

The pump is defined by one design point: speed ω_d (`design_speed`), mass flow
ṁ_d (`design_mdot`), pressure rise ΔP_d (`design_dp`), efficiency η_d
(`design_eff`, default 0.65) and inlet density ρ_d (`design_density`, by
default the density of the initial inlet state). The compiler converts these
to the design volume flow and head

```
Q_d = ṁ_d / ρ_d
H_d = ΔP_d / (ρ_d g),   g = 9.80665 m/s²
```

At run time, with the inlet density ρ, speed ratio n = ω_m/ω_d and flow ratio
q = ṁ/(ρ Q_d):

```
H   = H_d (a n|n| + (1 − a) q|q|)
P_d = ρ g H_d Q_d / η_d
τ   = (P_d/ω_d) |n| (p₀ n + (1 − p₀) q)
P   = τ ω_m = P_d (p₀ n²|n| + (1 − p₀) n|n| q)
```

a is the shutoff head ratio (`shutoff_head`, head at zero flow over design
head at the same speed, default 1.15) and p₀ the shutoff power ratio
(`shutoff_power`, default 0.45). The torque is written without dividing by ω,
so it is finite at zero speed. These curves satisfy the affinity laws
(H ∝ n², P ∝ n³ at constant q/n), give H = H_d and P = P_d at the design point,
and are defined for all signs of n and q. The efficiency follows from head and
power:

```
η = ΔP ṁ / (ρ P),   ΔP = ρ g H
```

which equals η_d at the design point and at every affinity-similar point
(η = 0 if |P| ≤ 1×10⁻⁹ P_d).

### Pump: tabulated maps

Measured maps replace the generic curves above 20 % speed. They are given in
coefficients relative to the design point, with the flow coefficient φ = q/n:

- `head_curve` [[φ, ψ], …] with ψ = (H/H_d)/n²;
- `eff_curve` [[φ, η/η_d], …].

The compiler converts the efficiency map into a power coefficient
π = (P/P_d)/n³:

```
π(φ) = φ ψ(φ) / (η/η_d)(φ)
```

at each table point with φ > 0 and η/η_d > 0.05, using `head_curve` for ψ if
given and the generic curve ψ = a + (1 − a) φ|φ| otherwise. The point
(φ = 0, π = p₀) is added, which anchors the power map at the shutoff power so
that it stays defined at zero flow where η = 0. Both curves are piecewise
linear and extrapolate linearly beyond their end points (`machines::Curve`).

At run time the tabulated values are blended with the generic ones with the
weight

```
w = smoothstep((n − 0.05)/0.15),   smoothstep(t) = t²(3 − 2t), t clamped to [0, 1]
φ = q / max(n, 0.05)
H = w H_d n² ψ(φ) + (1 − w) H_generic
τ = w (P_d/ω_d) n² π(φ) + (1 − w) τ_generic
```

so the maps apply fully at n ≥ 0.2, not at all at n ≤ 0.05 (including
reverse rotation), and starts from rest keep the generic zero-speed
behaviour.

### Pump: efficiency speed correction

`speed_eff_curve` [[N/N_d, factor], …] is an efficiency multiplier versus speed
ratio (1 at the design speed), for Reynolds-number and clearance effects at
part speed. It divides the shaft torque and leaves the head unchanged:

```
f = curve(clamp(|n|, max(n_first, 0.2), n_last)),   f ≥ 0.1
w = smoothstep((|n| − 0.05)/0.15)
τ ← τ (1 + w (1/f − 1))         only if τ · n > 0
```

The factor is held at the table edges and at its 20 % speed value below 20 %
speed, faded out between 20 % and 5 % speed, and applied only when the pump
absorbs power (not when it is driven backwards by the flow).

### Pump: hydraulics and energy

The pump is a branch with a mass-flow state
([FNT-SLV-DOC-001](network-formulation.md)):

```
I dṁ/dt = P_in − P_out + ρ_in g H(ω_m, ṁ)
```

with I the fluid inertance (`inertance`, L/A, default 2000 1/m). The shaft
power P = τ ω_m is added in full to the fluid enthalpy of the node that
receives the flow: hydraulic work and all losses heat the fluid. The torque
load on the shaft is −τ.

The available net positive suction head is

```
NPSH = (P_in − P_sat(T_in)) / (ρ_in g)
```

(NaN when T_in has no saturation pressure). It is an output only; the pump
model has no cavitation head loss.

### Turbine: nozzle flow

The turbine is a quasi-steady branch. The upstream side is the node with the
higher pressure. Its flow is the real-gas restriction flux of
[FNT-CMP-DOC-001](flow-elements.md) (`auto` model, choked or subsonic, with the
same smoothing near ΔP = 0), through the nozzle area CdA (`diameter` or
`area` of the total stator throat, `cd` default 0.95) times an optional area
factor:

```
ṁ = sign · CdA · F_A(u/c₀, PR) · G(state_up, P_down)
PR = P_up / P_down
```

### Turbine: torque and efficiency

The isentropic enthalpy drop is taken from the inlet state to the outlet
pressure along the inlet entropy, with the fluid tables:

```
Δh_s = h_up − h(P_down, s_up)     (≥ 0; 0 if the inversion fails)
c₀   = √(2 Δh_s)                  spouting velocity
u    = ω_m r_m                    blade speed, r_m = mean_diameter/2
x    = u/c₀
```

**Generic map.** The efficiency is a parabola in x with its peak η_d
(`design_eff`, default 0.6) at x_d (`design_velocity_ratio`, default 0.35),
which corresponds to a torque linear in blade speed:

```
τ = ṁ r_m (η_d/x_d) (c₀ − u/(2 x_d))
η = τ ω_m / (ṁ Δh_s) = η_d (2 x/x_d − x²/x_d²)
```

The torque is finite at stall (u = 0) and zero at runaway, x = 2 x_d.

**Tabulated map.** `eff_curve` [[u/c₀, η/η_d], …] replaces the parabola; the
point (0, 0) is added if the table has no point at x = 0. The torque is

```
τ = ½ ṁ r_m c₀ η_d · (η/η_d)(x) / x
```

which gives the same power τ ω_m = ṁ Δh_s η. For |x| < 1×10⁻⁶ the ratio
(η/η_d)/x is replaced by the slope of the first table segment, so the stall
torque is finite. τ = 0 if c₀ < 1×10⁻⁹ m/s. The curve extrapolates linearly
beyond its end points.

The power extracted, τ ω_m, is removed from the fluid enthalpy of the node that
receives the turbine flow. The reported efficiency is τ ω_m / (ṁ Δh_s), a
total-to-static efficiency on static node states.

### Turbine: nozzle area map

`area_map` [[PR, u/c₀, factor], …] gives the effective nozzle area factor F_A as
a function of velocity ratio and pressure ratio, for turbines whose rotor
blockage changes the flow capacity. For each pressure ratio in the table the
factor is a piecewise-linear curve in x = |u/c₀|, held at the end points of that
curve; between pressure ratios the two adjacent curves are interpolated
linearly, and outside the pressure-ratio range the nearest curve is used. Each
pressure ratio needs at least two velocity ratios; PR and the factor must be
positive. Without a map F_A = 1.

### Shaft

A shaft with a speed state obeys

```
J dω/dt = Σ_turbines r_i τ_i − Σ_pumps r_i τ_i − τ_b tanh(ω / 1 rad/s) − c ω
```

with J the polar moment of inertia (`inertia`, must be > 0), τ_b a
speed-independent bearing friction torque (`bearing_torque`, default 0 N m),
smoothed through zero speed by the tanh, and c a viscous loss coefficient
(`viscous_loss`, default 0 N m s). The factor r_i converts a machine torque at
machine speed to shaft torque, so that power is conserved. The initial speed is
`N0` (default 0 rpm). A shaft with `speed` given is prescribed (motor drive):
it has no state, and its inertia and losses are not used.

For the steady-state solver each free shaft has a speed guess: `N0` if
positive, otherwise the design speed of the first pump on the shaft divided by
its gear ratio. It is used for the shaft continuation of
[FNT-SLV-DOC-002](solvers.md).

## Numerical treatment

- The pump mass flow is a state; the turbine flow is algebraic. A turbine
  therefore responds instantaneously to its inlet and outlet pressures, while
  the pump flow follows its inertance.
- Pump torque and head are continuous in n and q through zero speed and zero
  flow, including reverse flow and reverse rotation. The map blending and the
  speed correction use smoothstep weights, so the right-hand side has no jumps.
- The shaft equation reads the speed state and writes the shaft row; pump and
  turbine branches add the shaft state to their reads and the shaft row to
  their writes ([FNT-SLV-DOC-001](network-formulation.md)).
- The isentropic enthalpy drop of the turbine uses `FluidTable::h_ps_near`
  with the start value h_up − v_up (P_up − P_down) (at most eight Newton
  iterations, then a bracketing search).

## Parameters and outputs

Pump: `shaft` or `speed`, `ratio`, `design_speed`, `design_mdot`, `design_dp`,
`design_eff`, `design_density`, `shutoff_head`, `shutoff_power`, `head_curve`,
`eff_curve`, `speed_eff_curve`, `inertance`. Turbine: `shaft` or `speed`,
`ratio`, `diameter` or `area`, `cd`, `mean_diameter`, `design_eff`,
`design_velocity_ratio`, `eff_curve`, `area_map`. Shaft: `inertia`, `N0`,
`bearing_torque`, `viscous_loss`, `speed`. Units and defaults are in the
[Component reference](../03-user-guide/component-reference.md).

| Element | Channel | Unit | Meaning |
|---|---|---|---|
| Pump | `.mdot`, `.dp` | kg/s, Pa | Mass flow, pressure rise P_out − P_in |
| Pump | `.head`, `.eff`, `.npsh` | m, –, m | Map head, efficiency, NPSH available |
| Pump | `.power`, `.torque`, `.speed` | W, N m, rad/s | \|P\|, \|τ\|, machine speed |
| Turbine | `.mdot`, `.dp`, `.pr` | kg/s, Pa, – | Mass flow, P_in − P_out, P_up/P_down |
| Turbine | `.eff`, `.u_c0`, `.choked` | –, –, – | Efficiency, velocity ratio, nozzle choked |
| Turbine | `.power`, `.torque`, `.speed` | W, N m, rad/s | \|P\|, \|τ\|, machine speed |
| Shaft | `.speed`, `.turbine_torque` | rad/s, N m | Shaft speed, sum of r_i τ_i of its turbines |

## Limits and assumptions

- The generic pump map is a two-parameter quadratic family; it represents the
  normal operating quadrant well and the other quadrants only qualitatively.
  Measured maps apply only above 5 % speed and fully above 20 %; there are no
  four-quadrant or low-speed (head and torque versus atan(N/Q)) maps.
- The pump has no cavitation model: the head does not fall when NPSH is
  insufficient. NPSH is reported for checking.
- The turbine is one impulse-type stage with one nozzle; multi-stage and
  reaction turbines are represented through the efficiency and area maps
  only. The rotating inertia is that of the shaft; the gas inside the turbine
  has no inertia or volume of its own.
- Turbine efficiency uses the static inlet enthalpy and the downstream static
  pressure (no total-pressure or exit kinetic-energy accounting).
- All machine losses go into the fluid; no heat is exchanged with the casing.
- Shaft losses are a constant bearing torque and a linear viscous term; seal
  and disc-friction losses must be included through these or through the
  efficiency maps.

## Verification

- Unit tests in `crates/fluidnet-core/src/machines.rs`:
  `pump_design_point` (design head and efficiency, shutoff head = 1.15 H_d,
  affinity scaling at half speed), `turbine_peak_and_runaway` (η = η_d at x_d,
  zero torque at 2 x_d), `tabulated_maps_reproduce_generic_curves` (tables
  built from the generic curves reproduce them to 1×10⁻³ in ΔP and 2×10⁻³ in
  power; turbine torque to 1×10⁻³, 2 % at stall), `speed_correction_divides_torque`
  (torque divided by the factor, head unchanged, hold below 20 % and fade to
  5 %), `turbine_area_map_bilinear_and_held`.
- `pump_matches_affinity_map_hand_calculation` in `tests/validation.rs`: water
  pump at design speed against an orifice, ṁ = 8.38489 kg/s within 2×10⁻⁴
  and η = 0.6229 within 5×10⁻⁴ of an independent Python solution of the same
  map.
- `expander_cycle_conserves_energy_and_torque`: turbine torque equals fuel pump
  torque plus geared LOX pump torque (ratio 0.3) plus bearing friction to
  1×10⁻⁶; pump power equals ṁ Δh of the fuel to 1×10⁻⁶.
- RL10A-3-3A with digitized pump and turbine maps:
  [RL10A-3-3A validation](../04-verification/rl10.md). Analytic cases:
  [Analytic verification cases](../04-verification/analytic-cases.md).

## Code location

| Topic | Location |
|---|---|
| Pump map | `crates/fluidnet-core/src/machines.rs`: `PumpMap::eval`, `Curve` |
| Turbine map, area map | `machines.rs`: `TurbineMap::eval`, `TurbineMap::area_factor`, `AreaMap` |
| Pump and turbine evaluation, NPSH | `crates/fluidnet-core/src/network/mod.rs`: `Network::evaluate` (`BranchKind::Pump`, `BranchKind::Turbine`) |
| Pump momentum, shaft equation | `network/mod.rs`: `Network::rhs` |
| Machine compilation, map conversion | `crates/fluidnet-core/src/network/compile.rs`: match arms `"pump"`, `"turbine"`, `drive_of`, `curve_param`, `area_map_param` |
| Design density, speed guesses | `compile.rs`, sections "pump design points" and "speed guesses for steady-state continuation" |

## Open items

- The pump's design density defaults to the initial inlet state. If the inlet
  is initialised warm (for example a pump primed with gas), the design head
  and design volume flow derived from `design_dp` and `design_mdot` are
  those of that state; models should set `design_density` explicitly when the
  initial inlet state differs from the design inlet state.
- `head_curve` and `eff_curve` extrapolate linearly beyond the measured flow
  coefficients, while `speed_eff_curve` and `area_map` are held at their
  edges. This difference is intended for the head (shutoff and runout trends)
  but is not documented in the parameter help.
- Low-speed pump maps in the head and torque versus atan(N/Q) form are on the
  roadmap (`README.md`, item 1).

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
