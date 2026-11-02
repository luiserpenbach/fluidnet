# Platform development plan

FNT-PLT-PLN-001 · Rev A · Draft · 2026-11-02 · Owner: Luis Erpenbach

## 1. Purpose

This plan describes how fluidnet develops from a validated solver (v0.5.0) into
an engineering platform that a propulsion team uses for complex projects: whole
vehicle propulsion systems, engine cycles with start and shutdown, test-bench
predictions and the correlation of models with test data. It defines what
"usable for complex projects" means, the requirements that follow, the target
architecture, the work packages with exit criteria, the order in which they are
done, and the decisions that are still open.

Status Draft: the plan is a proposal. Section 11 lists the decisions needed
before it is released.

## 2. Starting point (v0.5.0)

What exists and works:

| Area | State |
|---|---|
| Solver core | Rust; (P, h) node states; TR-BDF2 transient with colored finite-difference Jacobian and sparse LU; steady solver with flow guess, shaft continuation, Newton, pseudo-transient continuation and transient fallback; 5–50 ms for typical feed systems, 0.25 s for an expander steady state |
| Physics | Liquid, gas, supercritical and two-phase (homogeneous equilibrium) flow; cavitation and flashing; pipes with inertia and friction; valves (scheduled and pressure-actuated), check valves, regulators; tanks; centrifugal pumps and impulse turbines with tabulated maps; shafts; walls with heat capacity, Bartz and Gnielinski heat transfer; equilibrium combustion chamber (H2, CH4, ethanol with O2) |
| Fluids | Eight CoolProp-tabulated fluids: H2, O2, N2, He, CH4, H2O, N2O, ethanol |
| Front ends | CLI, Python package (numpy results, threaded sweeps, warm starts), browser schematic editor (WebAssembly) |
| Evidence | 30 automated tests; RL10A-3-3A steady state within 0.62 % mean / 2.05 % max at four test points ([FNT-VAL-ANA-002](../04-verification/rl10.md)) |
| Documentation | This documentation set |

What prevents use on a complex project today:

| Gap | Consequence |
|---|---|
| A model is one flat file | A vehicle model with an engine, two tank systems and a pressurant system becomes one file of several hundred components that nobody else can review; an engine model cannot be reused in a vehicle model and a test-bench model |
| No named parameters or cases | Design points, variants and test configurations are copies of the model file; they drift apart |
| No study definitions or run records | Sweeps, Monte Carlo and calibrations are ad-hoc scripts (the RL10 scripts are an example); results cannot be traced to the model version and inputs that produced them |
| No test-data workflow | Comparing with a test means writing a script per test |
| One fluid per circuit | No pressurant gas over a liquid propellant in the same tank; no self-pressurising N2O tank model with separate liquid and vapour temperatures |
| No N2O-oxidiser combustion tables | The E2 engine (N2O/ethanol) cannot be modelled |
| No controllers or sequences beyond pressure-actuated valves | No closed-loop throttle, no event-driven sequences |
| No heat exchanger, no low-speed pump maps, no design mode | Dual-expander cycles, starts and shutdowns, and sizing inside the solver are not possible |
| Builds only on Linux; no CI | Team members on Windows cannot install it; regressions are found by hand |

## 3. What "usable for complex projects" means

The platform is accepted when it carries the following reference projects. Each
is also the integration test of the work packages it depends on.

| ID | Reference project | Scope of the model | Acceptance |
|---|---|---|---|
| RP1 | H2 lander propulsion system (Hopper) | N2 COPV, regulators, ethanol and N2O tanks, feed lines, valves, E2 engine (N2O/ethanol, 2.2 kN, 25 bar); fill, pressurisation, start, 10 s hover throttle profile, shutdown, water hammer at valve actions | Built from subsystems by a team member other than the author; validated against cold-flow and hot-fire data of the test site; used in an analysis page (ANA) for pressurant budget and feed margins |
| RP2 | E60-DEX dual-expander engine | Two turbopumps, two turbines, H2/O2 heat exchange, regenerative jacket, chamber; design point, 10:1 throttle map, start and shutdown | Design point found by design mode in under 1 s; cross-checked against `resa.cycle` (O/F flat over the throttle range) |
| RP3 | Test bench and GSE | Run-tank fill and chill-down, blowdown, relief valve and burst disc cases, bench feed system | Bench predictions issued before a campaign and compared with the measured data after it, with one command each |
| RP4 | RL10A-3-3A (regression) | Existing validation | Reproduced by a study file and a test-data file instead of a custom script; start transient within the measured range once low-speed maps are in |

