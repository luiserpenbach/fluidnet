# Installation and first run

FNT-USR-HBK-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Purpose

This handbook takes a new user from a fresh checkout of the repository to a
first simulation with each of the three front ends: the command-line tool, the
Python package and the web user interface. It also states where fluidnet looks
for its fluid property tables, because a missing table directory is the most
common first-run failure.

All commands are given for a POSIX shell (Linux, macOS, WSL). Where a command
differs on Windows, the PowerShell equivalent follows in
[Windows notes](#windows-notes).

## Prerequisites

| Tool | Needed for | Version | Notes |
|---|---|---|---|
| Rust toolchain (`rustup`, `cargo`) | everything (the solver is Rust) | stable; this page was checked with cargo 1.97.0 | The workspace uses edition 2021 and declares no minimum Rust version. |
| Python | Python package, table generators, validation scripts | 3.8 or newer (`requires-python = ">=3.8"` in `pyproject.toml`) | `numpy` is required; `matplotlib` and `pandas` are optional (plotting, data frames). |
| maturin | building the Python wheel | 1.5 or newer, below 2 | `pip install maturin` |
| Node.js and npm | web user interface | Node 18 or newer | Only needed to build the UI. |
| `wasm32-unknown-unknown` Rust target | web user interface | – | `rustup target add wasm32-unknown-unknown`; a fallback for systems where the target cannot be installed is in [FNT-DEV-HBK-001](../05-development/build-and-release.md). |

The fluid property tables (`fluids/*.fntb`, about 1.4 MB each) and the
combustion tables (`crates/fluidnet-core/data/combustion.json`, compiled into
the binary) are part of the repository. Python with CoolProp and Cantera is
needed only to regenerate them (see
[FNT-DEV-HBK-001](../05-development/build-and-release.md)).

## Steps

### 1. Build the command-line tool

```sh
cargo build --release
```

This builds the default workspace members, `fluidnet-core` and `fluidnet-cli`,
and writes the binary to `target/release/fluidnet` (`fluidnet.exe` on Windows).
The release profile uses link-time optimisation with one code-generation unit,
so the first build takes noticeably longer than a debug build; later builds of
unchanged crates are immediate. Check the binary:

```sh
./target/release/fluidnet catalog > catalog.json
```

`catalog` writes the component catalog as JSON (15 component types) and needs
no fluid tables, so it confirms that the binary runs independently of the table
search below. Redirect console output to a file rather than piping it into
`head`: when the reading end of a pipe closes early, the tool stops with a
`failed printing to stdout: Broken pipe` panic message.

### 2. Make sure the fluid tables are found

Every command except `catalog` loads the fluid tables. The command-line tool
searches in this order (`find_fluids` in `crates/fluidnet-cli/src/main.rs`):

1. the directory given with `--fluids DIR`;
2. the directory in the environment variable `FLUIDNET_FLUIDS`;
3. `fluids/` in the current working directory;
4. `fluids/` next to the executable and in up to three parent directories of
   the executable's directory (for `target/release/fluidnet` this reaches the
   repository root, so the binary finds `fluids/` from any working directory).

Candidates 3 and 4 are accepted only if they contain `N2.fntb`. An explicit
directory (1 or 2) is used as given; if it does not exist the run fails with
`fluid error: DIR: No such file or directory`. If no candidate is found the
tool prints `cannot find fluid tables; pass --fluids DIR or set FLUIDNET_FLUIDS`
and exits with code 2.

The Python package searches `FLUIDNET_FLUIDS`, then the `fluids/` directory
inside the installed package, then `./fluids`, then the `fluids/` directory of
the source tree it was built from (`fluidnet.fluids_dir()` in
`python/fluidnet/__init__.py`). The web user interface carries its own copy of
the tables (see step 5).

