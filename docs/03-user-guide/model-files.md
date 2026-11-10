# Model file standard

FNT-USR-STD-001 · Rev B · Released · 2026-11-10 · Owner: Luis Erpenbach

## 1. Purpose

This standard defines the fluidnet model file: its top-level structure, how
components and connections are written, how quantities, units, time schedules
and table parameters are given, and the simulation settings. The command-line
tool, the Python package and the web user interface all read and write this
format, so a model built in one front end runs unchanged in the others.

The parameters of each component type are not repeated here; they are listed,
with units and defaults, in the generated
[component reference](component-reference.md). The output channels a model
produces are defined in [outputs](outputs.md).

The authority is the code: the schema in `crates/fluidnet-core/src/model.rs`,
units and schedules in `crates/fluidnet-core/src/units.rs`, parameter access in
`crates/fluidnet-core/src/catalog.rs` (`Params`) and the topology rules in
`crates/fluidnet-core/src/network/compile.rs` (`Network::compile`).

## 2. Rules

### 2.1 File format

1. A model file is YAML or JSON text. If the first non-blank character is `{`
   the text is read as JSON, otherwise as YAML (`Model::parse`). JSON is a
   subset of YAML in practice, but the two parsers report errors differently
   (`parse error: YAML: ...` or `parse error: JSON: ...`).
2. Use the file extension `.yaml` (or `.yml`, `.json`). The Python package
   treats a string ending in one of these extensions as a path; the web
   interface's Open dialog accepts these three extensions.
3. YAML comments (`#`) are allowed and are kept in the file, but a model saved
   by the web interface or by `fluidnet.save` is regenerated from the parsed
   model and loses them. Keep the commented master file under version control.

### 2.2 Top-level keys

| Key | Type | Required | Default | Meaning |
|---|---|---|---|---|
| `name` | string | no | empty | Model name, printed by the tools and stored in results. |
| `description` | string | no | empty | Free text. |
| `components` | list | yes, at least one entry | – | Component definitions (2.3). |
| `connections` | list | no | empty | Port-to-port connections (2.5). |
| `simulation` | mapping | no | all defaults | Run settings (2.11). |

Any other top-level key is an error (`unknown field`). The optional key
`fluidnet: "0.5"` states the fluidnet version the file was written for; a file
written for a newer minor version than the installed one is rejected.

### 2.3 Components

Each entry of `components` is a mapping with:

- `id` — the component identifier (rules in 2.4);
- `type` — one of the 15 component types below;
- the parameters of that type, as keys at the same level (`{id: V-101, type:
  valve, diameter: 18 mm, opening: 1}`).

| Type | Ports | Node-like | Typical use |
|---|---|---|---|
| `boundary` | `p` | yes | Pressure and temperature (or quality) source or sink |
| `tank` | `p` | yes | Rigid volume with mass and energy balance |
| `junction` | `p` | yes | Small volume where several elements meet |
| `pipe` | `in`, `out` | no | Discretised line with inertia, friction, heat transfer |
| `orifice` | `in`, `out` | no | Fixed restriction |
| `venturi` | `in`, `out` | no | Cavitating venturi |
| `valve` | `in`, `out` | no | Scheduled or pressure-actuated valve |
| `check_valve` | `in`, `out` | no | Valve opened by forward differential pressure |
| `regulator` | `in`, `out` | no | Pressure-reducing regulator |
| `loss` | `in`, `out` | no | Minor loss coefficient K |
| `mass_flow` | `in`, `out` | no | Imposed mass flow |
| `shaft` | none | no | Rotor shared by pumps and turbines |
| `pump` | `in`, `out` | no | Centrifugal pump |
| `turbine` | `in`, `out` | no | Gas turbine |
| `combustion_chamber` | `fuel`, `ox` | no | Thrust chamber with equilibrium combustion |

Rules:

