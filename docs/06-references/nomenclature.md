# Nomenclature

FNT-REF-REG-002 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

Symbols, abbreviations and terms used across the documentation. Units are SI
unless stated.

## Symbols

| Symbol | Meaning | Unit |
|---|---|---|
| A | Flow area | m² |
| A_t | Throat area | m² |
| a | Speed of sound; also shutoff head ratio of the generic pump map | m/s; – |
| c* | Characteristic velocity | m/s |
| c0 | Turbine spouting velocity, √(2 Δh_s) | m/s |
| C_d, cd | Discharge coefficient | – |
| CF | Thrust coefficient | – |
| c_p | Specific heat at constant pressure | J/(kg·K) |
| D | Diameter | m |
| f | Darcy friction factor | – |
| G | Mass flux | kg/(m²·s) |
| H | Pump head | m |
| h | Specific enthalpy; heat transfer coefficient (context) | J/kg; W/(m²·K) |
| Δh_s | Isentropic enthalpy drop | J/kg |
| I | Pump fluid inertance | 1/m |
| J | Shaft polar moment of inertia | kg·m² |
| K | Minor-loss coefficient | – |
| L | Length | m |
| L* | Characteristic chamber length, V_c/A_t | m |
| ṁ, mdot | Mass flow rate | kg/s |
| N, n | Shaft speed; n = N/N_d | rpm or rad/s; – |
| O/F | Oxidizer-to-fuel mass ratio | – |
| P | Pressure | Pa |
| Pc | Chamber pressure | Pa |
| PR | Turbine pressure ratio, inlet over exit | – |
| p0 | Shutoff power ratio of the generic pump map | – |
| Q | Volumetric flow (pumps); heat flow (walls) | m³/s; W |
| q | Q/Q_d; heat flux (walls) | –; W/m² |
| r | Pressure recovery ratio of a restriction | – |
| s | Specific entropy | J/(kg·K) |
| T | Temperature | K |
| u | Turbine blade speed at the mean diameter | m/s |
| V | Volume | m³ |
| v | Specific volume | m³/kg |
| x | Vapour quality (mass fraction of vapour); also u/c0 in turbine maps (context) | – |
| X | RL10 turbine velocity ratio, √2 · u/c0 | – |
| Z | Oxidizer mass fraction of the chamber gas | – |
| γ | Ratio of specific heats; TR-BDF2 stage fraction 2 − √2 (context) | – |
| ε | Nozzle area ratio | – |
| η | Efficiency | – |
| κ | Real-gas isentropic exponent, −(v/P)(∂P/∂v)_s | – |
| ρ | Density | kg/m³ |
| σ | Bartz property correction factor | – |
| τ | Torque | N·m |
| φ | Pump flow coefficient | – |
| ψ | Pump head coefficient | – |
| ω | Angular speed; Leung omega parameter (context) | rad/s; – |

Subscripts: d design point, s isentropic, t throat, c chamber, w wall,
b bulk, l liquid, v vapour, sat saturation.

## Abbreviations

| Abbreviation | Meaning |
|---|---|
| ANA | Analysis (document type) |
| CEA | NASA Chemical Equilibrium with Applications |
| COPV | Composite overwrapped pressure vessel |
| DAQ | Data acquisition |
| FCV | Fuel cooldown valve (RL10) |
| FMI, FMU | Functional Mock-up Interface, Functional Mock-up Unit |
| GSE | Ground support equipment |
| HBK | Handbook (document type) |
| HEM | Homogeneous equilibrium model |
| HEX | Heat exchanger |
| HTC | Heat transfer coefficient |
| MES | Main engine start (signal) |
| NPSH | Net positive suction head |
| OCV | Oxidizer control valve (RL10) |
| P&ID | Piping and instrumentation diagram |
| PTC | Pseudo-transient continuation |
| ROCETS | Rocket Engine Transient Simulator (NASA) |
| TCV | Thrust control valve, the RL10 turbine bypass |
| TP1–TP5 | RL10A-3-3A test points of CR-190786 |
| TR-BDF2 | Trapezoidal rule / second-order backward differentiation formula, one-step two-stage integrator |

## Terms

| Term | Meaning in fluidnet |
|---|---|
| Node | A control volume with states P and h, or a boundary with prescribed state |
| Branch | A flow element between two nodes; inertial (state ṁ) or quasi-steady (algebraic ṁ) |
| Port | A named connection point of a component (`valve.in`) |
| Channel | A named output time series (`valve.mdot`) |
| Schedule | A time table `[[t, value], ...]` for a parameter |
| Catalog | The list of component types with their parameters, defined in `catalog.rs` |
| Steady time | The time at which schedules are evaluated for a steady-state solve |
| Warm start | Starting a solve from the final state of an earlier result |
| Run record | Directory with the resolved model, results, report and a manifest of versions and hashes (planned, WP3) |
| Subsystem | A model file used as one component of another model (planned, WP2) |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
