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

**Jacobian** (`jacobian.rs`): finite differences on the structural pattern from
the network graph, with distance-2 column colouring (typically 8–10 RHS
evaluations regardless of size). The base point is always evaluated fresh.

**Linear algebra** (`linsys.rs`): dense LU below 48 states, faer sparse LU
(partial pivoting, fill-reducing ordering, symbolic analysis reused) above.

**Steady state** (`steady.rs`), same equations with d/dt = 0:
1. Damped Newton with backtracking line search;
2. pseudo-transient continuation (implicit Euler in pseudo-time, step grown by
   switched evolution relaxation and geometrically), which damps stiff and
   acoustic modes;
3. time-marching with the transient integrator and inputs frozen at t, followed
   by Newton polishing.
Dead-end volumes keep their initial states. A system that is closed at t = 0
has no unique steady state and is reported as such.

**Initial conditions**: nodes without P0/T0 take the state of the nearest
specified node, propagating first through elements that are open at t = 0, so a
closed valve separates (for example) a LOX-filled line from a GOX-filled manifold.

## 4. Front ends

* `fluidnet-cli`: run / check / catalog / props; CSV and JSON output.
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

Planned rotating machinery follows this pattern: a pump is a branch whose ΔP
comes from a head map at shaft speed ω; a turbine is a branch with a flow
parameter map; a `shaft` component owns ω as a state with
I dω/dt = Σ τ_turbine − Σ τ_pump − τ_loss, coupling the branches through the
sparsity pattern.