1. An unknown `type` is an error (`model error: X: unknown component type 'oriface'`).
2. A parameter that the type does not define is an error that names the
   closest valid parameter (`model error: vent_valve: unknown parameter 'cdd';
   did you mean 'cd'?`). The keys `ui`,
   `label`, `notes`, `x`, `y` and `position` are reserved for front ends and
   are accepted and ignored (`IGNORED_KEYS` in `catalog.rs`); the web
   interface stores the schematic position as `ui: {x, y, flip}`. The
   `boundary` type uses `x` as its vapour-quality parameter, so on a boundary
   `x` is read as quality.
3. A parameter set to `null` or to an empty string takes its catalog default.
4. Required parameters without a default must be given
   (`model error: X: missing required parameter 'length'`).
5. Restrictions and turbines take either `diameter` or `area`; valves may give
   `Cv` (US gpm/psi^0.5) or `Kv` (m³/h/bar^0.5) instead, which take precedence
   over `diameter`, `area` and `cd`.
6. Machines (`pump`, `turbine`) name a `shaft` by its id or give a fixed
   `speed`; a pipe with a wall heated by a chamber names it in `hot_chamber`; a
   pressure-actuated valve names the sensed node in `sense`. A reference to a
   missing component or one of the wrong type is an error.

### 2.4 Component identifiers

1. Every `id` is a non-empty string, unique within the model.
2. Use only letters, digits, underscore and hyphen (`[A-Za-z0-9_-]`). Do not use
   dots: the connection syntax `id.port` splits at the last dot, and output
   channels are named `<id>.<quantity>`. The web interface replaces any other
   character with an underscore when a component is renamed.
3. The compiler generates names of the form `<id>.<port>` (implicit
   junctions), `<pipe>.n<k>`, `<pipe>.s<k>` and `<pipe>.w<k>` (pipe internals,
   see [outputs](outputs.md)). Ids without dots cannot collide with them.
4. Recommendation: use the P&ID tag of the hardware item as its id
   (`MOV-101`, `CV-101`, `INJ-101`, `T-101`). Results, plots and test data then
   share one naming scheme, and the model can be checked line by line against
   the P&ID. Model-only elements without a tag (lumped line segments, implicit
   volumes) take short descriptive lower-case names (`feed_line`, `manifold`).

### 2.5 Ports and connections

1. A connection joins two ports. Two forms are accepted and are equivalent:

   ```yaml
   connections:
     - [L-101.out, MOV-101.in]             # list form, exactly two entries
     - {from: CV-101.out, to: M-101}       # mapping form
   ```

2. A port is written `<id>.<port>`. A component with exactly one port (the
   node-like types) may be written by its id alone. A two-port component must
   name the port (`model error: connection 'o': component 'o' has ports
   ["in", "out"], name one (e.g. o.in)`).
3. `from`/`to` and the order in the list form carry no meaning for the flow
   direction. The sign convention of a flow element is fixed by its own ports:
   positive mass flow runs from `in` to `out`.
4. All ports connected to each other, directly or through a chain of
   connections, form one fluid node. Several elements may therefore share a port
   (`[M-101, INJ-101.in]` and `[M-101, PURGE-101.in]`).
5. A node contains at most one node-like component (`boundary`, `tank`,
   `junction`). Connecting two of them directly is an error (`model error: a
   and b are connected directly; put a flow element (pipe, orifice, valve)
   between them`).
6. A node without a node-like component becomes an implicit junction. It is
   named after the first port in the group that does not belong to a pipe (for
   example `main_valve.in`), otherwise after the first pipe port. Its volume is
   half a segment volume from each pipe that ends there; where no pipe ends
   there, it is a short stub of five bore diameters per connected element, and
   at least 1 mL (`DEFAULT_JUNCTION_VOLUME` in `compile.rs`).
7. Every port of a non-node-like component must be connected
   (`model error: port o.out is not connected`). A node-like component may be
   left unconnected.
8. The `in` and `out` ports of one element must not end in the same node, and a
   port must not be connected to itself.
9. Each inlet of a `combustion_chamber` (`fuel`, `ox`) connects to exactly one
   flow element, normally the injector orifice, and to nothing else.
