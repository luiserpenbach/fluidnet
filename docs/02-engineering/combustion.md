# Combustion chamber

FNT-CMB-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes the `combustion_chamber` component: its two states
(chamber pressure and oxidizer mass fraction), the filling and mixing
equations, the equilibrium combustion tables and their generation, the
correction for injection enthalpy, the pre-ignition gas and the ignition ramp,
the throat flow, the thrust and specific impulse, and the coupling to the
injector circuits through chamber port nodes. The use of the chamber state by
regeneratively cooled walls is described in [FNT-HTX-DOC-001](heat-transfer.md).

## Model and equations

### Coupling to the feed circuits

A chamber has two ports, `fuel` and `ox`. Each port becomes a `ChamberPort`
node ([FNT-SLV-DOC-001](network-formulation.md)) that must be connected to
exactly one flow element, normally an injector orifice, a valve or a pipe. The
port node has the chamber pressure P_c (limited to at least 1 Pa) and the
enthalpy of the node on the other side of that element, evaluated with the
port's fluid. The injector element therefore computes its flow against P_c and
carries the feed enthalpy into the chamber. The port flows are

```
ṁ_f = Σ sign · ṁ   over the branches at the fuel port      (positive into the chamber)
ṁ_o = Σ sign · ṁ   over the branches at the oxidizer port
```

The fuel and oxidizer circuits are separate fluid circuits; their two fluids
select the combustion table.

### States and equations

The chamber gas is a well-stirred volume V with pressure P_c and oxidizer mass
fraction Z (O/F = Z/(1 − Z)):

```
dP_c/dt = (RT / V) (ṁ_f + ṁ_o − ṁ_t)
m dZ/dt = max(ṁ_o, 0) (1 − Z) − Z max(ṁ_f, 0),     m = P_c V / RT  (≥ 1×10⁻⁹ kg)
```

RT is the product of the gas constant and temperature of the chamber gas, and
ṁ_t the throat flow. The pressure equation assumes that RT changes slowly
compared with the filling; the change of RT with mixture ratio and ignition
enters only through its current value. Z is limited to [1×10⁻⁶, 1 − 1×10⁻⁶]
when evaluated. The volume is `volume`, or L\* · A_t with `l_star` (default
0.9 m). The throat area A_t is `throat_area` or π D_t²/4 from
`throat_diameter`. Initial values are P_c = `P0` (default 1 bar, at least
100 Pa) and Z = 0.5.

### Equilibrium combustion tables

For each propellant pair the table gives, on a grid of O/F and P_c:

| Field | Quantity | Unit |
|---|---|---|
| `cstar` | characteristic velocity c\* | m/s |
| `Tc` | adiabatic chamber temperature | K |
| `M` | molar mass of the chamber gas | g/mol |
| `gamma_s` | isentropic exponent at the throat, ρ a²/P (shifting equilibrium) | – |
| `gamma_f` | frozen ratio of specific heats c_p/c_v in the chamber | – |
| `dcstar_dh` | ∂c\*/∂h_mix | (m/s)/(J/kg) |
| `dTc_dh` | ∂T_c/∂h_mix | K/(J/kg) |
| `dh_wall` | h_c − h_eq(600 K, P_c), equilibrium gas enthalpy drop to a 600 K wall | J/kg |
| `cp_wall` | equilibrium c_p of the gas between 600 K and 700 K | J/(kg K) |

`CombTable::eval` interpolates bilinearly in O/F and log₁₀ P_c and clamps both
to the table range. The tables are stored in
`crates/fluidnet-core/data/combustion.json`, compiled into the core, and found
by fuel and oxidizer fluid key (`combustion::find`, case-insensitive). A
missing pair is a compile error that lists the available pairs.

| Fuel / oxidizer | Fuel reference state | Oxidizer reference state | O/F range | O/F points |
|---|---|---|---|---|
| H2 / O2 | gaseous hydrogen, 250 K, 50 bar | liquid oxygen, 90 K, 20 bar | 0.5–12 | 35 |
| CH4 / O2 | methane, 200 K, 60 bar (supercritical) | liquid oxygen, 90 K, 20 bar | 1.0–6.0 | 34 |
| C2H5OH / O2 | liquid ethanol, 293.15 K, 20 bar | liquid oxygen, 90 K, 20 bar | 0.4–3.5 | 33 |

