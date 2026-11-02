# 05 Development

How to build, test and release fluidnet, the rules that code, data and
documentation changes follow, and the procedure for adding a component. These
pages are for developers who change the code; the use of the tools is
described in [03 User Guide](../03-user-guide/index.md) and the design of the
code in [02 Engineering](../02-engineering/index.md).

> **Status (v0.5.0):** all pages released and checked against the v0.5.0 code.
> `cargo test --release` passes (14 unit tests, 16 validation tests, 91 s on
> the two-CPU Linux test machine); the WebAssembly fallback build and the
> generators for the component reference and the Notion export were run.
> There is no continuous integration yet, and releases up to 0.5.0 are not
> tagged (work package WP1 of the [platform plan](../01-project/platform-plan.md)).

## Handbooks

- [Build, test and release](build-and-release.md) — FNT-DEV-HBK-001. Building the core, CLI, WebAssembly engine, web interface and Python wheel; tests and validation; regenerating fluid tables, combustion tables and the component reference; documentation site and Notion export; versioning; release checklist.
- [Adding a component](adding-a-component.md) — FNT-DEV-HBK-002. From the catalog entry through compilation, evaluation, sparsity and output channels to the interface symbol, tests and documentation, with the check valve as worked example.

## Standards

- [Code and repository conventions](conventions.md) — FNT-DEV-STD-001. Repository layout, module responsibilities, error handling, units, naming of components, channels and states, required tests, documentation update rules, commit messages.

## Open items

Development and platform items (continuous integration, Windows wheels,
unknown parameters as errors, solver issues) are kept in the project register
of [open items](../01-project/open-items.md).
