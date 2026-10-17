# fluidnet

Fast, modular simulation of fluid systems: steady state and transients of feed
lines, pressurant systems and (next) engine cycles. Lumped-parameter networks in
the style of EcosimPro / GFSSP, real-fluid properties from CoolProp, a Rust core
that runs natively and as WebAssembly, and a schematic editor in the browser.

```
fluids/          CoolProp-generated property tables (H2, O2, CH4, N2, He, H2O, N2O, ethanol)
crates/
  fluidnet-core  model, components, equations, solvers, properties (pure Rust)
  fluidnet-cli   `fluidnet` command-line runner
  fluidnet-wasm  C-ABI WebAssembly bindings used by the UI
ui/              React + React Flow schematic editor, solver in a web worker
examples/        example systems (YAML), also used as regression tests
tests_models/    validation cases
tools/           property-table generator (Python + CoolProp)
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

Regenerating fluid tables (adds fluids or changes ranges):

```sh
pip install CoolProp numpy
python tools/gen_fluid_tables.py fluids            # all fluids, or name keys: ... fluids H2 O2
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
start the transient from it).

## Components (v0.1)

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

Every node exposes `P`, `T`, `rho`, `h`, `x`; every flow element `mdot`, `dp` and,
where relevant, `choked`, `opening`, `Q`, `u_in`. Nodes where two-port elements
meet are named after a port, e.g. `main_valve.in`; pipe internals are
`pipe.n1 … pipe.n(N-1)` (volumes) and `pipe.s1 … pipe.sN` (segments).

## Performance

Typical feed-system models (20–60 states) run a full transient in 5–50 ms
natively; the browser engine is about 2× slower. A 150-state water-hammer case
runs in 0.4 s; 600 states in about 3–6 s at rtol 1e-3–1e-4, where the step count
is set by resolving a sharp pressure front through 200 cells.

## Status and roadmap

Validated (see docs/VALIDATION.md): adiabatic gas blowdown, Bernoulli and
cavitating flow, Darcy friction, Joukowsky surge and 4L/a period, HEM flashing,
energy balance in heated lines, regulator droop.

Next, in order of what the expander-cycle work needs:

1. Rotating machinery: pump (head/efficiency maps with speed), turbine (velocity
   ratio / efficiency maps, partial admission), shaft with inertia and torque balance.
2. Wall thermal nodes with heat capacity and conduction, so cooling-jacket
   transients and chill-down are captured; regen channel component with
   hot-gas-side heat flux input; two-phase boiling correlations.
3. Combustion chamber (CEA-tabulated c* and gamma vs O/F and Pc) and nozzle,
   closing the expander cycle balance.
4. Design-mode steady state: solve for chosen parameters (orifice areas, pump
   speed) to meet targets (chamber pressure, mixture ratio).
5. Python bindings (PyO3) for scripting sweeps from your existing toolchain, and a
   Tauri desktop build using the native core.
6. Tank models with liquid/ullage stratification and pressurant collapse.

Known limits: one fluid per connected circuit (no mixing of species); homogeneous
equilibrium everywhere (no thermal non-equilibrium or slip); pipe momentum
neglects the convective acceleration term (Fanno choking in long gas lines is not
captured); property accuracy degrades within ~10 % of the triple-point pressure.
