# Platform development plan

FNT-PLT-PLN-001 · Rev B · Draft · 2026-11-02 · Owner: Luis Erpenbach

## 1. Purpose

This plan describes how fluidnet develops from a validated solver (v0.5.0)
into an engineering platform for complex fluid systems and liquid rocket
engines. It sets the target by system complexity rather than by one
application: the platform is accepted when it reproduces the most complex
public engine data available, a staged-combustion engine with four
turbopumps, two preburners and closed-loop control, steady and through the
start transient. It defines the reference problems that measure progress,
the requirements that follow from them, the target architecture, the work
packages with exit criteria, and the schedule.

Decisions that frame the plan ([decision log](decisions.md)):

- D-019: private GitHub repository under the owner's account.
- D-020: fluidnet is the owner's private project.
- D-022: no single target application; reference problems of increasing
  system complexity drive the development. Application models (a pressure-fed
  lander propulsion system, a dual-expander engine) are demonstrations.
- D-023: validation uses public literature data only. The data sets are
  catalogued in [FNT-VAL-REG-002](../04-verification/data-engines.md)
  (engines and turbomachinery) and
  [FNT-VAL-REG-003](../04-verification/data-fluid-systems.md) (fluid systems
  and thermal phenomena).

Status Draft: open questions are in section 11.

## 2. Starting point (v0.5.0)

| Area | State |
|---|---|
| Solver core | Rust; (P, h) node states; TR-BDF2 transient with colored finite-difference Jacobian and sparse LU; steady solver with flow guess, shaft continuation, Newton, pseudo-transient continuation and transient fallback; 5–50 ms for typical feed systems, 0.25 s for an expander steady state |
| Physics | Liquid, gas, supercritical and two-phase (homogeneous equilibrium) flow; cavitation and flashing; pipes with inertia and friction; scheduled and pressure-actuated valves, check valves, regulators; rigid tanks; centrifugal pumps and impulse turbines with tabulated maps; shafts; walls with heat capacity, Bartz and Gnielinski heat transfer; one equilibrium combustion chamber per circuit pair (H2, CH4, ethanol with O2) |
| Fluids | Eight CoolProp-tabulated pure fluids |
| Front ends | CLI, Python package, browser schematic editor |
| Evidence | 30 automated tests; RL10A-3-3A steady state within 0.62 % mean / 2.05 % max at four test points ([FNT-VAL-ANA-002](../04-verification/rl10.md)) |

What limits system complexity today:

| Limit | Consequence |
|---|---|
| One pure fluid per connected circuit; combustion products exist only inside the chamber | No gas generators or preburners, no turbines driven by combustion gas, no pressurant over propellant, no gas in liquid lines. This excludes every gas-generator and staged-combustion engine. |
| A model is one flat file without parameters | A four-turbopump engine is several hundred components with repeated blocks that cannot be reused or reviewed |
| Turbomachinery: impulse turbines only, no hydraulic turbines, no low-speed or four-quadrant pump maps, no pump inlet two-phase degradation | SSME low-pressure pumps and turbopump starts cannot be represented |
| No heat exchanger, no boiling heat transfer, no solid thermal networks | No RL10 derivatives with oxidiser heat exchangers, no chill-down |
| No controllers, sequences or linear analysis | No closed-loop Pc/MR control, no stability analysis |
| No study definitions, run records or literature-data workflow | Calibrations and comparisons are custom scripts; results are not traceable |
| No CI, Linux-only wheels | Regressions are found by hand; no Windows install |

## 3. Reference problems

Progress is measured by a ladder of reference problems. Each level needs the
capabilities of the levels below it. Data sources are in FNT-VAL-REG-002 and
FNT-VAL-REG-003; accuracy targets are first estimates and are revised when a
problem is set up and the data scatter is known.

### Level 1: fluid and thermal phenomena

