# fluidnet architecture

## 1. Formulation

A model is a set of components connected port to port. The compiler
(`network.rs`) turns it into a network of **nodes** and **branches**:

* Every group of connected ports becomes a node. A group containing a `tank`,
  `junction` or `boundary` becomes that node; otherwise an implicit junction is
  created and named after one of its ports.
* A `pipe` with N segments creates N−1 internal volume nodes and N momentum
  branches. Its end cells contribute half a segment volume each to the nodes at
  its ports, so a pipe–orifice–pipe chain needs no explicit junction.
* Restrictions (orifice, venturi, valve, check valve, regulator, loss) and
  mass-flow sources become algebraic branches.

### States

| Element | States | Equation |
|---|---|---|
| Volume node (V) | P, h | mass and energy balance |
| Pipe segment (L, A) | ṁ | momentum with inertia |
| Restriction | – | quasi-steady ṁ(P_up, h_up, P_down) |
| Boundary | – | prescribed P(t), T(t) or x(t) |

Volume node, rigid, homogeneous equilibrium:

```
V (∂ρ/∂P)_h dP/dt + V (∂ρ/∂h)_P dh/dt = Σ ṁ_in − Σ ṁ_out                    (mass)
ρ V dh/dt − V dP/dt               = Σ ṁ_in (h_in − h) + Q                    (energy)
```

Solved for dP/dt and dh/dt with the determinant V²ρ(∂ρ/∂P)_s, which is positive
for any stable state. Using (P, h) as states keeps one formulation for liquid,
gas, supercritical and two-phase nodes; the property derivatives come straight
from the tables.

Pipe segment:

```
(L/A) dṁ/dt = P_a − P_b − (f L/D + K) ṁ|ṁ| / (2 ρ̄ A²) − ρ̄ g Δz
```

with Churchill's friction factor (continuous from laminar to fully rough), ρ̄ the
mean of the two end nodes, and energy carried with the upstream enthalpy.

### Restrictions

`flow.rs::restriction_flux` gives the mass flux through the effective area
(cd·A·opening) from the upstream state:

* **Subcooled liquid** (also supercritical pressure below Tc): Leung's omega
  method with the subcooled-inlet extension. ω_s is the API 520 Annex C
  two-point value from an isentropic flash of saturated liquid from P_sat to
  0.9 P_sat. If the downstream pressure stays above P_sat this reduces to
  √(2ρΔP); at high subcooling it reduces to the cavitating limit √(2ρ(P₀−P_v));
  near saturation it chokes like HEM.
* **Two-phase inlet**: omega method with two-point ω from the inlet isentrope;
  choking found by maximising the omega mass flux.
* **Gas and supercritical fluid**: isentropic nozzle with the real-gas
  isentropic exponent κ = 1/ω = −(v/P)(∂P/∂v)_s; choked below the critical ratio.
  Blended with the two-phase model across x = 0.97–1.03.
* **Pressure recovery** r (venturi): the throat pressure used for the flux is
  P₀ − (P₀ − P_down)/(1 − r), which sets where cavitation or choking begins.
* Near ΔP = 0 every law is multiplied by √|ΔP| / (ΔP² + ε²)^¼ (ε = 10 Pa +
  1e-5 P), which keeps it smooth with finite slope through flow reversal.

Valves scale the area by the characteristic of the scheduled opening; check
valves and regulators compute the opening from ΔP or the outlet pressure with a
smoothstep, so there are no discrete events.

### Heat transfer

Pipe walls with prescribed temperature use Gnielinski (blended with the laminar
Nu = 3.66) on the node's mixture properties; a prescribed flux adds q·A_wall.
Tanks take a scheduled heat input.

### Turbomachinery (`machines.rs`)

Pump: with n = N/N_d and q = Q/Q_d, head H = H_d (a n|n| + (1 − a) q|q|) and
shaft power P = P_d (p0 n²|n| + (1 − p0) n|n| q), a = shutoff head ratio,
p0 = shutoff power ratio. Efficiency follows from the two and equals η_d at the
design point; torque P/ω is evaluated without dividing by ω. The pump is a
branch with a mass-flow state: I dṁ/dt = P_in − P_out + ρ g H. All shaft power
enters the fluid enthalpy (hydraulic work plus losses).

Turbine: nozzle flow from the real-gas isentropic flux, spouting velocity
c0 = √(2 Δh_s) from the table isentrope, blade speed u = ω r_m. A parabolic
efficiency η(u/c0) peaking at η_d at x_d corresponds to a torque linear in blade
speed, τ = ṁ r_m (η_d/x_d)(c0 − u/(2 x_d)): finite stall torque at standstill
and runaway at x = 2 x_d. The extracted power leaves the fluid enthalpy.

Shaft: J dω/dt = Σ ratio·τ_turbine − Σ ratio·τ_pump − τ_bearing tanh(ω) − c ω.

### Walls with heat capacity

