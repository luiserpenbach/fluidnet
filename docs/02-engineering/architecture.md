# Software architecture

FNT-ARC-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes how fluidnet v0.5.0 is organised as software: the crates
and packages and what each is responsible for, the path a model takes from a
YAML file to a set of result channels, the component catalog, unit handling,
error handling, how results are passed to the front ends, and the external
dependencies. The physical models and the numerical methods are described in
the other pages of this section; this page names them only where they define
an interface.

## Model and equations

### Crates and packages

The repository is a Cargo workspace (`Cargo.toml`, version 0.5.0, edition
2021) with four crates, plus a Python package and a browser front end.

| Part | Path | Role |
|---|---|---|
| `fluidnet-core` | `crates/fluidnet-core` | Pure Rust library: model schema, component catalog, units, fluid property tables, compiler from model to network, governing equations, correlations, solvers and results. All other parts depend on it. |
| `fluidnet-cli` | `crates/fluidnet-cli` | Binary `fluidnet`: `run` (transient or `--steady`, output to CSV or JSON), `check` (compile and list nodes, branches and states), `catalog` (component catalog as JSON), `props` (property lookup at P and T). Diagnostic commands `bench`, `jaccheck` and `propsbatch` are also present. |
| `fluidnet-wasm` | `crates/fluidnet-wasm` | WebAssembly build of the core with a plain C ABI (no wasm-bindgen): `fn_alloc`, `fn_free`, `fn_load_fluid`, `fn_fluids`, `fn_catalog`, `fn_check`, `fn_run`, and `fn_out_json_*` / `fn_out_data_*` to read results. It imports one function, `env.fn_progress(t) -> i32`. |
| `fluidnet-py` | `crates/fluidnet-py` | PyO3 extension module `fluidnet._core`: `run`, `check`, `parse_model`, `to_yaml`, `catalog_json`, `props`. Built with maturin (`pyproject.toml`). |
| `python/fluidnet` | `python/fluidnet/__init__.py` | Pure Python layer over `_core`: `run`, `load`, `save`, `get_param`, `set_param`, `sweep` (thread pool), `props`, `catalog`, `check`, and the `Results` class (numpy arrays, `last`, `find`, `to_dataframe`, `plot`). The wheel includes the fluid tables in `python/fluidnet/fluids/`. |
| `ui/` | `ui/src` | React schematic editor (React Flow), inspector, uPlot result charts. The engine runs in an inline web worker (`ui/src/worker.ts`) that instantiates `fluidnet.wasm`; `ui/src/engine.ts` is the main-thread client. |

The fluid property tables (`fluids/*.fntb`) are generated offline by
`tools/gen_fluid_tables.py` and loaded at run time. The combustion tables
(`crates/fluidnet-core/data/combustion.json`) are generated offline by
`tools/gen_combustion_tables.py` and compiled into the core with
`include_str!`; they are parsed once on first use (`combustion::tables`,
`OnceLock`).

### Modules of `fluidnet-core`

