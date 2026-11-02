# Adding a component

FNT-DEV-HBK-002 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

## Purpose

This handbook takes a developer through adding a component type to fluidnet,
from the catalog entry to the web interface symbol, the tests and the
documentation. The worked example is the existing `check_valve`: every excerpt
below is the v0.5.0 code that implements it, so the steps can be followed in
the repository. Where a new component needs more than a check valve does (its
own state variables, a new flow law, extra couplings), the step says which
existing component to follow.

## Prerequisites

- A working build and test setup ([build, test and release](build-and-release.md)).
- The [code and repository conventions](conventions.md), in particular naming
  (2.5), tests (2.6) and documentation updates (2.7).
- The network formulation: nodes with pressure and enthalpy states, branches
  carrying mass flow between two nodes
  ([network formulation](../02-engineering/network-formulation.md)).

## Steps

### 1. Decide what kind of element the component is

The compiler turns every component into network elements. Choose the smallest
change that represents the new component:

| Kind | Existing examples | What has to be added |
|---|---|---|
| Restriction with a new opening law (algebraic flow from the end states) | `valve`, `check_valve`, `regulator` | A `Ctrl` variant; no new states |
| Algebraic branch with a new flow law | `mass_flow`, `turbine` | A `BranchKind` variant evaluated in `evaluate` |
| Branch with inertia (mass-flow state) | `pipe` segments, `pump` | A `BranchKind` variant with a state index, derivative in `rhs` |
| Node-like component | `tank`, `junction`, `boundary` | Entries in `NODE_LIKE`; node creation in `compile` |
| Component with its own dynamics | `shaft`, `combustion_chamber`, pipe walls, pressure-actuated valve actuators | Its own vector on `Network`, a `StateKind`, evaluation in `evaluate`, derivatives in `rhs`, weights, sparsity terms |