Each pipe chain node may carry a wall element: m c(T) dT_w/dt = Q_hot − Q_cool.
The heat capacity follows the Debye model scaled to the 300 K value (copper's
c drops by about 4 between 300 K and 50 K, which governs chill-down). Conduction
through half the wall thickness is in series with each side. Coolant side:
Gnielinski with an optional (T_b/T_w)^n property correction (0.55 for
supercritical hydrogen) and a fin/area factor. Hot side: Bartz with the chamber's
current Pc, c*, frozen γ, molar mass and temperature, the local area ratio of the
element (from `hot_contour`, hot area per element in proportion to the local
radius) and the wall-temperature correction σ; or a prescribed gas temperature
and HTC; or a heat flux. The default driving potential is enthalpy based:
q = h_g/c_p,frozen · (h_c − h_eq(T_w)), where h_c − h_eq(T_wall,ref) and the
near-wall equilibrium c_p are tabulated with the combustion tables. It includes
the heat released when dissociated species recombine at the cold wall, which
makes the heat flux fall with mixture ratio on the fuel-rich side (−4 % from O/F
5.63 to 4.67 for LOX/LH2), where c_p,frozen (T_c − T_w) rises 3 %. On the RL10 this
brings the turbine inlet temperature from 5–7 % to within 0.7 % of the test data.

Pump and turbine maps: generic (affinity laws from one design point, impulse
turbine parabola) or tabulated: ψ(φ) and η/η_d(φ) for pumps, converted to a
power coefficient anchored at the shutoff power so the map holds at zero flow,
and blended into the generic zero-speed behaviour below 20 % speed; an
efficiency speed correction η(N/N_d) divides the shaft torque (held at its 20 %
value below that speed and faded out towards 5 %); η/η_d(u/c0) for turbines,
with torque ṁ r c0 η / (2 u/c0), finite at stall, and a nozzle flow-area factor
bilinear in u/c0 and pressure ratio (held at the table edges).

### Combustion chamber (`combustion.rs`)

States Pc and the oxidizer mass fraction Z of the chamber gas:

```
dPc/dt = (R T / V) (ṁ_f + ṁ_o − ṁ_throat)          R T = (c* Γ(γ))²
m dZ/dt = ṁ_o (1 − Z) − Z ṁ_f                      m = Pc V / (R T)
```

c*, T_c, M and γ come from equilibrium tables (O/F × Pc) generated with Cantera
(`tools/gen_combustion_tables.py`): HP equilibrium from reactants at reference
injection states, shifting-equilibrium expansion to the sonic throat. First-order
sensitivities dc*/dh and dT_c/dh correct for the actual injected enthalpy (warm
hydrogen from a jacket). Before ignition the chamber holds an ideal-gas mixture of
the propellants; a smooth ramp blends to burning gas. Throat flow is choked or
subsonic; thrust from the ideal thrust coefficient with the throat isentropic
exponent and an efficiency factor. The injector circuits see the chamber as a
pressure node (`NodeKind::ChamberPort`) carrying their own fluid's enthalpy.

## 2. Fluid properties (`props.rs`, `tools/gen_fluid_tables.py`)

Tables are generated offline with CoolProp on a (log₁₀P, h) grid (~143 × 315),
storing v, T, s, μ, k, c_p, (∂v/∂P)_h and (∂v/∂h)_P, plus a saturation table.
Two-phase derivatives come from CoolProp's analytic `first_two_phase_deriv`.

Lookup is saturation-aware below the critical pressure:

* inside the dome: exact lever rule on the saturation table, with (∂v/∂P)_h from
  the slopes of the saturation curves;
* liquid and vapour: values on the two bracketing pressure rows are taken at the
  same subcooling (or superheat) relative to each row's saturation point, with
  that saturation point as the interpolation end point, so liquid and vapour
  data are never mixed across the saturation line. v is interpolated in P by
  cubic Hermite using the stored derivative (exact for ideal gas behaviour), the
  rest linearly.

Above the critical pressure the plain (log P, h) bilinear/Hermite scheme is used.
Below the table's minimum pressure, properties extrapolate as an ideal gas at
constant h (ρ ∝ P), which keeps evacuated chambers and lines smooth down to
vacuum. Within a 1 % quality band around both saturation lines the derivatives
(∂v/∂P)_h and (∂v/∂h)_P are blended: the equilibrium compressibility jumps by
orders of magnitude at x = 0, and a cavitating liquid line would otherwise force
the integrator into very small steps. Density and enthalpy remain exact.
Accuracy against CoolProp (random states, p99): density 2–4×10⁻⁴ single phase,
~1×10⁻³ on the saturation lines, ~2×10⁻⁴ two-phase; temperature < 0.1 K.
Lookup cost ≈ 0.25 µs.

## 3. Solvers (`solver/`)