10. Shafts have no fluid ports. Pumps and turbines are coupled to a shaft
    through their `shaft` parameter, not through a connection.

### 2.6 Fluids

1. Each connected circuit (all nodes joined through flow elements) carries one
   fluid. Mixing of species happens only in a combustion chamber.
2. `fluid` is required on `boundary` and `tank` and optional on `junction` and
   `pipe`. The fluid propagates through all flow elements of the circuit.
3. A circuit without any `fluid` is an error (`no fluid specified for the
   circuit containing X`); two different fluids in one circuit are an error
   (`fluid conflict: b uses O2 but is connected to a using N2`).
4. Use the table key exactly as listed below. The lookup ignores case and also
   accepts the table label, but the conflict check compares the strings as
   written, so `N2` and `Nitrogen` in one circuit are reported as a conflict.

| Key | Fluid | Table range T | Table range P | Critical point |
|---|---|---|---|---|
| `H2` | Parahydrogen | 13.80–1000 K | 100 Pa–600 bar | 32.94 K, 12.86 bar |
| `O2` | Oxygen | 54.36–1000 K | 100 Pa–600 bar | 154.60 K, 50.46 bar |
| `CH4` | Methane | 90.69–1000 K | 100 Pa–600 bar | 190.56 K, 45.99 bar |
| `N2` | Nitrogen | 63.15–1000 K | 100 Pa–600 bar | 126.19 K, 33.96 bar |
| `He` | Helium | 2.18–1000 K | 100 Pa–600 bar | 5.20 K, 2.28 bar |
| `H2O` | Water | 273.16–1000 K | 100 Pa–600 bar | 647.10 K, 220.64 bar |
| `N2O` | Nitrous oxide | 182.33–700 K | 100 Pa–200 bar | 309.52 K, 72.45 bar |
| `C2H5OH` | Ethanol | 159.10–650 K | 100 Pa–300 bar | 514.71 K, 62.68 bar |

The ranges are read from the headers of `fluids/*.fntb` (CoolProp 8.0.0).
Below the minimum table pressure the properties are extrapolated as an ideal
gas; how the tables are built and interpolated is described in
[fluid properties](../02-engineering/fluid-properties.md).

5. A `combustion_chamber` needs a fuel and an oxidizer pair with a combustion
   table. v0.5.0 ships H2/O2, CH4/O2 and C2H5OH/O2 (fuel on the `fuel` port).

### 2.7 Quantities and units

1. A quantity is a bare number, read in SI units, or a string of a number
   followed by a unit: `25 bar`, `90 K`, `-12.5degC`, `1.5e-3 m`, `18 mm`. The
   space between number and unit is optional. Numbers may carry a sign, a
   decimal point and an exponent (`e` or `E`).
2. Each parameter has a dimension (listed in the component reference). The unit
   must belong to that dimension (`model error: o.diameter: parse error: '2 bar':
   expected a Length quantity (m), got Pressure`).
3. Dimensionless parameters accept bare numbers and `%`. The parser accepts any
   known unit for a dimensionless parameter and applies its conversion factor
   without an error (`opening: 2 bar` is read as 200,000 and then limited to 1),
   so give dimensionless values as bare numbers or percentages only.
4. All pressures are absolute. `psi` and `psia` are the same unit; gauge units
   (`psig`, `barg`) do not exist.
5. `C` and `F` are degrees Celsius and Fahrenheit; `R` is the Rankine scale.
6. Parameters whose catalog unit is not in the list below (`viscous_loss` in
   N m s, pump `inertance` in 1/m) take bare SI numbers only.

Accepted units (`unit_info` in `units.rs`; aliases in brackets):

