# Example models

FNT-USR-REG-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Purpose

This register lists every model file in `examples/` and `tests_models/`, the
Python example and the RL10 validation scripts: what each models, its main
components, which run modes it is meant for, and how long it takes. Use it to
find a starting point for a new model and to know which files the validation
suite depends on.

## How the register was produced

- Run times were measured on 2026-11-02 with the v0.5.0 release CLI
  (`fluidnet run FILE` and `fluidnet run FILE --steady`) on the two-CPU Linux
  test machine. The time is the CLI's "solved in" figure (compilation plus
  solution); repeated runs vary by up to a factor of about 1.5.
- "Simulated" is `simulation.t_end`; a file without a `simulation` block runs
  for the default 1 s.
- "Test" names the function in `crates/fluidnet-core/tests/validation.rs`
  that uses the file; results and tolerances of the tests are in
  [analytic cases](../04-verification/analytic-cases.md) and
  [RL10 validation](../04-verification/rl10.md).
- Every file in `examples/` is also run as a transient and, except
  `regulated_pressurization.yaml`, as a steady state by the test
  `all_examples_run_steady_and_transient`.
- When a file changes, rerun the two commands and update its row.

## Examples (`examples/`)

These files are also offered in the Examples menu of the
[web interface](web-ui.md).

| File | What it models | Key components | States | Transient (simulated, solve time) | Steady state (solve time, method) | Test |
|---|---|---|---|---|---|---|
| `lox_feed_startup.yaml` | Subcooled LOX feed line (25 bar, 90 K) with a quick-opening main valve (opening 0.1–0.35 s) against a chamber whose pressure rises from 1 bar to 15 bar at ignition; valve-opening surge and line dynamics. | boundary, pipe (3 m, 12 segments), valve, junction (80 mL), orifice | 38 | 1.5 s, 117 ms | 4.3 ms, pseudo-transient | all examples |
| `gn2_blowdown.yaml` | 50 L nitrogen bottle at 300 bar vented through a line, valve and 3 mm orifice; choked flow and adiabatic cooling of the bottle. | tank, pipe (6 segments), valve, orifice, boundary | 22 | 60 s, 7.7 ms | 5.4 ms, quasi-steady (valve closed at t = 0 s) | all examples |
| `regulated_pressurization.yaml` | 20 L GN2 bottle at 300 bar pressurising a 50 L ullage through a regulator set to 35 bar; draw-off valve opens at 5–5.5 s. | 2 tanks, pipe, regulator, valve, boundary | 16 | 60 s, 7.3 ms | No steady state at t = 0 s (the closed draw-off valve lets the regulator fill the ullage); fails by design | all examples (transient only) |
| `water_hammer.yaml` | 100 m, 50 mm water line at 1.8 m/s, valve closed in 2 ms; Joukowsky surge and period 4L/a. | boundary, pipe (50 segments), valve (`leak: 0`) | 150 | 0.6 s from the steady state, 328 ms (`rtol` 1e-3) | 14 ms, pseudo-transient | `water_hammer_matches_joukowsky` |
| `cavitating_venturi.yaml` | Water through a 3 mm cavitating venturi while the back-pressure rises from 1 bar to 29 bar; constant flow until the throat un-chokes near 85 % recovery. | boundary, 2 pipes, venturi | 18 | 10 s from the steady state, 7.6 ms | 1.7 ms, pseudo-transient | all examples |
| `lh2_heated_channel.yaml` | Supercritical hydrogen at 40 bar heated from 24 K in a 6 mm channel whose wall temperature rises from 100 K to 400 K; cooling-jacket proxy. | boundary, orifice, pipe (20 segments, `T_wall` schedule) | 62 | 4 s from the steady state, 45 ms | 36 ms, time-marching + Newton | all examples |
| `expander_cycle.yaml` | 20 kN LOX/LH2 closed expander at its design point (40 bar, O/F 5.8); turbine bypass valve opened from 0.0586 to 0.6 and back to throttle the engine. | pumps on one shaft (LOX pump geared 0.3), regeneratively cooled jacket (60 channels, CuCrZr wall, `hot_chamber`), turbine, bypass and main valves, combustion chamber | 63 | 6 s from the steady state, 154 ms | 120 ms, shaft continuation | `expander_cycle_conserves_energy_and_torque` |
| `expander_start.yaml` | Tank-head start of the same engine: main fuel valve opens onto a warm jacket, boil-off spins the turbopump, ignition at 0.5 s, fuel-rich LOX idle until 3 s, climb to mainstage. | as `expander_cycle.yaml`, with valve sequences and `ignition_time` | 63 | 10 s, 7.2 s | 157 ms at `steady_time` 10 s, shaft continuation | `expander_start_reaches_the_steady_design_point` |
| `rl10a33a.yaml` | Pratt & Whitney RL10A-3-3A at test point 1 of NASA CR-190786, calibrated by `validation/rl10/rl10.py`; P&W pump and turbine maps. | 3 pumps on one shaft, jacket, venturi, turbine with thrust control valve, 6 valves, 11 junctions, combustion chamber | 120 | 1 s from the initial state (no `start_from_steady`), 1.9 s | 43 ms, shaft continuation | `rl10a33a_reproduces_test_point_1` |
| `rl10a33a_start.yaml` | RL10A-3-3A ground-test start from chilled pumps at rest and a warm jacket (540 R): fuel shutoff valve opens at engine start, spark at 0.3 s, oxidizer control and cooldown valves sequenced by pump pressures, thrust control valve opening with chamber pressure. Generated by `validation/rl10/start.py`. | as `rl10a33a.yaml` with pressure-actuated and latching valves, `max_step` 0.01 s | 123 | 4 s, 10.7 s | 45 s at `steady_time` 4 s, transient settling + Newton | `rl10a33a_start_accelerates_to_mainstage` |

