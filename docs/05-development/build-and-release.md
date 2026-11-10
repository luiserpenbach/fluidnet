# Build, test and release

FNT-DEV-HBK-001 · Rev B · Released · 2026-11-10 · Owner: Luis Erpenbach

## Purpose

This handbook describes how to build every part of fluidnet from source, run
the test and validation suites, regenerate the generated data (fluid tables,
combustion tables, component reference), build the documentation site and its
Notion export, and make a release. It is written for developers changing the
code; users installing fluidnet follow
[installation](../03-user-guide/getting-started.md).

## Prerequisites

| Tool | Used for | Notes |
|---|---|---|
| Rust stable with `cargo` | core, CLI, WebAssembly engine, Python extension | This page was checked with cargo 1.97.0 |
| `wasm32-unknown-unknown` target, or the standard library source (`rust-src`) | WebAssembly engine | The source is needed only for the fallback build |
| Python 3.8 or newer, `maturin` 1.5–1.x, `numpy` | Python wheel, generators, validation scripts | |
| `CoolProp` | fluid tables, combustion tables | Tables of v0.5.0 were generated with CoolProp 8.0.0 |
| `cantera`, `scipy` | combustion tables; `scipy` also for `tools/size_expander.py` and `validation/rl10/` | Tables of v0.5.0 were generated with Cantera 3.2.0 |
| Node.js 18 or newer with npm | web interface | This page was checked with Node 22.22.0 and npm 10.9.4 |
| `mkdocs-material` | documentation site | |

## Steps

### 1. Repository layout for building

