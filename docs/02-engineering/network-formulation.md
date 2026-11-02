# Network formulation

FNT-SLV-DOC-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Scope

This page describes how `Network::compile` turns a model into a network of
nodes and branches, which state variables the network has, the conservation
equations evaluated by `Network::rhs`, how initial conditions are assigned,
and the sparsity pattern given to the Jacobian. The flow laws of the
individual elements, the machine models, wall heat transfer and the chamber
model are summarised here only as far as they enter the network equations;
their details are in [FNT-CMP-DOC-001](flow-elements.md),
[FNT-TRB-DOC-001](turbomachinery.md), [FNT-HTX-DOC-001](heat-transfer.md) and
[FNT-CMB-DOC-001](combustion.md). The solvers that integrate the equations are
in [FNT-SLV-DOC-002](solvers.md).

## Model and equations

### Nodes and branches

The network is a lumped-parameter staggered grid. Thermodynamic state lives in
**nodes**; mass flow lives in **branches** that connect two nodes, `a` and `b`.
A positive branch flow goes from `a` to `b`. For two-port elements `a` is the
`in` port and `b` the `out` port. Each node keeps a list `conn` of
(branch, sign) pairs, with sign −1 if the node is the branch's `a` end and +1
if it is the `b` end, so `Σ sign · ṁ` is the net inflow.

There are three node kinds (`NodeKind`):

| Kind | Created by | State |
|---|---|---|
| `Volume { vol }` | `tank`, `junction`, implicit junctions, pipe-internal nodes | P and h |
| `Boundary` | `boundary` | none; P(t) and T(t) or x(t) prescribed |
| `ChamberPort { ch, neighbor }` | each inlet (`fuel`, `ox`) of a `combustion_chamber` | none; pressure is the chamber pressure, enthalpy that of the node feeding it |

The branch kinds (`BranchKind`) are:

| Kind | Created by | State | Flow law |
|---|---|---|---|
| `Pipe` | each segment of a `pipe` | ṁ | momentum equation with inertia |
| `Pump` | `pump` | ṁ | momentum equation with fluid inertance and pump pressure rise |
| `Restriction` | `orifice`, `venturi`, `valve`, `check_valve`, `regulator`, `loss` | none | quasi-steady ṁ(P_up, h_up, P_down) |
| `Turbine` | `turbine` | none | quasi-steady nozzle flow |
| `MassFlow` | `mass_flow` | none | prescribed ṁ(t) |

Components that are neither nodes nor branches are held in separate vectors on
`Network` and own their states: `shafts`, `walls` (pipe walls with heat
capacity), `chambers` and `actuators` (pressure-actuated valves).

### Grouping ports into nodes

Every port of every component gets an index. Each connection `[x, y]` unites
the two ports in a union-find structure (`Uf` in `compile.rs`). A connection
end is written `component.port`, or just `component` when the component has
exactly one port. Each resulting group of connected ports becomes one node:

1. A group that contains a port of a `combustion_chamber` becomes a
   `ChamberPort` node named `<chamber>.<port>`. The group must contain exactly
   two ports, the chamber inlet and the port of one flow element (injector
   orifice, valve or pipe); anything else is an error.