The P_c grid is 1, 2, 5, 10, 20, 35, 50, 75, 100, 150 and 200 bar for all
pairs. The O/F grids are denser around the region of peak c\* (for H2/O2:
6 points from 0.5 to 3.0, 27 from 3.5 to 10.0, then 11 and 12).

### Table generation

`tools/gen_combustion_tables.py OUT.json` uses Cantera with an ideal-gas
mixture of 23 species from `nasa_gas.yaml` (H, O, C and N species including
H2O, OH, HO2, H2O2, CO, CO2, CH4, C2H5OH and nitrogen oxides). For each pair
and grid point (`point`):

1. **Reactant enthalpy** (`reactant_h`): the NASA ideal-gas enthalpy of the
   propellant at 298.15 K plus the CoolProp enthalpy change from
   298.15 K, 1 bar to the reference state. The mixture enthalpy is
   h_mix = h_f/(1 + O/F) + h_o O/F/(1 + O/F). The reference enthalpies in the
   CoolProp convention, h_ref,f and h_ref,o, are stored in the table so that
   the solver can compute deviations in its own enthalpy convention.
2. **Chamber**: equilibrium at constant enthalpy and pressure (HP) from the
   reactant mixture (first equilibrated at 2500 K to start within the NASA
   polynomial range): T_c, M, γ_f = c_p/c_v, chamber entropy s₀ and enthalpy
   h₀.
3. **Throat**: isentropic expansion in shifting equilibrium (SP equilibrium at
   s₀) to the pressure P_t at which the flow velocity √(2(h₀ − h)) equals the
   equilibrium sound speed, found with Brent's method on
   [0.35 P_c, 0.85 P_c]. The equilibrium sound speed is from a 0.1 % isentropic
   pressure perturbation. Then c\* = P_c/(ρ_t u_t) and γ_s = ρ_t a_t²/P_t.
4. **Sensitivities**: the same calculation with h_mix + 50 kJ/kg gives
   ∂c\*/∂h and ∂T_c/∂h by forward difference.
5. **Wall potential** (`wall_potential`): the chamber gas equilibrated at
   (600 K, P_c) and (700 K, P_c) gives dh_wall = h₀ − h_eq(600 K) and
   cp_wall = (h_eq(700 K) − h_eq(600 K))/100 K. `--augment FILE` adds these two
   fields to an existing table.

The shipped file was generated with Cantera 3.2.0 and CoolProp 8.0.0
(recorded in its `meta` block).

### Injection-enthalpy correction

The propellants rarely arrive at the reference states; hydrogen from a cooling
jacket, for example, is much warmer than 250 K. The deviation of the injected
mixture enthalpy from the reference, in the fluid-table convention, is

```
Δh = (1 − Z)(h_f − h_ref,f) + Z (h_o − h_ref,o)
```

with h_f and h_o the port node enthalpies. With the table values at the
current O/F (clamped to the table range) and P_c, and the c\* efficiency η_c\*
(`cstar_eff`, default 0.98):

```
c*_hot = η_c* · max(c* + (∂c*/∂h) Δh, 0.5 c*)
T_c,hot = η_c*² · max(T_c + (∂T_c/∂h) Δh, 0.5 T_c)
RT_hot = (c*_hot Γ(γ_s))²
Γ(γ)   = √γ (2/(γ + 1))^((γ + 1)/(2(γ − 1)))          Vandenkerckhove function
```

T_c is scaled with η_c\*² because c\* ∝ √(RT). The wall potential used by
chamber-heated walls is likewise η_c\*² · max(dh_wall + Δh, 0).

### Pre-ignition gas and ignition ramp

Before ignition the chamber holds an ideal-gas mixture of the propellants at
their injection temperatures:

```
M_cold  = 1 / ((1 − Z)/M_f + Z/M_o)
T_cold  = max((1 − Z) T_f + Z T_o, 100 K)
RT_cold = R_u T_cold / M_cold,   γ_cold = 1.4
```

with M_f, M_o the molar masses from the fluid tables and T_f, T_o the port node
temperatures. With `ignition_time` t_i given, the ignition progress is

```
ι = smoothstep((t − t_i)/τ_i),   τ_i = ignition_ramp (default 20 ms, at least 0.1 ms)
```

and ι = 1 from t = 0 s if no ignition time is given. RT, γ_s, γ_f, T_c and M
are blended linearly between the cold and hot values with weight ι, and
c\* = √(RT)/Γ(γ_s). The ignition time and the end of the ramp are schedule
breakpoints for the transient solver.