The Cargo workspace (`Cargo.toml`) has four members; the default members are
`fluidnet-core` and `fluidnet-cli`, so plain `cargo build` and `cargo test`
build only these two. `fluidnet-wasm` is built for the WebAssembly target and
`fluidnet-py` through maturin. The release profile uses `opt-level = 3`,
`lto = true` and `codegen-units = 1`. Module responsibilities are described in
[conventions](conventions.md#22-module-responsibilities) and in
[software architecture](../02-engineering/architecture.md).

### 2. Build the core and the CLI

```sh
cargo build --release          # target/release/fluidnet
```

Use release builds also for development runs of models: the solver is
evaluated millions of times per transient, and an unoptimised build is far
slower.

### 3. Run the tests

```sh
cargo test --release
```

This runs the 14 unit tests in `crates/fluidnet-core/src` (units, linear
algebra, turbomachinery maps, combustion tables) and the 16 validation tests in
`crates/fluidnet-core/tests/validation.rs`. With the build up to date the
suite took 91 s on the two-CPU Linux test machine (2026-11-02), almost all of
it in `all_examples_run_steady_and_transient`, the RL10 tests and the start
transients. What each validation test checks, with its reference and
tolerance, is in [analytic cases](../04-verification/analytic-cases.md) and
[RL10 validation](../04-verification/rl10.md).

Run one test, or the tests whose name contains a string:

```sh
cargo test --release -p fluidnet-core --test validation water_hammer
cargo test --release -p fluidnet-core --lib            # unit tests only
```

The validation tests load the fluid tables from `fluids/` in the repository
and the models from `examples/` and `tests_models/`, so they must be run from
a full checkout. `all_examples_run_steady_and_transient` runs every
`examples/*.yaml` as a transient and (except `regulated_pressurization.yaml`)
as a steady state: a new example file is tested automatically and must
therefore run in both modes or be added to that exception.

The RL10 calibration and start scripts in `validation/rl10/` are not part of
`cargo test`. They rewrite `examples/rl10a33a.yaml` and
`examples/rl10a33a_start.yaml` and their results files; run them when a change
affects the RL10 results, as described in
[RL10 validation](../04-verification/rl10.md), and commit the regenerated
files with the change.

Diagnostics for solver work: `fluidnet bench MODEL` times the solver building
blocks and `fluidnet jaccheck MODEL` compares the coloured Jacobian with a
full finite-difference Jacobian ([command-line tool](../03-user-guide/cli.md#bench-developer)).
`FLUIDNET_DEBUG=1` makes the solvers print their iteration history.

### 4. WebAssembly engine

```sh
rustup target add wasm32-unknown-unknown
cargo build --release -p fluidnet-wasm --target wasm32-unknown-unknown
```

Output: `target/wasm32-unknown-unknown/release/fluidnet_wasm.wasm`
(about 1.3 MB). The crate uses a plain C ABI without wasm-bindgen, so no
further tool is needed.

If the target cannot be installed with `rustup` (for example a toolchain
installed without `rustup`, or no network access to the Rust distribution
server), build the standard library for the target from source:

```sh
RUSTC_BOOTSTRAP=1 cargo build --release --target wasm32-unknown-unknown -p fluidnet-wasm -Zbuild-std=std,panic_abort
```

This needs the standard library source in the toolchain's sysroot
(`lib/rustlib/src/rust/library`, installed by `rustup component add rust-src`).
`RUSTC_BOOTSTRAP=1` enables the unstable `-Zbuild-std` option on a stable
compiler; use it only for this build. The output path is the same. On the test
machine (no `wasm32` target installed) this build took 1 min 18 s
(2026-11-02).

### 5. Web interface

```sh
cd ui
npm install                    # once, and after package.json changes
npm run wasm                   # engine and fluid tables into ui/public
npm run build                  # tsc --noEmit, then vite build into ui/dist
npm run artifact [OUT_DIR]     # single-page build into ui/artifact (or OUT_DIR)
```

`npm run wasm` (`ui/scripts/copy-assets.mjs`) copies the WebAssembly engine
from step 4 and writes every `fluids/*.fntb` as gzip-compressed base64 text to
`ui/public/fluids/<FLUID>.txt`; rerun it after rebuilding the engine or
regenerating tables. `npm run build` type-checks first, so a type error fails
the build. `npm run artifact` (`ui/scripts/make-artifact.mjs`) inlines
`dist/app.js` and `dist/app.css` into `fluidnet.html` and copies the engine and
tables next to it. `ui/dist`, `ui/public` and `ui/artifact` are build output
and are not committed (`.gitignore`). Development server: `npm run dev`.

The example models offered in the interface are imported from `examples/` at
build time (`App.tsx`); rebuild the interface after changing them.

### Reference-result baselines and CI

`python tools/baselines.py check` runs every model in `examples/` and
`tests_models/` steady and transient with the CLI and compares all channels
with the stored results in `validation/baselines/` (relative tolerance 1e-6
for steady states, 1e-4 for transients, on the largest magnitude of each
channel). When a change is meant to alter results, run
`python tools/baselines.py update --reason "..."`; the reason is appended to
`validation/baselines/CHANGELOG.md` and belongs in the commit message.
Baselines are generated and checked on Linux (open item OI-031).

GitHub Actions (`.github/workflows/ci.yml`) runs on every push to `main`:
rustfmt and clippy with warnings as errors; `cargo test --release` on Linux,
Windows and macOS; the baseline check; wheels for the three platforms with an
install-and-run check (downloadable as run artifacts); the WebAssembly and web
interface build; `mkdocs build --strict` and a check that the generated
component reference is up to date. The toolchain is pinned to Rust 1.97.0 in
the workflow; raise it deliberately.

### 6. Python wheel

```sh
mkdir -p python/fluidnet/fluids && cp fluids/*.fntb python/fluidnet/fluids/
maturin build --release        # target/wheels/fluidnet-<version>-cp38-abi3-<platform>.whl
maturin develop --release      # or: build and install into the active virtual environment
```

The tables must be copied first; `pyproject.toml` includes
`python/fluidnet/fluids/*.fntb` in the wheel and the sdist. The extension uses
the stable ABI for Python 3.8 (`abi3-py38`), so one wheel per platform covers
all Python versions from 3.8. Wheels are built on the platform they are for;
CI builds wheels for Linux, Windows and macOS on every push. Check a wheel in a clean virtual
environment with `python -c "import fluidnet as fn; print(fn.__version__, fn.fluids_dir())"`
and one example run.

### 7. Regenerate the fluid tables

```sh
pip install CoolProp numpy
python tools/gen_fluid_tables.py fluids              # all fluids
python tools/gen_fluid_tables.py fluids H2 O2        # selected fluids
```

`tools/gen_fluid_tables.py` writes `fluids/<KEY>.fntb` for the keys of its
`FLUIDS` dictionary (CoolProp name, label, maximum temperature and pressure per
fluid) on a grid of about 143 pressures by 310–320 enthalpies, plus a
saturation table. The file format and the interpolation are described in
[fluid properties](../02-engineering/fluid-properties.md). After regenerating:

1. run `cargo test --release` (the tables are read at run time, no rebuild of
   the Rust code is needed);
2. copy the tables into `python/fluidnet/fluids/` and rebuild the wheel;
3. run `npm run wasm` and rebuild the web interface;
4. record the CoolProp version in the commit message.

To add a fluid: add it to `FLUIDS` in the generator, generate its table, add it
to `FLUIDS` in `ui/src/engine.ts` (and a colour in `FLUID_COLORS` in
`ui/src/model.ts`), and list it in the
[model file standard](../03-user-guide/model-files.md#26-fluids).

### 8. Regenerate the combustion tables

```sh
pip install cantera CoolProp scipy numpy
python tools/gen_combustion_tables.py crates/fluidnet-core/data/combustion.json
python tools/gen_combustion_tables.py --augment crates/fluidnet-core/data/combustion.json   # add dh_wall/cp_wall only
```

The tables are compiled into the core with `include_str!` in
`crates/fluidnet-core/src/combustion.rs`, so every front end must be rebuilt
afterwards (CLI, WebAssembly engine and interface, wheel). The file's `meta`
block records the generator, the Cantera and CoolProp versions and the wall
temperature of the heat transfer potential. Propellant pairs and methods are
described in [combustion](../02-engineering/combustion.md).

### 9. Regenerate the component reference

```sh
python tools/gen_component_reference.py              # writes docs/03-user-guide/component-reference.md
python tools/gen_component_reference.py --cli PATH --out FILE
```

The script reads the catalog with `fluidnet catalog` from
`target/release/fluidnet` (or `fluidnet.exe`, or a `fluidnet` on the `PATH`;
`--cli` names another binary) and falls back to the Python package. Build the
CLI first, so the reference matches the current `catalog.rs`. Run it after
every change to the catalog and commit the regenerated page with the change.
Never edit `component-reference.md` by hand.

### 10. Build the documentation

```sh
pip install mkdocs-material
mkdocs serve                   # http://127.0.0.1:8000, rebuilds on change
mkdocs build --strict          # static site in site/, fails on warnings
```

`mkdocs.yml` in the repository root lists every page in its `nav`; a new page
must be added there, otherwise it is built but not shown in the navigation
(MkDocs reports it only as information). `mkdocs build --strict` fails on
warnings, which include links to pages that do not exist, so run it before
committing documentation changes. Links to headings inside a page are not
checked by default; check them when renaming a heading. The writing rules are in
[Start Here](../00-start-here.md); which page to update for which change is in
[conventions](conventions.md#27-documentation-updates).

### 11. Export the documentation to Notion

```sh
python tools/docs_to_notion.py                 # writes fluidnet-docs-notion.zip in the repository root
python tools/docs_to_notion.py OUT.zip
```

The zip holds one Markdown page per file in Notion's import layout (sections as
parent pages, page files named by their titles, relative links rewritten).
Import it with Notion's Markdown import. The Markdown in `docs/` remains the
source; regenerate the zip after changes rather than editing in Notion.

### 12. Versioning

The version appears in these places, which must agree:

| Place | Field |
|---|---|
| `Cargo.toml` | `[workspace.package] version`; all four crates inherit it (`version.workspace = true`) |
| `pyproject.toml` | `[project] version` |
| `Cargo.lock` | updated by the next `cargo build` after the workspace version changes |

`fluidnet.__version__` in Python is taken from the crate version at build time
(`env!("CARGO_PKG_VERSION")` in `crates/fluidnet-py/src/lib.rs`). The
`version` in `ui/package.json` (0.1.0) is the npm package version of the
interface and is not tied to the release. The documentation states the version
its released pages were checked against in the section status callouts.

Version numbers follow `MAJOR.MINOR.PATCH`. Before 1.0, a minor version marks a
release with new models, components or validated results (see the
[release history](../01-project/release-history.md)); commits between releases
are listed there in brackets.

## Release checklist

1. All intended changes are committed; the working tree is clean.
2. `cargo build --release` and `cargo test --release` pass.
3. If the change affects the RL10 results: `validation/rl10/rl10.py` and
   `start.py` rerun, their results files and the regenerated model files
   committed, and [RL10 validation](../04-verification/rl10.md) updated with a
   new revision.
4. Every example runs (`fluidnet run` and `fluidnet run --steady` as in the
   [example register](../03-user-guide/examples.md)), and changed run times are
   entered in the register.
5. Component reference regenerated (step 9) if `catalog.rs` changed.
6. WebAssembly engine and web interface built (steps 4 and 5); the interface
   opened and one example run in it.
7. Python wheel built (step 6) and checked in a clean virtual environment with
   `examples/python/throttle_map.py`.
8. Version raised in `Cargo.toml` and `pyproject.toml` (step 12).
9. Documentation updated as listed in
   [conventions](conventions.md#27-documentation-updates); status callouts carry
   the new version; `mkdocs build --strict` passes.
10. [Release history](../01-project/release-history.md) entry added with the
    commit, content and evidence; open items closed by the release marked with
    the version in [open items](../01-project/open-items.md).
11. Release commit made with the message `vX.Y.Z: <summary>` (as for 0.5.0)
    and tagged `vX.Y.Z`. Releases up to 0.5.0 are not tagged; tagging starts
    with WP1 of the [platform plan](../01-project/platform-plan.md).

## Troubleshooting

| Symptom | Cause and remedy |
|---|---|
| `error[E0463]: can't find crate for 'core'` building `fluidnet-wasm` | Target not installed; `rustup target add wasm32-unknown-unknown` or the fallback build of step 4. |
| `-Zbuild-std` fails with a missing `library/Cargo.lock` | The standard library source is not installed; `rustup component add rust-src`. |
| `npm run wasm` fails with `ENOENT ... fluidnet_wasm.wasm` | Step 4 not run, or built in debug mode (the script copies the release build). |
| `npm run artifact` fails with `ENOENT ... dist/app.js` | Run `npm run build` first. |
| Validation tests fail with `fluid tables` panic | Run from a full checkout; the tests read `fluids/` relative to the crate. |
| A new example breaks `all_examples_run_steady_and_transient` | The example has no steady state; make it solvable at `steady_time` or add it to the exception in the test, and say why in the commit. |
| `mkdocs build --strict` reports a broken link, or a page is missing from the navigation | Fix the relative link; add the page to `nav` in `mkdocs.yml`. |
| The Python package reports an old version | The extension was not rebuilt after the version change; rerun `maturin develop --release`. |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
| B | 2026-11-10 | v0.5.1: baselines and CI |