| Dimension | SI unit | Units |
|---|---|---|
| Pressure | Pa | `Pa` (`pa`), `kPa`, `MPa`, `bar` (`bara`), `mbar`, `psi` (`psia`), `atm` |
| Temperature | K | `K`, `degC` (`C`, `°C`, `degc`), `degF` (`F`, `°F`), `R` (`degR`) |
| Length | m | `m`, `cm`, `mm`, `um` (`µm`), `in`, `ft` |
| Area | m² | `m2` (`m^2`), `cm2` (`cm^2`), `mm2` (`mm^2`), `in2` (`in^2`) |
| Volume | m³ | `m3` (`m^3`), `L` (`l`, `liter`, `litre`), `mL` (`ml`, `cm3`, `cc`), `mm3`, `in3` |
| Mass flow | kg/s | `kg/s`, `g/s`, `kg/h`, `kg/min`, `lb/s` (`lbm/s`) |
| Time | s | `s`, `ms`, `us` (`µs`), `min`, `h` (`hr`) |
| Power | W | `W`, `kW`, `MW` |
| Heat flux | W/m² | `W/m2`, `kW/m2`, `MW/m2` |
| Rotational speed | rad/s | `rad/s`, `rpm` (`RPM`, `1/min`), `krpm`, `Hz` (`rev/s`) |
| Moment of inertia | kg m² | `kg m2` (`kg*m2`, `kg m^2`, `kg*m^2`, `kgm2`), `g cm2` (`g*cm2`, `g cm^2`) |
| Torque | N m | `N m` (`Nm`, `N*m`) |
| Density | kg/m³ | `kg/m3` (`kg/m^3`), `g/cm3` (`g/cc`) |
| Heat transfer coefficient | W/(m² K) | `W/m2/K` (`W/m2K`, `W/(m2 K)`, `W/m^2/K`), `kW/m2/K` (`kW/m2K`) |
| Dimensionless | – | none, `-`, `1`, `%` (factor 0.01) |

Unit names are case-sensitive except where an alias is listed (`mL` and `ml`,
`rpm` and `RPM`).

7. Integer parameters (`segments`, `parallel`) take a number or a numeric
   string and are rounded to the nearest integer. `segments` is limited to
   1–1000.
8. The boolean parameter `latch` accepts `true`/`false`, the strings `true`,
   `yes` or `1`, or a non-zero number.

### 2.8 Time schedules

Parameters marked schedulable in the component reference (boundary `P`, `T`,
`x`; valve `opening`; regulator `set_pressure`; `mass_flow` `mdot`; tank `Q`;
pipe `T_wall`, `q_wall`, `hot_T`, `hot_htc`; chamber `p_ambient`; shaft and
machine `speed`) accept a time table instead of a constant:

```yaml
opening: [[0, 0], [0.1, 0], [0.35, 1]]                  # t in s, value in SI
P: [[0 s, 1 bar], [350 ms, 1 bar], [800 ms, 15 bar]]    # units in either column
P: {table: [[0, 1 bar], [0.6, 10 bar]], interp: step}   # zero-order hold
```

1. Each row is `[t, value]`. `t` is a time quantity (bare seconds or with a time
   unit); `value` has the parameter's dimension.
2. Times must be non-decreasing (`schedule times must be non-decreasing`). A
   table with one row is a constant; an empty table is an error.
3. Between rows the value is interpolated linearly. Before the first row it
   equals the first value and after the last row the last value.
4. A repeated time gives a jump: in `[[0, a], [1, a], [1, b]]` the value is `a`
   up to 1 s and `b` from 1 s on.
5. The mapping form `{table: [...], interp: step}` holds each value until the
   next row time (zero-order hold). Without `interp: step` the mapping form is
   linear like the list form. The web interface's inspector edits only the
   list form; edit the mapping form in its YAML dialog.
6. All schedule times are breakpoints of the transient integrator: steps land
   exactly on them, so a valve that starts moving at 0.1 s starts at 0.1 s.
7. A steady-state solve evaluates every schedule at `simulation.steady_time`
   (2.11).
8. For a pressure-actuated valve (`sense` given), only the value of `opening`
   at t = 0 s is used, as the initial actuator position.

### 2.9 Table parameters