## 4. Requirements

Numbered so that work packages and tests can refer to them.

### Modelling

- **PLT-R-01** A model can contain subsystems: other model files with declared
  external ports and parameters, nested to any depth.
- **PLT-R-02** Models have named parameters with units; component parameters
  can reference them; a case file overrides parameters without copying the
  model.
- **PLT-R-03** Component templates from a parts library (valves with Cv,
  line sizes, regulators) can be referenced and overridden.
- **PLT-R-04** Component ids can follow the Hopper P&ID tag standard
  (HOPPER-PID-STD-001); the tool exports a tag list of a model.
- **PLT-R-05** Invalid model files are rejected with an error that names the
  file, the component, the key and the allowed values (unknown keys included).

### Physics

- **PLT-R-10** Tanks with a liquid propellant and an ullage gas of a
  different species (pressurant), with separate liquid, ullage and wall
  temperatures.
- **PLT-R-11** Self-pressurising tanks (N2O) with liquid and vapour out of
  thermal equilibrium.
- **PLT-R-12** Combustion tables for any propellant pair Cantera supports,
  generated from a configuration file; first N2O/ethanol, GOX/GH2,
  LOX/ethanol.
- **PLT-R-13** Two-phase injector and orifice flow with thermal
  non-equilibrium (Dyer model) as an option to the homogeneous model.
- **PLT-R-14** Pipe wall elasticity in the wave speed of liquid lines.
- **PLT-R-15** Sensors, PID controllers and event-driven sequences acting on
  valve openings, regulator set points and speeds.
- **PLT-R-16** Relief valves, burst discs and solenoid valves with opening
  and closing times.
- **PLT-R-17** Two-stream heat exchanger with wall.
- **PLT-R-18** Low-speed and four-quadrant pump maps.
- **PLT-R-19** Design mode: steady state with free parameters solved for
  targets inside the solver.
- **PLT-R-20** Boiling heat transfer for chill-down; radiation-cooled nozzle
  extensions.

### Studies, data and traceability

- **PLT-R-30** Studies are files: single run, sweep, Monte Carlo, operating
  envelope, sensitivity, calibration.
- **PLT-R-31** Every run writes a run record: software version, project
  commit, SHA256 of the fully resolved model and of the property tables,
  command, inputs, results and a report. Two runs with the same hashes give the
  same results.
- **PLT-R-32** Derived outputs and limit checks (margins, NPSH, pressure
  limits) are defined in the study and reported pass/fail.
- **PLT-R-33** Test data files are imported through a channel map (sensor tag,
  model channel, unit, offset, time shift); runs can be driven by measured
  inputs and compared with measured outputs.

### Quality

- **PLT-R-40** Every physical model has a verification case; the validation
  matrix ([FNT-VAL-REG-001](../04-verification/validation-matrix.md)) is
  complete for every model used in a released analysis.
- **PLT-R-41** Continuous integration builds and tests on Windows, macOS and
  Linux on every change and checks that results of the reference models did
  not change beyond stated tolerances.
- **PLT-R-42** Analyses state a credibility level (section 8).

### Use and deployment

- **PLT-R-50** Installation on Windows, macOS and Linux with one command
  (`pip install fluidnet`) or one installer for the desktop application.
- **PLT-R-51** A desktop application opens a project folder, edits models
  graphically including subsystems, runs studies and compares runs with each
  other and with test data.
- **PLT-R-52** Python is the scripting interface; everything the application
  does can be done from Python and from the command line.
- **PLT-R-53** A model can be stepped from external code (vehicle or GNC
  simulation) with inputs and outputs exchanged at each step.

### Maintainability

- **PLT-R-60** All project data are plain files in a Git repository: YAML
  models, parameters and studies, CSV/Parquet data, Markdown documentation. No
  database is required.
- **PLT-R-61** Documentation is updated in the same change as the code it
  describes ([FNT-DEV-STD-001](../05-development/conventions.md)).

## 5. Target architecture

### Layers