| ID | Problem | Data | Exercises | Target |
|---|---|---|---|---|
| R1.1 | Cryogenic fluid hammer with cavitation, LN2, 9.29 m × 19 mm line, valve closure 18 ms | DLR PS-LN2-Set1 (raw data, CC BY 4.0) | Pipe acoustics, column separation, cryogenic properties, wall elasticity | First pressure peak within 5 %, wave period within 2 %, cavity collapse timing within 10 % |
| R1.2 | Water hammer and column separation, 37.23 m copper pipe | Bergant & Simpson 1999 | Same, classic benchmark with measured wave speed | As R1.1 |
| R1.3 | Choked flow of subcooled LN2 and LOX through two specified nozzles | Hendricks, Simoneau & Barrows 1976, NASA TN D-8169 (tables) | Restriction flux, HEM against non-equilibrium (Dyer) | Mass flux within 5 % over the subcooling range with the non-equilibrium option |
| R1.4 | Chill-down of a 61 m copper transfer line with LN2 and LH2; single-tube LN2 chill-down | Brennan et al. 1966, NBS 9264; Darr et al. 2016 | Wall heat capacity, boiling heat transfer, two-phase line flow | Chill-down time within 15 %; wall temperature histories within 10 K |
| R1.5 | GH2 pressurisation and expulsion of LH2 tanks; He pressurant | Stochl et al. 1969, TN D-5387; Van Dresar & Stochl 1993, TM-106427; TN D-7019 | Two-fluid tank, interface and wall heat transfer | Pressurant mass within 10 % |
| R1.6 | LH2 tank self-pressurisation at three heat fluxes | Hasan, Lin & Van Dresar 1991, TM-103804 | Homogeneous against stratified tank (documents the limit) | Lowest flux within 10 %; limit quantified at the highest |
| R1.7 | Vessel blowdown, N2 and CO2 from 150 bar, wall temperatures | Imperial College (Haque et al., Richardson & Saville 1992) | Tank, wall heat transfer, choked gas and two-phase flow | Pressure within 5 %, gas temperature within 10 K |
| R1.8 | Self-pressurising N2O tank drain; N2O injector flow | Zimmerman 2015 thesis; Palacz & Cieślik 2021 | Two-node non-equilibrium tank, Dyer flow | Tank pressure history within 10 % |
| R1.9 | Priming of evacuated lines with water, ethanol and LN2 | DLR Lampoldshausen priming bench; VKI facility (Gouriet et al. 2016) | Non-condensable gas in liquid lines | Peak pressure within 15 % |

### Level 2: components

| ID | Problem | Data | Exercises |
|---|---|---|---|
| R2.1 | Mark 48-F LH2 turbopump (three-stage pump, two-stage reaction turbine) and Mark 48-O LOX turbopump | NASA CR-159821, CR-159509 | Multi-stage pump maps, stall region, zero-NPSH idle, reaction-turbine maps |
| R2.2 | J-2 hydrogen pump at zero tank NPSH, start transients | NASA TN D-6824 | Pump head degradation with inlet vapour fraction |
| R2.3 | RL10 oxidiser heat exchanger, GN2/LN2 component tests | NASA CR-179487 | Two-stream heat exchanger with boiling |

### Level 3: engines up to gas-generator complexity

