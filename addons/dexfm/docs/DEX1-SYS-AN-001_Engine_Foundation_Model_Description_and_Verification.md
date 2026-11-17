# DEX1-SYS-AN-001: Engine Foundation Model, Description and Verification

| Field | Value |
|---|---|
| Document | DEX1-SYS-AN-001 |
| Revision | A, v0.3 (draft for review; v0.2 added the fluidnet coupling, v0.3 the h2cea adapter) |
| Date | 2026-11-10 |
| Program | DEX1 (dual expander hydrogen/oxygen engine; program code proposed, see OP-M1) |
| Toolchain | `dexfm` 0.2.1 with `fluidnet` 0.5.0, 13 gates passing, 4 h2cea comparison gates not yet run |
| Related | DEX1-SYS-DD-001 (engine concept and powerpack preliminary design) |

## 1. Purpose and scope

This note describes `dexfm`, the Python model built as the foundation for the design and simulation of the DEX1 engine. It states what each module computes, which correlations and reference data it rests on, how it was checked against flown engines, and where its validity ends. Every number in DEX1-SYS-DD-001 marked ⚙️ is produced by this toolchain from a YAML configuration and carries the configuration hash.

The model was first written standalone. In v0.2 it is coupled to `fluidnet` 0.5.0 (section 7), which now carries the network model and the start transient. In v0.3 an adapter to `h2cea` (the RocketCEA front end in the e2-propulsion-tools repository) is in place but not yet executed, so all numbers still come from the Cantera provider.

## 2. Structure

The package follows the conventions of the existing toolchain: Pydantic v2 configuration, YAML as the single source of truth, CoolProp for real-fluid properties, stateless physics functions, a SHA-256 configuration hash on every output, Plotly HTML reports and one command line entry point (`python -m dexfm run <config>`).

| Module | Function | Fidelity |
|---|---|---|
| `thermochem` | chamber and nozzle thermochemistry behind a provider interface | shifting equilibrium, one-dimensional |
| `fluids` | parahydrogen and oxygen states from pressure and temperature or pressure and enthalpy, tolerant of two-phase states | reference equations of state |
| `chamber` | contour, hot-gas heat transfer, cooling-channel layout and coolant march, cooling zones per propellant | L1 (one-dimensional) |
| `turbo` | pump and turbine sizing, speed selection inside limits, off-design maps | L1 (correlations) |
| `cycle` | power balance of one powerpack, maximum closing chamber pressure | L1 |
| `offdesign` | throttle solution with frozen hardware, detailed cooling in the loop | L1 |
| `transient` | lumped start model | L0, experimental |
| `architecture`, `studies` | chamber and powerpack count trade, sensitivity, mass and envelope | L0 to L1 |
| `fluidnet_model` | generates, calibrates and runs the fluidnet model of one powerpack; start sequences | network transient (fluidnet) |
| `adapters` | neutral netlist export | n/a |

Fidelity levels are used as follows. L0 is a lumped or scaling model that is useful for direction only. L1 is a one-dimensional or correlation-based model that is adequate for concept selection and preliminary sizing. L2 (not present yet) means component-resolved analysis such as conjugate heat transfer, mean-line blading with loss models, or a fluid network transient.

## 3. Physics

### 3.1 Thermochemistry

The built-in provider uses Cantera with the hydrogen/oxygen mechanism species set. The chamber state is the constant-enthalpy, constant-pressure equilibrium of liquid hydrogen at 20.3 K and liquid oxygen at 90.2 K. Expansion is isentropic with shifting equilibrium; the throat is located by maximising mass flux, and the exit by matching the area ratio. Delivered performance applies two efficiencies to the ideal values: a characteristic velocity efficiency (default 0.985) and a thrust coefficient efficiency (default 0.975). ⚠️ ASSUMPTION: both efficiencies are placeholders until the JANNAF-style delivered-performance stack replaces them.

### 3.2 Thrust chamber heat transfer

The contour is a cylinder sized from the characteristic length (chamber volume divided by throat area), a cosine-blended convergent and a parabolic bell of 80 % of the 15 degree cone length. The hot-gas-side heat transfer coefficient is the Bartz correlation with the sigma property correction, evaluated with local area ratio and Mach number, and multiplied by a scale factor `c_bartz`. The adiabatic wall temperature uses a recovery factor equal to the cube root of the Prandtl number.