2. A group that contains the port of a `boundary`, `tank` or `junction`
   becomes that component's node, named after the component id. Two such
   components in one group are an error ("connected directly; put a flow
   element between them").
3. Any other group becomes an **implicit junction**, a `Volume` node with no
   owning component. It is named `<component>.<port>` after the first port in
   the group that does not belong to a pipe; if all ports belong to pipes, after
   the first port. "First" follows the order of components in the model file
   and the order of ports in the catalog (`in` before `out`). For example, a
   pipe whose `out` is connected to `main_valve.in` creates the node
   `main_valve.in`.

Ports of flow elements must be connected; ports of node-like components may be
left open.

### Pipe discretisation

A `pipe` with length L, diameter d, `parallel` channels n_p and N `segments`
(clamped to 1…1000) has the flow area A = n_p π d²/4 and the segment length
ΔL = L/N. The compiler creates:

- N − 1 internal `Volume` nodes `<pipe>.n1 … <pipe>.n(N−1)`, each with the
  volume A ΔL;
- N `Pipe` branches `<pipe>.s1 … <pipe>.sN` (a single-segment pipe is one
  branch named after the pipe), chained from the inlet node through the
  internal nodes to the outlet node;
- a contribution of A ΔL / 2 to the volume of each end node, if that node is a
  `Volume`.

The internal volumes and the two half cells add up to the pipe volume A L.
Because the end cells are added to whatever volume sits at the pipe ends, a
pipe–orifice–pipe chain needs no explicit junction: the implicit junction
between pipe and orifice holds the half cell of the pipe. The fittings
coefficient K and the elevation change Δz are divided equally over the
segments (K/N and Δz/N per segment).

### Volume of implicit junctions

After all branches exist, every `Volume` node's volume is its own volume
(`tank` or `junction` parameter `volume`; zero for an implicit junction) plus
the pipe half cells. If the result is zero, which happens where restrictions,
pumps or turbines meet without a pipe, the compiler assigns a stub volume: for
each connected element a flow area a and a length of five bore diameters,
V = Σ a · 5 √(4a/π), with a = CdA/0.8 for restrictions and turbines and a
fixed 1×10⁻⁵ m² for pumps. The result is at least 1×10⁻⁶ m³ (1 mL,
`DEFAULT_JUNCTION_VOLUME`).

### Fluid assignment

Nodes are united over all branches (pumps and turbines included; chambers are
not branches), and each connected circuit must name exactly one fluid. The
fluid comes from the `fluid` parameter of a boundary, tank or junction, or
from a pipe's optional `fluid` parameter (which applies to its internal nodes
and to end nodes without a fluid). Different fluids in one circuit are an
error; a circuit without a fluid is an error. Fluid names are matched to the
loaded tables by key or label, ignoring case. A chamber's fuel and oxidizer
circuits are separate circuits and select the combustion table by their two
fluids.

### State variables

The compiler lays out the state vector in this order (state names in
brackets):

| States | Count | Name |
|---|---|---|
| P [Pa] and h [J/kg] of every `Volume` node | 2 per node | `<node>.P`, `<node>.h` |
| ṁ [kg/s] of every pipe segment and pump | 1 per branch | `<branch>.mdot` |
| ω [rad/s] of every shaft without prescribed speed | 1 per shaft | `<shaft>.speed` |
| T_w [K] of every wall element | 1 per element | `<pipe>.w<i>.T` |
| P_c [Pa] and oxidizer mass fraction Z [–] of every chamber | 2 per chamber | `<chamber>.Pc`, `<chamber>.Z` |
| valve position u [–] of every pressure-actuated valve | 1 per valve | `<valve>.position` |

`StateKind` records the kind of each state; it selects the tolerance weights
and finite-difference perturbations (see [FNT-SLV-DOC-002](solvers.md)). The
initial values are: (P, h) from the initial-condition propagation below;
ṁ = 0 kg/s; ω = `N0`; T_w = `wall_T0` or the initial fluid temperature of the
wall's node (290 K if the node has none); P_c = `P0` (at least 100 Pa) and
Z = 0.5; u = the valve's `opening` at t = 0 s.

### Volume nodes: mass and energy

Each `Volume` node is a rigid volume V in homogeneous equilibrium. With the
property derivatives from the fluid table,

```
V (∂ρ/∂P)_h dP/dt + V (∂ρ/∂h)_P dh/dt = Σ ṁ_in − Σ ṁ_out                       (mass)
ρ V dh/dt − V dP/dt = Σ_inflows ṁ (h_up − h) + Q + Σ W_machine                  (energy)
```

h_up is the enthalpy of the node at the other end of an inflowing branch;
outflows leave with the node enthalpy h and therefore do not appear in the
energy equation. Q is the heat added to the node (tank heat input, pipe-wall
heat, wall elements; see [FNT-HTX-DOC-001](heat-transfer.md)). W_machine is the
power a pump adds to or a turbine removes from the fluid. It is assigned to the
node that receives the machine's flow, with a smooth split near zero flow:
the `b` node receives the fraction ½ (1 + tanh(ṁ / 1×10⁻⁴ kg/s)) and the `a`
node the rest.

The two equations are solved for the derivatives in closed form
(`Network::rhs`):

```
det   = V (ρ (∂ρ/∂P)_h + (∂ρ/∂h)_P) = V ρ (∂ρ/∂P)_s
dP/dt = (ρ M − (∂ρ/∂h)_P E) / det
dh/dt = (M + (∂ρ/∂P)_h E) / det
```

