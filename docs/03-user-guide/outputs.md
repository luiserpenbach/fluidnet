# Output channels and result files

FNT-USR-HBK-002 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Purpose

This handbook explains what a fluidnet run returns: the output channels, how
they are named for each component type, their units, the names of the internal
volumes and segments of pipes, the CSV and JSON files written by the
command-line tool, and the fields of the results object that all three front
ends share. It is the reference for reading a result file or finding the
channel to plot.

The channel list of a model is fixed when the model is compiled
(`Network::build_channels` in `crates/fluidnet-core/src/network/compile.rs`);
the values are sampled at each output time (`Recorder` in
`crates/fluidnet-core/src/sim.rs`).

## Prerequisites

- A model file ([model file standard](model-files.md)).
- One of the front ends: the [command-line tool](cli.md), the
  [Python package](python.md) or the [web interface](web-ui.md).

To see the channels of a model without running it, use `fluidnet.check(model)`
in Python, which returns the channel names and units.

## Content

### 1. Units

All channels are in SI units: Pa, K, kg/m³, J/kg, kg/s, m/s, m, W, N m, rad/s,
N, s. The unit string of every channel is stored with it (`[Pa]` in the CSV
header, `unit` in JSON and Python). Front ends convert only for display: the
console prints pressures in bar, and the web interface shows bar, kJ/kg, kW,
rpm and kN. Rotational speeds are in rad/s (multiply by 30/π for rpm).

Pressures are absolute. Specific enthalpy follows the CoolProp reference state
of each fluid table, so only enthalpy differences carry meaning across fluids.

### 2. Channel naming

Every channel is named `<element>.<quantity>`. The element is a node, a branch
(flow element) or a component.

