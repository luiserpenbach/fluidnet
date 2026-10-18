# fluidnet

Fast, modular simulation of fluid systems and liquid rocket engine cycles:
steady state and transients of feed lines, pressurant systems, turbopumps,
regeneratively cooled walls and thrust chambers. Lumped-parameter networks in
the style of EcosimPro / GFSSP, real-fluid properties from CoolProp,
equilibrium combustion from Cantera, a Rust core that runs natively, as
WebAssembly in a browser schematic editor, and from Python.

```
fluids/          CoolProp-generated property tables (H2, O2, CH4, N2, He, H2O, N2O, ethanol)
crates/
  fluidnet-core  model, components, equations, solvers, properties (pure Rust)
  fluidnet-cli   `fluidnet` command-line runner
  fluidnet-wasm  C-ABI WebAssembly bindings used by the UI
  fluidnet-py    PyO3 extension (fluidnet._core)
  fluidnet-core/data/combustion.json  Cantera equilibrium tables
python/fluidnet/ Python package (numpy results, sweeps, plotting)
ui/              React + React Flow schematic editor, solver in a web worker
examples/        example systems (YAML), also used as regression tests
examples/python/ scripted studies (throttle map)
tests_models/    validation cases
tools/           table generators (CoolProp, Cantera), expander sizing script
docs/            ARCHITECTURE.md, VALIDATION.md
```

## Quick start

Native CLI:

```sh
cargo build --release
./target/release/fluidnet run examples/lox_feed_startup.yaml --out lox.csv
./target/release/fluidnet run examples/lh2_heated_channel.yaml --steady
./target/release/fluidnet check examples/water_hammer.yaml     # compile only, list nodes and states
./target/release/fluidnet catalog                              # component catalogue as JSON
cargo test --release                                           # validation suite
```

The CLI finds `fluids/` next to the working directory or the binary; otherwise
pass `--fluids DIR` or set `FLUIDNET_FLUIDS`. `--out x.json` writes all channels
as JSON, `--out x.csv` as CSV, `--print P,mdot` filters the console summary.

Web UI (needs Node 18+ and the wasm32 target):

```sh
rustup target add wasm32-unknown-unknown
cargo build --release -p fluidnet-wasm --target wasm32-unknown-unknown
cd ui && npm install && npm run wasm && npm run dev       # http://localhost:5173
npm run build                                             # static site in ui/dist (serve it with any web server)
```

`npm run wasm` copies the engine and the fluid tables (gzip + base64 text) into
`ui/public`. The built site is fully static and works offline.

Python (needs a Rust toolchain; builds a wheel with the fluid tables inside):

```sh
pip install maturin numpy
mkdir -p python/fluidnet/fluids && cp fluids/*.fntb python/fluidnet/fluids/
maturin develop --release          # or: maturin build --release, then pip install the wheel
```

```python
import fluidnet as fn
res = fn.run("examples/expander_cycle.yaml", mode="steady")
res.last("tca.Pc"), res["turbopump.speed"], res.units["tca.thrust"]

m = fn.load("examples/expander_cycle.yaml")          # model as a dict
runs = fn.sweep(m, "tbv", "opening", [0.06, 0.3, 0.6, 1.0])   # parallel threads
fn.run(m).plot("tca.thrust")                           # transient, matplotlib
fn.props("H2", 40e5, T=30)                             # table properties
```

Regenerating fluid tables (adds fluids or changes ranges):

```sh
pip install CoolProp numpy
python tools/gen_fluid_tables.py fluids            # all fluids, or name keys: ... fluids H2 O2
pip install cantera scipy
python tools/gen_combustion_tables.py crates/fluidnet-core/data/combustion.json
```

## Model files

YAML is the source of truth. The UI reads and writes the same format (Open, YAML
dialog), and the CLI runs it directly. Quantities take units; time-varying
inputs are tables of `[t, value]`.