with M the net mass inflow and E the right-hand side of the energy equation.
det is positive for any thermodynamically stable state; its absolute value is
limited to at least 1×10⁻³⁰⁰ to avoid a division by zero. Using (P, h) as
states keeps one formulation for liquid, gas, supercritical and two-phase
nodes, and the derivatives come from the tables, including the two-phase
derivatives (see [FNT-FLD-DOC-001](fluid-properties.md)).

### Pipe segments: momentum

Each pipe segment between nodes a and b carries the momentum equation

```
(ΔL/A) dṁ/dt = P_a − P_b − ΔP_f(ṁ) − ρ̄ g Δz_s
ΔP_f(ṁ)      = (f ΔL/d + K_s) ṁ |ṁ| / (2 ρ̄ A²)
```

with f the Churchill Darcy friction factor at Re = |ṁ| d / (A μ̄), K_s = K/N,
Δz_s = Δz/N and g = 9.80665 m/s². ρ̄ is the reciprocal of the mean specific
volume of the two end nodes, ρ̄ = 2 / (v_a + v_b), and μ̄ the arithmetic mean
of their viscosities. The convective acceleration term is not included. See
[FNT-CMP-DOC-001](flow-elements.md) for the friction factor.

### Pumps: momentum

A pump is a branch with a mass-flow state and a fluid inertance I (parameter
`inertance`, L/A in 1/m):

```
I dṁ/dt = P_in − P_out + ρ_in g H(ω, ṁ)
```

The head H comes from the pump map ([FNT-TRB-DOC-001](turbomachinery.md)).

### Quasi-steady branches

Restrictions, turbines and mass-flow sources have no state; their flow is an
algebraic function of the current node states, evaluated in
`Network::evaluate`:

```
ṁ = sign · CdA · u · G(state_up, P_down)
```

The upstream side is the node with the higher pressure, sign is +1 for flow
from a to b, u is the opening (1 for fixed restrictions) and G the mass flux
from `flow::restriction_flux`. Turbines use the same flux with their nozzle
area times an optional area-map factor. A `mass_flow` branch imposes ṁ(t) and
carries the upstream enthalpy like any other branch. Because these branches
have no inertia, a chain of restrictions between two volumes responds
instantaneously to the volume pressures.

### Boundaries

A `Boundary` node has a prescribed pressure P(t) and either a temperature T(t)
or a quality x(t) (x takes precedence if both are given). Its state is
(P, h(P, T)) or (P, h_l + x (h_v − h_l)), evaluated with `boundary_props`. A
boundary supplies fluid at its own state and absorbs outflow at any state;
it has no equations. A boundary whose schedules are all constant is evaluated
once during compilation (`fixed`). A boundary pressure ≤ 0 Pa is an error.

### Chamber ports

A `ChamberPort` node has the chamber pressure P_c (limited to at least 1 Pa)
and the enthalpy of the node on the other side of its single flow element
(`neighbor`), evaluated with the port's own fluid. The injector element
therefore sees the chamber pressure as its downstream pressure and carries the
feed enthalpy. The flows of the branches at a port enter the chamber
equations instead of a node balance ([FNT-CMB-DOC-001](combustion.md)).

### Other state equations

| Element | Equation | Page |
|---|---|---|
| Shaft | J dω/dt = Σ r_i τ_turbine,i − Σ r_i τ_pump,i − τ_b tanh(ω / 1 rad/s) − c ω | [FNT-TRB-DOC-001](turbomachinery.md) |
| Wall element | m c(T_w) dT_w/dt = Q_hot − Q_cool | [FNT-HTX-DOC-001](heat-transfer.md) |
| Chamber | dP_c/dt = (RT/V)(ṁ_f + ṁ_o − ṁ_t); m dZ/dt = ṁ_o (1 − Z) − Z ṁ_f | [FNT-CMB-DOC-001](combustion.md) |
| Actuated valve | du/dt = (u_cmd − u)/τ, one-way if latched | [FNT-CMP-DOC-001](flow-elements.md) |

During the steady-state shaft continuation a shaft may be held at a speed
ω_hold, in which case its equation is replaced by dω/dt = (ω_hold − ω) /
1×10⁻³ s ([FNT-SLV-DOC-002](solvers.md)).

## Numerical treatment

### Initial-condition propagation