```
┌──────────────────────────────────────────────────────────────────────────┐
│ Front ends   desktop app (Tauri)   web viewer   CLI   Python notebooks   │
├──────────────────────────────────────────────────────────────────────────┤
│ Project layer (Python, Pydantic v2)                                      │
│   project, parameters and cases · studies · run store and run records    │
│   reports (Plotly) · test-data import and comparison · calibration       │
├──────────────────────────────────────────────────────────────────────────┤
│ Bindings     PyO3 module · WebAssembly C ABI · stepping API (and FMU)    │
├──────────────────────────────────────────────────────────────────────────┤
│ Core (Rust)  model resolution (subsystems, parameters, library)          │
│              compile · components · solvers · properties · combustion    │
└──────────────────────────────────────────────────────────────────────────┘
```

Rules for the split:

- Anything that changes the equations or the resolved model belongs in the
  core, so that the CLI, Python, the desktop application and the web viewer
  give the same answer. Subsystems, parameters and library templates are
  therefore resolved in Rust, before compilation.
- Anything that organises runs, files and reports belongs in the Python
  project layer. It is where most users work, it is easy to change, and it
  uses the team's existing stack (Pydantic v2, Plotly, numpy, pandas, scipy).
- The Rust CLI remains the minimal solver entry point for CI and debugging.
  The Python package installs the user-facing `fluidnet` command, which covers
  both solver commands and project commands (decision D-016 below).

### Project folder

```
h2-propulsion/                      Git repository of one project
  fluidnet-project.yaml             name, fluidnet version, default fluids, folders
  models/
    h2_propulsion.yaml              top-level vehicle model
    e2_engine.yaml                  subsystem with ports fuel_in, ox_in
    pressurant.yaml                 subsystem
  params/
    baseline.yaml                   named parameters of the design point
    cases/hover_hot.yaml            overrides for one case
  library/
    valves.yaml                     component templates (Cv, cd, actuation times)
    lines.yaml
  studies/
    hover_profile.yaml              transient with throttle sequence and checks
    pressurant_mc.yaml              Monte Carlo on regulator band and temperatures
  data/
    tests/2027-03-14_CF-012.parquet test data with metadata (campaign, run id)
    channel_maps/atb.yaml           sensor tag -> model channel
  runs/                             run records (Git-ignored or Git LFS)
    2026-11-20T1432Z_hover_profile_3f9a12c4/
      manifest.json                 versions, hashes, command, timings, checks
      model.resolved.yaml           the flattened model that was solved
      results.parquet               all channels
      summary.json                  final values, derived outputs, pass/fail
      report.html                   Plotly report
  docs/                             analysis pages (ANA) that cite run ids
```

### File formats

Subsystem (in the parent model):

```yaml
components:
  - {id: E2, type: subsystem, model: e2_engine.yaml,
     params: {Pc_design: 25 bar, throat_diameter: 27.6 mm}}
connections:
  - [FU-V-110.out, E2.fuel_in]
  - [OX-V-210.out, E2.ox_in]
```

Subsystem file (declares its interface):

```yaml
name: E2 engine
ports: {fuel_in: fuel_manifold, ox_in: ox_manifold}   # external name -> internal port
parameters:
  Pc_design: 25 bar
  throat_diameter: 27.6 mm
components:
  - {id: fuel_manifold, type: junction, volume: 15 mL}
  - {id: tca, type: combustion_chamber, throat_diameter: =throat_diameter, ...}
```

Inside the resolved model, internal names are prefixed with the subsystem id
(`E2.tca.Pc`), so output channels stay unique and readable.

Parameters are referenced with `=name` and may use arithmetic
(`=0.5*throat_diameter`); all evaluation is in SI after unit conversion.

Study:

```yaml
study: monte_carlo
model: models/h2_propulsion.yaml
params: [params/baseline.yaml]
mode: transient
samples: 500
seed: 1
vary:
  REG-101.set_pressure: {dist: normal, mean: 30 bar, std: 0.5 bar}
  N2O_tank.T0:          {dist: uniform, low: 278 K, high: 293 K}
outputs:
  ox_injector_dp_min: min(E2.ox_injector.dp)
  pressurant_left:     last(PR-T-001.m)
checks:
  - ox_injector_dp_min > 0.2 * E2.tca.Pc
```

Run record manifest: fluidnet version and commit, project commit and dirty
flag, SHA256 of `model.resolved.yaml`, of each property table and of the
combustion tables, study file hash, command line, start time, wall time,
solver statistics, check results. The hash of the resolved model is the
identity of a result; an analysis page cites the run directory name.

## 6. Work packages

