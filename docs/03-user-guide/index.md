# 03 User Guide

How to install fluidnet, write model files, run them from the command line,
from Python and in the browser, read the results, and build models that solve
reliably. The pages here describe use; the physics and numerics behind each
component are in [02 Engineering](../02-engineering/index.md), and the evidence
that the results are right is in
[04 Verification and Validation](../04-verification/index.md).

> **Status (v0.5.0):** all pages released and checked against the v0.5.0 code;
> commands and console outputs were run with the v0.5.0 CLI and Python package
> on Linux. Windows commands are given but were not run. Prebuilt Python wheels
> exist for Linux only.

## Handbooks

- [Installation and first run](getting-started.md) — FNT-USR-HBK-001. Prerequisites, building the CLI, the Python wheel and the web interface, where fluid tables are found, Windows notes.
- [Output channels and result files](outputs.md) — FNT-USR-HBK-002. Channel names per component type, units, pipe internals, CSV and JSON files, the results fields.
- [Command-line tool](cli.md) — FNT-USR-HBK-003. `run`, `check`, `catalog`, `props` and the developer commands, options, console output, exit codes.
- [Python API](python.md) — FNT-USR-HBK-004. Functions, the `Results` class, warm starts, threaded sweeps, property lookups.
- [Web user interface](web-ui.md) — FNT-USR-HBK-005. Schematic editor, inspector, runs, charts and profiles, files, hosting.
- [Modelling practice and troubleshooting](modelling-practice.md) — FNT-USR-HBK-006. Line discretisation, volumes, tolerances, steady-state convergence, start transients, two-phase flow, performance, error messages.
- [Component reference](component-reference.md) — FNT-CMP-HBK-001. Every component type with its ports, parameters, units and defaults; generated from the component catalog.

## Standards

- [Model file standard](model-files.md) — FNT-USR-STD-001. Top-level keys, components and ids, ports and connections, fluids, units, schedules, table parameters, initial conditions, simulation settings, annotated example.

## Registers

- [Example models](examples.md) — FNT-USR-REG-001. Every model in `examples/` and `tests_models/`, the Python example and the RL10 scripts, with run modes and measured run times.

## Reading order

New users: installation, then the model file standard, then the handbook of
the front end in use. The component reference and the output channels page
are reference material; modelling practice is for the first model that does
not behave.

## Open items

Open items concerning the user tools are kept in the project register of
[open items](../01-project/open-items.md).
