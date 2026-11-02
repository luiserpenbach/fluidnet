# Verification against analytic and reference solutions

FNT-VAL-ANA-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Question

Do the implemented equations, property tables and solvers reproduce known
solutions to the stated accuracy? Each case isolates one physical model or
conservation property, so a failure points at one part of the code.

## Model and inputs

All cases are model files in `tests_models/` or `examples/`. References are
analytic or computed with CoolProp 8.0. Each case is an automated test in
`crates/fluidnet-core/tests/validation.rs` (system level) or a `#[test]`
function in the source module (component level); `cargo test --release` runs
all of them.

## Results: system-level cases

| Case and model file | Reference | fluidnet result | Test and tolerance |
|---|---|---|---|
| N2 tank blowdown, 10 bar → 1 bar through a 3 mm orifice, choked (`tests_models/blowdown_n2.yaml`) | Isentropic ideal-gas blowdown | Tank pressure within 0.1 % over 6 s; the remainder is real-gas behaviour | `n2_blowdown_matches_isentropic_ideal_gas`, 0.2 % |
| Water orifice, 10 → 5 bar (`water_orifice.yaml`) | 0.7 A √(2ρΔP) = 0.434338 kg/s | 0.43434 kg/s | `liquid_orifice_matches_bernoulli`, 0.02 % |
| Water pipe, 10 m × 20 mm, 2 bar (`water_pipe.yaml`) | Darcy–Colebrook, 1.9452 kg/s | 1.9382 kg/s (−0.36 %, Churchill against Colebrook) | `pipe_friction_matches_darcy_colebrook`, 1 % |
| Cavitating venturi, water 30 bar (`cav_venturi.yaml`) | 0.95 A √(2ρ(P₀ − P_v)) = 0.230795 kg/s | 0.23080 kg/s, choked flag set | `cavitating_venturi_chokes_at_vapour_pressure`, 0.1 % |
| Saturated LN2 at 5 bar through a 2 mm orifice to 1 bar (`ln2_flash.yaml`) | HEM by isentropic integration, 0.02037 kg/s | 0.02051 kg/s (+0.7 %), choked | `saturated_ln2_flashing_matches_hem`, 3 % |
| Heated water line, 100 kW/m², 0.1 kg/s (`heated_pipe.yaml`) | Δh = Q/ṁ = 62.83 kJ/kg | 62.73 kJ/kg (−0.16 %) | `heated_pipe_closes_energy_balance`, 0.5 % (heat input 10⁻⁹) |
| Water hammer, 100 m × 50 mm, 1.8 m/s, valve closed in 2 ms (`examples/water_hammer.yaml`) | Joukowsky plateau 66.24 bar; period 4L/a = 0.2687 s (a = 1488.8 m/s at 40 bar, 293.15 K) | 66.6 bar; 0.2690–0.2695 s | `water_hammer_matches_joukowsky`, plateau 2 %, period 1 % |
| Small water hammer exceeding the static pressure | Column separation at P_v(20 °C) = 0.023 bar | Valve end settles at 0.0255 bar | manual check |
| GN2 regulator, set 30 bar, band 2 bar (`regulator.yaml`) | Outlet within the band | 30.68 bar | `regulator_holds_set_pressure_within_band`, 29–32 bar |
| Water pump at design speed against an orifice (`pump_design.yaml`) | Same affinity map solved in Python: 8.38489 kg/s, η 0.6229 | 8.3857 kg/s, η 0.6228 | `pump_matches_affinity_map_hand_calculation`, 0.02 % and 5·10⁻⁴ |
| Pressure-fed GH2/LOX chamber, 40 mm throat, ε 40 (`chamber_pressure_fed.yaml`) | Pc = ṁ c*/A_t with table c* and enthalpy correction: 45.4245 bar; ideal CF 1.9503, F = 11,132.5 N | 45.4245 bar; 11,132.5 N; O/F 5.5165 | `chamber_pressure_and_thrust_match_tables`, 0.01 % and 0.1 % |
| Pressure-actuated valves with and without latch (`actuated_valve.yaml`) | Opening follows the sensed-pressure command with first-order lag; latched valve stays open | Openings at 0.2, 0.5, 1.0, 2.0 s as commanded | `pressure_actuated_valves_follow_and_latch`, 0.03 |
| Expander cycle steady state (`examples/expander_cycle.yaml`) | Shaft torque balance; jacket ṁΔh = Q; pump ṁΔh = P; throat flow = injected flow | All to 10⁻⁶ (mass 10⁻⁹); Pc 40 bar and O/F 5.8 within 1 % | `expander_cycle_conserves_energy_and_torque` |
| Expander start transient (`examples/expander_start.yaml`) | Steady-state solver at the same inputs | Pc, thrust, speed and O/F within 0.3 % | `expander_start_reaches_the_steady_design_point`, 0.3 % |
| All example models | Run without error, transient and steady (except `regulated_pressurization.yaml`, which is closed at t = 0 and has no unique steady state) | pass | `all_examples_run_steady_and_transient` |

## Results: component-level cases