**Nodes.** One node exists for each node-like component (`boundary`, `tank`,
`junction`, named by its id), for each group of connected ports without a
node-like component (an implicit junction, named after one of its ports, for
example `main_valve.in`; the naming rule is in the
[model file standard](model-files.md#25-ports-and-connections)), and for each
internal volume of a discretised pipe (`<pipe>.n1` to `<pipe>.n<N-1>` for N
segments, numbered from the `in` end). Every node has:

| Channel | Unit | Meaning |
|---|---|---|
| `<node>.P` | Pa | Pressure |
| `<node>.T` | K | Temperature |
| `<node>.rho` | kg/m³ | Density (homogeneous mixture density in the two-phase region) |
| `<node>.h` | J/kg | Specific enthalpy |
| `<node>.x` | – | Vapour quality, limited to 0–1 below the critical pressure (0 for subcooled liquid, 1 for superheated vapour); not a number at or above the critical pressure and below the triple-point pressure |
| `<tank>.m` | kg | Fluid mass, for `tank` nodes only |

The inlet nodes of a combustion chamber are part of the chamber model and have
no node channels; use the injector's upstream node and the chamber channels.

**Branches.** Each flow element is one branch named by its id, except a pipe
with more than one segment, whose segments are branches `<pipe>.s1` to
`<pipe>.s<N>`. Every branch has `<branch>.mdot` in kg/s, positive from `in` to
`out`. The other branch channels depend on the type:

| Type | Channels |
|---|---|
| `orifice`, `venturi`, `loss` | `mdot`, `dp` (Pa, `in` minus `out`), `choked` (1 if the flow is choked, else 0) |
| `valve`, `check_valve`, `regulator` | as above plus `opening` (–, effective area fraction after the valve characteristic, at least the leakage fraction) |
| `mass_flow` | `mdot`, `dp` (Pa, `in` minus `out`) |
| `pipe` with one segment | `mdot`, `dp` (Pa, `in` minus `out`), `u_in` (m/s, velocity at the segment's upstream density) |
| pipe segment `<pipe>.s<k>` | `mdot` |
| `pump` | `mdot`, `dp` (Pa, pressure rise `out` minus `in`), `head` (m), `eff` (–), `npsh` (m), `power` (W, absorbed), `torque` (N m), `speed` (rad/s, pump speed including the gear ratio) |
| `turbine` | `mdot`, `dp` (Pa, `in` minus `out`), `pr` (–, upstream over downstream pressure), `eff` (–), `u_c0` (–, blade speed over spouting velocity), `power` (W, delivered), `torque` (N m), `speed` (rad/s), `choked` |

Power and torque of pumps and turbines are reported as magnitudes.

**Component channels.**

| Component | Channel | Unit | Meaning |
|---|---|---|---|
| `pipe` with more than one segment | `<pipe>.mdot_in`, `<pipe>.mdot_out` | kg/s | Flow in the first and last segment |
| | `<pipe>.u_in` | m/s | Velocity in the first segment |
| | `<pipe>.dp` | Pa | Pressure at `in` minus pressure at `out` |
| `pipe` with `T_wall` or `q_wall` (no wall heat capacity) | `<pipe>.Q` | W | Heat into the fluid over the whole line |
| `pipe` with `wall_thickness` (wall with heat capacity) | `<pipe>.Q` | W | Heat into the coolant over the whole line |
| | `<pipe>.Q_hot` | W | Heat into the wall from the hot side |
| | `<pipe>.Tw_max` | K | Highest hot-side wall surface temperature |
| | `<pipe>.w<i>.T` | K | Wall element temperature (state), i = 0 to N from `in` to `out` |
| | `<pipe>.w<i>.T_hot` | K | Hot-side surface temperature of element i |
| | `<pipe>.w<i>.Q` | W | Heat into the coolant from element i |
| `shaft` | `<shaft>.speed` | rad/s | Shaft speed |
| | `<shaft>.turbine_torque` | N m | Sum of the turbine torques on the shaft, referred to shaft speed |
| `combustion_chamber` | `<ch>.Pc` | Pa | Chamber pressure |
| | `<ch>.OF` | – | Mixture ratio of the chamber gas |
| | `<ch>.thrust` | N | Thrust at the ambient pressure `p_ambient` |
| | `<ch>.Isp` | s | Specific impulse, thrust / (ṁ g₀); not a number without flow |
| | `<ch>.cstar` | m/s | Characteristic velocity |
| | `<ch>.Tc` | K | Chamber temperature |
| | `<ch>.mdot` | kg/s | Throat mass flow |
| | `<ch>.mdot_fuel`, `<ch>.mdot_ox` | kg/s | Injected fuel and oxidizer flows |
| | `<ch>.ignition` | – | Combustion progress, 0 before ignition to 1 after the ignition ramp |

A pump or turbine with a fixed `speed` instead of a shaft has no shaft
channels; its own `speed` channel gives the speed.

The physical definitions behind these quantities (head, NPSH, velocity ratio,
thrust coefficient, wall heat flux) are in
[flow elements](../02-engineering/flow-elements.md),
[turbomachinery](../02-engineering/turbomachinery.md),
[heat transfer](../02-engineering/heat-transfer.md) and
[combustion](../02-engineering/combustion.md).

**Order.** Channels appear in this order: all node channels (node-like and
implicit nodes in port-group order, then pipe internals), the pipe aggregate
channels, the branch channels in component order, shafts, chambers, and the
wall and heat channels of pipes.

### 3. Example: names in a small model

For `examples/gn2_blowdown.yaml` (bottle, 6-segment line, vent valve, orifice,
ambient), `fluidnet check` lists the nodes `bottle`, `vent_valve.in`,
`vent_valve.out`, `ambient` and `line.n1` to `line.n5`, and the branches
`line.s1` to `line.s6`, `vent_valve` and `orifice`. The channels therefore
include `bottle.P`, `bottle.m`, `line.n3.T`, `line.mdot_in`, `line.dp`,
`line.s6.mdot`, `vent_valve.opening`, `orifice.choked` and `ambient.P`.

### 4. Files written by the command-line tool

`fluidnet run MODEL --out FILE` writes all channels. A file name ending in
`.json` produces JSON; any other name produces CSV.

**CSV.** One header line, then one line per output time:

```
time [s],lox_tank.P [Pa],lox_tank.T [K],lox_tank.rho [kg/m3],lox_tank.h [J/kg],lox_tank.x [-],main_valve.in.P [Pa],...
0.000000000e0,2.500000000e6,9.000000000e1,...
```

- Column 1 is time in s; every other column is `<channel> [<unit>]`.
- Values are written in scientific notation with nine decimals.
- Values that are not finite (for example `x` above the critical pressure, or
  `Isp` without flow) are written as empty fields.
- A steady-state run writes one data line, at `steady_time`.
- The `examples/lox_feed_startup.yaml` transient gives 99 columns (time and 98
  channels) and 751 data lines (0 s to 1.5 s in 2 ms steps).

**JSON.** The serialised results object (section 5):

```json
{
  "name": "Water hammer", "mode": "transient",
  "time": [0.0, 0.0005, ...],
  "channels": [{"name": "reservoir.P", "unit": "Pa", "data": [4000000.0, ...]}, ...],
  "warnings": [], "stats": {...}, "steady": {...},
  "nodes": [{"name": "reservoir", "fluid": "H2O", "volume": null, "boundary": true}, ...],
  "components": [{"id": "line", "type": "pipe", "ports": [["in", "reservoir"], ["out", "valve.in"]],
                  "nodes": ["line.n1", ...], "branches": ["line.s1", ...]}, ...],
  "n_states": 150, "error": null,
  "state_names": ["valve.in.P", "valve.in.h", ...], "final_state": [6989497.77, 90629.55, ...]
}
```

Values that are not finite are written as `null`. JSON files are large for
long transients (every channel is a full array); use CSV, or the Python
package, for post-processing large runs.

### 5. Fields of the results object

The core returns one `Results` structure (`crates/fluidnet-core/src/sim.rs`);
the CLI writes it as JSON, Python wraps it in the `fluidnet.Results` class
([Python handbook](python.md)), and the web interface receives the same
metadata plus one block of numbers.

| Field | Content |
|---|---|
| `name` | Model name |
| `mode` | `transient` or `steady` |
| `time` | Output times in s (one entry for a steady state) |
| `channels` | List of `{name, unit, data}`; `data` aligned with `time` |
| `warnings` | Compilation warnings, for example `X: unknown parameter 'k' ignored` |
| `stats` | Transient integrator statistics (null for a steady run) |
| `steady` | Steady-solver statistics (for a steady run, and for a transient with `start_from_steady`) |
| `nodes` | Per node: `name`, `fluid`, `volume` (m³, null for boundaries), `boundary` |
| `components` | Per component: `id`, `type`, `ports` as `[port, node]` pairs, internal `nodes`, `branches` |
| `n_states` | Number of state variables |
| `error` | Null, or the solver message if the transient stopped early (section 6) |
| `state_names` | Names of the state variables |
| `final_state` | Last recorded value of each state, aligned with `state_names` |

**Transient statistics** (`stats`): `steps` (accepted), `rejected`,
`newton_failures`, `rhs_evals`, `jacobians`, `factorizations`, `n_states`,
`jacobian_colors` (right-hand-side evaluations per Jacobian), `nonzeros` and
`sparse` (linear solver), `h_min_used` and `h_max_used` (s). These are the
numbers on the CLI's second console line.

**Steady statistics** (`steady`): `iterations`, `rhs_evals`, `jacobians`,
`residual_history` (scaled residual norm per iteration), `converged` and
`method`. The method names the path that converged: `newton`,
`pseudo-transient`, `pseudo-transient (quasi-steady)`,
`time-marching + newton`, `transient settling + ...`, a
`shaft continuation (...)` prefix for turbopump models, or `algebraic` for a
model without states. The methods are described in
[solvers](../02-engineering/solvers.md); what to do when the steady state does
not converge is in [modelling practice](modelling-practice.md#4-steady-state-convergence).

**State names and warm starts.** `state_names` and `final_state` describe the
state vector at the last output time. State names are:

| State | Name |
|---|---|
| Pressure and enthalpy of a volume node | `<node>.P`, `<node>.h` |
| Mass flow of a pipe segment or pump | `<branch>.mdot` |
| Speed of a shaft that is not prescribed | `<shaft>.speed` |
| Wall element temperature | `<pipe>.w<i>.T` |
| Chamber pressure and oxidizer mass fraction | `<ch>.Pc`, `<ch>.Z` |
| Position of a pressure-actuated valve | `<valve>.position` |

Passing these values as the initial state of another run of the same or a
similar model (`init=` in Python) is a warm start; names that do not exist in
the other model are ignored. See the [Python handbook](python.md#warm-starts).

### 6. Runs that stop early

If the transient integrator fails after the first output time (for example a
step-size underflow or the step limit `max_steps`),
the run is not discarded: the results contain all output times up to the
failure and `error` holds the message. The CLI prints

```
  STOPPED EARLY at t = 5.600000e-2 s: solver error: maximum number of steps (300) reached at t = 5.790888e-2 s
```

and still exits with code 0 and writes the output file. Python returns the
results with `res.error` set; the web interface shows "Stopped early" with the
message. Always check `error` before using the last value of a channel.

A failure before the first output, a failed steady state and every model error
raise an error instead (CLI exit code 1, Python exception, web interface
message).

## Troubleshooting

| Symptom | Cause and remedy |
|---|---|
| `KeyError: no channel 'inj.mdot'; channels of that component: [...]` in Python | Misspelt channel. The message lists channels whose names contain the component id; `res.find("inj")` searches by substring. |
| A pipe has no `<pipe>.mdot` | Pipes with more than one segment report `<pipe>.mdot_in`, `<pipe>.mdot_out` and `<pipe>.s<k>.mdot`. |
| `x` is 0 or 1 although the fluid is near saturation | Quality is limited to 0–1; use `T` and `P` against the saturation curve, or `h`, to judge subcooling or superheat. |
| Empty CSV cells | Non-finite values (section 4). |
| Last values are not at `t_end` | The run stopped early; read `error`. |
| `opening` missing on a valve | Only `valve`, `check_valve` and `regulator` have it; an `orifice` is fixed. |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