Effort is in focused working weeks (one person, with AI-assisted
development). Dependencies are given by work package number.

### WP1 Foundation: repository, CI, packaging (2 weeks)

- Private Git repository (GitHub), branch protection, issue tracker.
- CI (GitHub Actions): `cargo fmt`, `clippy`, `cargo test --release` on
  Linux, Windows and macOS; Python wheels with maturin for win_amd64,
  macosx_arm64 and manylinux x86_64; WebAssembly and UI build; `mkdocs build
  --strict`; check that the generated component reference is up to date.
- Reference-result baselines: steady and transient results of all examples
  stored as compressed files; CI fails when a channel moves beyond its
  tolerance; an explicit command updates the baselines with a reason.
- Model file version key (`fluidnet: 0.6`) and a migration function per
  format change.
- Strict model validation: unknown keys (today a warning), wrong units and
  wrong types are errors with the file, component, key and nearest valid key
  named; a JSON Schema generated from
  the catalog for editor completion and checking in VS Code.
- Exit: CI green on three platforms; `pip install` of a released wheel works
  on Windows; the RL10 regression runs in CI.

### WP2 Model composition (3 weeks, after WP1)

- `subsystem` component, `ports` and `parameters` sections, resolution and
  flattening in the core (PLT-R-01).
- Named parameters with `=name` references and arithmetic; parameter files
  and case files (PLT-R-02).
- Component templates from library files with `use:` and overrides (PLT-R-03).
- Optional `tag` and `description` fields on every component; tag-list export
  as CSV (PLT-R-04).
- UI: subsystems shown as one block with their ports; open to edit.
- Exit: E2 engine as a subsystem inside a vehicle model and inside a
  test-bench model, both solving; a sweep over a named parameter.

### WP3 Studies, run records and reports (4 weeks, after WP2)

- Python project layer with Pydantic v2 models for project, parameter, case
  and study files; one `fluidnet` command: `fluidnet run`, `fluidnet study
  run|status|report`, `fluidnet runs list|compare`.
- Study types: single, sweep (list and full factorial), Monte Carlo (random
  and Latin hypercube, fixed seed), envelope (2-D grid with failure map),
  sensitivity (normalised derivatives from warm-started steady solves),
  calibration (least squares on factors with bounds against targets; the RL10
  calibration is the first user).
- Run store with manifests and SHA256 identity (PLT-R-31); reuse of results
  when the hash matches; Parquet results.
- Derived outputs and checks (PLT-R-32) with a small expression language
  (`min`, `max`, `last`, `mean`, `at(t)`, arithmetic on channels).
- Plotly HTML reports: per run, per study (sweep plots, Monte Carlo
  histograms and percentiles, envelope maps), and run-to-run comparison.
- A Markdown run-record block for analysis pages.
- Parallel execution on the local machine (threads; the solver releases the
  GIL).
- Exit: RP4: the RL10 validation runs as `fluidnet study run
  studies/rl10_validation.yaml` and reproduces `results.json`.

### WP4 Test-data correlation (2 weeks, after WP3)

- Importers for CSV, Parquet and HDF5 DAQ logs (the LabJack/ZeroMQ DAQ
  format first) with metadata: campaign, run id, date, configuration.
- Channel maps (PLT-R-33); unit conversion; time alignment on an event
  channel.
- Replay: measured signals as model inputs (valve commands, tank pressures).
- Comparison report: overlays, error metrics per window, pass/fail against
  stated tolerances.
- Exit: one cold-flow test of the Hopper test site replayed and compared with
  one command.

### WP5 Physics for the H2 lander (8 weeks, parallel to WP3–WP4 after WP2)

- Two-fluid tank (PLT-R-10): liquid node of the propellant, ullage node with
  pressurant and propellant vapour as an ideal mixture of two tabulated
  fluids, wall nodes; heat and mass transfer at the interface and the walls;
  outflow from the liquid, gas ingestion flagged when the liquid is depleted.
- Self-pressurising N2O tank (PLT-R-11): two-node model (liquid and vapour
  at separate temperatures) with a wall node, the model recommended in
  E2-PRS-DOC-01; validated against published N2O blowdown data (Zimmerman et
  al. 2013 and the data sets reviewed there) before use.
- Correct the ethanol reference enthalpy (open item OI-019: the liquid
  vaporisation enthalpy is missing, T_c about 60 K high at O/F 1.5).