| ID | Problem | Data | Exercises | Target |
|---|---|---|---|---|
| R3.1 | RL10A-3-3A steady and start | done (FNT-VAL-ANA-002) | Regression | Keep 0.62 % mean; start inside the measured 1.43–2.56 s once low-speed maps are in |
| R3.2 | RL10-IIB tank-head idle and pumped idle with the oxidiser heat exchanger; transition THI → PI | NASA CR-174914, Tables E-2/E-3 and transient plots | Heat exchanger, two-phase injection, low-pressure two-phase hydrogen in the jacket, mode transitions | Steady points within 3 %; transition timing within 10 % |
| R3.3 | STME gas-generator booster module: eight chambers, four turbopumps, H2 spin start, start, cutoff and chamber-out cases | OEPSS Data Book Vol. 8, NASA CR-207443 (fully specified inputs; results from Rocketdyne's transient model) | Gas generator, combustion-gas turbine drive, spin start, manifolded multi-chamber network, subsystems | Code-to-code: within 5 % of the published transients |
| R3.4 | Bootstrap start of an LH2 turbopump system heated by large solid masses | NASA TM X-1917 | Solid thermal networks, stall region, start without spin gas | Peak flow and speed within 10 % |

### Level 4: staged combustion

| ID | Problem | Data | Exercises | Target |
|---|---|---|---|---|
| R4.1 | SSME steady balance from 17 % to 109 % rated power level | Bradley 1997, NASA CR-205360 (tables at 17–40 % RPL, pump operating points); Santi 1994 (TTB-50 at 100 %); Santi 1993, CR-195253 (power balance model at 109 %) | Two fuel-rich preburners, four shafts, two pumps on one shaft, hydraulic turbine, reaction turbines, Pc/MR control | Calibrated at 100–109 %, predicts 17–40 % RPL within the scatter of Rocketdyne's own predictions in CR-205360 |
| R4.2 | SSME start transient to 80 % RPL with nominal and reduced LOX inlet pressure | Bradley 1997, NASA CR-205359 (measured traces against Rocketdyne's transient model) | Preburner and chamber priming and ignition, closed-loop valve control, tank-head start | Prime times within 0.1 s; MCC Pc, HPOTP discharge pressure and HPFTP speed within 10 % after 2.4 s |

Level 4 is the acceptance level of release 1.0. LE-7/LE-7A start-transient
papers (JAXA) are the second staged-combustion case once the full texts are
obtained.

### Level 5: demonstration models (no test data)

Models that show complexity beyond the public data, built from published
design data and the validated components: a stage with autogenous
pressurisation, multi-engine feed and start/shutdown; a full-flow
staged-combustion engine at the published IPD design point; a dual-expander
hydrolox engine; a pressure-fed lander propulsion system with N2O
self-pressurisation. They are verified (balances, sensitivities, code
behaviour) but not validated, and are labelled so.

## 4. Requirements

### Modelling

- **PLT-R-01** Subsystems: model files with declared external ports and
  parameters used as components, nested to any depth (a turbopump subsystem
  used four times in R3.3).
- **PLT-R-02** Named parameters with units; parameter and case files.
- **PLT-R-03** Component templates from library files.
- **PLT-R-04** Optional tag and description fields on components; tag-list
  export.
- **PLT-R-05** Invalid model files rejected with file, component, key and
  nearest valid key named.

### Physics

- **PLT-R-10** Multi-component flow: each node carries a composition (mass
  fractions of a small set of declared components); mixture properties of
  non-reacting components by ideal mixing of real-fluid tables; transport of
  composition through branches.
- **PLT-R-11** Reacting flow: combustion devices with gas outlets (gas
  generators, preburners, main chambers feeding downstream lines);
  equilibrium product properties tabulated in pressure, enthalpy and mixture
  fraction, including fuel-rich and oxidiser-rich regimes; turbines and lines
  carrying combustion gas.
- **PLT-R-12** Combustion tables for any propellant pair Cantera supports,
  from a configuration file, each compared with CEA.
- **PLT-R-13** Non-equilibrium two-phase flow through restrictions (Dyer).
- **PLT-R-14** Pipe wall elasticity in the wave speed.
- **PLT-R-15** Two-fluid tanks: liquid, ullage (pressurant and vapour) and
  wall nodes with interface and wall heat and mass transfer; two-node
  non-equilibrium self-pressurising tanks.
- **PLT-R-16** Non-condensable gas in liquid lines (priming).
- **PLT-R-17** Boiling heat transfer (film, transition, nucleate) for
  chill-down; solid thermal networks (lumped masses with conduction and
  convection links); radiation-cooled nozzle extensions.
- **PLT-R-18** Two-stream heat exchangers with wall elements.
- **PLT-R-19** Turbomachinery: reaction turbines with tabulated maps in
  pressure ratio and velocity ratio; hydraulic (liquid-driven) turbines;
  low-speed and four-quadrant pump maps; pump head degradation with inlet
  vapour fraction; multi-stage pumps.
- **PLT-R-20** Injector dome and chamber priming and ignition; combustion
  time lag for low-frequency combustion dynamics.
- **PLT-R-21** Sensors, PID controllers, engine controllers (Pc and MR loops
  acting on valves), event-driven sequences, relief valves, burst discs,
  solenoid valves.
- **PLT-R-22** Design mode: steady state with free parameters solved for
  targets.
- **PLT-R-23** Linearisation at an operating point: state-space matrices
  from the Jacobian, eigenvalues and frequency responses, for stability and
  control analysis.

### Studies, data and traceability

- **PLT-R-30** Studies as files: single, sweep, Monte Carlo, envelope,
  sensitivity, calibration.
- **PLT-R-31** Run records with software version, project commit, SHA256 of
  the resolved model and property tables, command, results and report.
- **PLT-R-32** Derived outputs and limit checks in studies.
- **PLT-R-33** Literature data library: each data set stored with its source
  (report, table or figure), digitisation method and uncertainty, units and
  licence; comparison reports with error metrics against it.

### Quality

- **PLT-R-40** Every physical model has a verification case; every model
  used at a reference problem is in the validation matrix.
- **PLT-R-41** CI on Windows, macOS and Linux with reference-result
  baselines.
- **PLT-R-42** Analyses state a credibility level (section 8).

### Use, scale and deployment

- **PLT-R-50** One-command installation on Windows, macOS and Linux.
- **PLT-R-51** Desktop application with hierarchical schematics, study runner
  and run comparison.
- **PLT-R-52** Python as the scripting interface for everything.
- **PLT-R-53** Stepping API for co-simulation; FMI export as an option.
- **PLT-R-54** Scale: a 5,000-state model (SSME with discretised lines and
  walls) solves steady in under 5 s and a 5 s start transient in under 2 min
  on a laptop.

### Maintainability

- **PLT-R-60** All data are plain files in Git (YAML, CSV/Parquet,
  Markdown); no database.
- **PLT-R-61** Documentation updated in the same change as the code.

## 5. Target architecture

### Layers

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Front ends   desktop app (Tauri)   web viewer   CLI   Python notebooks   │
├──────────────────────────────────────────────────────────────────────────┤
│ Project layer (Python, Pydantic v2)                                      │
│   project, parameters, cases · studies · run store and run records       │
│   reports (Plotly) · literature data library and digitiser · calibration │
├──────────────────────────────────────────────────────────────────────────┤
│ Bindings     PyO3 module · WebAssembly C ABI · stepping API (and FMU)    │
├──────────────────────────────────────────────────────────────────────────┤
│ Core (Rust)  model resolution (subsystems, parameters, library)          │
│              compile · components · solvers · linearisation              │
│              properties: pure fluids, mixtures, combustion products      │
└──────────────────────────────────────────────────────────────────────────┘
```

- Everything that changes equations or the resolved model is in the core, so
  all front ends give the same answer.
- Runs, files, data and reports are in the Python project layer.
- The Python package installs the user-facing `fluidnet` command (D-016);
  the Rust CLI stays for CI and debugging.

### Multi-component and reacting flow in the core

This is the largest architectural change and the one that opens gas-generator
and staged-combustion engines.

- A model declares its components, for example `fuel: H2`, `ox: O2`,
  `pressurant: He`, and `products` of a propellant pair. Each volume node gets
  composition states (mass fractions; one per component beyond the first), in
  addition to P and h. Pure-fluid circuits keep their current two states, so
  existing models and their performance are unchanged.
- Non-reacting mixtures (pressurant with propellant vapour, gas in a liquid
  line) use ideal mixing of the pure-fluid tables: partial pressures for gas,
  volume-additive liquid, with phase equilibrium of the condensable component
  only.
- Reacting mixtures use a three-dimensional equilibrium table in pressure,
  enthalpy and mixture fraction (fuel-rich to oxidiser-rich) generated with
  Cantera, giving T, ρ, its derivatives, transport properties and the
  isentropic exponent. Gas-generator and preburner gases at low mixture ratio
  are inside the table; the limits of equilibrium at very low temperature are
  documented and an empirical option is left for kerosene gas generators.
- Combustion devices become volume nodes with injection ports and outlet
  ports; the current chamber with its nozzle is one case.
- The flow laws keep their form; the properties they call depend on
  composition. The Jacobian coloring and sparsity extend unchanged, since
  composition states are read and written by the same terms as h.

### Project folder

```
ssme/                                Git repository of one project
  fluidnet-project.yaml              name, fluidnet version, components, folders
  models/
    ssme.yaml                        top-level engine
    turbopump_hpftp.yaml             subsystem
    preburner.yaml                   subsystem used twice
  params/   baseline.yaml, cases/rpl_040.yaml
  library/  valves.yaml, lines.yaml, maps/hpftp.yaml
  studies/  steady_rpl_sweep.yaml, start_902-641.yaml, calibrate_100rpl.yaml
  data/     literature/CR-205360_table_fig9.yaml (values, source, uncertainty)
  runs/     <UTC time>_<study>_<hash8>/ manifest.json, model.resolved.yaml,
            results.parquet, summary.json, report.html
```

### File formats

Subsystem use and definition:

```yaml
components:
  - {id: FPB, type: subsystem, model: preburner.yaml, params: {mr_design: 0.9}}
connections:
  - [FPOV.out, FPB.ox_in]
  - [FPB.gas_out, HPFTP.turbine_in]
```

```yaml
name: preburner
ports: {fuel_in: fuel_dome, ox_in: ox_dome, gas_out: chamber}
parameters: {mr_design: 0.9, volume: 4 L}
components:
  - {id: chamber, type: combustor, volume: =volume, products: H2-O2}
  ...
```

Resolved names are prefixed with the subsystem id (`FPB.chamber.T`).
Parameters are referenced with `=name` and may use arithmetic in SI.

Study:

```yaml
study: calibration
model: models/ssme.yaml
params: [params/baseline.yaml]
mode: steady
free:
  HPFTP.turbine.eff_scale: {init: 1.0, bounds: [0.8, 1.2]}
  FPB.injector.cd:         {init: 0.8, bounds: [0.5, 1.1]}
targets:
  data: data/literature/santi1994_ttb50.yaml
  match: [HPFTP.speed, HPOTP.speed, MCC.Pc, FPB.T, OPB.T]
```

Literature data file:

```yaml
source: {report: NASA CR-205360, item: Fig. 9 (table), page: 7}
method: transcribed            # or: digitised, with the digitiser record
uncertainty: {type: reading, value: 0.5 %}
licence: NTRS public
units: {HPFTP_speed: rpm, MCC_Pc: psia}
points:
  - {id: 40RPL, HPFTP_speed: ..., MCC_Pc: ...}
```

## 6. Work packages

Effort is in focused working weeks, one person with AI-assisted development.

### WP1 Foundation (2 weeks)

- Private GitHub repository; CI (GitHub Actions): fmt, clippy, tests on
  Linux, Windows and macOS; wheels for win_amd64, macosx_arm64 and
  manylinux; WebAssembly and UI build; `mkdocs build --strict`; component
  reference up to date.
- Reference-result baselines for all examples; CI fails on unexplained
  changes.
- Model format version key and migrations; strict validation (open items
  OI-017, OI-024, OI-025); CLI exit codes and argument errors (OI-026).
- Fix OI-019 (ethanol enthalpy reference), OI-020, OI-021, OI-030.
- Exit: CI green on three platforms; Windows wheel installs; RL10 regression
  in CI.

### WP2 Model composition (3 weeks, after WP1)

- Subsystems, parameters, cases, library templates, tags (PLT-R-01 to 05);
  hierarchical display in the UI.
- Exit: the RL10A-3-3A rebuilt from a turbopump subsystem and an injector
  subsystem gives identical results.

### WP3 Studies, run records and calibration (4 weeks, after WP2)

- Python project layer with Pydantic v2; `fluidnet` command with run, study,
  runs subcommands.
- Study types (PLT-R-30), derived outputs and checks (PLT-R-32), run store
  with SHA256 identity (PLT-R-31), Parquet results, Plotly reports, parallel
  execution, warm-started sweeps (OI-022).
- Exit: the RL10 validation runs from one study file and reproduces
  `results.json`.

### WP4 Literature data library (2 weeks, after WP3; then continuous)

- Data file format with provenance, uncertainty and licence (PLT-R-33).
- Digitiser generalised from the RL10 map work (`validation/rl10/maps/`):
  deskew, grid calibration, marker and curve extraction, check plots, with
  the reading uncertainty recorded.
- Comparison reports: overlays, error metrics per channel and window,
  pass/fail against the reference-problem targets.
- Load the Level 1 data sets first (R1.1–R1.3, R1.5 tables).
- Exit: R1.1 and R1.3 data in the library, compared with the current model.

### WP5 Multi-component and reacting flow (8 weeks, after WP2)

- Composition states, mixture properties, combustion product tables in
  (P, h, mixture fraction), combustor nodes with outlets, combustion-gas
  turbines (PLT-R-10, 11, 12, 16).
- Generalised combustion table generator; tables for H2/O2 (fuel-rich and
  oxidiser-rich), CH4/O2, ethanol/O2 (OI-019 corrected), N2O/ethanol,
  RP-1/O2; each compared with CEA.
- Exit: R3.3 (STME gas-generator module) runs start and cutoff; a
  gas-in-line priming case (R1.9) runs.

### WP6 Fluid and thermal phenomena (8 weeks, parallel to WP5 after WP2)

- Dyer non-equilibrium flow (PLT-R-13); wall elasticity (PLT-R-14);
  two-fluid and two-node tanks (PLT-R-15, uses WP5 mixtures); boiling heat
  transfer, solid thermal networks, tabulated material properties, nozzle
  radiation (PLT-R-17).
- Exit: Level 1 problems R1.1–R1.8 within their targets or the remaining
  difference explained in an analysis page each.

### WP7 Turbomachinery and engine components (6 weeks, after WP5)

- Heat exchanger (PLT-R-18); reaction and hydraulic turbines, low-speed and
  four-quadrant maps, inlet two-phase degradation, multi-stage pumps
  (PLT-R-19); injector priming, ignition and combustion time lag (PLT-R-20);
  design mode (PLT-R-22).
- Exit: Level 2 problems; R3.2 (RL10-IIB); R3.1 start with low-speed maps;
  R3.3 within the code-to-code target.

### WP8 Control and linear analysis (3 weeks, after WP5)

- Sensors, PID, engine controllers, sequences, protection devices
  (PLT-R-21); linearisation and eigenvalue analysis (PLT-R-23); stepping API
  (PLT-R-53).
- Exit: Pc/MR loop closed on a gas-generator model; eigenvalues of the RL10
  operating point compared with a time-domain step response.

### WP9 Staged combustion (8 weeks, after WP6–WP8)

- SSME model from public descriptions: two preburners, four turbopumps,
  hydraulic turbine, nozzle and chamber cooling circuits, controller.
- Calibration at 100–109 % RPL; prediction at 17–40 % RPL (R4.1); start
  transient with measured valve positions (R4.2). An analysis page for each.
- Exit: Level 4 targets met or the remaining differences explained.

### WP10 Scale and performance (3 weeks, spread over WP5–WP9)

- Profiling on the SSME model; parallel colored Jacobian evaluation;
  property caching; reuse of symbolic factorisation across study runs.
- Exit: PLT-R-54.

### WP11 Desktop application (6 weeks, from WP3 on, parallel)

- Tauri application with the native core and the current light interface:
  project explorer, hierarchical schematic, YAML side by side, study runner,
  run browser and comparison with literature data.
- Exit: R3.3 built and run from the application without editing YAML.

### WP12 Documentation (continuous, about 10 %)

- Design description, user guide and validation matrix updated in each
  change; one analysis page per reference problem; tutorials for a feed
  system, a tank pressurisation and an expander engine.

## 7. Schedule

Proposal, assuming about half of the working time for fluidnet.

| Phase | Months | Work packages | Milestone |
|---|---|---|---|
| 0 Foundation | Nov – Dec 2026 | WP1 | M0: CI, Windows wheel, strict validation, known bugs fixed |
| 1 Structure and data | Dec 2026 – Feb 2027 | WP2, WP3, WP4 | M1: subsystems, studies, run records, literature data library; RL10 as a study |
| 2 Mixtures and phenomena | Feb – Jun 2027 | WP5, WP6 | M2: Level 1 validated; STME gas-generator module runs |
| 3 Components and control | Jun – Sep 2027 | WP7, WP8 | M3: Level 2 and 3 validated; design mode; linear analysis |
| 4 Staged combustion | Sep 2027 – Jan 2028 | WP9, WP10 | M4: SSME steady and start (Level 4) |
| Application | Mar 2027 onwards | WP11 | M5: desktop application |
| Release 1.0 | Q1 2028 | – | Level 4 accepted, documentation released, model format frozen for 1.x |

Total effort about 55 focused weeks plus documentation.

Next two weeks:

1. Push the repository to GitHub (private).
2. CI for Linux and Windows; Windows wheel.
3. Fix OI-019 (ethanol table) and OI-020/021 with baseline checks.
4. Download DLR PS-LN2-Set1 and set up R1.1 as the first literature case.
5. Release v0.5.1.

## 8. Verification and model credibility

| Level | Use | Required |
|---|---|---|
| C1 Exploratory | Trade studies, early sizing | Model runs without warnings; all components inside their verified regime |
| C2 Design | Sizing, margins | C1, plus: independent model review with the checklist below; numerical sensitivity (segments ×2, rtol ÷10) below the stated tolerance; inputs traced to sources |
| C3 Decision-critical | Safety margins, go/no-go | C2, plus: dominant effects validated against data of similar hardware; result uncertainty quantified |

Review checklist: topology against the schematic; each parameter traced to a
source; units; initial and boundary conditions; fluids and components;
warnings; mass and energy balance of the result; one operating point against
a hand calculation.

For literature validation specifically: the parameters calibrated on a data
set are listed, and agreement is only claimed on data not used for the
calibration.

## 9. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| Public engine data is incomplete (SSME maps, volumes and resistances not published) | Calibration has more unknowns than data; agreement is not evidence | Calibrate on steady points, predict other power levels and the transient; report calibrated parameters and their sensitivity; prefer fully specified sets (OEPSS, RL10-IIB) for code checks |
| Digitisation error | Wrong reference values | Record reading uncertainty; check plots; cross-check against tabulated values where both exist |
| Licence and export restrictions | Use of data that may not be used | Exclude ITAR-marked AEDC J-2 reports and the Boeing-proprietary SSME Orientation as a data source; record licence per data file; CC BY-NC data only for non-commercial use |
| Multi-component core change destabilises the solver | Regressions, slower pure-fluid models | Pure-fluid circuits keep two states; baselines in CI before the change; feature behind a model-level declaration until validated |
| Single developer, large scope | Plan slips | Each phase ends with a usable release; WP11 can move without blocking validation |
| Fuel-rich equilibrium limits | Gas-generator gas properties wrong at low temperature | Compare with published GG temperatures; empirical tables where equilibrium fails |

## 10. Decisions taken in this plan (proposed)

D-012 subsystems and parameters in the core; D-013 Python project layer with
Pydantic v2; D-014 run identity by SHA256; D-015 Parquet; D-016 one
`fluidnet` command; D-017 Tauri desktop application; D-025 composition
states per node with ideal mixing and (P, h, mixture fraction) product
tables; D-026 reference-problem ladder as the measure of progress, Level 4
(SSME) as the 1.0 acceptance.

## 11. Open questions for the owner

| ID | Question | Needed by |
|---|---|---|
| D-024 | Desktop application first or a hosted web application | Start of WP11 |
| Q-1 | Paywalled or blocked sources that would strengthen the ladder; access through a university library would be enough: Zimmerman 2015 thesis (PDF), AIAA 2013-4045, Dyer et al. AIAA 2007-5702, Lecourt & Steelant 2007 (JPP), LE-7A AIAA 2004-3850, JAXA-RR-04-010, NAL TR-868 (LE-5) | Before WP6 and WP9 |
| Q-2 | Accept a 1.0 date in Q1 2028 with SSME as acceptance, or set a shorter first release at Level 3 | M1 |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First draft for review |
| B | 2026-11-02 | Owner decisions D-019 to D-023: private GitHub repository, no single target application, literature data only. Reference-problem ladder (Levels 1–5) from the literature research; multi-component and reacting flow added as the main architecture change; work packages reorganised (WP1–WP12); SSME as 1.0 acceptance |