Cooling channels are rectangular. The channel count is set at the throat from a 1.0 mm channel and a 0.8 mm rib and doubles wherever the channel width would exceed 2.5 mm. At each station the model solves the series resistance of hot-gas film, wall conduction and coolant film including rib fin efficiency. In design mode the channel height at each station is the largest value between 0.8 mm and 6 mm that keeps the hot-gas-side wall temperature under the limit (750 K default for a copper alloy liner). In off-design mode the channel heights are frozen.

Coolant-side Nusselt numbers follow the form Nu = C Re^0.8 Pr^0.4 (T_wall/T_bulk)^n with C = 0.025 and n = -0.55 for hydrogen (McCarthy and Wolf form) and C = 0.023 and n = -0.30 for oxygen. ⚠️ ASSUMPTION: the oxygen correlation is a placeholder; supercritical and boiling oxygen need a dedicated correlation and test data (OP-M3). Friction uses the Haaland equation with 6 micrometre roughness, plus momentum pressure change and manifold losses.

A chamber can be divided into zones cooled by different propellants. Three layouts exist: hydrogen only, oxygen on the injector end of the cylinder ("barrel" zone), and oxygen on the nozzle downstream of a chosen area ratio.

### 3.3 Turbopumps

Pump efficiency is the best-efficiency envelope for radial pumps by Guelich, a function of the stage specific speed nq and of the volume flow, multiplied by a derate of 0.92 for rocket-pump clearances and surface finish. Because it depends on volume flow it carries the size effect that distinguishes one large pump from several small ones. The speed and stage count are selected for the highest efficiency inside four limits: impeller tip speed (600 m/s for hydrogen, 260 m/s for oxygen), suction specific speed (60,000 and 30,000 in US units), bearing DN (bore in millimetres times speed in revolutions per minute: 2.7 million for hydrogen, 1.8 million for oxygen, with a 25 mm minimum bore) and a minimum specific speed. A stage is added only if it buys three efficiency points.

Turbine efficiency is a parabola in the velocity ratio (mean blade speed over isentropic spouting velocity) with a reference peak of 0.82 at a velocity ratio of 0.55, reduced for tip clearance and partial admission. Blade speed is limited to 480 m/s for hydrogen and 200 m/s for oxygen. Real-gas expansion work comes from CoolProp.

For off-design work the pump is a homologous polynomial map that remains valid from locked rotor to overspeed, and the turbine is a fixed flow area with the same efficiency parabola.

### 3.4 Cycle power balance

One powerpack is one hydrogen turbopump and one oxygen turbopump feeding a number of chambers in parallel. On the fuel side all hydrogen passes through the hydrogen cooling zones, then through the fuel turbine with a bypass held as control margin, then to the injector. On the oxygen side a fraction of the pump flow is heated in the oxygen zones and drives the oxygen turbine; the rest goes to the injector as liquid through the oxidizer control valve. For each side the model finds the lowest pump discharge pressure at which turbine shaft power covers pump power at the specified bypass. A configuration "closes" if both sides do so without exceeding the wall temperature limit or the hot oxygen limit (450 K). The maximum closing chamber pressure is found by a scan and bisection.

### 3.5 Off-design and start

The throttle solver freezes all hardware from a design point and solves eight unknowns (two shaft speeds, three flows, two turbine bypass valve areas and the oxidizer control valve area) for a commanded chamber pressure and mixture ratio, with the detailed cooling model re-evaluated in an outer loop. An optional back-pressure valve downstream of the oxygen turbine holds the oxygen cooling circuit above a pressure floor.

The start model integrates shaft speeds and one liner temperature per cooling zone with quasi-steady flows. It is experimental (section 6).

## 4. Calibration and verification