Eight fluids are shipped: H2 (parahydrogen), O2, CH4, N2, He, H2O, N2O and
C2H5OH (ethanol). Their table ranges are listed in the
[model file standard](model-files.md#26-fluids).

### 3. Run a first example

```sh
./target/release/fluidnet run examples/lox_feed_startup.yaml --out lox.csv
```

Output with v0.5.0 (the solve time varies with the machine):

```
LOX feed startup [transient]: 38 states, solved in 170.4 ms
  steps 1813 (rejected 37), rhs evals 8268, jacobians 68 (10 colors), LU 329 (dense, nnz 252), h in [8.61e-9, 7.50e-2] s
  lox_tank.P                   25.0000 bar
  lox_tank.T                   90.00 K
  main_valve.in.P              24.8017 bar
  main_valve.in.T              90.01 K
  manifold.P                   24.5807 bar
  manifold.T                   90.01 K
  chamber.P                    15.0000 bar
  chamber.T                    300.00 K
  main_valve.mdot              1.45000 kg/s
  injector.mdot                1.45000 kg/s
  wrote lox.csv
```

The model is a 3 m, 20 mm LOX line from a 25 bar, 90 K supply through a
quick-opening valve and a 7.5 mm injector orifice into a chamber whose pressure
rises from 1 bar to 15 bar. The console lists the final values of the pressure,
temperature and mass-flow channels; `lox.csv` holds all 98 channels at the
751 output times (0 s to 1.5 s every 2 ms). The
[command-line handbook](cli.md) explains every line, and
[outputs](outputs.md) explains the channel names and the file formats.

To solve the steady state instead of the transient:

```sh
./target/release/fluidnet run examples/expander_cycle.yaml --steady --print tca.
```

This prints the chamber channels of the 20 kN closed expander example
(chamber pressure 39.96 bar, thrust 19,979 N, vacuum specific impulse 457.2 s
with v0.5.0).

### 4. Build and install the Python package

The package is a Rust extension (`fluidnet._core`, built with PyO3) plus a pure
Python layer (`python/fluidnet/__init__.py`). maturin builds both into one
wheel. The fluid tables are copied into the package first so the wheel is
self-contained (`pyproject.toml` includes `python/fluidnet/fluids/*.fntb`):

```sh
python -m venv .venv && . .venv/bin/activate
pip install maturin numpy
mkdir -p python/fluidnet/fluids && cp fluids/*.fntb python/fluidnet/fluids/
maturin develop --release          # builds and installs into the active environment
```

`maturin develop` needs an active virtual environment (or conda environment).
To produce a wheel file instead, for installation elsewhere:

```sh
maturin build --release            # writes target/wheels/fluidnet-0.5.0-*.whl
pip install target/wheels/fluidnet-0.5.0-*.whl
```

The extension is built against the stable Python ABI for Python 3.8
(`abi3-py38` in `crates/fluidnet-py/Cargo.toml`), so one wheel serves every
Python version from 3.8 on, for the platform it was built on. Check the
installation:

```sh
python -c "import fluidnet as fn; print(fn.__version__, fn.fluids_dir())"
```

which prints `0.5.0` and the table directory inside the installed package.
Optional extras: `pip install matplotlib pandas` for `Results.plot` and
`Results.to_dataframe`. The [Python handbook](python.md) continues from here.

`python/fluidnet/fluids/` is listed in `.gitignore`; the copied tables are
build input, not source.

### 5. Build the web user interface

```sh
rustup target add wasm32-unknown-unknown
cargo build --release -p fluidnet-wasm --target wasm32-unknown-unknown
cd ui
npm install
npm run wasm        # copies the engine and the fluid tables into ui/public
npm run dev         # development server on http://localhost:5173
```

`npm run wasm` (`ui/scripts/copy-assets.mjs`) copies
`target/wasm32-unknown-unknown/release/fluidnet_wasm.wasm` to
`ui/public/fluidnet.wasm` and writes each fluid table as gzip-compressed base64
text to `ui/public/fluids/<FLUID>.txt`. For a static build:

```sh
npm run build       # type check, then writes the static site to ui/dist
```

`ui/dist` contains `index.html`, `app.js`, `app.css`, `fluidnet.wasm` and
`fluids/`; serve it with any static web server, for example
`python -m http.server -d ui/dist 8000` and then http://localhost:8000. The
page loads the engine and the tables with relative URLs, so it can be hosted in
any directory. The [web UI handbook](web-ui.md) describes its use and the
single-file artifact build.

If `rustup target add` is not possible (for example a toolchain without a
prebuilt standard library for the target), use the `-Zbuild-std` fallback in
[FNT-DEV-HBK-001](../05-development/build-and-release.md#4-webassembly-engine).

## Windows notes

From v0.5.1 fluidnet is built and tested on Windows in CI (MSVC target), and
each CI run on GitHub provides a Windows wheel as a run artifact
(`wheel-windows-latest`): download it from the run page under Actions and
install it with `pip install fluidnet-<version>-cp38-abi3-win_amd64.whl`. To
build from source, use the standard Rust toolchain for the MSVC target, which
requires the Microsoft C++ build tools (installed with Visual Studio or the
separate Build Tools installer, as `rustup` prompts).

| Task | PowerShell |
|---|---|
| Build the CLI | `cargo build --release` |
| Run an example | `.\target\release\fluidnet.exe run examples\lox_feed_startup.yaml --out lox.csv` |
| Set the table directory for this session | `$env:FLUIDNET_FLUIDS = "C:\path\to\fluidnet\fluids"` |
| Set it permanently for the user | `setx FLUIDNET_FLUIDS "C:\path\to\fluidnet\fluids"` (applies to new terminals) |
| Create and activate a virtual environment | `python -m venv .venv` then `.\.venv\Scripts\Activate.ps1` |
| Copy the tables into the package | `New-Item -ItemType Directory -Force python\fluidnet\fluids \| Out-Null` then `Copy-Item fluids\*.fntb python\fluidnet\fluids\` |
| Build and install the package | `maturin develop --release` |
| Build a wheel and install it | `maturin build --release` then `pip install (Get-Item target\wheels\fluidnet-0.5.0-*.whl).FullName` |
| WebAssembly fallback build | `$env:RUSTC_BOOTSTRAP = "1"; cargo build --release --target wasm32-unknown-unknown -p fluidnet-wasm -Zbuild-std=std,panic_abort; Remove-Item Env:RUSTC_BOOTSTRAP` |
| Serve the built UI | `python -m http.server -d ui\dist 8000` |

If PowerShell refuses to run `Activate.ps1`, allow local scripts for the
current user with `Set-ExecutionPolicy -Scope CurrentUser RemoteSigned`.

The npm scripts in `ui/scripts/` derive their own directory from
`new URL(import.meta.url).pathname`, which on Windows has the form
`/C:/...`. They have not been verified on Windows. If `npm run wasm` or
`npm run artifact` fails with a malformed path, run the UI build from WSL; the
built site itself is platform-independent.

Model files are plain text and identical on all platforms. Paths inside model
files do not occur; paths on the command line may use either separator.

## Troubleshooting

| Symptom | Cause and remedy |
|---|---|
| `cannot find fluid tables; pass --fluids DIR or set FLUIDNET_FLUIDS` (exit code 2) | The binary was copied away from the repository and no `fluids/` directory is in the working directory. Pass `--fluids` or set `FLUIDNET_FLUIDS`. |
| `fluid error: DIR: No such file or directory` | `--fluids` or `FLUIDNET_FLUIDS` points to a directory that does not exist. |
| `model error: fluid 'Xe' not loaded (available: ...)` | The model names a fluid without a table. Use one of the listed keys, or generate a table ([FNT-DEV-HBK-001](../05-development/build-and-release.md)). |
| `FileNotFoundError: fluid tables not found; set FLUIDNET_FLUIDS ...` in Python | The package was built without copying the tables into `python/fluidnet/fluids/`, and it is used outside the source tree. Copy the tables and rebuild, or set `FLUIDNET_FLUIDS`. |
| `ValueError: no fluid tables (*.fntb) in DIR` in Python | The directory exists but holds no tables. |
| `ModuleNotFoundError: No module named 'fluidnet._core'` | The pure Python files are importable (for example from the source tree) but the extension was not built into this environment. Run `maturin develop --release` in the active environment. |
| `maturin develop` reports that it cannot find a virtual environment | Activate a virtual environment first, or use `maturin build` and `pip install` the wheel. |
| `error[E0463]: can't find crate for 'core'` when building `fluidnet-wasm` | The `wasm32-unknown-unknown` target is not installed. Run `rustup target add wasm32-unknown-unknown` or use the fallback build. |
| The web page shows "Engine failed to load" or `cannot load fluidnet.wasm (404)` | `npm run wasm` was not run after building the engine, or the site is opened without a web server. Copy the assets and serve the directory over HTTP. |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