**Transient: TR-BDF2** (`transient.rs`). L-stable, second order, one-step: a
trapezoidal stage to t + γh and a BDF2 stage to t + h, γ = 2 − √2, sharing the
iteration matrix I − (γ/2)hJ. Local error from the embedded third-order weights,
filtered through the same matrix. Steps land exactly on schedule breakpoints
(valve times, ramps). Modified Newton with convergence judged on the stage
residual, so a stale iteration matrix cannot accept an unconverged stage. The
matrix is refactored only when h changes by more than the hysteresis band or the
Jacobian is refreshed.

**Jacobian** (`jacobian.rs`): finite differences on the structural pattern,
with distance-2 column colouring (typically 8–16 RHS evaluations regardless of
size). The pattern is built from (reads, writes) terms: every branch, node,
wall, shaft and chamber contribution declares the states it reads and the
equations it writes, so new components only extend `sparsity()`. The base point
is always evaluated fresh.

Newton convergence uses the Hairer–Wanner rate estimate on the corrections,
plus acceptance once the stage residual is below 0.1 of the tolerance (iterates
that straddle property kinks would otherwise cycle). The Jacobian is refreshed on
convergence failure or slow convergence, at schedule breakpoints and every 40
steps, not on error-test failures.

**Linear algebra** (`linsys.rs`): dense LU below 48 states, faer sparse LU
(partial pivoting, fill-reducing ordering, symbolic analysis reused) above.

**Steady state** (`steady.rs`), same equations with d/dt = 0:
1. Damped Newton with backtracking line search;
2. pseudo-transient continuation (implicit Euler in pseudo-time, step grown by
   switched evolution relaxation and geometrically), which damps stiff and
   acoustic modes;
3. time-marching with the transient integrator and inputs frozen at t, followed
   by Newton polishing;
4. last resort: an ordinary transient from the initial state through the actual
   input history up to t (e.g. an engine start sequence), then held at t until
   the rates are small, then Newton. This replay keeps latching valves latched
   and polishes from the latest state if a small limit cycle (e.g. a feed-line
   oscillation) never meets the rate tolerance.
Flow states left at zero get a guess first: pumps at their design flow (and
imposed mass flows), propagated through the network by mass balance and split
at branch points in proportion to the effective flow areas. This lets Newton
converge from a cold initial state of a turbopump cycle (RL10 model: 15 s to
20 ms). A warm start from an earlier result (`init=` in Python) skips all of it.
Free shafts are first held at a speed guess (N0 or the design speed of their
pumps) and released once that problem converges (continuation), which finds the
operating point of turbopump cycles from cold initial states. Residuals below
1e-3 of the steady weights per second (e.g. a tank draining through a closed
valve's leakage over days) are accepted as quasi-steady. `steady_time` sets the
time at which schedules are evaluated. Dead-end volumes keep their initial states.

**Pressure-actuated valves**: a valve with `sense` gets a position state with
first-order lag toward a command linear in the sensed pressure (or difference
of two pressures; a chamber id senses Pc). `latch` makes the motion one-way in
transients (pneumatic sequencing valves); the direct steady methods ignore the
latch.

**Initial conditions**: nodes without P0/T0 take the state of the nearest
specified node, propagating first through elements that are open at t = 0, so a
closed valve separates (for example) a LOX-filled line from a GOX-filled manifold.

## 4. Front ends

* `fluidnet-cli`: run / check / catalog / props; CSV and JSON output.
* `fluidnet-py` + `python/fluidnet`: PyO3 module returning metadata JSON plus a
  float64 block; the Python layer exposes numpy arrays, dict-based model editing,
  threaded parameter sweeps (the solver releases the GIL) and plotting.
* `fluidnet-wasm`: C ABI (`fn_load_fluid`, `fn_catalog`, `fn_check`, `fn_run`),
  results as JSON metadata plus one f64 block (time, then each channel), so large
  transients avoid JSON number encoding. No wasm-bindgen.
* `ui/`: React Flow schematic, inspector generated from the component catalogue
  (`catalog.rs` is the single source of truth for ports, parameters, units and
  defaults), uPlot charts with synced cursors, line profiles, values table. The
  engine runs in an inline web worker; Stop terminates and restarts it.

## 5. Adding a component

1. Add a `CompSpec` in `catalog.rs` (ports, parameters with dimensions, defaults,
   help). The UI picks it up automatically.
2. In `Network::compile`, read the parameters with `Params` and create nodes
   and/or branches. Prefer existing branch kinds; add a `BranchKind` variant for
   new flow laws, or new state kinds for components with their own dynamics.
3. Evaluate it in `Network::evaluate` / `rhs`; extend `sparsity()` if it couples
   states beyond its two end nodes.
4. Add output channels in `build_channels`, a symbol in `ui/src/symbols.tsx`, and
   a validation test.

Components that are neither nodes nor branches (shafts, chambers, walls) live in
their own vectors on `Network`, own their states, and are evaluated in
`evaluate()` in dependency order: nodes, chamber ports, shaft speeds, branch
flows and machine torques, chambers, walls and heat.
