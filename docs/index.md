# fluidnet

> **fluidnet** is a simulator for fluid systems and liquid rocket engine
> cycles: steady state and transients of feed lines, tanks, valves,
> regulators, turbopumps, regeneratively cooled walls and thrust chambers.
> Rust core, YAML models, command-line tool, Python package and browser
> schematic editor. Validated against RL10A-3-3A ground-test data.
>
> New here? Read [Start Here](00-start-here.md) first.

**Current version:** 0.5.0 · **Owner:** Luis Erpenbach · **Status:** validated
solver; platform development planned against literature reference problems
up to staged combustion ([FNT-PLT-PLN-001](01-project/platform-plan.md))

## Sections

| Section | Content |
|---|---|
| [Start Here](00-start-here.md) | How this documentation is organised, document IDs, writing rules |
| [01 Project](01-project/index.md) | Platform development plan, release history, decision log, open items |
| [02 Engineering](02-engineering/index.md) | Architecture and physical models: network formulation, solvers, fluid properties, flow elements, turbomachinery, heat transfer, combustion |
| [03 User Guide](03-user-guide/index.md) | Installation, model file standard, component reference, outputs, CLI, Python API, web UI, modelling practice, examples |
| [04 Verification and Validation](04-verification/index.md) | Analytic cases, RL10A-3-3A validation, map digitization, validation matrix |
| [05 Development](05-development/index.md) | Build, test and release; conventions; adding a component |
| [06 References](06-references/index.md) | Literature, data sources, nomenclature |

## Common tasks

| I want to … | Go to |
|---|---|
| Install fluidnet and run an example | [Installation and first run](03-user-guide/getting-started.md) |
| Write a model file | [Model file standard](03-user-guide/model-files.md), [Component reference](03-user-guide/component-reference.md) |
| Script runs and sweeps from Python | [Python API](03-user-guide/python.md) |
| Understand why a steady solve fails | [Modelling practice and troubleshooting](03-user-guide/modelling-practice.md) |
| Know how accurate a result is | [Validation matrix](04-verification/validation-matrix.md) |
| Add a new component type | [Adding a component](05-development/adding-a-component.md) |
| See what is planned | [Platform development plan](01-project/platform-plan.md) |
