# Validation matrix

FNT-VAL-REG-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

This register lists every physical model in fluidnet with the evidence that it
is correct. **Verified** means an analytic or code-to-code check passes
([FNT-VAL-ANA-001](analytic-cases.md)); **validated** means agreement with
measured data of real hardware. A model used for a design decision should be
at least verified in the regime it is used in; the last column says what is
missing.

| Model | Code | Verification | Validation against test data | Gap |
|---|---|---|---|---|
| Fluid properties, single phase | `props.rs` | CoolProp comparison, 2–4·10⁻⁴ density | via RL10 | none for the eight tabulated fluids; mixtures not supported |
| Fluid properties, two-phase | `props.rs` | CoolProp comparison, about 2·10⁻⁴ | none | flashing-flow test data (e.g. N2O blowdown) |
| Incompressible orifice flow | `flow.rs` | Bernoulli, 0.02 % | RL10 injectors (calibrated cd) | – |
| Cavitating venturi | `flow.rs` | Cavitation limit, 0.1 % | RL10 venturi (indirect) | cold-flow venturi test with measured flow |
| Two-phase restriction (omega) | `flow.rs` | HEM, 0.7 %; omega/HEM table 0.98–1.01 | none | N2O injector or orifice test data; non-equilibrium (Dyer) option |
| Real-gas choked flow | `flow.rs` | Isentropic ideal gas, 0.2 % (blowdown) | RL10 turbine nozzle (calibrated area) | GN2 orifice test |
| Pipe friction | `flow.rs` | Colebrook, 0.36 % | RL10 ducts and jacket (calibrated K) | – |
| Pipe acoustics, water hammer | `network/` | Joukowsky plateau 0.5 %, period 0.3 % | none | wall elasticity (wave speed) not modelled; feed-line test with fast valve closure |
| Column separation | `props.rs`, `network/` | Settles at vapour pressure | none | test data; the model is homogeneous (no discrete vapour cavity model) |
| Heated pipe, prescribed flux | `network/` | Energy balance, 0.16 % | – | – |
| Gnielinski convection, wall elements | `network/`, `machines.rs` | energy balances | RL10 jacket (via turbine inlet temperature, 0.84 %) | boiling heat transfer (chill-down) not modelled |
| Bartz hot-gas heat transfer, enthalpy potential | `network/`, `combustion.rs` | Wall potential trend | RL10 jacket heat (calibrated factor 0.937) | second engine (different size or propellants) |
| Wall heat capacity (Debye) | `machines.rs` | Copper −16 % at 50 K | RL10 start (indirect) | tabulated c_p(T) per material |
| Tank (homogeneous) | `network/` | Blowdown, 0.2 % | none | stratified tank, pressurant/propellant two-fluid tank, self-pressurising N2O tank |
| Valves, scheduled | `flow.rs`, `network/` | Opening characteristics | RL10 OCV/TCV (solved openings) | – |
| Valves, pressure-actuated, latch | `network/` | Follow and latch test | RL10 start (indirect) | – |
| Check valve, regulator | `flow.rs` | Regulator within band | none | regulator test data; dynamic regulator model |
| Pump, generic affinity map | `machines.rs` | Hand calculation, 0.02 % | RL10 (generic-map comparison case, 0.67 % mean) | – |
| Pump, tabulated maps and speed correction | `machines.rs` | Table reproduces generic curve | RL10, 0.62 % mean | low-speed and four-quadrant maps for start and shutdown |
| Turbine, impulse parabola and tabulated efficiency | `machines.rs` | Peak and runaway | RL10 (peak efficiency calibrated 6 % above the P&W map) | second turbine data set |
| Turbine flow-area map | `machines.rs` | Bilinear and held | RL10 | – |
| Shaft dynamics | `network/` | Torque balance 10⁻⁶ | RL10 start (1.14 s against 1.43–2.56 s) | low-speed pump maps; bearing loss data |
| Combustion chamber, equilibrium tables | `combustion.rs` | CEA, c* within 2 m/s | RL10 Pc and thrust (cf_eff calibrated 0.932) | ethanol table omits the vaporisation enthalpy (OI-019); N2O/ethanol tables; CEA comparison over the full O/F range |
| Chamber filling and ignition | `combustion.rs` | none | RL10 start (indirect) | chamber pressure trace from a hot-fire start |
| Steady solver | `solver/steady.rs` | Start transient ends at the steady state, 0.3 % | – | – |
| Transient integrator | `solver/transient.rs` | Joukowsky, blowdown | – | order-of-accuracy convergence study |

## How to add an entry

When a model is added or changed, add or update its row in the same change,
with the test function or analysis that covers it. When a new validation
against test data is done, write it as an analysis (FNT-VAL-ANA-NNN) and link
it here.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