```yaml
name: LOX feed startup
components:
  - {id: lox_tank, type: boundary, fluid: O2, P: 25 bar, T: 90 K}
  - {id: feed_line, type: pipe, length: 3 m, diameter: 20 mm, segments: 12, dz: -1 m, K: 1.2}
  - {id: main_valve, type: valve, diameter: 18 mm, characteristic: quick_opening,
     opening: [[0, 0], [0.1, 0], [0.35, 1]]}
  - {id: manifold, type: junction, volume: 80 mL}
  - {id: injector, type: orifice, diameter: 7.5 mm, cd: 0.7}
  - {id: chamber, type: boundary, fluid: O2, P: [[0, 1 bar], [0.35, 1 bar], [0.8, 15 bar]], T: 300 K}
connections:
  - [lox_tank, feed_line.in]
  - [feed_line.out, main_valve.in]
  - [main_valve.out, manifold]
  - [manifold, injector.in]
  - [injector.out, chamber]
simulation: {t_end: 1.5, output_dt: 0.002, rtol: 1e-4}
```

Units: Pa kPa MPa bar mbar psi atm · K degC degF · m cm mm um in ft · m2 cm2 mm2 in2 ·
m3 L mL cm3 in3 · kg/s g/s kg/h lb/s · s ms min h · W kW MW · W/m2 kW/m2 MW/m2 · %.
Bare numbers are SI.

`simulation`: `t_end`, `output_dt` (default t_end/500), `rtol` (1e-4), `atol_scale`,
`max_step`, `start_from_steady` (solve the steady state with inputs at t = 0 and
start the transient from it), `steady_time` (time at which schedules are
evaluated for a steady-state solve, e.g. the end of a start sequence).

## Components

| Type | Ports | What it models |
|---|---|---|
| `boundary` | p | Pressure and temperature (or quality) source/sink, constant or scheduled |
| `tank` | p | Rigid volume, mass and energy balance, homogeneous equilibrium, optional heat input |
| `junction` | p | Small volume where several elements meet (tee, manifold) |
| `pipe` | in, out | Discretised line: staggered volumes and momentum segments with inertia, Churchill friction, K losses, elevation, optional wall temperature (Gnielinski) or heat flux |
| `orifice` | in, out | Fixed restriction; liquid with cavitation/flashing, two-phase HEM, choked real gas; optional pressure recovery |
| `venturi` | in, out | Cavitating venturi (orifice with diffuser recovery, default 0.85) |
| `valve` | in, out | Scheduled opening, linear / equal-percentage / quick-opening, area or Cv/Kv |
| `check_valve` | in, out | Opens between cracking and full-open differential pressure |
| `regulator` | in, out | Proportional pressure regulator sensing outlet pressure |
| `loss` | in, out | Minor loss K (elbows, tees, filters) |
| `mass_flow` | in, out | Imposed mass flow (ideal pump or flow controller) |
| `shaft` | – | Rotor inertia, bearing and viscous losses; or a prescribed speed (motor) |
| `pump` | in, out | Centrifugal pump, affinity-law head and power maps from one design point, fluid inertance; all losses heat the fluid; NPSH output |
| `turbine` | in, out | Gas turbine: real-gas nozzle flow (choked or not) and impulse-stage torque linear in blade speed (stall to runaway) |
| `combustion_chamber` | fuel, ox | Equilibrium tables (H2, CH4, ethanol with O2) corrected for injection enthalpy, chamber filling and mixing dynamics, ignition, nozzle thrust and Isp |

Pumps and turbines name a `shaft` (with a gear `ratio`) or take a fixed `speed`.
Pipes take `parallel` channels and an optional wall with heat capacity
(`wall_thickness`, `wall_material`: CuCrZr, copper, GRCop-42, nickel, stainless,
Inconel 718, aluminium, Debye heat capacity) heated by a chamber
(`hot_chamber`, Bartz correlation with the local area ratio and wall-temperature
correction), by a prescribed gas (`hot_T`, `hot_htc`) or flux (`q_wall`).