### Throat flow

The throat is an isentropic nozzle with the blended γ = γ_s and the ambient
pressure P_a (`p_ambient`, schedule, default 0 Pa):

```
ṁ_t = A_t (P_c/√(RT)) · F(ΔP) · √(2γ/(γ − 1) (η_e^(2/γ) − η_e^((γ+1)/γ)))
η_e = max(P_a/P_c, (2/(γ + 1))^(γ/(γ − 1)))
```

for P_a < P_c, and ṁ_t = 0 otherwise (no reverse flow from ambient). F is the
smoothing factor near ΔP = P_c − P_a = 0 of
[FNT-CMP-DOC-001](flow-elements.md), with ε = 10 Pa + 1×10⁻⁵ P_c. When choked
this reduces to ṁ_t = P_c A_t / c\*.

### Thrust and specific impulse

When the throat is choked (P_a/P_c below the critical ratio):

```
F   = η_F · C_F(γ, ε, P_a/P_c) · A_t P_c           (≥ 0)
C_F = Γ(γ) √(2γ/(γ − 1) (1 − (P_e/P_c)^((γ−1)/γ))) + ε (P_e/P_c − P_a/P_c)
```

with ε the nozzle area ratio (`area_ratio`, default 40, at least 1), η_F the
nozzle efficiency (`cf_eff`, default 0.98) and P_e/P_c from the supersonic exit
Mach number for ε (`exit_mach`, Newton iteration on the log of the area-Mach
relation). Otherwise (unchoked) the nozzle is treated as expanding ideally to
ambient pressure:

```
F = η_F ṁ_t √(2γ/(γ − 1) RT (1 − (P_a/P_c)^((γ−1)/γ)))
```

The specific impulse is I_sp = F/(ṁ_t g) with g = 9.80665 m/s² (NaN for
ṁ_t ≤ 1×10⁻⁹ kg/s).

## Numerical treatment

- P_c is a `Pressure` state and Z a `MixFrac` state; their tolerance weights
  and perturbations are in [FNT-SLV-DOC-002](solvers.md).
- The chamber is evaluated in `Network::evaluate` after all branch flows, since
  it needs the injector flows and port enthalpies, and before the walls, which
  need its state ([FNT-SLV-DOC-001](network-formulation.md)).
- The sparsity term of a chamber reads P_c, Z and the states of both port
  nodes (P_c, Z and the neighbour states) and writes P_c and Z; the injector
  branches write the chamber rows through the port nodes.
- O/F outside the table is clamped for the table lookup only; Z itself is not
  limited by the table.
- The ignition ramp, the throat smoothing and the max(…, 0.5 c\*) guards keep
  the right-hand side continuous through ignition, chamber filling from vacuum
  and flow reversal at the injectors.

## Parameters and outputs