| Check | Model | Reference | Status |
|---|---|---|---|
| RL10A-3-3A ideal vacuum specific impulse at mixture ratio 5.0, 32.75 bar, area ratio 61 | 461.7 s | delivered 444.4 s (96.3 % of ideal) | consistent |
| RL10A-3-3A delivered specific impulse with efficiencies 0.99 and 0.972 | 444.3 s | 444.4 s | calibrated |
| RL10A-3-3A propellant flow | 16.85 kg/s | 16.84 kg/s from 73.4 kN and 444.4 s; published approximately 6 lb/s hydrogen plus 30 lb/s oxygen (16.3 kg/s) | consistent |
| RL10A-3-3A jacket heat pickup | 7.81 MW | 8.37 MW (2.79 kg/s from 26.5 K, 75.8 bar to 213 K, 56.1 bar) | 7 % low |
| RL10A-3-3A jacket outlet temperature | 200 K | 213 K | 13 K low |
| RL10A-3-3A jacket pressure drop | 19.4 bar | 19.7 bar | within 2 % |
| RL10A-3-3A fuel pump efficiency at 32,000 rpm, two stages | 0.604 | 0.585 (first stage) | within 2 points |
| RL10A-3-3A turbine efficiency | 0.740 | about 0.74 | consistent |
| RL10A-3-3A power balance at published turbine state | turbine 660 kW, pumps 565 kW | engine balances with gearbox losses | consistent |
| Vinci-class fuel pump power (5.8 kg/s to 224 bar at 90,000 rpm) | 2.51 MW | about 2.5 MW quoted | consistent |
| DEX1-A design point rebuilt in fluidnet: turbine efficiency needed for torque balance | 0.746 fuel, 0.796 oxygen | 0.762 and 0.793 from dexfm correlations | within 2 points |
| DEX1-A in fluidnet with free shafts | 49.95 bar, mixture ratio 6.00 | 50.0 bar, 6.0 | consistent |

The Bartz scale factor of 0.80 is the single calibration of the heat transfer model. It was set so that the total RL10A-3-3A jacket heat pickup is reproduced with the tube bundle idealised as 3 mm channels in a 0.33 mm stainless wall. ⚠️ ASSUMPTION: one engine at one operating point is a thin basis. Absolute heat loads should be read as plus or minus 20 %, and the distribution along the chamber is less certain than the total: the model predicts an RL10 peak flux of 24 MW/m², which is higher than the tube wall could carry, so Bartz is over-predicting at the throat and under-predicting elsewhere.

The gates in `tests/test_gates.py` encode the checks above together with energy bookkeeping, reproduction of the design point by the off-design solver, throttling to 50 % and independence of results from call history.

## 5. Numerical notes

One defect found during verification is recorded here because it silently changed results. CoolProp state objects can keep a stale phase guess after a failed flash near the oxygen saturation line; later enthalpy-pressure flashes then fail for valid inputs, which made cycle closure depend on what had been solved earlier in the same process. The property wrapper now rebuilds the state object after any failed flash, a regression gate covers it, and all studies were rerun afterwards.

## 6. Limits of validity

The heat transfer model is one-dimensional with a single calibration point. Oxygen cooling below the critical pressure (50.4 bar) is computed with a homogeneous two-phase mixture model and is not credible beyond indicating that the regime is entered. Pump and turbine performance come from correlations and not from designed blading; rotordynamics, axial thrust balance, seals and bearings are not modelled beyond a DN limit. The throttle solver assumes constant combustion and nozzle efficiencies. The lumped start model in `transient.py` has no line inertance, no manifold filling and no choking logic; it is superseded by the fluidnet transient and kept only as a quick sandbox. Combustion stability, injector design, structures and life are outside the model.

## 7. Seams for h2cea and fluidnet

`thermochem.ThermochemProvider` is a protocol with one method, `solve(of, pc, eps)`, returning chamber temperature, characteristic velocity, isentropic exponent, molar mass, specific heat, viscosity, Prandtl number, ideal vacuum specific impulse, thrust coefficient and exit pressure. `H2CeaAdapter` calls `h2cea.cea_si.point(pc, mr, eps, pamb, ox, fuel)` with pressure in bar, zero ambient pressure and the RocketCEA built-in LOX and LH2 cards, and maps the result (chamber temperature, characteristic velocity, equilibrium isentropic exponent, molar mass, frozen specific heat, viscosity, frozen Prandtl number, shifting-equilibrium vacuum specific impulse, thrust coefficient, exit pressure). The package is found through the environment variable `H2CEA_PATH`; the provider is selected with `DEXFM_THERMOCHEM` (default `cantera`) and its name is written into every summary file. ⚠️ The adapter has not been run against RocketCEA: the environment in which it was written has no Fortran compiler. `tests/test_h2cea.py` holds the agreement gates (characteristic velocity and specific impulse within 0.5 %, chamber temperature within 1 %, transport properties within 5 to 15 %) and `tools/compare_thermochem.py` prints both providers side by side on the baseline. The Bartz scale factor was calibrated with the Cantera provider and has to be re-checked after switching.