Every node exposes `P`, `T`, `rho`, `h`, `x`; every flow element `mdot`, `dp` and,
where relevant, `choked`, `opening`, `Q`, `u_in`. Pumps: `head`, `eff`, `npsh`,
`power`, `torque`, `speed`. Turbines: `pr`, `eff`, `u_c0`, `power`, `torque`.
Shafts: `speed`. Chambers: `Pc`, `OF`, `thrust`, `Isp`, `cstar`, `Tc`, `mdot`,
`mdot_fuel`, `mdot_ox`. Cooled pipes: `Q`, `Q_hot`, `Tw_max` and per wall
element `w<i>.T`, `w<i>.T_hot`, `w<i>.Q`. Nodes where two-port elements
meet are named after a port, e.g. `main_valve.in`; pipe internals are
`pipe.n1 … pipe.n(N-1)` (volumes) and `pipe.s1 … pipe.sN` (segments).

## Engine example: 20 kN closed expander

`examples/expander_cycle.yaml` and `examples/expander_start.yaml` model a LOX/LH2
closed expander: LH2 pump and geared LOX pump on one shaft, a 60-channel CuCrZr
jacket heated by the chamber, turbine with bypass valve, injectors and a
57 mm throat chamber with an area ratio 60 nozzle. `tools/size_expander.py` sized the
injectors and turbine nozzle for the design point:

| | |
|---|---|
| Chamber pressure / O/F | 40.0 bar / 5.80 |
| Thrust / Isp (vacuum, ε 60) | 20.0 kN / 457 s |
| Shaft speed | 49,960 rpm (LOX pump geared 0.3) |
| Fuel pump | +107 bar, 163 kW, η 0.61 |
| Turbine | inlet 104 bar / 223 K, PR 1.72, η 0.62, 194 kW |
| Jacket | 2.0 MW heat pickup, hot wall 595 K |

The start example runs a tank-head start: boil-off from the warm jacket spins the
turbopump, ignition at 0.5 s, fuel-rich LOX idle, then the climb to mainstage by
about 6.5 s, ending within 0.3 % of the steady-state solution. The throttling
example opens the turbine bypass from the design point (66 % thrust and back).

## Performance

Typical feed-system models (20–60 states) run a full transient in 5–50 ms
natively; the browser engine is about 2× slower. The expander steady state
solves in 0.25 s, the 6 s throttling transient in 0.2 s, the 10 s start
(two-phase priming, chugging and bootstrap) in about 6 s. A 150-state
water-hammer case runs in 0.4 s; 600 states in about 3–6 s at rtol 1e-3–1e-4.

## Status and roadmap

Validated (see docs/VALIDATION.md): adiabatic gas blowdown, Bernoulli and
cavitating flow, Darcy friction, Joukowsky surge and 4L/a period, HEM flashing,
energy balance in heated lines, regulator droop, pump map, chamber pressure and
thrust coefficient, shaft torque and jacket energy balances of the expander.

Next:

1. Design-mode steady state: solve for chosen parameters (orifice areas, valve
   positions, pump speed) to meet targets (Pc, O/F, power balance) inside the
   solver, replacing the external sizing loop.
2. Local Bartz along the contour (area ratio per wall element) and nozzle
   extension radiation; two-phase boiling heat transfer for chill-down.
3. Multi-stage pumps and turbines with full Suter-type four-quadrant maps;
   dual-expander (two turbines, two shafts) and gas-generator topologies.
4. Tanks with liquid/ullage stratification and pressurant collapse.
5. Tauri desktop build with the native core; event-driven controls (valve
   sequencing on pressure or speed thresholds instead of time).

Known limits: one fluid per connected circuit (no mixing of species except in
the chamber); homogeneous equilibrium everywhere (no thermal non-equilibrium or
slip); pipe momentum neglects the convective acceleration term; combustion is
instantaneous equilibrium with a single isentropic exponent for the nozzle
(vacuum Isp a few seconds above CEA shifting equilibrium before the default
0.98 nozzle efficiency); pre-ignition chamber gas is an ideal mixture of the
propellants at their injection temperatures.
