# Code and repository conventions

FNT-DEV-STD-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## 1. Purpose

This standard sets the rules for changing fluidnet: where code and data live,
which module is responsible for what, how errors and units are handled, how
components, channels and states are named, which tests a change needs, which
documentation page a change must update, and how commits are written. It
describes the v0.5.0 code as it is; where the code does not yet meet a rule,
the rule says so.

## 2. Rules

### 2.1 Repository layout

| Path | Content | Generated? |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | Cargo workspace (four crates), release profile | `Cargo.lock` by cargo |
| `crates/fluidnet-core/` | Core library: `src/` (model, catalog, units, properties, network, correlations, machines, combustion, solvers, results), `tests/validation.rs`, `data/combustion.json` | `data/combustion.json` by `tools/gen_combustion_tables.py` |
| `crates/fluidnet-cli/` | `fluidnet` binary | – |
| `crates/fluidnet-wasm/` | WebAssembly engine with C ABI | – |
| `crates/fluidnet-py/` | PyO3 extension `fluidnet._core` | – |
| `python/fluidnet/` | Python package; `fluids/` is a build-time copy of the tables (ignored by Git) | `fluids/` copied |
| `pyproject.toml` | maturin build of the Python package | – |
| `fluids/` | Fluid property tables `*.fntb` | by `tools/gen_fluid_tables.py` |
| `examples/` | Example models (also regression tests and web interface examples); `examples/python/` scripted studies | `rl10a33a.yaml` partly and `rl10a33a_start.yaml` fully by `validation/rl10/` |
| `tests_models/` | Small models with analytic references for `tests/validation.rs` | – |
| `validation/rl10/` | RL10A-3-3A calibration, prediction and start scripts, test data, results files, map digitization (`maps/`) | results files by the scripts |
| `tools/` | Table generators, component reference generator, Notion export, expander sizing script | – |
| `ui/` | Web interface: `src/`, `scripts/`, `index.html`, `package.json`, `vite.config.ts`; `public/`, `dist/`, `artifact/` are build output (ignored) | – |
| `docs/` | Documentation (this tree) | `03-user-guide/component-reference.md` by `tools/gen_component_reference.py` |
| `mkdocs.yml` | Documentation site configuration and navigation | – |
| `site/`, `fluidnet-docs-notion.zip` | Documentation site and Notion export (ignored) | by `mkdocs build` and `tools/docs_to_notion.py` |
| `target/` | Cargo build output (ignored) | – |

Rules:

1. Generated files are regenerated with their tool, never edited by hand,
   and committed together with the change that requires the regeneration.
2. Build output (`target/`, `ui/public/`, `ui/dist/`, `ui/artifact/`,
   `python/fluidnet/fluids/`, the documentation site `site/`, the Notion export
   `fluidnet-docs-notion.zip`, `*.csv`) is not committed; `.gitignore` lists
   it.
3. Model files that a test or a front end depends on stay at their path;
   moving one requires updating the test, `ui/src/App.tsx` (example imports)
   and the [example register](../03-user-guide/examples.md).

### 2.2 Module responsibilities

`fluidnet-core` (`crates/fluidnet-core/src/`):

| Module | Responsibility |
|---|---|
| `model.rs` | Model file schema (`Model`, `ComponentDef`, `Connection`, `SimSettings`), YAML/JSON parsing |
| `units.rs` | Dimensions, unit table, quantity parsing, time schedules (`Schedule`) |
| `catalog.rs` | Component catalog (`CompSpec`, `ParamSpec`): ports, parameters, dimensions, defaults, help; typed parameter access with defaults (`Params`); catalog JSON |
| `network/compile.rs` | `Network::compile`: model to nodes, branches, shafts, walls, chambers, actuators; fluid assignment; initial states; state vector; output channels (`build_channels`) |
| `network/mod.rs` | Network data structures; `evaluate` (all intermediate quantities at (t, y)), `rhs` (time derivatives), `flow_guess`, `weights`, `sparsity`, `probe` (channel values) |
| `flow.rs` | Restriction flux (liquid, two-phase, gas), pipe friction, heat transfer correlations |
| `machines.rs` | Pump and turbine maps, curves, wall materials |
| `combustion.rs` | Combustion tables (compiled in), chamber thermodynamics, nozzle |
| `props.rs` | Fluid property tables: file format, (P, h) lookup, saturation, `FluidLibrary` |
| `solver/transient.rs` | TR-BDF2 integrator |
| `solver/steady.rs` | Steady-state solver (Newton, pseudo-transient continuation, time-marching, settling, shaft continuation) |
| `solver/jacobian.rs`, `linsys.rs`, `linalg.rs` | Coloured finite-difference Jacobian; dense and sparse LU |
| `sim.rs` | Entry points `run_steady`, `run_transient`, `run_model`; `Results` and its CSV/JSON output |
| `error.rs` | `Error` and `Result` |

