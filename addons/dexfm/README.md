# dexfm

Add-on package of fluidnet: engine-level design model that generates, calibrates and runs fluidnet models.

Foundation model for a dual expander liquid hydrogen / liquid oxygen rocket engine with several
thrust chambers fed by shared powerpacks. One YAML file defines an engine; one command produces the
design point, the throttle map, a fluid-network netlist and an HTML report.

    pip install -e .[dev]
    python -m dexfm run configs/dex1_baseline.yaml --out reports      # design + throttle + report
    python -m dexfm.studies arch configs/trade_base.yaml reports      # chamber / powerpack architecture trade
    python -m dexfm.studies ox   configs/trade_base.yaml reports      # oxygen cooling zone and chamber length trade
    python -m dexfm.studies sens configs/dex1_baseline.yaml reports   # sensitivity of the closing chamber pressure
    pytest                                                        # physics gates

## Layout

| Module | Content | Fidelity |
|---|---|---|
| `config.py` | Pydantic v2 engine configuration, YAML load, SHA-256 hash | n/a |
| `fluids.py` | CoolProp real-fluid states for parahydrogen and oxygen, two-phase tolerant | reference EOS |
| `thermochem.py` | `ThermochemProvider` interface, Cantera shifting-equilibrium provider, `H2CeaAdapter` seam | equilibrium, 1D |
| `chamber.py` | contour, Bartz hot-gas side, channel layout, coolant march, cooling zones per propellant | L1 (1D) |
| `turbo.py` | pump and turbine sizing, speed selection inside limits, off-design maps | L1 (correlations) |
| `cycle.py` | dual expander power balance per powerpack, maximum closing chamber pressure | L1 |
| `offdesign.py` | throttle solution with frozen hardware and detailed cooling in the loop | L1 |
| `transient.py` | lumped start model (shaft speeds and liner temperatures as states) | L0, experimental |
| `architecture.py`, `studies.py` | chamber and powerpack count trade, mass and envelope estimate | L0 to L1 |
| `fluidnet_model.py` | fluidnet model generation, calibration and start sequences | network transient |
| `adapters.py` | neutral netlist export | n/a |
| `report.py` | summary tables and Plotly report | n/a |

## Conventions

YAML is the single source of truth. Every result file carries the configuration hash. Physics
functions are stateless. Numbers quoted in controlled documents come from `python -m dexfm run`
and the study runners; anything else is flagged as an assumption in the document.

## fluidnet coupling (v0.2)

With `fluidnet` (0.5.0 or 0.5.1, tested with both) installed, `python -m dexfm run` also generates a fluidnet model of
one powerpack from the design point, calibrates it to that point (`fluidnet_model.calibrate`), writes
`<name>_powerpack_fluidnet.yaml` and `<name>_start_fluidnet.yaml`, and runs the tank-head start
(`fluidnet_model.DEFAULT_START`). Both YAML files open directly in the fluidnet CLI and web editor.
The generated model caps the nozzle area ratio at 100 (fluidnet 0.5.0 and 0.5.1 thrust is wrong above that)
and keeps valves cracked at t = 0 (a valve opening from its leak area against trapped liquid stalls
the integrator).

## h2cea coupling (v0.2.1)

`h2cea` (folder `h2cea/` of the e2-propulsion-tools repository) is a RocketCEA front end. dexfm uses
its `cea_si.point()` with the built-in LOX and LH2 cards through `thermochem.H2CeaAdapter`:

    pip install rocketcea
    set H2CEA_PATH=<e2-propulsion-tools>\\h2cea        # export on Linux/macOS
    set DEXFM_THERMOCHEM=h2cea                          # default is cantera
    python tools/compare_thermochem.py                  # both providers side by side on the baseline
    pytest tests/test_h2cea.py                          # agreement gate (skipped without RocketCEA)

The provider is chosen explicitly and its name is written into every summary file. The adapter was
written against the h2cea source but has not been executed with RocketCEA yet (no Fortran compiler in
the environment it was written in); run the two commands above once before switching the default.
The Bartz scale factor 0.80 was calibrated with the Cantera provider and must be re-checked
(`tests/test_gates.py::test_rl10_jacket_heat_pickup`) after switching.

## Known limits

* Heat transfer is one-dimensional and calibrated on one engine (RL10A-3-3A total jacket heat pickup,
  Bartz scale factor 0.80). Treat absolute heat loads as +/- 20 %.
* Oxygen coolant correlations in the two-phase and near-critical region are placeholders
  (homogeneous mixture model). Sub-critical oxygen operation is flagged, not resolved.
* Pump and turbine efficiencies come from correlations, not from designed blading.
* The start transient has quasi-steady flows and no line dynamics. It is a sequencing sandbox.
* No combustion stability, no injector design, no structural or life analysis.