- Combustion table generator for arbitrary propellant pairs from a YAML
  configuration; tables for N2O/ethanol, GOX/GH2 and LOX/ethanol, each
  compared with CEA over the full O/F range (PLT-R-12).
- Dyer non-equilibrium option for two-phase restrictions (PLT-R-13).
- Wave speed with wall elasticity (Korteweg) for pipes with a wall material
  and thickness (PLT-R-14).
- Control components (PLT-R-15): `sensor` (any channel, first-order lag,
  noise, sample rate), `pid` (anti-windup, output limits, acting on valve
  opening, regulator set point or speed), `sequence` (state machine:
  conditions on channels and times trigger set-point changes).
- `relief_valve`, `burst_disc`, `solenoid_valve` (PLT-R-16).
- Exit: RP1 at model level: full H2 propulsion model with a hover throttle
  profile, verified component by component, then compared with the first
  cold-flow campaign.

### WP6 Physics for expander cycles (6 weeks, after WP2)

- Two-stream counter- and co-flow heat exchanger with wall elements
  (PLT-R-17).
- Low-speed pump maps in the head/torque against atan(N/Q) form, extended to
  four quadrants (Suter representation); RL10 start rechecked with
  TM-107318 figures 4.1.10–4.1.15 (PLT-R-18).
- Design mode (PLT-R-19): `targets` (channel = value) and `free` (parameter
  with bounds) in a steady run, solved in the same Newton system.
- Nucleate and film boiling correlations for chill-down; radiation from
  nozzle extensions (PLT-R-20).
- Exit: RP2: E60-DEX design point by design mode, throttle map, start
  transient; RL10 start within the measured 1.43–2.56 s or the remaining
  difference explained.

### WP7 Desktop application (6 weeks, after WP3)

- Tauri application with the native core; the existing React interface and
  its light theme.
- Project explorer (models, parameters, studies, data, runs), hierarchical
  schematic with subsystems, YAML view side by side with the schematic.
- Study runner with progress and cancel; run browser reading the run store;
  overlays of runs and test data.
- Windows installer built in CI; the web build kept for read-only viewing of
  models and runs (hosted with Azure Static Web Apps or Container Apps when
  needed).
- Exit: a team member builds a 30-component model from the library, runs a
  sweep and compares two runs without editing YAML by hand.

### WP8 Interfaces (2 weeks, after WP2)

- Stepping API (PLT-R-53): `Session(model).step(dt, inputs) -> outputs`
  in Python and Rust, for vehicle and GNC simulations; real-time factor
  measured for the H2 model.
- FMI 2.0/3.0 co-simulation export as a later option if a tool needs it.
- Import of CEA output for comparison; tag-list and parameter export for the
  Hopper Notion registers.
- Exit: the lander GNC simulation runs with the propulsion model in the loop.

### WP9 Documentation and onboarding (continuous, about 10 % of effort)

- Every work package updates its design description, the user guide and the
  validation matrix in the same change.
- Three tutorials: pressure-fed feed system, tank pressurisation, expander
  cycle; each with model files and expected results checked in CI.
- Model review checklist (section 8).
- Docs moved to the team's documentation system once it is decided
  (Notion import script exists; MkDocs site possible).

### Performance (within WP1, WP3, WP6)

Targets for a 2,000-state model (full RP1 vehicle with discretised lines):
steady state under 2 s, 10 s transient under 60 s. Measures as needed:
parallel colored Jacobian evaluation, property-lookup caching per node,
reuse of the symbolic factorisation across runs of a study.

## 7. Schedule

The calendar is a proposal for review; it assumes about half of the working
time is available for fluidnet and that the H2 lander needs its propulsion
model before the first hover tests in 2027.

| Phase | Months | Work packages | Milestone and exit |
|---|---|---|---|
| 0 Foundation | Nov 2026 | WP1 | M0: CI on three platforms, Windows wheel, strict validation |
| 1 Project structure | Dec 2026 – Jan 2027 | WP2, WP3 | M1: subsystems, parameters, studies, run records; RL10 as a study (RP4) |
| 2 Lander physics | Feb – Apr 2027 | WP5, WP4, WP8 | M2: H2 propulsion model with hover profile; first cold-flow comparison (RP1, RP3) |
| 3 Expander physics | May – Jul 2027 | WP6 | M3: E60-DEX model by design mode; RL10 start with low-speed maps (RP2) |
| 4 Application | Mar – Aug 2027, parallel | WP7 | M4: desktop application used by a second engineer |
| Release 1.0 | Sep 2027 | – | All reference projects accepted; documentation released; model file format frozen for 1.x |