## Python example (`examples/python/`)

| File | What it does | Run time |
|---|---|---|
| `throttle_map.py` | Throttle map of `expander_cycle.yaml`: twelve steady states over the turbine bypass valve opening 0.0586–1.0 with `fluidnet.sweep` (parallel threads), thrust, shaft speed and O/F plotted next to the throttling transient; writes `throttle_map.png`. Needs matplotlib. | 5.9 s (including Python start-up and plotting) |

See the [Python handbook](python.md#example-throttle-map).

## Test models (`tests_models/`)

Small models with an analytic or independently computed reference, used by
the validation suite.

| File | What it models | Key components | States | Transient (simulated, solve time) | Steady state (solve time) | Test |
|---|---|---|---|---|---|---|
| `water_orifice.yaml` | Water, 10 bar to 5 bar, through a 5 mm orifice (cd 0.7). | 2 boundaries, orifice | 0 | 1 s, 0.3 ms | 0.2 ms, algebraic | `liquid_orifice_matches_bernoulli` |
| `water_pipe.yaml` | Water, 3 bar to 1 bar, through 10 m of 20 mm pipe (15 µm roughness, 10 segments). | 2 boundaries, pipe | 28 | 1 s, 154 ms | 1.8 ms | `pipe_friction_matches_darcy_colebrook` |
| `cav_venturi.yaml` | Water at 30 bar, 300 K through a 2 mm cavitating venturi to 1 bar. | 2 boundaries, venturi | 0 | 1 s, 1.1 ms | 0.1 ms, algebraic | `cavitating_venturi_chokes_at_vapour_pressure` |
| `ln2_flash.yaml` | Saturated liquid nitrogen (5 bar, x = 0) flashing through a 2 mm orifice to 1 bar. | 2 boundaries, orifice | 0 | 1 s, 0.7 ms | 0.1 ms, algebraic | `saturated_ln2_flashing_matches_hem` |
| `blowdown_n2.yaml` | 50 L nitrogen tank, 10 bar and 300 K, discharging through a 3 mm orifice (cd 1.0). | tank, orifice, boundary | 2 | 6 s, 0.3 ms | 0.1 ms | `n2_blowdown_matches_isentropic_ideal_gas` |
| `heated_pipe.yaml` | Water at 0.1 kg/s through a 2 m, 10 mm pipe with 100 kW/m² wall heat flux. | boundary, mass_flow, pipe (`q_wall`), orifice | 32 | Not intended: the 1 s default transient stops early at 0.74 s after 2,000,000 steps (246 s) | 0.9 ms, Newton | `heated_pipe_closes_energy_balance` |
| `regulator.yaml` | GN2 regulator (set 30 bar, band 2 bar) from a 200 bar supply into a vented 2 L plenum. | boundary, regulator, tank, orifice | 2 | 5 s, 0.6 ms | 0.5 ms | `regulator_holds_set_pressure_within_band` |
| `pump_design.yaml` | Water pump at a prescribed 3000 rpm against a 20 mm orifice. | 2 boundaries, pump (fixed `speed`), orifice | 3 | 1 s, 0.8 ms | 0.2 ms | `pump_matches_affinity_map_hand_calculation` |
| `chamber_pressure_fed.yaml` | Pressure-fed GH2/LOX chamber, 40 mm throat, area ratio 40, efficiencies 1.0. | 2 boundaries, 2 injector orifices, combustion chamber | 2 | 0.1 s, 1.2 ms | 0.6 ms | `chamber_pressure_and_thrust_match_tables` |
| `actuated_valve.yaml` | Two valves sensing the same upstream pressure, which rises through their set range and falls back; one latching. | boundary, orifice, junction, 2 pressure-actuated valves | 4 | 2 s, 4.4 ms | 0.2 ms | `pressure_actuated_valves_follow_and_latch` |
| `water_hammer.yaml` | The water hammer line of `examples/water_hammer.yaml` with 200 segments and `rtol` 1e-4. | boundary, pipe (200 segments), valve | 600 | 0.6 s from the steady state, 9.6 s | 80 ms | none (reference for discretisation studies) |

## Validation scripts (`validation/rl10/`)

These Python scripts use the package API to calibrate and check the RL10 model
and write results next to themselves. They are not run by `cargo test`; their
run times are given in the [verification section](../04-verification/index.md)
and their method and results in [RL10 validation](../04-verification/rl10.md)
and [RL10 map digitization](../04-verification/rl10-maps.md). They were not run
for this register.

| File | Purpose | Writes |
|---|---|---|
| `rl10.py` | Calibration at test point 1 (nine factors) and prediction of test points 1–5 against CR-190786; comparison cases `generic`, `nominal_tcv`, `frozen`, `headcurve`. Usage `python validation/rl10/rl10.py [all\|predict\|generic\|nominal_tcv\|frozen\|headcurve]`. | `results.json`, `results_<case>.json`, calibrated factors into `examples/rl10a33a.yaml` |
| `start.py` | Builds the start model from the calibrated steady model and runs the start transient with sensitivity cases. | `examples/rl10a33a_start.yaml`, `start_results.json` |
| `maps/digitize.py`, `maps/digitize_area.py`, `maps/build_maps.py`, `maps/apply_maps.py`, `maps/plot_maps.py` | Digitization of the P&W pump and turbine maps from NASA TM-107318 and their transfer into the model; check plots. See `validation/rl10/maps/README.md`. | `rl10_maps.json`, map tables in `examples/rl10a33a.yaml`, `check_*.png` |
| `test_data.json` | Test data of CR-190786 used by `rl10.py`. | – |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