A check valve is a restriction whose open area depends on the differential
pressure, so it is a new `Ctrl` variant of the existing
`BranchKind::Restriction` and reuses the restriction flux of `flow.rs` for all
fluids and phases. Its physics is described in
[flow elements](../02-engineering/flow-elements.md#check-valve).

### 2. Add the catalog entry (`catalog.rs`)

The catalog declares the type, its ports and every parameter with dimension,
display unit, default and help text. The compiler reads defaults from it, the
web interface builds its palette and inspector from it, and the
[component reference](../03-user-guide/component-reference.md) is generated
from it. The check valve (`specs()` in `crates/fluidnet-core/src/catalog.rs`):

```rust
let mut cv = area_params("0.7");
cv.extend([
    q(
        "cracking_dp",
        "Cracking pressure",
        Dim::Pressure,
        "bar",
        Some("0.2 bar"),
        "Differential pressure at which the valve starts to open.",
    ),
    q(
        "full_open_dp",
        "Full-open pressure",
        Dim::Pressure,
        "bar",
        Some("0.7 bar"),
        "Differential pressure at which the valve is fully open.",
    ),
    q(
        "leak",
        "Leakage",
        Dim::None,
        "-",
        Some("1e-6"),
        "Closed-position area fraction.",
    ),
    restriction_model(),
]);
v.push(CompSpec {
    kind: "check_valve",
    label: "Check valve",
    category: "Restrictions",
    ports: &["in", "out"],
    node_like: false,
    help: "Opens with forward differential pressure, closes on reverse.",
    params: cv,
});
```

Points to observe:

1. Shared parameter groups come from helpers: `area_params(cd_default)` gives
   `diameter`, `area` and `cd`; `restriction_model()` gives `model`
   (`auto`/`incompressible`). Other helpers are `init_params()` (`P0`, `T0`,
   `x0`), `drive_params()` (`shaft`, `speed`, `ratio`) and `fluid_param()`.
2. `q(key, label, dim, display_unit, default, help)` declares a quantity.
   `default: None` makes it required; `.opt()` makes it optional without a
   default; `.sched()` allows a time table. The default is written as in a
   model file (`"0.2 bar"`).
3. `category` places it in the palette (Boundaries, Volumes, Lines,
   Restrictions, Machines, Engine). Two-port flow elements use the ports
   `in` and `out`; the compiler's branch loop relies on these names (step 3).
4. The help texts appear in the inspector and in the reference page; write
   them as one sentence with the physical meaning.

After this step `fluidnet catalog` lists the type, and a model using it fails
with `component type 'check_valve' not supported in this build` until step 3
is done.

### 3. Compile the component (`network/compile.rs`)

`Network::compile` loops over all components that are not node-like, shafts or
chambers, finds the nodes at their `in` and `out` ports and creates branches:

```rust
let na = port_node(ci, "in");
let nb = port_node(ci, "out");
```

The check valve shares the restriction arm with orifice, venturi, loss, valve
and regulator. Its own part reads and checks the parameters and builds the
`Ctrl` value:

```rust
"check_valve" => {
    let crack = p.q("cracking_dp")?;
    let full = p.q("full_open_dp")?;
    if full <= crack {
        return Err(Error::Model(format!(
            "{id}: full_open_dp must exceed cracking_dp"
        )));
    }
    (
        p.cda()?,
        Ctrl::Check {
            crack,
            full,
            leak: p.q("leak")?.max(0.0),
        },
        rmodel(p)?,
    )
}
```

and the shared part of the arm creates the branch:

```rust
if !(cda > 0.0) {
    return Err(Error::Model(format!("{id}: effective area must be > 0")));
}
comps[ci].branches.push(branches.len());
branches.push(Branch {
    name: id.clone(),
    comp: ci,
    a: na,
    b: nb,
    kind: BranchKind::Restriction {
        cda,
        ctrl,
        model,
        recovery,
    },
});
```

Rules for this step:

1. Read every parameter through `Params` (`p.q`, `p.q_opt`, `p.sched`,
   `p.sched_opt`, `p.str_opt`, `p.int`, `p.cda`, `p.area`). These apply the
   catalog default and the unit conversion. Reading a key that is not in the
   catalog panics with `internal: parameter ... not in catalog`, which
   catches a missing catalog entry at the first test run.
2. Check the parameter values the equations need and return
   `Error::Model` with the component id and the parameter names.
3. For every schedule, add its times to `breakpoints`, so the integrator lands
   on them (the valve arm does `breakpoints.extend(opening.breakpoints())`).
4. Register the created elements with the component: `comps[ci].branches`
   (and `comps[ci].nodes` for internal nodes). The results use this to list a
   component's ports, nodes and branches.
5. Check two places that classify branches outside the main arm:
   - `closed_at_0` in the initial-state propagation decides which branches
     the initial state crosses last; the check valve is listed there
     (`ctrl: Ctrl::Check { .. }` returns `true`), so an upstream liquid state does not fill a
     downstream gas region through it.
   - `Network::flow_guess` in `network/mod.rs` uses an opening per branch to
     split the guessed steady flows; the check valve counts as open
     (`Ctrl::Fixed | Ctrl::Check { .. } | Ctrl::Regulator { .. } => 1.0`).

A component with its own states allocates them in the state-vector section of
`compile` with `push_state(StateKind::..., "<element>.<quantity>", initial
value, ...)` and stores the index in its data structure; see the actuator
states (`<valve>.position`) at the end of `compile` as a compact example.

### 4. Evaluate the equations (`network/mod.rs`)

Add the data the evaluation needs to the network structures. For the check
valve this is one variant of `Ctrl`:

```rust
pub enum Ctrl {
    Fixed,
    Valve {
        opening: Schedule,
        charac: Charac,
        leak: f64,
    },
    Check {
        crack: f64,
        full: f64,
        leak: f64,
    },
    Regulator {
        set: Schedule,
        band: f64,
        leak: f64,
    },
    /// Opening set by an actuator state driven by a sensed pressure.
    Actuated {
        act: usize,
        charac: Charac,
        leak: f64,
    },
}
```

`Network::evaluate` computes all intermediate quantities at (t, y) in a fixed
order: node properties, chamber ports, shaft speeds, branch flows with machine
power and torque, chambers, heat. The restriction branch evaluates the opening
and then the flux; the check valve's line is the opening law:

```rust
Ctrl::Check { crack, full, leak } => {
    let dp = pa.p - pb.p;
    leak + (1.0 - leak) * smoothstep((dp - crack) / (full - crack))
}
```

followed by the common restriction code, which stores `ev.opening[k]`, picks
the upstream side, calls `restriction_flux` and sets `ev.choked[k]` and the
flow. `smoothstep` is the cubic 3s² − 2s³ limited to 0–1; it keeps the opening
continuously differentiable, which the Newton iterations of both solvers need.
Use smooth functions for every switching behaviour; do not introduce discrete
events.

`Network::rhs` turns the evaluated quantities into time derivatives. Algebraic
branches need nothing there: the node mass and energy balances sum
`ev.mdot` of all branches at each node. A branch with a mass-flow state writes
its momentum equation in `rhs` (see `BranchKind::Pipe` and `BranchKind::Pump`);
a component with its own states writes their derivatives there.

A new state kind also needs entries in `Network::weights` (absolute tolerance
of the transient integrator), `steady_weights` in `solver/steady.rs` and
`Network::typical` (finite-difference perturbation size).

### 5. Declare the sparsity (`Network::sparsity`)

The Jacobian is built by finite differences on a structural pattern with
column colouring. The pattern comes from (reads, writes) terms of every
equation contribution. A branch reads the states of its two end nodes
(`branch_reads`) and writes the balances of those nodes, which covers the
check valve without any change. A component that reads more states must add
them; the actuated valve is the example:

```rust
BranchKind::Restriction {
    ctrl: Ctrl::Actuated { act, .. },
    ..
} => r.push(self.actuators[*act].state),
```

and its actuator adds its own term (reads its state and the sensed node
states, writes its state). A missing dependency does not cause an error
message; it makes the Jacobian wrong, which shows as slow Newton convergence
and many rejected steps. Check every new component with:

```sh
./target/release/fluidnet jaccheck tests_models/<your_model>.yaml
```

The worst relative difference between the coloured and the full Jacobian
should be of the order of the finite-difference error: for every example of
v0.5.0 it lies between 5×10⁻¹⁰ and 1.1×10⁻⁹. A missing dependency gives a
difference of order one.

### 6. Add output channels (`build_channels`)

Channels are declared once in `Network::build_channels` (`compile.rs`) with a
name, an SI unit and a `Probe` that `Network::probe` evaluates from `Eval`. The
check valve gets the restriction channels; `opening` is added for every
restriction whose `Ctrl` is not `Fixed`:

```rust
BranchKind::Restriction { ctrl, .. } => {
    push(format!("{}.dp", b.name), "Pa", Probe::Dp(k));
    push(format!("{}.choked", b.name), "-", Probe::Choked(k));
    if !matches!(ctrl, Ctrl::Fixed) {
        push(format!("{}.opening", b.name), "-", Probe::Opening(k));
    }
}
```

(`<branch>.mdot` is added for every branch before this match.) For a new
quantity, store it in `Eval` during `evaluate` and add a `Probe` variant or
reuse `Probe::Br(k, BrF::Aux1..3)`, the three auxiliary values per branch that
pumps and turbines use. Follow the naming rules of
[conventions](conventions.md#25-naming).

### 7. Add the schematic symbol (`ui/src/symbols.tsx`)

The palette and the inspector need no change: they are generated from the
catalog. The symbol is drawn in `symbols.tsx`, with its size in
`SYMBOL_SIZE`; two-port symbols are drawn from left (`in`) to right (`out`):

```tsx
export const SYMBOL_SIZE: Record<string, [number, number]> = {
  ...
  check_valve: [56, 32],
  ...
};
```

```tsx
case 'check_valve':
  body = (
    <>
      <line x1={2} y1={cy} x2={w - 2} y2={cy} {...S} strokeWidth={1} />
      <path d={`M14 ${cy - 10} L${w - 18} ${cy} L14 ${cy + 10} Z`} {...S} fill="var(--surface)" />
      <line x1={w - 16} y1={cy - 11} x2={w - 16} y2={cy + 11} {...S} strokeWidth={2} />
    </>
  );
  break;
```

`S` is the common stroke style (`currentColor`, so the symbol takes the fluid
colour). Without a case the component is drawn as a rectangle. Further places
in the interface that are type-specific:

| File | Item | Check valve |
|---|---|---|
| `ui/src/App.tsx` | `NEW_DEFAULTS`: starting values for required parameters of a newly placed component | `check_valve: { diameter: '8 mm' }` |
| `ui/src/App.tsx` | id prefix of new components in `addComp` | `check` |
| `ui/src/CompNode.tsx` | ports and anchors; two-port types get `in`/`out` automatically | no change |
| `ui/src/App.tsx` `overlayFor`, `ui/src/Results.tsx` `ValuesTable`, `defaultSelection` | values shown on the schematic and in the table; generic for flow elements | no change |

Rebuild the engine and the interface (`npm run wasm`, `npm run build`) and
place the component once in the browser.

### 8. Add tests

Add a test model to `tests_models/` and a test to
`crates/fluidnet-core/tests/validation.rs` that compares the component with a
reference it does not share code with. v0.5.0 has no check valve test
([conventions 2.6](conventions.md#26-tests-required-with-a-change)); a test of
this kind would use a model such as:

```yaml
name: check valve forward and reverse
components:
  - {id: up, type: boundary, fluid: H2O, P: [[0, 3 bar], [1, 3 bar], [1, 1 bar]], T: 293.15 K}
  - {id: cv, type: check_valve, diameter: 10 mm, cd: 0.7, cracking_dp: 0.2 bar, full_open_dp: 0.7 bar}
  - {id: down, type: boundary, fluid: H2O, P: 2 bar, T: 293.15 K}
connections: [[up, cv.in], [cv.out, down]]
simulation: {t_end: 2, output_dt: 0.5}
```

With 1 bar forward differential pressure (above `full_open_dp`) the valve is
fully open and the flow equals cd A √(2ρΔP) = 0.7 × 78.54 mm² ×
√(2 × 998.30 kg/m³ × 1×10⁵ Pa) = 0.77684 kg/s (density of water at 3 bar and
293.15 K from `fluidnet.props`); v0.5.0 gives 0.776843 kg/s. From 1 s the
differential pressure is −1 bar and the flow is the leakage,
−7.77×10⁻⁷ kg/s (10⁻⁶ of the open flow), with `cv.opening` = 1×10⁻⁶. A test
asserts these values with stated tolerances and documents the reference in a
comment, as the existing tests do:

```rust
#[test]
fn check_valve_opens_forward_and_leaks_reverse() {
    let r = run("tests_models/check_valve.yaml", Mode::Transient);
    // 0.7 A sqrt(2 rho dP), water 3 bar 293.15 K (rho 998.30 kg/m3), dP 1 bar
    assert!(rel(r.channel("cv.mdot").unwrap()[0], 0.776843) < 1e-4);
    // reverse: leakage area fraction 1e-6
    assert!(r.last("cv.mdot").unwrap() < 0.0 && r.last("cv.opening").unwrap() < 2e-6);
}
```

(This test and model are an illustration for this handbook; they are not in
the repository.) Also cover the partly open range between `cracking_dp` and
`full_open_dp` and, for components with dynamics, a transient against an
analytic or independently computed response. If the component appears in an
example, the example is covered by `all_examples_run_steady_and_transient`.

### 9. Update the documentation

As listed in [conventions 2.7](conventions.md#27-documentation-updates):

1. regenerate the [component reference](../03-user-guide/component-reference.md)
   (`python tools/gen_component_reference.py`);
2. describe the model, its equations, limits and code location in the
   [02 Engineering](../02-engineering/index.md) page it belongs to (restrictions
   in [flow elements](../02-engineering/flow-elements.md));
3. add the type to the table in the
   [model file standard](../03-user-guide/model-files.md#23-components) and its
   channels to [outputs](../03-user-guide/outputs.md);
4. add the test to [analytic cases](../04-verification/analytic-cases.md) and the
   [validation matrix](../04-verification/validation-matrix.md);
5. add any new example to the [example register](../03-user-guide/examples.md).

### 10. Final checks

```sh
cargo build --release
cargo test --release
./target/release/fluidnet jaccheck tests_models/<your_model>.yaml
./target/release/fluidnet run tests_models/<your_model>.yaml --steady
python tools/gen_component_reference.py
mkdocs build --strict
```

Then commit the code, the test model, the test, the regenerated reference and
the documentation together ([conventions 2.8](conventions.md#28-commit-messages)).

## Troubleshooting

| Symptom | Cause and remedy |
|---|---|
| `component type 'x' not supported in this build` | Catalog entry exists but no arm in `Network::compile` handles the type. |
| Panic `internal: parameter k not in catalog for x` | The compiler reads a parameter that the catalog does not declare; add it to the `CompSpec`. |
| `x: unknown parameter 'k' ignored` for a parameter the component uses | The key is read in `compile` under a different name than in the catalog. |
| Many rejected steps or slow Newton convergence after adding the component | Missing sparsity dependency (`jaccheck` shows a large difference), or a non-smooth function in the evaluation. |
| Panic in `compile` (missing key in a hash map) for a component whose ports are not `in`/`out` | The branch loop of `compile` looks up the ports `in` and `out` of every component it handles; components with other ports (like the chamber's `fuel`/`ox`) must be skipped there and handled separately, as the chamber is. |
| The component is a rectangle in the interface | No `case` in `symbols.tsx`. |
| The reference page does not show the new type | Rebuild the CLI before running the generator, or pass `--cli`. |

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