| Case | Reference | Result | Test |
|---|---|---|---|
| Omega method, ideal-gas limit | Leung's critical pressure ratio η_c ≈ 0.562 for ω = 1/1.4 | Within 0.003 | `flow.rs::omega_ideal_gas_limit` |
| Ideal-gas choking | γ 1.4 at pressure ratio 0.1: choked, flux function 0.6847 | Within 10⁻³ | `flow.rs::gas_choke_ideal` |
| Churchill friction factor | Laminar 64/Re = 0.64 at Re 100; Blasius 0.018 at Re 10⁵, smooth | Within 0.01 and 0.001 | `flow.rs::churchill_limits` |
| Pump design point | Design head and efficiency; shutoff head 1.15 H_d; half speed and half flow give a quarter of the head at the same efficiency | Within 10⁻⁹ | `machines.rs::pump_design_point` |
| Turbine peak and runaway | η = η_d at u/c0 = x_d; zero torque at u/c0 = 2 x_d | Within 10⁻⁹ | `machines.rs::turbine_peak_and_runaway` |
| Copper heat capacity | 385 J/kg/K at 300 K; tabulated about 98–100 J/kg/K at 50 K and 254 J/kg/K at 100 K | Debye model 82 and 243 J/kg/K (−16 %, −4 %); test tolerance 20 J/kg/K | `machines.rs::copper_heat_capacity` |
| Tabulated maps from generic curves | Tables sampled from the generic pump and turbine curves | Pump Δp within 0.1 %, power within 0.2 %; turbine torque within 0.1 % (2 % at stall, piecewise-linear first segment) | `machines.rs::tabulated_maps_reproduce_generic_curves` |
| Pump efficiency speed correction | Torque divided by the factor; held at the table edges | As specified | `machines.rs::speed_correction_divides_torque` |
| Turbine area map | Bilinear interpolation, held at the edges | As specified | `machines.rs::turbine_area_map_bilinear_and_held` |
| Combustion tables against CEA | LOX/GH2, 50 bar, O/F 6: CEA about 2330 m/s and 3490–3510 K | c* 2328.7 m/s, T_c 3508 K (test tolerance 2 m/s, 5 K); CH4 and ethanol tables present | `combustion.rs::tables_load_and_match_cea` |
| Nozzle relations | Vandenkerckhove function 0.6847 at γ 1.4; γ 1.2, ε 10: exit Mach 3.2783, vacuum CF 1.7423 (independent root solve) | Within 10⁻⁴ | `combustion.rs::nozzle_relations` |
| Wall enthalpy potential | LOX/LH2 at 32.6 bar: equilibrium wall potential falls 4 % from O/F 5.63 to 4.67 while c_p,frozen (T_c − T_w) rises | Ratio 0.959 ± 0.01 | `combustion.rs::wall_enthalpy_potential_follows_mixture_ratio` |
| Unit parsing | Unit strings to SI | Pass | `units.rs::units` |
| Dense LU | Known solution | Pass | `linalg.rs::solves` |

### Restriction mass flux, two-point omega against full HEM

Mass flux of the omega method used in `flow.rs` against homogeneous
equilibrium integration with CoolProp:

| Fluid, inlet | G_omega / G_HEM |
|---|---|
| N2, saturated, 5 bar | 1.008 |
| N2, saturated, 10 bar | 1.008 |
| O2, saturated, 5 bar | 1.007 |
| N2O, saturated, 50 bar | 1.008 |
| N2, 3 K subcooled, 5 bar | 0.994 |
| N2O, 5 K subcooled, 50 bar | 0.979 |

### Property tables against CoolProp

Random states, 99th percentile of the relative error: density 2–4·10⁻⁴ in
single phase, about 1·10⁻³ on the saturation lines, about 2·10⁻⁴ in the
two-phase region; temperature below 0.1 K
([FNT-FLD-DOC-001](../02-engineering/fluid-properties.md)).

## Discussion

- The Churchill friction factor differs from Colebrook by up to about 0.5 %
  in the transition to fully rough flow; the −0.36 % flow difference in the
  pipe case is that difference.
- The +0.7 % on the flashing LN2 orifice and the 0.98–1.01 ratios of the
  omega table are the accuracy of the two-point omega method against full HEM;
  real flashing flows deviate from HEM by more than this (thermal
  non-equilibrium), which fluidnet does not model.
- The single-parameter Debye model underestimates the heat capacity of metals
  at low temperature (−16 % for copper at 50 K), which makes chill-down
  slightly faster than in reality.
- The combustion tables give vacuum Isp a few seconds above CEA shifting
  equilibrium before the default 0.98 nozzle efficiency, because a single
  isentropic exponent is used for the nozzle expansion.

## Conclusions

All analytic and conservation checks pass at the stated tolerances. Coverage
gaps (models with no verification case yet) are listed in the
[validation matrix](validation-matrix.md).

## Run record

| Item | Value |
|---|---|
| Software | fluidnet 0.5.0, commit 36580d1 |
| Command | `cargo test --release` (14 unit tests, 16 validation tests) |
| References | CoolProp 8.0.0, Cantera 3.2.0 (combustion tables), NASA CEA (manual comparison) |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release (content of docs/VALIDATION.md, extended with the unit tests) |