Some parameters are tables that are not functions of time. They use the same
row syntax:

| Parameter | Rows | Meaning |
|---|---|---|
| pipe `hot_contour` | `[x/L, A/At]` | Local area ratio along a chamber-cooled wall |
| pump `head_curve` | `[φ/φ_d, ψ/ψ_d]` | Head coefficient versus flow coefficient, relative to the design point |
| pump `eff_curve` | `[φ/φ_d, η/η_d]` | Efficiency versus flow coefficient |
| pump `speed_eff_curve` | `[N/N_d, factor]` | Efficiency correction versus speed |
| turbine `eff_curve` | `[u/c0, η/η_d]` | Efficiency versus velocity ratio |
| turbine `area_map` | `[PR, u/c0, factor]` | Nozzle flow-area factor versus pressure ratio and velocity ratio |

1. Two-column curves need at least two distinct points; the first column must
   be non-decreasing. Between points the curve is linear; outside the given
   range it is extrapolated linearly from the end segments.
2. `area_map` rows have three numbers; pressure ratio and factor must be
   positive, and each pressure ratio needs two or more velocity ratios. The map
   is bilinear and held constant beyond its edges.
3. Give table entries as bare numbers.

The definitions of φ, ψ, η and u/c0 are in
[turbomachinery](../02-engineering/turbomachinery.md).

### 2.10 Initial conditions

1. A `tank` requires `P0` and either `T0` (single phase) or `x0` (two-phase).
   `junction` and `pipe` accept the same three keys optionally; on a pipe they
   apply to its internal volumes and to implicit junctions at its ends.
   `P0` without `T0` or `x0` is an error.
2. A volume without an initial state takes the state of the nearest node that
   has one (a boundary or a node with `P0`). The search first crosses only
   elements that are open at t = 0 s; valves closed at t = 0 s, check valves and
   mass-flow sources are crossed last. A closed valve therefore separates, for
   example, a LOX-filled line from a gas-filled manifold.
3. A volume that receives no initial state is an error (`X: no initial state
   (set P0/T0 on it or on a connected tank/boundary)`).

### 2.11 Simulation settings

Times (`t_end`, `output_dt`, `max_step`, `steady_time`) are bare numbers in
seconds or strings with a time unit (`t_end: 1 min`, `output_dt: 2 ms`). An
unknown key is an error that lists the valid keys.

| Key | Default | Meaning |
|---|---|---|
| `t_end` | 1 s | End time of a transient; must be greater than 0 s. |
| `output_dt` | `t_end`/500 | Output interval. It is not allowed below `t_end`/200,000. Output times are 0, `output_dt`, 2 `output_dt`, ... and `t_end`. |
| `rtol` | 1e-4 | Relative tolerance of the transient integrator. |
| `atol_scale` | 1 | Multiplier on the absolute tolerances (1 Pa, 10 J/kg, 1e-6 kg/s, 0.1 rad/s, 0.01 K, 1e-5 mixture fraction, 1e-4 valve position). |
| `max_step` | `t_end`/20 | Largest time step. |
| `max_steps` | 2,000,000 | Limit on accepted plus rejected steps; reaching it stops the run early. |
| `start_from_steady` | false | Solve the steady state with the inputs at t = 0 s and start the transient from it. |
| `steady_time` | 0 s | Time at which schedules are evaluated for a steady-state solve (for example the end of a start sequence). |