| Module | File | Content |
|---|---|---|
| `model` | `src/model.rs` | Serde schema of a model file: `Model` (name, description, components, connections, simulation), `ComponentDef` (id, type, free parameter map), `Connection` (`[a, b]` or `{from, to}`), `SimSettings`. `Model::parse` accepts YAML or JSON (JSON if the text starts with `{`). |
| `catalog` | `src/catalog.rs` | Component specifications (`CompSpec`, `ParamSpec`) and typed parameter access (`Params`). |
| `units` | `src/units.rs` | Unit parsing (`parse_str`, `parse_value`), dimensions (`Dim`) and time schedules (`Schedule`). |
| `props` | `src/props.rs` | Fluid table reader and lookup (`FluidTable`, `FluidLibrary`, `Props`). See [FNT-FLD-DOC-001](fluid-properties.md). |
| `network` | `src/network/mod.rs`, `src/network/compile.rs` | Compiled network (`Network`), compiler (`Network::compile`), evaluation (`Network::evaluate`), right-hand side (`Network::rhs`), sparsity, output channels. See [FNT-SLV-DOC-001](network-formulation.md). |
| `flow` | `src/flow.rs` | Restriction mass flux, Churchill friction, Gnielinski Nusselt number. See [FNT-CMP-DOC-001](flow-elements.md). |
| `machines` | `src/machines.rs` | Pump and turbine maps, wall materials. See [FNT-TRB-DOC-001](turbomachinery.md) and [FNT-HTX-DOC-001](heat-transfer.md). |
| `combustion` | `src/combustion.rs` | Combustion tables, nozzle relations, Bartz coefficient. See [FNT-CMB-DOC-001](combustion.md). |
| `solver` | `src/solver/` | `transient.rs` (TR-BDF2), `steady.rs`, `jacobian.rs`, `linsys.rs`, `linalg.rs`. See [FNT-SLV-DOC-002](solvers.md). |
| `sim` | `src/sim.rs` | Entry points `run_steady`, `run_transient`, `run_model` and the `Results` structure. |
| `error` | `src/error.rs` | `Error` and `Result`. |

### Data flow

A run passes through the same stages in every front end:

```
model text (YAML or JSON)
  → Model::parse                         model.rs        serde schema, no validation of parameters
  → Network::compile(&Model, &FluidLibrary)
                                         network/compile.rs
        parameters read through catalog::Params (defaults, units)
        ports grouped into nodes, branches created, pipes discretised
        fluids assigned per connected circuit, combustion tables attached
        initial (P, h) propagated, state vector laid out, output channels built
  → Network (nodes, branches, shafts, walls, chambers, actuators, y0, breakpoints, channels)
  → Network::evaluate / Network::rhs     network/mod.rs  dy/dt = f(t, y)
  → solver::transient::integrate         solver/transient.rs
    or solver::steady::solve             solver/steady.rs
  → Recorder (sim.rs): at each output time, evaluate(t, y) and probe every channel
  → Results (sim.rs)
  → front end: CSV / JSON (CLI), metadata JSON + f64 block (Python, WebAssembly)
```

`Network::compile` does all model checking. It rejects, with a message naming
the component, unknown component types, duplicate or empty ids, connections to
unknown components or ports, unconnected ports of flow elements, two node-like
components (`boundary`, `tank`, `junction`) connected directly, an element
whose inlet and outlet meet at the same node, fluid conflicts within one
connected circuit, missing required parameters, parameters out of range, and
volume nodes for which no initial state can be found. Unknown parameter keys
are not errors; they are collected in `Network::warnings` (the keys `ui`,
`label`, `notes`, `x`, `y` and `position` are ignored silently).

The compiled `Network` is immutable during a run. The steady-state solver
clones it to change two flags (`latch_active` and `Shaft::hold`); see
[FNT-SLV-DOC-002](solvers.md). `Network` derives `Clone`, and fluid tables and
combustion tables are shared through `Arc`, so a clone does not copy table
data.

`sim::run_transient` builds the output time list from `simulation.output_dt`
(default `t_end / 500`, never smaller than `t_end / 200000`), optionally solves
the steady state at t = 0 s first (`start_from_steady`), and calls
`transient::integrate` with a sink that records every output time.
`sim::run_steady` solves the steady state at `simulation.steady_time` and
records one time point.

### Component catalog

`catalog.rs` is the single source of truth for component types. `specs()`
returns one `CompSpec` per type with its ports, whether it is node-like, a help
text and a list of `ParamSpec` entries. Each parameter has a key, a kind
(`Quantity`, `Fluid`, `Choice`, `Int`, `Bool`, `Ref`, `Text`), a dimension
(`Dim`), a default as it would be written in a model file (or none, meaning
required or optional), flags for optional and schedulable, choices, a UI
display unit and, for references, the target component type.