Parameters: `throat_diameter` or `throat_area`, `volume` or `l_star`,
`area_ratio`, `p_ambient`, `cstar_eff`, `cf_eff`, `ignition_time`,
`ignition_ramp`, `P0`. Units and defaults:
[Component reference](../03-user-guide/component-reference.md#combustion_chamber).

| Channel | Unit | Meaning |
|---|---|---|
| `<chamber>.Pc` | Pa | Chamber pressure |
| `<chamber>.OF` | – | Mixture ratio Z/(1 − Z) of the chamber gas |
| `<chamber>.thrust` | N | Thrust |
| `<chamber>.Isp` | s | Specific impulse |
| `<chamber>.cstar` | m/s | c\* of the chamber gas (blended, with η_c\*) |
| `<chamber>.Tc` | K | Chamber temperature (blended, with η_c\*²) |
| `<chamber>.mdot` | kg/s | Throat flow |
| `<chamber>.mdot_fuel`, `.mdot_ox` | kg/s | Injector flows |
| `<chamber>.ignition` | – | Ignition progress ι |

The port nodes `<chamber>.fuel` and `<chamber>.ox` produce no channels; a
valve can sense the chamber pressure by naming the chamber
([FNT-CMP-DOC-001](flow-elements.md)).

## Limits and assumptions

- Instantaneous equilibrium combustion: no atomisation, vaporisation or
  combustion delay; the c\* efficiency is a constant factor.
- Well-stirred chamber: one O/F for the whole volume, with mixing governed by
  the residence time m/ṁ.
- The filling equation neglects the time derivative of RT.
- One isentropic exponent (γ_s at the throat) for the whole nozzle expansion.
  With the default nozzle efficiency removed, the vacuum I_sp is a few
  seconds above CEA shifting equilibrium (`README.md`).
- No nozzle flow separation: an overexpanded nozzle at high ambient pressure
  gets the full pressure-thrust deficit ε (P_e − P_a) A_t, limited only by
  F ≥ 0.
- The pre-ignition gas is an ideal mixture of the propellants at their
  injection temperatures (floored at 100 K) with γ = 1.4; liquid propellant
  accumulation before ignition is not modelled.
- Linear injection-enthalpy correction; valid for moderate deviations from the
  reference states.
- Only the three tabulated propellant pairs are available.

## Verification

- `tables_load_and_match_cea` (`src/combustion.rs`): H2/O2 at O/F 6 and 50 bar,
  c\* = 2328.7 m/s within 2 m/s and T_c = 3508 K within 5 K of CEA.
- `nozzle_relations`: Γ(1.4) = 0.6847; for γ = 1.2 and ε = 10, exit Mach
  number 3.27834 and vacuum C_F 1.74231 to 1×10⁻⁴.
- `wall_enthalpy_potential_follows_mixture_ratio`: the H2/O2 wall potential
  ratio between O/F 4.67 and 5.63 at 32.6 bar is 0.959 ± 0.01.
- `chamber_pressure_and_thrust_match_tables` (`tests/validation.rs`): a
  pressure-fed chamber; P_c = 45.4245 bar to 1×10⁻⁴, thrust 11 132.5 N to
  1×10⁻³ and O/F 5.5165 to ±1×10⁻³ against an independent Python evaluation of
  P_c = ṁ c\*/A_t and the ideal C_F at ε = 40.
- `expander_cycle_conserves_energy_and_torque`: throat flow equals the sum of
  the injector flows to 1×10⁻⁹; design point P_c = 40 bar and O/F 5.8 within
  1 %.
- System level, including the start transient with ignition:
  [RL10A-3-3A validation](../04-verification/rl10.md). Analytic cases:
  [Analytic verification cases](../04-verification/analytic-cases.md).

## Code location

| Topic | Location |
|---|---|
| Chamber evaluation | `crates/fluidnet-core/src/network/mod.rs`: `Network::eval_chamber` |
| Chamber equations | `network/mod.rs`: `Network::rhs` (chambers) |
| Chamber compilation, ports, table selection | `crates/fluidnet-core/src/network/compile.rs`: sections "chambers (port nodes are linked below)", "nodes from port groups", "chambers: propellant pair and combustion table" |
| Tables, interpolation | `crates/fluidnet-core/src/combustion.rs`: `CombTable::eval`, `tables`, `find`, `pairs` |
| Nozzle relations | `combustion.rs`: `vdk`, `exit_mach`, `thrust_coefficient` |
| Throat flux | `crates/fluidnet-core/src/flow.rs`: `gas_flux` |
| Table generation | `tools/gen_combustion_tables.py`: `reactant_h`, `point`, `wall_potential`, `augment`, `main` |
| Table data | `crates/fluidnet-core/data/combustion.json` |

## Open items

- **Ethanol reference enthalpy.** `reactant_h` adds the CoolProp enthalpy
  change from 298.15 K, 1 bar to the reference state to the NASA ideal-gas
  formation enthalpy. CoolProp gives ethanol as a liquid at 298.15 K and
  1 bar, so the enthalpy of vaporisation (about 0.92 MJ/kg at 298 K) is not
  subtracted and the reactant enthalpy is that of ethanol vapour. A check with
  Cantera on 2026-11-02 at O/F 1.5 and 20 bar gives T_c = 3257 K with the
  generator's enthalpy (the table holds 3253 K at O/F 1.49) and 3193 K with the
  vaporisation enthalpy subtracted. The C2H5OH/O2 table therefore
  overestimates T_c and c\* for liquid ethanol. For oxygen and methane the
  1 bar, 298.15 K state is gaseous and the correction is consistent. Fix the
  generator and regenerate the ethanol table.
- The filling equation neglects dRT/dt; its effect on start and shutdown
  transients has not been assessed.
- There is no flow separation model for overexpanded nozzles at sea level.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