Total effort is about 33 focused weeks plus documentation.

Next two weeks (start of WP1):

1. Create the private repository and push the current history.
2. Set up CI for Linux and Windows first (cargo test, wheel build).
3. Add the model format version key; turn the unknown-key warning into an
   error with a suggestion.
4. Store reference-result baselines for all examples.
5. Release v0.5.1 with a Windows wheel.

## 8. Verification and model credibility

Three levels of credibility, stated on every analysis page that uses fluidnet
results (adapted from the intent of NASA-STD-7009A, without its full
assessment):

| Level | Use | Required |
|---|---|---|
| C1 Exploratory | Trade studies, early sizing | Model runs without warnings; all components inside their verified regime (validation matrix) |
| C2 Design | Component sizing, margins in design reports | C1, plus: second-person model review with the checklist below; numerical sensitivity done (segments ×2 and rtol ÷10 change the reported results by less than the stated tolerance); inputs traced to sources |
| C3 Decision-critical | Flight and hot-fire safety margins, test go/no-go | C2, plus: the models of the dominant effects validated against test data of the same or similar hardware; uncertainty of the result quantified (Monte Carlo or bounding cases) |

Model review checklist: topology against the P&ID; every component parameter
traced to a drawing, data sheet or analysis; units; initial conditions;
boundary conditions and schedules; fluids; warnings; mass and energy balance
of the result; comparison with a hand calculation for one operating point.

Test strategy for the code: unit tests per model function; verification
cases per component (analytic or reference code); reference-result baselines
for all examples and reference projects; validation analyses against test
data. A change that alters a baseline must state why in the commit.

## 9. Risks

| Risk | Effect | Mitigation |
|---|---|---|
| One developer | Work stops or knowledge is lost when the author is unavailable | Documentation as part of every change; CI; Python project layer that others can extend; second maintainer trained during WP3 |
| Scope larger than available time | Hopper deadlines slip or the platform stays half done | Phases ordered by Hopper needs (lander first); each phase ends with a usable release; WP7 can be delayed without blocking analyses |
| N2O physics (non-equilibrium, two-phase injectors) not predictive | Wrong pressurant budget or feed margins on the lander | Validate tank and injector models against literature and own cold-flow tests before C2/C3 use; bounding cases with equilibrium and frozen limits |
| Rust as a barrier for contributors | Physics changes depend on one person | Component authoring handbook ([FNT-DEV-HBK-002](../05-development/adding-a-component.md)); most project work is YAML and Python |
| Results change between versions unnoticed | Analyses cite results that the current version no longer reproduces | Run records with version and hashes; reference baselines in CI |
| Ownership and licence unclear | Later conflict between team use and other uses | Decide before the repository is shared (D-020) |
| Windows-specific build problems | Team cannot install | Windows in CI from the first week; prebuilt wheels and installer |

## 10. Decisions taken in this plan (proposed)

Recorded in the [decision log](decisions.md) as proposed until this plan is
released: D-012 subsystems and parameters resolved in the core; D-013 Python
project layer with Pydantic v2; D-014 run identity by SHA256 of the resolved
model; D-015 Parquet for results and test data; D-016 one user-facing
`fluidnet` command from the Python package; D-017 Tauri for the desktop
application; D-018 documentation as Markdown in the repository.

## 11. Open decisions for the owner

| ID | Question | Options | Needed by |
|---|---|---|---|
| D-019 | Where is the repository hosted? | Personal GitHub account; Hopper team organisation; a company organisation | Start of WP1 |
| D-020 | Who owns fluidnet and under which licence is it shared with the team? | Personal with a licence to the team; team-owned; open source (MIT/Apache-2.0) | Before sharing the repository |
| D-021 | Where does the documentation live long term? | Notion (import script); MkDocs site on Azure Static Web Apps; both | M1 |
| D-022 | Which reference project goes first after WP3? | H2 lander (RP1, proposed); E60-DEX (RP2) | M1 |
| D-023 | Access to test data | Which campaigns, which format, who maintains the channel maps | Start of WP4 |
| D-024 | Desktop application or hosted web application first | Desktop (proposed: file-based, offline); hosted on Azure Container Apps with a run service | Start of WP7 |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First draft for review |
