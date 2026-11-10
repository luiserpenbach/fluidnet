# Command-line tool

FNT-USR-HBK-003 · Rev B · Released · 2026-11-10 · Owner: Luis Erpenbach

## Purpose

This handbook describes the `fluidnet` command-line tool: every subcommand,
its options, its console output and its exit codes. The tool runs model files
directly, checks them, prints the component catalog and looks up fluid
properties. It is the reference front end for scripted runs, regression checks
and quick inspection of a model. Its source is
`crates/fluidnet-cli/src/main.rs`.

## Prerequisites

- The binary built as in [installation](getting-started.md):
  `target/release/fluidnet` (`fluidnet.exe` on Windows).
- The fluid tables, found as described in
  [installation, step 2](getting-started.md#2-make-sure-the-fluid-tables-are-found).
- A model file ([model file standard](model-files.md)).

All outputs below were produced with v0.5.0 on Linux; solve times depend on the
machine.

## Synopsis

```
fluidnet run MODEL [--steady] [--out FILE.csv|FILE.json] [--print PATTERNS] [--fluids DIR]
fluidnet check MODEL [--fluids DIR]
fluidnet catalog
fluidnet props FLUID P T [--fluids DIR]     P and T with units ("10 bar" "90 K") or in Pa and K
fluidnet version

developer commands:
fluidnet bench MODEL [--fluids DIR]
fluidnet jaccheck MODEL [--t TIME] [--probe STATE] [--fluids DIR]
fluidnet propsbatch FLUID [--fluids DIR]   < "P h" lines on standard input
```

The subcommand is always the first argument and the model file the second.
Options may follow in any order. `MODEL` is a YAML or JSON file.

## Common rules

| Item | Behaviour |
|---|---|
| Fluid tables | Every subcommand except `catalog` loads all tables from the directory found by the search in [installation](getting-started.md#2-make-sure-the-fluid-tables-are-found); `--fluids DIR` overrides it. |
| Warnings | Compilation warnings are printed to standard error as `warning: <text>` before the run. |
| Errors | Model, parse, fluid and solver errors are printed to standard error as one line starting with `model error:`, `parse error:`, `fluid error:` or `solver error:`. |
| Standard output and error | Run summaries and statistics go to standard error; channel values (`run`), the node list (`check`), the catalog and property values go to standard output. |
| Pipes | Output can be piped into a program that closes early (`| head`); the tool then ends quietly. |

### Exit codes

| Code | Meaning |
|---|---|
| 0 | Success. |
| 1 | The model file cannot be read or parsed, the model does not compile, the steady state does not converge, the transient fails before its first output, the output file cannot be written, or `props` names an unknown fluid. |
| 2 | Usage error (no subcommand, missing arguments, unknown subcommand, unreadable number or unit in `props`, unknown fluid or state name in the developer commands) or no fluid table directory found. The subcommand is checked before anything is read. |
| 3 | A transient stopped early after its first output time (`STOPPED EARLY` line). The results up to the stop are printed and written. |

## Subcommands

### run

```
fluidnet run MODEL [--steady] [--out FILE] [--print PATTERNS]
```

Compiles the model and runs the transient defined by its `simulation` settings
or, with `--steady`, solves the steady state with all schedules evaluated at
`simulation.steady_time`.

| Option | Effect |
|---|---|
| `--steady` | Steady state instead of transient. |
| `--out FILE` | Write all channels to `FILE`: JSON if the name ends in `.json`, CSV otherwise. Formats in [outputs](outputs.md#4-files-written-by-the-command-line-tool). |
| `--print PATTERNS` | Comma-separated substrings; print the final value of every channel whose name contains one of them. Without it, the final values of all channels ending in `.P`, `.T` or `.mdot` are printed, except those whose name contains `.n` or `.s` (this hides pipe internals, and also any other channel with those letter pairs after a dot). |

Console output of a transient:

```
$ fluidnet run examples/lox_feed_startup.yaml --out lox.csv
LOX feed startup [transient]: 38 states, solved in 170.4 ms
  steps 1813 (rejected 37), rhs evals 8268, jacobians 68 (10 colors), LU 329 (dense, nnz 252), h in [8.61e-9, 7.50e-2] s
  lox_tank.P                   25.0000 bar
  lox_tank.T                   90.00 K
  main_valve.in.P              24.8017 bar
  ...
  injector.mdot                1.45000 kg/s
  wrote lox.csv
```

Line 1 gives the model name, the mode, the number of states and the wall-clock
time for compilation plus solution. Line 2 gives the integrator statistics:
accepted and rejected steps, right-hand-side evaluations, Jacobian evaluations
and the number of colours (right-hand-side evaluations per Jacobian), LU
factorisations, whether the sparse or the dense solver was used and its number
of non-zeros, and the smallest and largest step size used. For a transient with
`start_from_steady: true` a third line reports the initial steady solve. The
remaining lines are the final channel values, with pressures in bar,
temperatures in K, mass flows in kg/s and every other unit in SI with
scientific notation.

Console output of a steady state with a channel filter:

```
$ fluidnet run examples/expander_cycle.yaml --steady --print tca.,turbopump.speed
Closed expander throttling [steady]: 63 states, solved in 215.5 ms
  steady: 192 iterations, 3496 rhs evals, converged true (shaft continuation (time-marching + newton) + pseudo-transient)
  turbopump.speed              5.23337e3 rad/s
  tca.Pc                       39.9646 bar
  tca.OF                       5.80588e0 -
  tca.thrust                   1.99793e4 N
  tca.Isp                      4.57158e2 s
  tca.cstar                    2.28835e3 m/s
  tca.Tc                       3311.25 K
  tca.mdot                     4.45649 kg/s
  tca.mdot_fuel                0.65480 kg/s
  tca.mdot_ox                  3.80169 kg/s
  tca.ignition                 1.00000e0 -
```

The `steady:` line gives the iterations, right-hand-side evaluations and the
solution path (see [outputs](outputs.md#5-fields-of-the-results-object)).
5233.37 rad/s is 49,975 rpm.

A transient that stops early prints the reason and keeps the data up to that
time:

```
LOX feed startup [transient]: 38 states, solved in 36.4 ms
  STOPPED EARLY at t = 5.600000e-2 s: solver error: maximum number of steps (300) reached at t = 5.790888e-2 s
```

(Produced by adding `max_steps: 300` to the example.) The exit code is still 0.

A steady state that does not converge exits with code 1:

```
$ fluidnet run examples/regulated_pressurization.yaml --steady
solver error: steady state did not converge (residual 1.674e5, largest in press_line.n2.P). The system may have no steady state (e.g. a closed tank that keeps filling) or may oscillate.
```

This model has no steady state at t = 0 s (the draw-off valve is closed, so the
regulator keeps filling the ullage tank); see
[modelling practice](modelling-practice.md#4-steady-state-convergence).

### check

```
fluidnet check MODEL
```

Compiles the model without running it and lists every node with its fluid and
volume, and every branch with its end nodes. Use it after editing a model to
confirm the topology and the node names before a run.

```
$ fluidnet check examples/gn2_blowdown.yaml
GN2 bottle blowdown: 9 nodes, 8 branches, 22 states
  node bottle                   fluid N2    V = 5.0013e-2 m3
  node vent_valve.in            fluid N2    V = 1.3090e-5 m3
  node vent_valve.out           fluid N2    V = 2.1166e-6 m3
  node ambient                  fluid N2    "boundary"
  node line.n1                  fluid N2    V = 2.6180e-5 m3
  ...
  node line.n5                  fluid N2    V = 2.6180e-5 m3
  branch line.s1                bottle -> line.n1
  ...
  branch line.s6                line.n5 -> vent_valve.in
  branch vent_valve             vent_valve.in -> vent_valve.out
  branch orifice                vent_valve.out -> ambient
```

The bottle volume (50.013 L) includes half a line segment; `vent_valve.in`
is the other half segment; `vent_valve.out` has no pipe and gets the stub
volume of the valve and the orifice (rules in the
[model file standard](model-files.md#25-ports-and-connections)). Branch
directions are `in` to `out`. The volume strings are printed in quotes.

### catalog

```
fluidnet catalog > catalog.json
```

Prints the component catalog as JSON: for each of the 15 component types its
`type`, `label`, `category`, `ports`, `node_like` flag, `help` text and
`params`; for each parameter its `key`, `label`, `kind` (`quantity`, `fluid`,
`choice`, `int`, `bool`, `ref`, `text`), `dim` and SI unit `si`, display
`unit`, `default`, `required`, `optional`, `schedulable`, `choices`, `target`
(for references) and `help`. The web interface and the
[component reference](component-reference.md) are generated from this output.
`catalog` loads no fluid tables.

### props

```
fluidnet props FLUID P T
```

Prints the table properties of a fluid at pressure `P` in Pa and temperature
`T` in K (bare numbers, no units):

```
$ fluidnet props O2 25e5 90
Props {
    p: 2500000.0,
    h: -132455.2951720936,
    v: 0.000871468106496463,
    rho: 1147.4889242020226,
    t: 90.00000000000001,
    s: 2929.0071941707906,
    mu: 0.00020073219124607235,
    k: 0.1528432593015989,
    cp: 1687.3049083926685,
    dvdp: -2.812362498584168e-12,
    dvdh: 2.1892877340260576e-9,
    a: 916.3362755857597,
    x: NaN,
}
```

Fields in SI: pressure `p` (Pa), enthalpy `h` (J/kg), specific volume `v`
(m³/kg), density `rho` (kg/m³), temperature `t` (K), entropy `s` (J/kg/K),
viscosity `mu` (Pa s), conductivity `k` (W/m/K), heat capacity `cp` (J/kg/K),
the derivatives `dvdp` = (∂v/∂P)_h (m³/kg/Pa) and `dvdh` = (∂v/∂h)_P (m³/J),
sound speed `a` (m/s) and quality `x` (not computed by this command).
Temperatures on the saturation line are ambiguous in (P, T); use the Python
`props` function with `h` for two-phase states. P and T accept units (`"25 bar"`, `"90 K"`); an argument that cannot be read
exits with code 2, an unknown fluid with code 1.

### bench (developer)

```
fluidnet bench MODEL
```

Times the building blocks of the transient solver on the model's initial
state: right-hand side (average of 2000 evaluations), finite-difference
Jacobian (50), LU factorisation (50), triangular solve (2000), and a fluid
property lookup (200,000 calls on the first fluid).

```
$ fluidnet bench examples/expander_cycle.yaml
63 states (16 colors, nnz 623, sparse): rhs 25.7 us, jacobian 371.6 us, factor 70.6 us, solve 2.9 us, props_ph 360 ns (2566383)
```

The number in brackets is a checksum that keeps the property loop from being
optimised away. Use it to compare solver changes on the same machine.

### jaccheck (developer)

```
fluidnet jaccheck MODEL [--t TIME] [--probe STATE]
```

Compares the coloured finite-difference Jacobian with a full column-by-column
finite-difference Jacobian at the initial state (or at the steady state if the
model sets `start_from_steady`), evaluated at time `--t` (default 0 s), and
reports the largest relative difference. A large difference means that the
sparsity pattern (`Network::sparsity`) misses a dependency, which is the first
check after adding a component ([adding a component](../05-development/adding-a-component.md)).

```
$ fluidnet jaccheck examples/lox_feed_startup.yaml
10 colors for 38 states; worst rel diff 1.034e-9 at d(manifold.h)/d(manifold.h): full -3.8641e-1 colored -3.8641e-1
```

`--probe STATE` additionally sweeps the perturbation of one state (by name, as
in `state_names`) over eight orders of magnitude and prints the diagonal entry
for each, which shows non-smooth behaviour of that equation:

```
$ fluidnet jaccheck tests_models/blowdown_n2.yaml --probe tank.P
sweep of tank.P (y = 1.000000000e6):
  d =    1.0e-6  df/dy for own row = -3.700552e-2   (tank.P row)
  ...
  d =     1.0e4  df/dy for own row = -3.834042e-2   (tank.P row)
2 colors for 2 states; worst rel diff 5.220e-10 at d(tank.h)/d(tank.h): full -1.7380e-2 colored -1.7380e-2
```

A misspelt state name in `--probe` lists the state names and exits with code 2.

### propsbatch (developer)

```
fluidnet propsbatch FLUID < input.txt
```

Reads lines of `P h` (Pa, J/kg) from standard input and writes one line of
`rho T s a` (kg/m³, K, J/kg/K, m/s) per input line, for comparing the tables
against CoolProp in bulk. Lines that do not hold two numbers are skipped. An
unknown fluid exits with code 2.

```
$ printf "25e5 -130000\n1e5 100000\n" | fluidnet propsbatch O2
1.140395226214e3 9.145376374240e1 2.956084256293e3 9.046665430951e2
3.502006039206e0 1.118084127252e2 5.509467881733e3 1.992829422198e2
```

## Environment variables

| Variable | Effect |
|---|---|
| `FLUIDNET_FLUIDS` | Fluid table directory (used when `--fluids` is not given). |
| `FLUIDNET_DEBUG` | If set (any value), the steady-state and transient solvers print their iteration history to standard error: Newton and pseudo-transient iterations with step and residual, rejected transient steps with the state that limited the step. Use it to find the state that prevents convergence. |

## Examples

Run every example and keep the results:

```sh
for f in examples/*.yaml; do
  ./target/release/fluidnet run "$f" --out "results/$(basename "$f" .yaml).csv"
done
```

PowerShell:

```powershell
Get-ChildItem examples\*.yaml | ForEach-Object {
  .\target\release\fluidnet.exe run $_.FullName --out "results\$($_.BaseName).csv"
}
```

(Create the `results` directory first.) The run times of all examples are in
the [example register](examples.md).

Steady state of the RL10A-3-3A model with the turbopump channels:

```sh
./target/release/fluidnet run examples/rl10a33a.yaml --steady --print turbopump.,tca.Pc
```

## Troubleshooting

| Symptom | Cause and remedy |
|---|---|
| Usage text and exit code 2 | Subcommand missing or misspelt, or the model argument missing. |
| `cannot find fluid tables ...` | See [installation, troubleshooting](getting-started.md#troubleshooting). |
| `MODEL: No such file or directory (os error 2)` | Wrong path to the model file. |
| `parse error: YAML: ...` with a line and column | YAML syntax: check indentation, unbalanced brackets in flow mappings, and a missing space after a colon. |
| `model error: X: ...` | The message names the component and the parameter or connection; the rules are in the [model file standard](model-files.md). |
| A channel is not printed | The default filter shows only `.P`, `.T` and `.mdot` and hides names containing `.n` or `.s`. Use `--print` with a substring of the name. |
| `STOPPED EARLY` | The transient failed part-way. See [modelling practice](modelling-practice.md#8-reading-warnings-and-errors). |
| `solver error: steady state did not converge ...` | See [modelling practice](modelling-practice.md#4-steady-state-convergence). |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
| B | 2026-11-10 | v0.5.1: exit code 3 for an early stop; subcommand checked first; `props` reads units and reports bad arguments; `version`; quiet end on a closed pipe |