The compiler reads every parameter through `Params` (`Params::q`, `q_opt`,
`sched`, `sched_opt`, `str_opt`, `int`, `cda`, `area`). A missing value takes
the catalog default, so defaults exist in one place only. `Params::pspec`
panics if the compiler asks for a key that the catalog does not define, which
keeps the two consistent during development.

`catalog_json()` exports the same data as JSON. The CLI prints it
(`fluidnet catalog`), the Python package exposes it (`fluidnet.catalog()`), and
the web UI builds its palette and inspector from it (`fn_catalog`). The
component parameter reference page is generated from the same data; see
[Component reference](../03-user-guide/component-reference.md).

### Units and schedules

`units.rs` converts every quantity to SI when the model is compiled; the
solver works in SI throughout (Pa, K, J/kg, kg/s, rad/s, m, m², m³, W, N m).

- A parameter may be a bare number, which is taken as SI, or a string of a
  number followed by a unit, such as `"300 bar"`, `"-12.5degC"` or `"6 mm"`.
  `parse_str` splits the string at the first character that cannot belong to
  the number and looks the unit up in `unit_info`, which returns a dimension,
  a factor and an offset (SI = value × factor + offset; offsets are used for
  °C and °F).
- The unit's dimension must match the parameter's `Dim`; a mismatch is a
  parse error. Parameters with `Dim::None` accept any known unit; `%`
  converts to a fraction (factor 0.01).
- Recognised units are Pa, kPa, MPa, bar, bara, mbar, psi, psia, atm; K,
  degC, degF, R; m, cm, mm, um, in, ft; m2, cm2, mm2, in2; m3, L, mL, cm3,
  mm3, in3; kg/s, g/s, kg/h, kg/min, lb/s; s, ms, us, min, h; W, kW, MW;
  W/m2, kW/m2, MW/m2; rad/s, rpm, krpm, Hz; kg m2, g cm2; N m; kg/m3, g/cm3;
  W/m2/K, kW/m2/K; and `%`. The full alias list is in `unit_info`.
- A schedulable parameter may also be a table `[[t, value], ...]`, interpolated
  linearly and held constant before the first and after the last point, or
  an object `{table: [...], interp: step}` for a piecewise-constant table.
  Times accept time units. A table with one row is a constant. Times must be
  non-decreasing. `Schedule::breakpoints` returns the table times; the
  compiler collects the breakpoints of all schedules so the transient solver
  can step onto them (see [FNT-SLV-DOC-002](solvers.md)).

Results are always reported in SI; every output channel carries its SI unit
string.

### Error handling

`error.rs` defines one error type with four variants: `Model` (model
definition), `Fluid` (property table), `Solver` (step-size underflow, maximum
number of steps, singular iteration matrix, steady state not converged) and
`Parse` (YAML, JSON, units). Each front end maps it as follows:

- CLI: the message is printed to standard error and the exit code is 1.
- Python: model parse errors and fluid-library errors raise `ValueError`; in
  `run`, compile and solver errors raise `RuntimeError` (in `check`, compile
  errors raise `ValueError`).
- WebAssembly: the function returns −1 and the output JSON is
  `{"error": message}`.

A transient that fails after at least one output time has been recorded does
not return an error: `run_transient` returns the results up to the failure
with `Results::error` set to the message. Steady-state failures are always
errors. A user stop (the progress callback returning false) is reported as a
solver error with the message "stopped by user"; it also keeps the data
recorded so far.

### Result transport

`sim::Results` holds the run name and mode, the time vector, the channels
(`name`, SI `unit`, `data`), warnings, transient statistics (`Stats`), steady
statistics (`SteadyStats`), node and component summaries, the number of
states, the error message, the state names and the last recorded state vector.

To avoid encoding large time histories as JSON numbers, the Python and
WebAssembly bindings split a result into two parts:

