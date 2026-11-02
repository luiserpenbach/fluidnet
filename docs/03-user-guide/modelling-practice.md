# Modelling practice and troubleshooting

FNT-USR-HBK-006 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Purpose

This handbook collects the decisions a user makes when building a fluidnet
model beyond the file syntax: how finely to discretise lines, how large to
make junction volumes, when to change the integrator tolerances, how to obtain
a steady state and what to do when it fails, how to set up start transients
with sequenced and pressure-actuated valves, what to expect with two-phase
flow, how long runs take, and how to read the warnings and errors.

Each recommendation is tied to the solver code or to an example that
demonstrates it. The equations and numerical methods themselves are described
in [network formulation](../02-engineering/network-formulation.md) and
[solvers](../02-engineering/solvers.md) and are not repeated here.

## Prerequisites

- The [model file standard](model-files.md) and the
  [output channels](outputs.md).
- One of the front ends ([CLI](cli.md), [Python](python.md),
  [web interface](web-ui.md)).

## 1. Pipe segments

A `pipe` with N `segments` becomes N momentum segments (one mass-flow state
each, with inertia L/A, friction, losses and elevation) and N − 1 internal
volumes (pressure and enthalpy states each), so it adds 3N − 2 states. The two
end half-segments are added to the volumes of the nodes at its ports. With
`segments: 1` the line has inertia and friction but no internal volume.

Choose N for the phenomena of interest:

| Purpose | Choice | Example |
|---|---|---|
| Line inertia and pressure drop only; no wave effects of interest | 1 segment | `fuel_line` in `examples/expander_cycle.yaml` (0.8 m), `fuel_inlet` in `examples/rl10a33a.yaml` |
| Feed-line surge and line dynamics during valve opening | Several segments per metre of line | `feed_line` in `examples/lox_feed_startup.yaml`: 3 m in 12 segments |
| Axial temperature profile in a heated line | Enough segments to resolve the temperature rise; one wall element per chain node | `channel` in `examples/lh2_heated_channel.yaml`: 1.5 m in 20 segments; `jacket` in `examples/expander_cycle.yaml`: 8 segments |
| Water hammer: plateau and period | Segment transit time Δx/a well below the period 4L/a | `examples/water_hammer.yaml`: 100 m in 50 segments |

The lumped line is a chain of volumes and inertances. A pressure wave crosses
one segment in Δx/a; the chain reproduces waves whose length spans several
segments and smears steeper fronts over a few segments. In
`examples/water_hammer.yaml` (Δx = 2 m, a ≈ 1489 m/s in water at 40 bar and
293 K, so Δx/a ≈ 1.3 ms against a period of 0.269 s) the validation test
`water_hammer_matches_joukowsky` holds the Joukowsky plateau within 2 % and the
period within 1 % (results in
[analytic cases](../04-verification/analytic-cases.md)).
`tests_models/water_hammer.yaml` is the same line with 200 segments
(600 states) for comparison.

The time step is not limited by a Courant condition: the integrator is
implicit and L-stable and chooses the step from its error estimate. To see a
wave front in the results, the output interval `output_dt` must be shorter
than the transit time of the features to be seen (0.5 ms in the water hammer
example). The integrator reduces its steps while fronts travel: the water
hammer example takes 2,469 steps for 0.6 s, 0.24 ms on average against
Δx/a ≈ 1.3 ms.

Each segment adds three states; the cost of a transient grows with the number
of states and with the number of steps the wave content forces (section 7).
Do not discretise lines whose dynamics do not matter for the question.

## 2. Junction and node volumes

Every point where flow elements meet is a volume with pressure and enthalpy
states:

