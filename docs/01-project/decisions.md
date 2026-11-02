# Decision log

FNT-PLT-REG-002 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

Architecture and process decisions with their reasons. A decision is changed
by adding a new entry that supersedes the old one, never by editing it.

Status: **Accepted** (in force), **Proposed** (in the platform plan, not yet
confirmed), **Open** (needs the owner's decision), **Superseded**.

| ID | Date | Decision | Reason | Status |
|---|---|---|---|---|
| D-001 | 2026-10 | Core written in Rust, one code base compiled natively, to WebAssembly and as a Python extension | Transients of stiff networks need compiled speed; one implementation for all front ends avoids diverging results | Accepted |
| D-002 | 2026-10 | Node states are pressure and specific enthalpy (P, h) | One formulation for liquid, gas, supercritical and two-phase states; property derivatives come directly from the tables | Accepted |
| D-003 | 2026-10 | Fluid properties from CoolProp, pre-tabulated on a (log P, h) grid and shipped as files | Lookup in about 0.25 µs and no CoolProp dependency at run time (CoolProp does not compile to WebAssembly in a practical way); accuracy 10⁻⁴ in density | Accepted |
| D-004 | 2026-10 | Homogeneous equilibrium two-phase model everywhere | Covers cavitation, flashing and boiling lines with one model; slip and non-equilibrium are later options (PLT-R-13) | Accepted |
| D-005 | 2026-10 | Model files in YAML are the source of truth; the UI reads and writes the same format | Reviewable in Git, editable by hand, no hidden state in a binary project file | Accepted |
| D-006 | 2026-10 | Component catalog in `catalog.rs` is the single source of ports, parameters, units, defaults and help; the UI inspector and the component reference are generated from it | One place to change; documentation cannot drift from the code | Accepted |
| D-007 | 2026-10 | TR-BDF2 as the transient integrator, with a colored finite-difference Jacobian | L-stable one-step method suited to stiff networks with frequent schedule breakpoints; finite differences keep new components free of hand-written derivatives | Accepted |
| D-008 | 2026-10 | WebAssembly through a C ABI without wasm-bindgen; results as JSON metadata plus one f64 block | Small, dependency-free interface; large transients avoid JSON number encoding | Accepted |
| D-009 | 2026-10 | Equilibrium combustion from Cantera tables over O/F and Pc with enthalpy sensitivities | Fast and smooth inside Newton iterations; corrects for warm injected fuel | Accepted |
| D-010 | 2026-10 | No discrete events in flow laws; check valves, regulators and actuated valves use smooth functions and states | Robust Newton convergence and step control; the cost is a small leakage and smooth transitions | Accepted |
| D-011 | 2026-11 | RL10 validation uses the published P&W maps and an assumed TCV position (half open at TP1, 0.16 in² full open) | The TCV position was not recorded; the published 0.0158 in² is a part-open value; documented in FNT-VAL-ANA-002 | Accepted |
| D-012 | 2026-11 | Subsystems, named parameters and library templates are resolved in the core before compilation | CLI, Python, desktop and web give identical results for the same files | Proposed |
| D-013 | 2026-11 | Project layer (studies, run store, reports, test data) in Python with Pydantic v2 | Most users work in Python; matches the team's stack; easy to extend without Rust | Proposed |
| D-014 | 2026-11 | A result is identified by the SHA256 of the fully resolved model, the property tables and the software version | Traceability from an analysis page to a reproducible run; enables result reuse | Proposed |
| D-015 | 2026-11 | Parquet for results and imported test data | Compact, typed, read by pandas and most tools; keeps CSV for export | Proposed |
| D-016 | 2026-11 | One user-facing `fluidnet` command installed by the Python package; the Rust CLI stays for CI and debugging | One command for users, one install step | Proposed |
| D-017 | 2026-11 | Desktop application with Tauri around the existing React interface and the native core | Offline, file-based, native speed; reuses the web interface | Proposed |
| D-018 | 2026-11 | Documentation as Markdown in the repository, Hopper document conventions, exportable to Notion and buildable with MkDocs | Versioned with the code; reviewable; independent of the final documentation system | Accepted |
| D-019 | – | Repository hosting | – | Open (FNT-PLT-PLN-001 §11) |
| D-020 | – | Ownership and licence | – | Open |
| D-021 | – | Long-term documentation location | – | Open |
| D-022 | – | First reference project after WP3 | – | Open |
| D-023 | – | Test-data access and format | – | Open |
| D-024 | – | Desktop or hosted application first | – | Open |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release; D-001 to D-011 recorded retrospectively |