- a JSON metadata object with `name`, `mode`, `n_time`, the channel list
  (`name`, `unit`), `warnings`, `stats`, `steady`, `error`, the component
  summaries and `n_states` (WebAssembly adds `nodes`; Python adds
  `elapsed_s`, `state_names` and `final_state`);
- one block of `(n_channels + 1) × n_time` float64 values: the time vector
  followed by each channel in the order of the channel list.

In WebAssembly the block stays in module memory (`fn_out_data_ptr`,
`fn_out_data_len`); the worker copies it into a `Float64Array` and transfers
it to the main thread. In Python it is returned as little-endian bytes and
reshaped by `numpy.frombuffer` into one array per channel. The CLI writes
either CSV (`time [s]` and `name [unit]` headers, values in scientific notation
with nine decimals, empty cells for non-finite values) or the full `Results` as JSON.

The Python `Results.state` dictionary (state name → final value) can be
passed as `init=` to a later run of the same or an edited model;
`Network::apply_init` overwrites the initial state vector by state name. This
warm start is available only through the Python binding.

### Front-end behaviour that affects the core

- Python releases the GIL for the whole compile and run (`py.allow_threads`),
  so `fluidnet.sweep` runs models in parallel threads. Fluid libraries are
  loaded once per directory and cached (`LIBS` in `fluidnet-py/src/lib.rs`).
- The web worker's `fn_progress` callback always returns 1; the UI stops a run
  by terminating the worker and starting a new one (`Engine::stop`). Progress
  messages are posted at most every 60 ms.
- The UI ships the fluid tables as gzip-compressed, base64-encoded text
  (`ui/public/fluids/*.txt`, written by `ui/scripts/copy-assets.mjs`) and
  decodes them with `DecompressionStream`, so the built site is static.

## Numerical treatment

The numerical methods (network formulation, TR-BDF2, Jacobian, linear
algebra, steady-state sequence) are described in
[FNT-SLV-DOC-001](network-formulation.md) and [FNT-SLV-DOC-002](solvers.md).
Two points matter at the architecture level:

- The right-hand side is evaluated into a reusable workspace (`Eval`), which is
  allocated once per solver context, so the inner loops do not allocate.
- The release profile uses `opt-level = 3`, `lto = true` and
  `codegen-units = 1` (workspace `Cargo.toml`).

Performance figures stated in `README.md` for v0.5.0:

| Case | Figure |
|---|---|
| Typical feed-system models, 20–60 states, full transient, native | 5–50 ms |
| Same, browser (WebAssembly) | about 2 × slower than native |
| Expander cycle steady state (`examples/expander_cycle.yaml`) | 0.25 s |
| Expander 6 s throttling transient | 0.2 s |
| Expander 10 s start (`examples/expander_start.yaml`) | about 6 s |
| Water hammer, 150 states | 0.4 s |
| Water hammer, 600 states, rtol 1×10⁻³–1×10⁻⁴ | about 3–6 s |

These figures are not tied to a results file; see open items.

## Parameters and outputs

The `simulation` block of a model (`SimSettings` in `model.rs`):

| Key | Default | Meaning |
|---|---|---|
| `t_end` | 1 s | End time of a transient |
| `output_dt` | `t_end / 500` | Output interval (lower limit `t_end / 200000`) |
| `rtol` | 1×10⁻⁴ | Relative tolerance of the transient integrator |
| `atol_scale` | 1 | Multiplier on the absolute tolerances |
| `max_step` | `t_end / 20` | Largest step of the transient integrator |
| `max_steps` | 2 000 000 | Limit on accepted plus rejected steps |
| `start_from_steady` | false | Solve the steady state with inputs at t = 0 s and start the transient from it |
| `steady_time` | 0 s | Time at which schedules are evaluated for a steady-state run |

Output channel names follow `<node or element>.<quantity>`. The channels each
component type produces are listed in the
[Component reference](../03-user-guide/component-reference.md) and in the
model pages of this section.