- An explicit `junction` has the stated `volume` (default 1 mL).
- An implicit junction (ports connected without a node-like component) gets
  half a segment volume from each pipe ending there; where no pipe ends there,
  a stub of five bore diameters per connected restriction, at least 1 mL
  (rules in the [model file standard](model-files.md#25-ports-and-connections)).
  `fluidnet check` prints each volume.

Recommendations:

1. Give manifolds, injector domes, turbine inlet and outlet plenums and
   similar cavities an explicit `junction` with the real volume. The volume
   sets the filling time and the compliance that couples the elements around
   it (`manifold`, 80 mL, in the LOX feed example; 0.3–0.5 L plenums in the
   expander and RL10 models).
2. Leave implicit junctions where elements are physically close-coupled (a
   valve bolted to an orifice). Their small volume makes the pressure there
   adjust quickly, which the implicit integrator handles, but a liquid-filled
   small volume that is closed off on both sides (trapped liquid) is very
   stiff; the step-size underflow message lists it as a typical cause.
3. A `tank` is a node with mass and energy balance and an optional heat input
   `Q`; a `boundary` imposes pressure and temperature. Use a boundary for a
   large run tank whose pressure is controlled, and a tank when its pressure
   history is part of the result (blowdown, pressurisation).
4. Volumes that are dead ends in the steady state (no through-flow) keep their
   initial state in a steady-state solve.

## 3. Tolerances and step size

The transient integrator (TR-BDF2, `crates/fluidnet-core/src/solver/transient.rs`)
controls the local error of each state against the weight

```
w = atol_scale · atol + rtol · |y|
```

with `atol` = 1 Pa for pressures, 10 J/kg for enthalpies (with |y| taken as at
least 2×10⁵ J/kg), 1×10⁻⁶ kg/s for mass flows, 0.1 rad/s for shaft speeds,
0.01 K for wall temperatures, 1×10⁻⁵ for the chamber mixture fraction and
1×10⁻⁴ for valve positions (`Network::weights` in
`crates/fluidnet-core/src/network/mod.rs`).

| Setting | Default | When to change |
|---|---|---|
| `rtol` | 1e-4 | 1e-3 for fast surveys of large models (the water hammer example uses 1e-3); 1e-5 when comparing small differences, for example against analytic solutions (`tests_models/blowdown_n2.yaml` and `tests_models/actuated_valve.yaml` use 1e-5). |
| `atol_scale` | 1 | Reduce it for models with small mass flows: at 0.01 kg/s and `rtol` 1e-4 the relative term equals the absolute flow tolerance of 1e-6 kg/s, so below about 0.01 kg/s the absolute term dominates the flow error control. Increase it only if absolute errors of 1 Pa and 10 J/kg are finer than needed. |
| `max_step` | `t_end`/20 | Set it when an event is not on a schedule time and could be stepped over: a check valve cracking, a pressure-actuated valve moving, a pressure crossing a set point. The RL10 start model uses 0.01 s. |
| `max_steps` | 2,000,000 | Lower it to make a run that cannot progress stop early instead of running for minutes. |
| `output_dt` | `t_end`/500 | Output times do not constrain the step; they are interpolated from the steps. Choose it for the plots and files you need. |

Schedule times (valve ramps, boundary steps, ignition time and the end of the
ignition ramp) are breakpoints: the integrator always lands a step on them, so
they need no small `max_step`.

The steady-state solver uses its own fixed weights and is not affected by
`rtol` or `atol_scale`.

## 4. Steady-state convergence

A steady state solves the same equations with all time derivatives zero, with
every schedule evaluated at `steady_time` (`--steady`, `mode="steady"`, the
Steady state button). With `start_from_steady: true`, a transient first solves
the steady state with the schedules evaluated at t = 0 s (not at
`steady_time`) and starts from it.

The solver (`crates/fluidnet-core/src/solver/steady.rs`) proceeds as follows;
the `method` field of the result names the path that converged:

1. Mass-flow states that are still zero get a guess: pumps at their design
   flow and `mass_flow` sources at their value, propagated through the network
   and split at branch points in proportion to the effective flow areas.
2. If the model has free shafts, they are first held at a speed guess (`N0` if
   it is greater than zero, otherwise the design speed of a pump on the shaft
   divided by its gear ratio), the problem is solved, and the shafts are then
   released (`shaft continuation (...)`).
3. Damped Newton iteration with line search (`newton`).
4. Pseudo-transient continuation (`pseudo-transient`), accepting very slow
   residual drift as quasi-steady (`pseudo-transient (quasi-steady)`, for
   example a tank draining through a closed valve's leakage).
5. Time-marching with the inputs frozen, then Newton (`time-marching + newton`).
6. Last resort: an ordinary transient through the actual input history from
   0 s to `steady_time`, then held at `steady_time` for up to 30 s, then the
   direct methods (`transient settling + ...`).

Practical rules:

1. **Steady state at the end of a sequence.** For a model whose schedules
   describe a start, set `steady_time` to a time when the sequence is
   complete. `examples/expander_start.yaml` sets `steady_time: 10` and its
   steady state is the design point; `examples/rl10a33a_start.yaml` sets
   `steady_time: 4.0`, which takes the transient settling path (45 s with
   v0.5.0).
2. **Systems without a steady state.** A closed tank that keeps filling, or a
   system whose inputs never balance, has no steady state.
   `examples/regulated_pressurization.yaml` fails at t = 0 s because the
   draw-off valve is closed and the regulator keeps filling the ullage; the
   validation test skips its steady state for this reason. Solve such models as
   transients.
3. **Oscillating systems.** If the network has an unstable mode (feed-system
   coupling, a regulator with a narrow band) the steady state may exist but the
   time-marching paths do not settle. Run the transient and look at the
   oscillation.
4. **Initial guesses.** The direct methods start from the model's initial
   state. Initial pressures and temperatures close to the operating point
   (`P0`/`T0` on junctions and pipes), `N0` near the operating speed on
   shafts, and `design_speed`/`design_mdot` values close to the operating point
   on pumps help the first steps.
5. **Warm starts.** In Python, `fn.run(model, mode="steady", init=previous)`
   starts from a converged state of a similar model and converges in a few
   iterations (expander example: 17–24 ms instead of 120–270 ms). In v0.5.0
   the shaft continuation of step 2 still runs on a warm start and first
   resets free shaft speeds to their guess (open item OI-022).
   Step through a parameter range in small increments, feeding each result
   into the next run ([Python handbook](python.md#warm-starts)).
6. **Latching valves.** In the direct methods, latching valves follow their
   command without the latch; only the transient settling path (6) replays the
   history with the latches active. If a steady state of a start model must
   keep a latched valve open, rely on `steady_time` at the end of the sequence.

When it fails, the error names the largest residual:

```
solver error: steady state did not converge (residual 1.674e5, largest in press_line.n2.P). The system may have no steady state (e.g. a closed tank that keeps filling) or may oscillate.
```

Then:

1. Check whether the system can have a steady state at `steady_time` (rule 2).
2. Run with `FLUIDNET_DEBUG=1` (CLI or Python process environment) to see the
   Newton and pseudo-transient iteration history and where the residual
   stalls.
3. Run the transient with the same inputs held constant over a long `t_end`
   and see where it goes; a converged end state can be used as a warm start.
4. Simplify: prescribe a shaft `speed` instead of solving the torque balance,
   solve, then release the shaft with the result as warm start
   (`tools/size_expander.py` solves with the speed prescribed for this
   reason).

## 5. Start transients and pressure-actuated valves

A start transient begins from the initial state of the model, not from a
steady state (`start_from_steady: false`, the default).

1. **Initial fill.** Each volume takes `P0`/`T0` where given, otherwise the
   state of the nearest specified node, propagating first through elements
   that are open at t = 0 s. A valve closed at t = 0 s therefore separates a
   liquid-filled feed line from a gas-filled downstream region. Give the
   downstream region its own `P0`/`T0` (in `examples/expander_cycle.yaml` and
   `expander_start.yaml` the jacket starts at 3 bar and 280 K with a 280 K wall).
2. **Valve sequences.** Write opening sequences as schedules, with a repeated
   time for an instantaneous change. The valve characteristic
   (`linear`, `equal_percentage`, `quick_opening`) is applied to the scheduled
   opening. Closed valves keep a leakage area fraction `leak` (default 1×10⁻⁶),
   which keeps isolated volumes connected to the network; set `leak: 0` only
   where a full closure is the point of the case (water hammer example).
3. **Ignition.** A `combustion_chamber` with `ignition_time` changes from an
   ideal mixture of the injected propellants to equilibrium combustion over
   `ignition_ramp` (default 20 ms); before that its pressure follows the cold
   flow. Without `ignition_time` the chamber burns from t = 0 s. `P0` (default 1 bar) is the chamber pressure at t = 0 s and
   `p_ambient` (default 0 bar, vacuum) the back-pressure of the nozzle.
4. **Turbopumps from rest.** Shafts start at `N0` (default 0 rpm). Pump and
   turbine maps fall back to their generic zero-speed behaviour below 20 % of
   design speed ([turbomachinery](../02-engineering/turbomachinery.md)).
   `bearing_torque` adds a speed-independent friction torque.
5. **Pressure-actuated valves.** A `valve` with `sense` gets a position state
   that follows, with first-order lag `actuator_time` (default 50 ms), a
   command linear in the sensed pressure: `opening_lo` at `sense_lo` and below,
   `opening_hi` at `sense_hi` and above. `sense` names a node-like component,
   a `component.port` node, or a chamber id (then the chamber pressure is
   sensed); with `sense_ref` the signal is the pressure difference
   `sense` − `sense_ref`. `latch: true` lets the valve move only towards
   `opening_hi` during transients (a pneumatic sequencing valve that stays
   open once actuated). The value of `opening` at t = 0 s is the initial
   position.
   `tests_models/actuated_valve.yaml` demonstrates both behaviours; the RL10
   start model sequences its oxidizer control valve and a cooldown valve on
   pump pressures and opens the thrust control valve with chamber pressure.
6. **Step size.** Actuated valves, check valves and regulators change their
   opening continuously with pressure, so these events are not breakpoints.
   Limit `max_step` to a fraction of the actuator time or of the event
   duration of interest.
7. **Checking the end state.** If a start sequence ends at a steady operating
   point, compare the last values of the transient with a steady state at
   `steady_time`. For `examples/expander_start.yaml` the validation test
   `expander_start_reaches_the_steady_design_point` requires chamber pressure,
   thrust, shaft speed and O/F to agree within 0.3 %.

## 6. Two-phase flow and cavitation

fluidnet treats all two-phase states as homogeneous equilibrium mixtures:
one pressure, one enthalpy and no slip between phases in every volume, with
properties from the fluid tables. Consequences for modelling:

1. **Restrictions.** The default flow model `auto` covers subcooled liquid with
   cavitation and flashing, two-phase inlets and real-gas nozzle flow, and sets
   `choked` when the flow is choked. `model: incompressible` uses
   √(2ρΔP) with the inlet density and is meant for single-phase liquid where
   these effects are excluded. Details in
   [flow elements](../02-engineering/flow-elements.md).
2. **Cavitating venturis.** A `venturi` uses a pressure recovery of 0.85 by
   default; its flow stays fixed while the outlet pressure stays below about
   the recovery fraction of the inlet pressure.
   `examples/cavitating_venturi.yaml` raises the back-pressure from 1 bar to
   29 bar to show the transition.
3. **Saturated inflow.** A `boundary` with `x` instead of `T` supplies a
   saturated mixture at the given quality; a `tank` with `x0` starts
   two-phase. A temperature exactly on the saturation line is ambiguous; give
   the quality instead.
4. **Column separation.** When a liquid line pressure falls to the vapour
   pressure, the volume becomes two-phase and its pressure stays at the
   saturation pressure until the vapour collapses; no special input is needed.
5. **Near the saturation lines** the compressibility of the equilibrium
   mixture jumps by orders of magnitude. The tables blend the derivatives in a
   1 % quality band to keep the integrator out of very small steps
   ([fluid properties](../02-engineering/fluid-properties.md)); expect more
   rejected steps while a node crosses a saturation line. The start of
   `examples/expander_start.yaml`, in which liquid hydrogen enters a warm
   jacket, takes 20,786 steps for 10 s with step sizes down to 1×10⁻⁹ s.
6. **Pump cavitation.** The pump head and power do not depend on suction
   conditions. `npsh` (net positive suction head available, m) is an output;
   compare it with the required NPSH of the pump.
7. **Heat transfer.** Wall heat transfer uses single-phase correlations on the
   mixture properties; there is no boiling heat transfer model, so chill-down
   transients are approximate ([heat transfer](../02-engineering/heat-transfer.md)).
8. **Table limits.** Each fluid table covers a pressure and temperature range
   (listed in the [model file standard](model-files.md#26-fluids)). Below the
   minimum table pressure the gas is extrapolated as an ideal gas; a state
   outside the other limits makes the integrator reduce its step until it
   stops with a step-size underflow.

## 7. Performance

Run times with the release CLI on the Linux test machine (v0.5.0); full table
in the [example register](examples.md):

| Model | States | Transient | Steady state |
|---|---|---|---|
| Small feed systems and test models | 2–40 | 0.3–170 ms | 0.1–5 ms |
| `lh2_heated_channel` | 62 | 45 ms (4 s simulated) | 36 ms |
| `expander_cycle` (throttling, 6 s simulated) | 63 | 154 ms | 120 ms |
| `expander_start` (10 s simulated) | 63 | 7.2 s | 157 ms |
| `water_hammer` (0.6 s, rtol 1e-3) | 150 | 0.33 s | 14 ms |
| `rl10a33a` (1 s from the initial state) | 120 | 1.9 s | 43 ms |
| `rl10a33a_start` (4 s simulated) | 123 | 10.7 s | 45 s |
| `tests_models/water_hammer` (0.6 s, rtol 1e-4) | 600 | 9.6 s | 80 ms |

What drives the cost:

- **Number of steps.** Valve events, wave fronts, ignition, crossings of
  saturation lines and tight tolerances force small steps; the start
  transients take 15,000–21,000 steps against a few hundred for smooth runs.
- **Jacobian evaluations.** The Jacobian is built by finite differences with
  column colouring; its cost is the number of colours (8–18 in the examples)
  times one right-hand-side evaluation, independent of the model size.
  `fluidnet bench` reports the cost per evaluation.
- **Linear algebra.** Models below 48 states use dense LU, larger ones sparse
  LU.
- **Front end.** The Python layer adds little to the solve time; runs in
  threads proceed in parallel. The browser engine runs the same code as
  WebAssembly; the README of v0.5.0 states it is about 2× slower than native
  (not re-measured for this page).

## 8. Reading warnings and errors

**Warnings.** In v0.5.0 the compiler issues one kind of warning:
`X: unknown parameter 'k' ignored`. It almost always means a misspelt
parameter whose intended value is not used (making it an error is open item
OI-017 in [open items](../01-project/open-items.md)). Unknown keys in `simulation` and
at the top level produce no warning; misspelt simulation settings silently
take their defaults.

**Model errors** (`model error: ...`, at compile time) name the component and
the parameter or connection; the most frequent are:

| Message | Meaning |
|---|---|
| `X: missing required parameter 'k'` | Give the parameter. |
| `X.k: parse error: '2 bar': expected a Length quantity (m), got Pressure` | Wrong unit for the parameter. |
| `port X.out is not connected` | Every port of a flow element needs a connection. |
| `a and b are connected directly; put a flow element (pipe, orifice, valve) between them` | Two node-like components share a node. |
| `fluid conflict: b uses O2 but is connected to a using N2` | One circuit, two fluids. |
| `no fluid specified for the circuit containing X` | Give `fluid` on a boundary, tank, junction or pipe of that circuit. |
| `X: no initial state (set P0/T0 on it or on a connected tank/boundary)` | A volume cannot reach any specified state. |
| `X: name a shaft or give a fixed speed` | Pump or turbine without drive. |
| `X: no combustion table for fuel F with oxidizer O (available: ...)` | Unsupported propellant pair, or fuel and oxidizer swapped on the ports. |

**Solver errors** (`solver error: ...`):

| Message | Meaning and action |
|---|---|
| `step size underflow at t = ... (h = ...)` | The integrator cannot make progress. The message lists the typical causes: a state left the fluid table, trapped liquid is compressed, or the model is inconsistent. Run with `FLUIDNET_DEBUG=1` to see which state limits the rejected steps, check the pressures and temperatures at that time against the table ranges, and check for volumes closed off on all sides. |
| `maximum number of steps (N) reached at t = ...` | The run needs more than `max_steps` steps. Either the model forces very small steps for a long time (look at `h_min_used` and the debug output) or `max_steps` was set low on purpose. |
| `singular iteration matrix` | The Newton iteration matrix could not be factorised even after a Jacobian refresh. This is not expected for a well-posed model; keep the model file and report it as an open item. |
| `steady state did not converge (...)` | Section 4. |
| `steady state: cannot evaluate the initial state` | The equations cannot be evaluated at the initial state, for example because it lies outside the fluid tables. |
| `stopped by user at t = ...` | The progress callback of the core requested a stop. The front ends of v0.5.0 do not use this path; the web interface's Stop terminates its worker instead. |

**Early stop.** A transient that fails after its first output time returns its
results up to the failure with `error` set (CLI: `STOPPED EARLY`, exit code 0;
Python: `res.error`; web interface: "Stopped early"). The data before the
failure is valid and usually shows the cause: a pressure approaching a table
limit, a volume emptying, an oscillation growing. Always check `error` in
scripts before using the last values.

## Troubleshooting

| Symptom | Section |
|---|---|
| Wave fronts look smeared or the surge period is off | 1 (segments) and 3 (`output_dt`) |
| A valve or check valve event is missed or late | 3 (`max_step`) and 5.6 |
| The steady state does not converge | 4 |
| Steady state and end of the start transient differ | 4 rule 6 (latching valves) and 5.7 |
| Many rejected steps around a phase change | 6.5 |
| The run takes much longer than the table in section 7 | 3 and 7; check `max_steps`, `rtol` and the step statistics |
| A run stops with step-size underflow | 8 |
| A parameter change has no effect | 8 (unknown parameter warning; misspelt simulation key) |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