The meaning of the tolerances and when to change them is in
[modelling practice](modelling-practice.md#3-tolerances-and-step-size).

## 3. Examples

### 3.1 Complete annotated model

The model below is a cold-flow test of a LOX feed line. It uses P&ID tags as
ids, a time table with units, a step schedule, a check valve and both
connection forms. It runs as written.

```yaml
# Cold-flow test of a LOX feed line: run tank, main valve, check valve,
# injector manifold and injector, discharging into a chamber held at a
# scheduled back-pressure.
name: LOX feed cold-flow test
description: >
  LOX run tank at 25 bar feeding a 3 m line, main valve opening over 250 ms,
  check valve, 80 mL manifold and a 7.5 mm injector orifice. The chamber
  back-pressure steps from 1 bar to 10 bar at 0.6 s.

components:
  # Node-like components (one fluid node each); P&ID tags as ids
  - {id: T-101, type: boundary, fluid: O2, P: 25 bar, T: 90 K}        # run tank as a pressure boundary
  - {id: M-101, type: junction, volume: 80 mL}                         # injector manifold
  - id: CH-101                                                          # chamber, step schedule
    type: boundary
    fluid: O2
    P: {table: [[0, 1 bar], [0.6, 10 bar]], interp: step}
    T: 300 K

  # Flow elements (ports in, out)
  - {id: L-101, type: pipe, length: 3 m, diameter: 20 mm, segments: 12, dz: -1 m, K: 1.2}
  - id: MOV-101
    type: valve
    diameter: 18 mm
    cd: 0.8
    characteristic: quick_opening
    opening: [[0 s, 0], [100 ms, 0], [350 ms, 100 %]]                 # time table with units
  - {id: CV-101, type: check_valve, diameter: 18 mm, cracking_dp: 0.2 bar, full_open_dp: 0.7 bar}
  - {id: INJ-101, type: orifice, diameter: 7.5 mm, cd: 0.7}

connections:
  - [T-101, L-101.in]                 # single-port component: id alone
  - [L-101.out, MOV-101.in]           # two-port components: id.port
  - [MOV-101.out, CV-101.in]
  - {from: CV-101.out, to: M-101}     # mapping form, same meaning
  - [M-101, INJ-101.in]
  - [INJ-101.out, CH-101]

simulation:
  t_end: 1.2            # s
  output_dt: 0.002      # s (default t_end / 500)
  rtol: 1e-4
  steady_time: 1.2      # steady state with the valve open and 10 bar back-pressure
```

`fluidnet check` reports 16 nodes, 15 branches and 40 states: the 12-segment
line gives 11 internal volumes and 12 momentum segments; the implicit junctions
`MOV-101.in` and `MOV-101.out` between line, valve and check valve, and the
manifold `M-101`, are volumes; `T-101` and `CH-101` are boundaries. At
t = 1.2 s (v0.5.0) the injector flow is 1.739 kg/s with the manifold at
23.78 bar, and the steady state at `steady_time` gives the same values.

What the annotations point at:

- `T-101` and `CH-101` are boundaries, so the run tank pressure and the chamber
  pressure are imposed; a `tank` would instead compute its pressure from its
  mass and energy balance.
- `M-101` is an explicit junction with a stated volume. Between the valve and
  the check valve no junction is written; the compiler creates one named
  `MOV-101.out` (rule 2.5.6).
- `opening` uses time units in the first column and `%` in the second.
- `CH-101.P` uses the step form; the integrator lands a step at 0.6 s.

### 3.2 The same structure in JSON

```json
{
  "name": "GN2 vent",
  "components": [
    {"id": "bottle", "type": "tank", "fluid": "N2", "volume": "50 L", "P0": "300 bar", "T0": "290 K"},
    {"id": "orf", "type": "orifice", "diameter": "3 mm", "cd": 0.8},
    {"id": "amb", "type": "boundary", "fluid": "N2", "P": 1e5, "T": 290}
  ],
  "connections": [["bottle", "orf.in"], {"from": "orf.out", "to": "amb"}],
  "simulation": {"t_end": 20, "output_dt": 0.1}
}
```

Bare numbers (`"P": 1e5`, `"T": 290`) are SI. After 20 s the bottle holds
111.8 bar, 219.5 K and 9.53 kg (v0.5.0).

### 3.3 Further examples

Every file in `examples/` follows this standard; the
[example register](examples.md) lists them with their features.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
| B | 2026-11-10 | v0.5.1: unknown parameters and unknown keys are errors; `fluidnet` version key; time units in `simulation`; dimensionless parameters reject dimensional units; tank `steady` parameter |
