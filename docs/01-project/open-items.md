# Open items

FNT-PLT-REG-003 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

Known limits, open technical questions and missing features. Each item links
to the work package of the [platform plan](platform-plan.md) that closes it.
Closed items stay in the table with the version that closed them.

| ID | Category | Item | Effect today | Plan | Status |
|---|---|---|---|---|---|
| OI-001 | Validation | RL10 calibrated turbine peak efficiency 0.788 is 6 % above the P&W map (0.744); depends on the assumed TCV bypass at TP1 | Power level and TP1→TP4 trend not both consistent with the P&W map | Revisit with low-speed maps and an independent TCV estimate (WP6) | Open |
| OI-002 | Validation | RL10 start reaches 200 psia in 1.14 s, measured 1.43–2.56 s; the test bound was widened to 1.0–2.56 s | Start transients of turbopump cycles are optimistic | Low-speed pump maps, TM-107318 figures 4.1.10–4.1.15 (WP6) | Open |
| OI-003 | Validation | RL10 turbine area map held at edge values below u/c0 = 0.07 (about 4 % low there) | Affects only the first tenths of a second of a start | Re-digitize the lowest markers by hand | Open |
| OI-004 | Physics | One fluid per connected circuit; species mix only in the combustion chamber | No pressurant over propellant in one tank | Two-fluid tank (WP5) | Open |
| OI-005 | Physics | Homogeneous equilibrium everywhere; no thermal non-equilibrium or slip | Flashing N2O flow and self-pressurising tanks less accurate | Dyer option and two-node N2O tank (WP5) | Open |
| OI-006 | Physics | Pipe momentum neglects the convective acceleration term | Small error in lines with large density change along the line | Assess magnitude on the RL10 jacket; add if above 0.5 % | Open |
| OI-007 | Physics | Pipe wave speed from the fluid only; no wall elasticity | Water-hammer amplitudes and periods slightly high in compliant lines | Korteweg correction (WP5) | Open |
| OI-008 | Physics | Combustion: instantaneous equilibrium, single isentropic exponent in the nozzle; vacuum Isp a few seconds above CEA shifting equilibrium before the 0.98 nozzle efficiency | Isp bias of about 1 % | Accept; document per analysis; optional frozen/shifting choice | Open |
| OI-009 | Physics | Combustion tables only for H2, CH4 and ethanol with O2 | E2 (N2O/ethanol) cannot be modelled | Table generator for arbitrary pairs (WP5) | Open |
| OI-010 | Physics | Pre-ignition chamber gas is an ideal mixture of the propellants at their injection temperatures | Approximate chamber filling before ignition | Accept | Open |
| OI-011 | Physics | Debye heat capacity underestimates metals at low temperature (copper −16 % at 50 K) | Chill-down slightly fast | Tabulated c_p(T) per material (WP6) | Open |
| OI-012 | Physics | No boiling heat transfer correlations | Chill-down of lines and jackets not predictive | WP6 | Open |
| OI-013 | Physics | Below 20 % speed the pumps use the generic zero-speed curves | See OI-002 | WP6 | Open |
| OI-014 | Features | No controllers (PID), sequences, relief valves, burst discs, heat exchangers | Closed-loop throttle and protection devices cannot be modelled | WP5, WP6 | Open |
| OI-015 | Features | No subsystems, parameters, studies, run records | Complex models are hard to build, review and trace | WP2, WP3 | Open |
| OI-016 | Platform | No CI; wheels only for Linux | Windows users must build from source | WP1 | Open |
| OI-017 | Platform | Unknown parameter keys produce a warning (`unknown parameter 'cdd' ignored`), not an error | A misspelled parameter falls back to its default if the warning is overlooked | Strict mode as default, with a suggestion of the nearest key (WP1) | Open |
| OI-018 | Data | RL10 test values transcribed from an OCR text; garbled entries resolved from printed percent differences (see `validation/rl10/test_data.json`) | Small risk of transcription error | Check against a clean scan of CR-190786 | Open |
| OI-019 | Physics (bug) | Ethanol combustion table: `tools/gen_combustion_tables.py::reactant_h` adds the CoolProp enthalpy change to the ideal-gas formation enthalpy, but CoolProp's reference state (298.15 K, 1 bar) is liquid for ethanol, so the vaporisation enthalpy (about 0.92 MJ/kg) is not subtracted. Not affected: H2, O2, CH4 (gaseous at the reference state) | C2H5OH/O2 tables overestimate T_c and c* (Cantera check at O/F 1.5, 20 bar: 3257 K against 3193 K corrected). Relevant for E2 | Reference the CoolProp enthalpy to the vapour at 298.15 K; regenerate the ethanol table; compare with CEA (WP5, first task) | Open |
| OI-020 | Code (bug) | The junction stub volume next to a pump is computed from the pump design flow before that flow is set, so the stub area is always 10⁻⁵ m² | Small error in the volume of the pump inlet and outlet nodes; affects pump-node dynamics slightly | Compute after the design point is read; check example results unchanged within baselines (WP1) | Open |
| OI-021 | Code (bug) | Wall elements at a pipe end connected to a boundary add their heat to the boundary node, where it leaves every balance | Heat lost in models where a heated pipe ends at a boundary | Assign the end-element heat to the adjacent internal volume (WP1) | Open |
| OI-022 | Solver | A warm start (`init=`) still runs the shaft continuation, which resets shaft speeds to N0 or the design speed; the flow guess is skipped only for non-zero flow states | Warm starts of turbopump cycles do less than intended; documentation previously said they skip all preparation | Skip the continuation when shaft speeds come from `init` (WP3, sweeps) | Open |
| OI-023 | Data | Property-table accuracy figures and lookup cost come from a comparison that is not stored in the repository | Figures not reproducible | Add the CoolProp comparison script and its results file (WP1) | Open |
| OI-024 | Platform | Unknown keys in `simulation` and at the top level of a model are ignored without a warning (a typo such as `t_ned` runs the default 1 s); `simulation` values cannot carry units; dimensionless parameters accept any unit without error | Silent wrong settings | Strict schema validation (WP1) | Open |
| OI-025 | Platform | Fluid conflict check compares names as written: `N2` and `Nitrogen` in one circuit are reported as a conflict although they load the same table | Spurious error | Compare resolved table keys (WP1) | Open |
| OI-026 | CLI | Exit code 0 when a transient stops early; unknown subcommand reported only after the model is compiled; `props` replaces non-numeric arguments by 1e5 Pa / 300 K; panic on a closed output pipe and on bad names in `jaccheck --probe` and `propsbatch` | Scripts cannot detect failures reliably | Non-zero exit on early stop; argument parsing with errors (WP1, with the Python `fluidnet` command, D-016) | Open |
| OI-027 | Python | `run` raises `RuntimeError` for a model that does not compile, `check` raises `ValueError` for the same error | Inconsistent error handling in scripts | One exception class for model errors (WP3) | Open |
| OI-028 | UI | A schedule in the `{table, interp: step}` form shows as `[object Object]` in the inspector; npm scripts derive paths from `import.meta.url` in a way that may fail on Windows | UI editing of step schedules; Windows UI build | Fix in WP7 or earlier if needed | Open |
| OI-029 | Tests | `tests_models/water_hammer.yaml` is not used by any test; `check_valve` and `loss` have no example or test; `tests_models/heated_pipe.yaml` as a transient with the default 1 s reaches the 2,000,000-step limit at 0.74 s | Untested components; a test model unsuitable for transients | Add verification cases; give the model a `simulation` block (WP1) | Open |
| OI-030 | Code | `cargo fmt --check` reports 24 differences | Noise in diffs once formatting is enforced | Format once and enforce in CI (WP1) | Open |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