## Limits and assumptions

- One model is compiled into one network; there is no incremental
  recompilation. A parameter change means a new compile, which is cheap
  compared with a run.
- The WebAssembly build is single-threaded and keeps its fluid library and
  output buffers in thread-local globals; one worker runs one model at a time.
- The warm start (`init=`) matches states by name. States that do not exist in
  the new model are ignored silently; the number of matches is returned by
  `apply_init` but not reported to the user.
- The CLI loads all `*.fntb` files of one directory: `--fluids DIR`, the
  environment variable `FLUIDNET_FLUIDS`, or `fluids/` in the working
  directory or next to the binary.

## Verification

- `all_examples_run_steady_and_transient` in
  `crates/fluidnet-core/tests/validation.rs` compiles and runs every model in
  `examples/` as a transient and (except `regulated_pressurization.yaml`,
  which has no steady state) as a steady state, which exercises the full data
  flow above.
- `units` in `src/units.rs` checks unit parsing, including offsets,
  exponents, dimension mismatch and rotational speed.
- The model-level verification cases are described in
  [Analytic verification cases](../04-verification/analytic-cases.md) and
  [RL10A-3-3A validation](../04-verification/rl10.md).

## Code location

| Topic | Location |
|---|---|
| Model schema | `crates/fluidnet-core/src/model.rs` |
| Catalog and parameter access | `crates/fluidnet-core/src/catalog.rs`: `specs`, `catalog_json`, `Params` |
| Units and schedules | `crates/fluidnet-core/src/units.rs`: `unit_info`, `parse_str`, `Schedule` |
| Compiler | `crates/fluidnet-core/src/network/compile.rs`: `Network::compile`, `build_channels` |
| Run entry points and results | `crates/fluidnet-core/src/sim.rs`: `run_steady`, `run_transient`, `run_model`, `Recorder`, `Results` |
| Errors | `crates/fluidnet-core/src/error.rs` |
| CLI | `crates/fluidnet-cli/src/main.rs` |
| WebAssembly C ABI | `crates/fluidnet-wasm/src/lib.rs`: `fn_run`, `pack` |
| Python binding | `crates/fluidnet-py/src/lib.rs`: `run`; `python/fluidnet/__init__.py` |
| UI engine | `ui/src/engine.ts`, `ui/src/worker.ts` |

### Dependencies

| Crate or package | Dependency | Version | Use |
|---|---|---|---|
| fluidnet-core | serde (derive) | 1 | Model schema, results serialisation |
| fluidnet-core | serde_json | 1 | JSON models, catalog, results, table headers |
| fluidnet-core | serde_yaml | 0.9 | YAML models |
| fluidnet-core | faer (std, sparse-linalg) | 0.24 | Sparse LU factorisation |
| fluidnet-cli, -wasm, -py | serde_json | 1 | Output metadata |
| fluidnet-py | pyo3 (extension-module, abi3-py38) | 0.22 | Python extension |
| python/fluidnet | numpy; optional matplotlib, pandas | – | Arrays, plotting, data frames |
| Build | maturin | ≥ 1.5, < 2 | Python wheel |
| ui | react, react-dom 18; @xyflow/react 12; uplot 1.6; js-yaml 4 | see `ui/package.json` | Schematic editor, charts, YAML I/O |
| tools | CoolProp, numpy; Cantera, scipy | – | Offline table generation only |

The core has no other run-time dependencies; in particular no CoolProp or
Cantera at run time.

## Open items

- The performance figures quoted from `README.md` have no recorded run
  (machine, commit, command). `fluidnet bench MODEL` prints right-hand-side,
  Jacobian, factorisation and property-lookup times and could produce a
  traceable record.
- The diagnostic CLI commands `bench`, `jaccheck` and `propsbatch` are not
  listed in the CLI usage text and are not documented for users.
- The warm start reports neither how many state names matched nor which were
  ignored.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