`fluidnet_model.build_model` turns a dexfm design point into a fluidnet model file for one powerpack: two shafts, pumps with the dexfm design points, two cooled lines heated by one equivalent chamber (throat area, channel counts and hot-gas areas multiplied by the number of chambers per pack, local area ratio along each line from the dexfm contour), turbines, both bypass valves, the oxidizer control valve, discharge lines and manifolds. `calibrate` fits thirteen parameters (injector, turbine nozzle, bypass and control valve areas, two hot-gas heat transfer factors, two channel hydraulic diameters, two turbine peak efficiencies) so that the fluidnet steady state with prescribed shaft speeds matches thirteen dexfm design values; it converges in about two seconds to a residual of 2e-5. `start_model` adds valve schedules, pressure-actuated valves and ignition; `DEFAULT_START` is the sequence documented in DEX1-SYS-DD-001 section 8. `python -m dexfm run` writes both model files next to the report when fluidnet is installed.

Three observations on fluidnet 0.5.0 from this work. Above a nozzle area ratio of about 100 the exit Mach number solve fails and thrust jumps to the infinite-expansion limit (the 20 kN example gives 468 s at area ratio 100 and 592 s at 120); the generated model therefore caps the area ratio at 100, which does not affect the cycle. A valve that sits at its leak area and then opens against trapped liquid stalls the integrator; the start sequences keep valves cracked at 0.3 % to 2 % from t = 0. Cooled lines are round tubes, so the network model matches heat pickup and pressure drop of a zone but not its wall temperature.

## 8. Open points for the model

| ID | Open point | Closure |
|---|---|---|
| OP-M1 | Program code `DEX1` and package name `dexfm` are proposals | Luis to confirm or rename |
| OP-M2 | Run the h2cea agreement gates and the comparison script on a machine with RocketCEA; then decide the default provider and re-check the Bartz calibration | Luis, laptop |
| OP-M8 | Report the three fluidnet observations of section 7 to the fluidnet open-items list | Luis |
| OP-M3 | Oxygen coolant correlation for supercritical and boiling regimes | literature library entry plus heated-tube test data |
| OP-M4 | Second calibration point for hot-gas heat transfer (a copper-liner calorimeter chamber, for example published subscale hydrogen/oxygen data) | before any liner drawing |
| OP-M5 | Replace the efficiency placeholders by the delivered-performance stack | when that stack exists |
| OP-M6 | Mean-line pump and turbine design with loss models; link to CFTurbo geometry | powerpack preliminary design review |
| OP-M7 | Couple liner results to `channel_fem` for strain range and life | after OP-M4 |

## 9. References

1. Binder, M., Tomsik, T., Veres, J. P., "RL10A-3-3A Rocket Engine Modeling Project", NASA TM-107318, 1997. https://ntrs.nasa.gov/citations/19970010379
2. Binder, M., "A Transient Model of the RL10A-3-3A Rocket Engine", NASA CR-195478, 1995. https://ntrs.nasa.gov/api/citations/19950022693/downloads/19950022693.pdf
3. Leto, A. et al., pump global design tool with RL10A-3-3A engine data table, Energy Procedia 101 (2016) 614 to 621.
4. Glassman, A. J., Jones, S. M., "Computer Code for Single-Point Thermodynamic Analysis of Hydrogen/Oxygen Expander-Cycle Rocket Engines", NASA TM-4275, 1991. https://ntrs.nasa.gov/api/citations/19910010893/downloads/19910010893.pdf
5. Bartz, D. R., "A Simple Equation for Rapid Estimation of Rocket Nozzle Convective Heat Transfer Coefficients", Jet Propulsion, 1957.
6. Guelich, J. F., "Centrifugal Pumps", Springer.
7. Bell, I. H. et al., CoolProp; Goodwin, D. G. et al., Cantera.