Front ends contain no physics: `fluidnet-cli`, `fluidnet-wasm`,
`fluidnet-py` and `python/fluidnet` only parse arguments, call the core and
convert results. The web interface builds its palette and inspector from the
catalog JSON and must not duplicate parameter lists (exceptions in v0.5.0:
starting values for new components in `NEW_DEFAULTS` and the fluid list in
`ui/src/engine.ts`). The design of these parts is described in
[software architecture](../02-engineering/architecture.md).

### 2.3 Error handling

1. The core returns `Result<T, Error>` (`error.rs`) with four kinds:
   `Error::Model` (definition problems: missing parameter, bad connection,
   fluid conflict), `Error::Parse` (YAML, JSON, numbers, units),
   `Error::Fluid` (table files and property evaluation) and `Error::Solver`
   (convergence, step size). Their display prefixes (`model error:`,
   `parse error:`, `fluid error:`, `solver error:`) are what users see.
2. Messages name the component id and, where applicable, the parameter, in
   the form `{id}: ...` or `{id}.{key}: ...`, and say what is expected
   (`{id}: full_open_dp must exceed cracking_dp`). The web interface relies on
   the `model error: {id}` prefix to mark the component on the canvas.
3. User input never causes a panic in the core. Panics are reserved for
   violated internal invariants; the one deliberate case is
   `Params::pspec` (`internal: parameter {key} not in catalog`), which fires
   when the compiler reads a parameter that the catalog does not declare.
   Developer commands in the CLI (`jaccheck`, `propsbatch`) may panic on bad
   arguments.
4. Recoverable problems in a model are warnings: strings pushed to
   `Network::warnings` during compilation, passed to every front end. In
   v0.5.0 the only warning is an unknown parameter key.
5. A transient that fails after its first output returns `Ok(Results)` with
   `Results::error` set and the data up to the failure (`run_transient` in
   `sim.rs`). Front ends must show this message.
6. Front ends map errors without changing their text: the CLI prints them and
   exits with code 1 (2 for usage), the Python layer raises `ValueError` for
   parse and input errors and `RuntimeError` for compilation and solver errors
   in `run`, and the WebAssembly engine returns −1 with `{"error": ...}`.

### 2.4 Units

1. All quantities inside the core are SI: Pa (absolute), K, kg/s, m, m², m³,
   J/kg, W, rad/s, N m, kg m², s. There is no unit conversion in equations or
   solver code.
2. Unit strings are parsed once, at compilation, by `Params::q`,
   `Params::sched` and friends with the parameter's `Dim` from the catalog.
   New code reads parameters only through `Params`, never from the raw map.
3. The `unit` field of a `ParamSpec` is a display unit for the interface and
   the reference page; it does not change how a value is read. Bare numbers
   are always SI.
4. Output channels are SI with the unit string from `build_channels`. Front
   ends convert only for display (bar, rpm, kW, kN, kJ/kg) and label the
   converted unit.