Every `Volume` node needs an initial (P, h). The compiler seeds a
breadth-first search with all boundaries (their state at t = 0 s) and all
nodes that have `P0` together with `T0` or `x0` (tanks must give `P0`;
junctions and pipes may). A pipe's `P0`/`T0` applies to its internal nodes and
to end nodes that are implicit junctions without their own initial state.

The search copies (P, h) from a node to each unassigned neighbour. It first
crosses only elements that are open at t = 0 s; a branch counts as closed if it
is a valve with `opening(0) ≤ 0`, an actuated valve with initial position
≤ 0, any check valve, or any mass-flow source. When the open search is
exhausted, the first node that was stopped at a closed element passes its
state to all its unassigned neighbours, across closed elements included, and
the open search resumes from them. This repeats until no stopped node is
left. Chamber ports are never
assigned. A volume node that remains without a state is an error asking for
`P0`/`T0` on it or on a connected tank or boundary.

This rule separates, for example, a LOX-filled line from a GOX-filled manifold
behind a closed valve: each side takes the state of its own tank or boundary.
The propagated state is a copy of pressure and enthalpy, not of temperature.

The same initial states also fix values that depend on them: pump design
density (if `design_density` is not given), wall initial temperatures, and the
schedule breakpoints. The breakpoints of all schedules (boundary P, T, x;
valve openings; mass flows; regulator set pressures; tank heat; shaft and
machine speeds; pipe-wall T, q and hot-gas schedules; chamber ambient
pressure; ignition time and end of ignition ramp) are collected, restricted to
t > 0 s, sorted and de-duplicated within 1×10⁻¹² s.

### Evaluation order

`Network::evaluate(t, y, ev)` fills the workspace `Eval` in dependency order:

1. Properties of `Volume` nodes from (P, h) and of `Boundary` nodes from their
   schedules.
2. Properties of `ChamberPort` nodes from P_c and the neighbour's enthalpy.
3. Shaft speeds (state or prescribed schedule).
4. Branch flows; for pumps also power, torque, head, efficiency and NPSH; for
   turbines also torque, power, efficiency, velocity ratio and pressure ratio;
   for restrictions also opening and choked flag.
5. Combustion chambers (needs the port flows and port enthalpies from steps 2
   and 4).
6. Heat: tank heat inputs and prescribed-temperature or heat-flux pipe walls
   per node, then wall elements with heat capacity (needs the chamber state
   from step 5 for hot-gas walls). The coolant-side heat of each wall element
   is added to its node.

`Network::rhs(t, y, dy, ev)` calls `evaluate` and then computes, in order, the
pipe and pump momentum derivatives, the volume node derivatives, shaft, wall,
chamber and actuator derivatives. `rhs` returns an error only if a boundary
state cannot be evaluated.

### Flow guess for cold steady-state starts

`Network::flow_guess` provides starting values for zero mass-flow states before
a steady-state solve ([FNT-SLV-DOC-002](solvers.md)). Pumps are seeded with
their design volume flow times the current inlet density, mass-flow sources
with their scheduled flow; restrictions whose opening is ≤ 1×10⁻⁴ are fixed at
zero. The guess is then propagated over the volume nodes by mass balance: at
each node with unknown branch flows, the known net inflow is split over the
unknown branches in proportion to a weight (pipe flow area, CdA × opening for
restrictions, CdA for turbines, 1 for pumps and sources). The pass repeats
until nothing changes. Only pipe-segment and pump flow states are returned.

### Sparsity pattern

The finite-difference Jacobian uses the structural pattern returned by
`Network::sparsity`. It is built from (reads, writes) terms: each equation
contribution declares the states it reads and the state rows it writes, and
every read column gets every written row. The terms are:

| Contribution | Reads | Writes |
|---|---|---|
| `Volume` node | own P, h; for a prescribed-temperature wall also the reads of its adjacent pipe branches | own P, h rows |
| Branch | states of both end nodes (for a chamber port: P_c, Z and the neighbour's states); own ṁ (pipe, pump); shaft speed (pump, turbine); actuator position (actuated valve) | rows of both end nodes (for a chamber port: P_c, Z); own ṁ row; shaft row (pump, turbine) |
| Shaft | own ω | own ω |
| Chamber | P_c, Z, states of both port nodes | P_c, Z |
| Actuator | own u, sensed node states (or P_c), reference node states | own u |
| Wall element | own T_w, node states, reads of its adjacent pipe branches; for a chamber-heated wall also P_c, Z, port node states and the reads of the branches at both ports | own T_w, node rows |

A new component that couples states beyond its two end nodes must extend
`sparsity()`, otherwise the Jacobian misses entries. The CLI command
`fluidnet jaccheck MODEL` compares the coloured Jacobian with a full
column-by-column finite difference and reports the worst relative difference,
which detects missing entries.

## Parameters and outputs

The parameters that shape the network are the connections, pipe `segments`,
`parallel`, `length`, `diameter`, `K`, `dz`, node `volume`, `fluid`, and the
initial conditions `P0`, `T0`, `x0`. They are listed with defaults in the
[Component reference](../03-user-guide/component-reference.md).

Every node except chamber ports produces the channels `.P`, `.T`, `.rho`, `.h`
and `.x` (quality clamped to 0…1, NaN above the critical pressure); tanks also
produce `.m` (mass, kg). Every branch produces `.mdot`. Multi-segment pipes
produce `.mdot_in`, `.mdot_out`, `.u_in` (inlet velocity with upstream
density) and `.dp` over the whole pipe. `fluidnet check MODEL` lists all nodes
with fluid and volume and all branches with their end nodes.

## Limits and assumptions

- Homogeneous equilibrium in every volume: no slip, no thermal
  non-equilibrium, no stratification.
- Rigid volumes; wall compliance is not modelled, so the wave speed in a
  liquid line is the fluid's sound speed.
- One fluid per connected circuit; species mix only in a combustion chamber.
- Energy equation without kinetic or potential energy; inflows carry the
  static enthalpy of the upstream node.
- Pipe momentum without the convective acceleration term; the pressure
  difference is between static node pressures.
- Mass flow through restrictions, turbines and mass-flow sources is
  quasi-steady; inertia is carried only by pipe segments and pumps.
- Initial-condition propagation copies (P, h); a node far from its source does
  not get a hydrostatic or temperature profile.

## Verification

- Mass and energy conservation in a heated line:
  `heated_pipe_closes_energy_balance`; jacket, pump and chamber balances of the
  expander cycle: `expander_cycle_conserves_energy_and_torque`
  (`crates/fluidnet-core/tests/validation.rs`).
- Pipe inertia and wave propagation on the staggered grid: Joukowsky surge and
  4L/a period, `water_hammer_matches_joukowsky`.
- Isentropic gas volume discharge: `n2_blowdown_matches_isentropic_ideal_gas`.
- Case descriptions and results: [Analytic verification
  cases](../04-verification/analytic-cases.md); system level:
  [RL10A-3-3A validation](../04-verification/rl10.md).

## Code location

| Topic | Location |
|---|---|
| Node, branch, state kinds | `crates/fluidnet-core/src/network/mod.rs`: `NodeKind`, `BranchKind`, `StateKind`, `Network` |
| Port grouping, node naming, pipe discretisation | `crates/fluidnet-core/src/network/compile.rs`: `Network::compile` (sections "ports and connectivity", "nodes from port groups", "branches, pipe-internal nodes, walls") |
| Junction stub volumes | `compile.rs`, loop after "adjacency" |
| Fluid assignment | `compile.rs`, section "fluid assignment over connected regions" |
| Initial-condition propagation | `compile.rs`, section "initial (P, h) per node" |
| State layout | `compile.rs`, section "state vector" |
| Evaluation | `network/mod.rs`: `Network::evaluate`, `boundary_props` |
| Equations | `network/mod.rs`: `Network::rhs` |
| Flow guess | `network/mod.rs`: `Network::flow_guess` |
| Sparsity | `network/mod.rs`: `Network::sparsity`, `branch_reads`, `node_states`, `node_rows` |
| Output channels | `compile.rs`: `build_channels`; `network/mod.rs`: `Network::probe` |

## Open items

- The stub volume of an implicit junction next to a pump uses a fixed flow
  area of 1×10⁻⁵ m². The code intends to use the pump's design volume flow,
  but `map.q_d` is still zero at that point of the compilation (it is set
  after the initial-condition propagation).
- Wall elements are created at both pipe ends even when an end node is a
  boundary; their coolant-side heat is added to the boundary node, where it
  does not enter any balance.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