5. A new unit is added in `unit_info` in `units.rs` with its dimension,
   factor and offset, covered by the unit test there, and listed in the
   [model file standard](../03-user-guide/model-files.md#27-quantities-and-units).
6. US units appear only in model files and validation data where the source
   is in US units (RL10: psi, lb/s, in, rpm, R), as allowed by
   [Start Here](../00-start-here.md).

### 2.5 Naming

**Component types, ports and parameters** (the model file interface):

1. Type names are lower-case snake_case nouns (`check_valve`,
   `combustion_chamber`). Port names are short lower-case words (`in`, `out`,
   `p`, `fuel`, `ox`); two-port elements use `in` and `out` with positive
   flow from `in` to `out`.
2. Parameter keys are snake_case words (`design_mdot`, `wall_thickness`,
   `hot_htc_factor`), except where an established engineering symbol is the
   clearer name; these keep their usual case (`P`, `T`, `x`, `K`, `Q`, `Cv`,
   `Kv`, `P0`, `T0`, `x0`, `N0`, `T_wall`, `q_wall`).
3. A parameter means the same thing on every component that has it
   (`diameter`, `area`, `cd`, `leak`, `opening`, `fluid`, `P0`/`T0`/`x0`,
   `shaft`/`speed`/`ratio`). Reuse the helpers in `catalog.rs`
   (`area_params`, `init_params`, `drive_params`, `fluid_param`,
   `restriction_model`) instead of redefining them.
4. Renaming a type, port or parameter breaks existing model files. Do it only
   with a release note and a revision of the
   [model file standard](../03-user-guide/model-files.md).

**Channels** (`build_channels`):

5. A channel is `<element>.<quantity>`, where the element is a node, a branch
   or a component. Quantity names are short: lower-case words or established
   symbols (`P`, `T`, `rho`, `h`, `x`, `m`, `mdot`, `dp`, `choked`, `opening`,
   `head`, `eff`, `npsh`, `power`, `torque`, `speed`, `pr`, `u_c0`, `Pc`, `OF`,
   `thrust`, `Isp`, `cstar`, `Tc`, `Q`, `Q_hot`, `Tw_max`).
6. Mass flows are `mdot` (kg/s), pressure differences `dp` (Pa) with the sign
   stated in [outputs](../03-user-guide/outputs.md).
7. Generated element names: `<id>.<port>` for implicit junctions,
   `<pipe>.n<k>` for internal volumes, `<pipe>.s<k>` for segments,
   `<pipe>.w<i>` for wall elements, `<id>.drive` for the implicit shaft of a
   machine with a fixed speed (no channels).
8. Each channel carries its SI unit string.

**States** (`compile.rs`, state vector):

9. State names are `<node>.P`, `<node>.h`, `<branch>.mdot`, `<shaft>.speed`,
   `<pipe>.w<i>.T`, `<chamber>.Pc`, `<chamber>.Z` and `<valve>.position`. A
   new state kind follows the pattern `<element>.<quantity>`.
10. State names are an interface (warm starts in Python, `jaccheck --probe`):
    renaming one is a breaking change.

**Code:**

11. Rust follows the standard style of rustfmt with default settings. The
    v0.5.0 code is not fully formatted (`cargo fmt --check` reports
    differences); format new and changed code, and do not reformat unrelated
    code in the same commit.
12. Physical symbols in Rust code follow the documentation's nomenclature in
    snake_case (`mdot`, `cda`, `rho`, `w_d` for design speed); every function
    that implements a model states its equations or its source in a doc
    comment.
13. Python follows PEP 8 with type hints on public functions. TypeScript in
    `ui/src` is compiled with `strict: false`; keep components typed at their
    props.

### 2.6 Tests required with a change

| Change | Required tests |
|---|---|
| New component type or new physical model | A validation test in `crates/fluidnet-core/tests/validation.rs` against an analytic solution or an independent calculation, with its model in `tests_models/` and the reference value and its source in a comment; `fluidnet jaccheck` on that model shows no sparsity error. |
| New parameter or option of an existing component | A test that exercises it (a test model or an assertion in an existing test). |
| Bug fix | A test that fails before the fix and passes after it. |
| New unit, map or correlation function | A unit test in the module's `#[cfg(test)] mod tests`. |
| Change to solvers or numerics | `cargo test --release` passes; `fluidnet bench` before and after on the expander and RL10 examples if performance is claimed; run times in the [example register](../03-user-guide/examples.md) updated if they change. |
| New example | Runs as transient and as steady state (`all_examples_run_steady_and_transient`), or is added to the test's exception with the reason. |
| Change that moves RL10 results | `validation/rl10/rl10.py` and `start.py` rerun; regenerated files committed; [RL10 validation](../04-verification/rl10.md) revised. |

Every commit leaves `cargo test --release` passing. Tolerances in tests are
not widened to make a change pass without a statement in the commit message
and in the verification page.

In v0.5.0 the `check_valve` and `loss` components are not used by any test or
example; new work on them adds the missing tests.

### 2.7 Documentation updates

A change is complete when the pages that describe the changed behaviour are
updated in the same commit. A change in substance raises the page's revision
letter and adds a row to its revision table ([Start Here](../00-start-here.md)).

| Change | Pages to update |
|---|---|
| Catalog: parameter added, removed, renamed, default or help changed | Regenerate the [component reference](../03-user-guide/component-reference.md); the design page of the physics in [02 Engineering](../02-engineering/index.md); [model file standard](../03-user-guide/model-files.md) if a schedulable or table parameter or an initial-condition rule changed |
| New component type | All of the above; the type table in the model file standard (2.3); [outputs](../03-user-guide/outputs.md) for its channels; [validation matrix](../04-verification/validation-matrix.md) and [analytic cases](../04-verification/analytic-cases.md) for its test; [web interface](../03-user-guide/web-ui.md) if it needs special handling there |
| Channel added or renamed | [Outputs](../03-user-guide/outputs.md); default plot selection and overlays in the interface (`Results.tsx`, `App.tsx`) |
| Topology rule, connection syntax, units, schedules | [Model file standard](../03-user-guide/model-files.md) |
| Simulation setting | Model file standard (2.11); [modelling practice](../03-user-guide/modelling-practice.md); [web interface](../03-user-guide/web-ui.md) if shown in the inspector |
| Solver behaviour | [Solvers](../02-engineering/solvers.md); [modelling practice](../03-user-guide/modelling-practice.md) |
| Results of validation cases change | The analysis pages in [04 Verification and Validation](../04-verification/index.md), new revision with the new run record |
| CLI | [Command-line tool](../03-user-guide/cli.md) |
| Python package | [Python API](../03-user-guide/python.md) |
| Web interface | [Web user interface](../03-user-guide/web-ui.md) |
| Build, dependencies, tools | [Build, test and release](build-and-release.md); [installation](../03-user-guide/getting-started.md) |
| Fluid or combustion tables | [Fluid properties](../02-engineering/fluid-properties.md) or [combustion](../02-engineering/combustion.md); fluid list in the model file standard (2.6) |
| Examples | [Example register](../03-user-guide/examples.md) with measured run times |
| Architecture or module boundaries | [Software architecture](../02-engineering/architecture.md); section 2.2 of this standard |
| A known limit found or closed, a design decision | [Open items](../01-project/open-items.md), [decision log](../01-project/decisions.md) |
| Release | [Release history](../01-project/release-history.md); status callouts of the section indexes |

`README.md` is a short summary for the repository front page. The earlier
`docs/ARCHITECTURE.md` and `docs/VALIDATION.md` are replaced by this tree. When
the README disagrees with a page of the tree, the tree and the code are
correct; correct the README in the same commit.

### 2.8 Commit messages

The history of v0.1 to v0.5.0 uses this form:

1. A summary line in plain English that names what changed, as noun phrases
   separated by commas or semicolons, without a trailing period
   (`Turbomachinery, regenerative walls and combustion chamber; expander-cycle examples`).
2. An optional area prefix when the change is confined to one part
   (`UI: ...`, `RL10: ...`, `RL10 data: ...`), or the version for a release
   commit (`v0.5.0: ...`; older releases put it in brackets at the end).
3. For larger changes, a blank line and a body of `- ` bullets with the
   details, including the numbers that changed (errors, run times) and the
   sources of new data.
4. Commits prepared with an AI assistant end with its trailer lines
   (`Co-Authored-By: ...`), as in the existing history.

One commit holds one logical change with its tests, regenerated files and
documentation.

## 3. Examples

**A change to a default value.** Changing the default `cracking_dp` of
`check_valve` from 0.2 bar to 0.3 bar touches `catalog.rs`; regenerates
`docs/03-user-guide/component-reference.md`; updates the check valve section
of [flow elements](../02-engineering/flow-elements.md) if it quotes the
default; adds a test because none covers the check valve yet; and is committed
as `Check valve: default cracking pressure 0.3 bar` with a body giving the
reason.

**A new output channel.** Adding a pump outlet temperature rise channel
`<pump>.dT` adds one `push` in `build_channels` (unit `K`) and a `Probe`; the
[outputs](../03-user-guide/outputs.md) branch table gains the channel; the
Python and CLI front ends need no change because they list channels from the
results.

**A new component.** Follow [adding a component](adding-a-component.md).

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
