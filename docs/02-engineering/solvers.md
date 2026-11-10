# Transient and steady-state solvers

FNT-SLV-DOC-002 · Rev B · Released · 2026-11-10 · Owner: Luis Erpenbach

## Scope

This page describes the numerical methods that solve the network equations
dy/dt = f(t, y) of [FNT-SLV-DOC-001](network-formulation.md): the TR-BDF2
transient integrator, the finite-difference Jacobian with column colouring,
the dense and sparse linear solvers, and the steady-state solution sequence
(flow guess, shaft continuation, damped Newton, pseudo-transient continuation,
time-marching and transient settling). It states the tolerances, weights,
thresholds and the statistics each solver reports.

## Model and equations

Both solvers work on the same right-hand side, `Network::rhs`. The transient
solver integrates

```
dy/dt = f(t, y),   y(t0) = y0
```

and the steady-state solver finds y with

```
f(t_s, y) = 0
```

with all schedules evaluated at the fixed time t_s (`simulation.steady_time`,
or 0 s when the transient starts from a steady state).

### TR-BDF2

The transient integrator is the one-step, second-order, L-stable TR-BDF2
method (Bank et al. 1985) in the form of Hosea and Shampine (1996). With
γ = 2 − √2, d = γ/2 and w = √2/4, one step of size h from (t_n, y_n) with
f_n = f(t_n, y_n) consists of two implicit stages:

```
stage 1, trapezoidal rule to t_n + γh:
    Y₁ − d h f(t_n + γh, Y₁) = y_n + d h f_n
stage 2, BDF2 to t_n + h:
    Y₂ − d h f(t_n + h, Y₂) = y_n + w h (f_n + f₁)
    y_{n+1} = Y₂
```

Both stages share the iteration matrix M = I − d h J. The stage derivatives
are recovered from the converged stage values without a further evaluation,
f₁ = (Y₁ − c₁)/(d h) and f₂ = (Y₂ − c₂)/(d h), where c₁ and c₂ are the
right-hand sides above.

The local error is estimated with the embedded third-order weights and
filtered through the iteration matrix (Shampine), so that stiff components do
not inflate it:

```
e₁ = w − (1 − w)/3   ≈  0.1381
e₂ = w − (3w + 1)/3  ≈ −0.3333
e₃ = 2d/3            ≈  0.1953
err = M⁻¹ · h (e₁ f_n + e₂ f₁ + e₃ f₂)
```

## Numerical treatment

### Tolerances and weights

Errors and Newton corrections are measured in a weighted root-mean-square
norm, ‖v‖ = √(Σ (v_i / w_i)² / n). The weights come from `Network::weights`,
with absolute parts scaled by `simulation.atol_scale` (default 1) and the
relative tolerance `simulation.rtol` (default 1×10⁻⁴):

| State kind | Weight w_i |
|---|---|
| Pressure (node P, chamber P_c) | atol_scale · 1 Pa + rtol · \|P\| |
| Enthalpy | atol_scale · 10 J/kg + rtol · max(\|h\|, 2×10⁵ J/kg) |
| Mass flow | atol_scale · 1×10⁻⁶ kg/s + rtol · \|ṁ\| |
| Shaft speed | atol_scale · 0.1 rad/s + rtol · \|ω\| |
| Wall temperature | atol_scale · 0.01 K + rtol · \|T_w\| |
| Chamber mixture fraction Z | atol_scale · 1×10⁻⁵ + rtol · \|Z\| |
| Valve position | atol_scale · 1×10⁻⁴ + rtol · \|u\| |

For the error test the weights are evaluated at the larger in magnitude of
y_n and y_{n+1}, component by component; for the Newton iteration at y_n.

### Modified Newton iteration

Each stage equation Y − d h f(t, Y) = c is solved by modified Newton with the
factored iteration matrix (`Ctx::newton` in `transient.rs`), at most 10
iterations:

1. Evaluate the residual g = −(Y − d h f(t, Y) − c) and its weighted norm r.
2. If r < 0.1, accept the stage. A residual below 0.1 tolerance units bounds
   the stage error well below the tolerance; this test also accepts iterates
   that straddle a kink in the property tables, where the correction-based
   test can cycle.
3. Solve M ΔY = g, update Y, and compute the weighted norm of the correction,
   d_k. The iterate must stay finite with all pressure states positive;
   otherwise the iteration fails.
4. From the second iteration on, the contraction rate is θ = d_k / d_{k−1}. If
   θ ≥ 0.95 the iteration fails (divergence). Otherwise
   η = θ / (1 − θ) (Hairer and Wanner).
5. Accept if (η d_k < 0.05 or d_k < 1×10⁻⁴) and r < 2. Before the first rate
   estimate of a stage, η = max(η_prev, 1×10⁻⁴)^0.8 from the previous stage.

Because the residual is checked directly, a stale iteration matrix cannot
produce a falsely converged stage. A stage that needs five or more iterations
is flagged as slow.

### Predictors

The stage-1 starting value is the cubic Hermite interpolant of the previous
step (y_{n−1}, f_{n−1}, y_n, f_n) extrapolated to t_n + γh, provided the
extrapolation distance is at most 2.5 previous steps and the result is valid;
otherwise the explicit Euler value y_n + γ h f_n. The stage-2 starting value is
Y₁ + (1 − γ) h f₁, or, when a previous step exists, the Hermite extrapolation
to t_n + h corrected by the stage-1 mismatch (extrapolation to t_n + γh minus
Y₁). After a step lands on a breakpoint the previous step is discarded, so no
extrapolation crosses an input discontinuity.

### Step-size control

- Initial step: h = min(0.01 / ‖f(t0, y0)‖, 0.01 (t_end − t0)) if the scaled
  derivative norm exceeds 1×10⁻¹⁰, otherwise 1×10⁻³ (t_end − t0); then
  clamped to [1×10⁻¹⁰ max(t_end − t0, 1×10⁻⁶ s), max_step].
- Accept if ‖err‖ ≤ 1. The next step is h · fac with
  fac = clamp(0.9 ‖err‖^(−1/3), 0.2, 4) (fac = 4 if ‖err‖ = 0), limited to
  `max_step` (default t_end/20). If 1 ≤ fac < 1.3 and the step did not land on
  a breakpoint, h is kept unchanged so that the factorisation can be reused.
- Reject if ‖err‖ > 1: h ← h · clamp(0.9 ‖err‖^(−1/3), 0.1, 0.5).
- Newton failure: if the Jacobian is not fresh, recompute it and retry with
  the same h; if it is fresh, h ← h/4.
- The step fails with "step size underflow" when h falls below
  1×10⁻¹³ max(|t|, 1×10⁻³ s). The message lists typical causes (a state
  outside the fluid table, trapped liquid being compressed, an inconsistent
  model).
- The integration stops with an error after `max_steps` accepted plus
  rejected steps (default 2 000 000).

### Breakpoints

Schedule breakpoints (table times of all schedules, the ignition time and the
end of the ignition ramp; see [FNT-SLV-DOC-001](network-formulation.md)) are
hit exactly. If t + h reaches the next breakpoint (within 1×10⁻¹² relative),
the step is shortened to land on it; if t + 1.5h would pass it, h is set to
half the remaining distance so that the following step is not tiny. After
landing, f is re-evaluated at the breakpoint (the inputs may have a kink or a
jump there), the previous step is discarded for extrapolation and the Jacobian
is refreshed.

### Jacobian refresh and refactorisation

The Jacobian J is computed at the start and refreshed:

- after an accepted step whose Newton iteration was slow,
- after every 40 accepted steps since the last refresh,
- after a step that landed on a breakpoint,
- after a Newton failure, if it is not already fresh,
- after three consecutive error-test failures, if it is not already fresh,
- when the factorisation of M fails with a Jacobian that is not fresh.

A single error-test failure does not refresh the Jacobian: it indicates that
the step was too long, not that J is stale. The iteration matrix is
refactored whenever h differs from the h of the last factorisation or J has
been refreshed. A singular M with a fresh Jacobian ends the integration with
"singular iteration matrix".

### Dense output

Output times do not constrain the step size. For each output time inside an
accepted step the state is interpolated with the cubic Hermite polynomial
through (y_n, f_n) and (y_{n+1}, f_{n+1}). Output times that remain after
t_end is reached receive the final state.

### Finite-difference Jacobian with column colouring

`JacobianBuilder` (`jacobian.rs`) computes J on the structural pattern from
`Network::sparsity`, with the diagonal always included:

- Columns are coloured greedily in index order such that no two columns of
  one colour have a non-zero in the same row (distance-2 colouring). Columns
  of one colour are perturbed together; one right-hand-side evaluation per
  colour gives all their entries. Typical networks need 8–16 colours,
  independent of size (16 colours for the 63-state expander cycle, 10 for the
  150-state water-hammer example, as printed by `fluidnet bench`).
- The perturbation of state j is δ_j = 1×10⁻⁷ · max(|y_j|, y_typ,j), with the
  typical magnitudes 1×10⁵ Pa (pressure), 1×10⁵ J/kg (enthalpy),
  1×10⁻² kg/s (mass flow), 100 rad/s (speed), 100 K (wall temperature), 0.1
  (Z and valve position).
- J_ij = (f_i(y + Σ δ_k e_k) − f_i(y)) / δ_j for the structural rows i of
  column j. The base value f(y) is always evaluated fresh inside `build`. A
  derived f, for example from an implicit stage, differs from f(y) by
  O(tolerance / h), which divided by the small perturbation would corrupt J.

The cost of one Jacobian is (number of colours + 1) right-hand-side
evaluations.

### Linear systems

`LinSys` (`linsys.rs`) factors M = a I − b J on the fixed pattern. The
transient solver uses a = 1, b = d h; the steady solver a = 1/τ (0 for Newton),
b = 1.

- For n ≤ 48 states (`DENSE_MAX`) M is assembled as a dense row-major matrix
  and factored by LU with partial pivoting (`linalg.rs`, `Lu::factor`). A zero
  or non-finite pivot reports the matrix as singular.
- For n > 48 M is assembled as a `faer::sparse::SparseColMat` from triplets
  and factored with faer's sparse LU (`faer` 0.24). The symbolic analysis
  (`SymbolicLu`, which includes the fill-reducing ordering) is computed at the
  first factorisation and reused, because the pattern does not change during a
  run; only the numeric factorisation is repeated.

### Steady-state solution sequence

`steady::solve(net, y0, t_s, max_iter)` (`steady.rs`) is called with
`max_iter` = 600 from `run_steady` and `run_transient`. It runs the following
sequence and returns at the first success.

**1. Flow guess.** Mass-flow states that are exactly zero in y0 are replaced by
the values of `Network::flow_guess` (pump design flows and mass-flow sources
propagated by mass balance; see [FNT-SLV-DOC-001](network-formulation.md)).
Non-zero flow states, for example from a warm start, are kept.

**2. Latches off.** If the network has latching actuated valves, the direct
methods below use a copy with `latch_active = false`, so the actuators settle
at their commanded positions in both directions.

**3. Shaft continuation.** If there are shafts with a speed state and a speed
guess ω_guess > 0 (the shaft's `N0`, or the design speed of its first pump
divided by the gear ratio), the direct methods are first applied to a copy in
which these shafts are held at ω_guess (dω/dt = (ω_guess − ω)/1×10⁻³ s), with
their speed states set to ω_guess. If that converges, the shafts are released
and the direct methods are applied again from the held solution. If either
solve fails, the sequence continues with step 4 from the original start. This
finds the operating point of turbopump cycles from cold initial states, where
an unconstrained Newton iteration tends to move the shaft to zero speed or
runaway.

**4. Direct methods** (`solve_direct`), each starting from the same initial
state:

a. *Damped Newton* (at most 40 iterations). Each iteration recomputes the
   Jacobian and solves −J d = f. A backtracking line search halves λ from 1 at
   most ten times until the weighted residual satisfies
   ‖f(y + λd)‖ ≤ (1 − 1×10⁻⁴ λ) ‖f(y)‖ (or ‖f‖ < 1×10⁻¹⁰) with a valid state
   (finite, pressures positive). Converged when a full step (λ = 1) has a
   weighted step norm λ‖d‖ < 1. A failed line search or a singular J ends the
   Newton attempt.

b. *Pseudo-transient continuation* (Kelley and Keyes 1998; at most
   min(max_iter, 150) = 150 iterations). Each iteration recomputes J and solves
   (I/τ − J) d = f, which is an implicit Euler step of size τ in pseudo-time.
   It starts with τ = 1×10⁻⁵ s. A step is retried with τ/4 (up to 16 tries)
   if M is singular, the new state is invalid or not evaluable, or the
   residual grows more than 20-fold while τ > 1×10⁻⁸ s. After an accepted step
   τ ← min(1.5 τ · clamp(r_old / r_new, 0.5, 10), 1×10¹⁴ s) (switched evolution
   relaxation with additional geometric growth). Converged when the weighted
   step norm is below 1 with τ > 1×10³ s on two consecutive iterations. Once
   τ > 1×10⁶ s, every tenth iteration tries Newton (30 iterations) from the
   current state and returns its solution if it converges. Pseudo-time
   stepping damps stiff and oscillatory (acoustic) modes that make plain
   Newton diverge, and becomes Newton's method as τ → ∞.

c. *Time-marching.* The transient integrator runs from the initial state with
   inputs frozen at t_s (`freeze_time`), `rtol` 1×10⁻⁴, `max_step`
   1×10⁸ s, at most 5 000 steps, up to t = 1×10⁹ s, and stops early once the
   largest scaled derivative max |f_i / w_i| (transient weights) is below
   1×10⁻³ 1/s. Newton (40 iterations) then polishes the result.

**5. Transient settling** (last resort, `settle`). If the direct methods fail,
an ordinary transient is run on the original network, with latches active and
from the user's initial state (without the flow guess):

- first from 0 s to t_s with the actual input history, `max_step`
  min(t_s/20, 0.25 s), at most 50 000 steps, so that a steady state at the end
  of a start sequence is reached the way the hardware reaches it and latching
  valves stay latched;
- then up to 30 s with inputs frozen at t_s, `max_step` 0.25 s, at most 50 000
  steps, stopping when max |f_i / w_i| < 1×10⁻³ 1/s. Outputs are recorded
  every 0.5 s, so that if a small limit cycle (for example a feed-line
  oscillation) never meets the rate criterion, the latest state is still
  available.

The direct methods (step 4, on the unlatched copy) are then applied from the
settled state.

**Quasi-steady acceptance.** Pseudo-transient continuation accepts its current
state as a steady state if it cannot make progress, or reaches its iteration
limit, while the weighted RMS residual is below 1×10⁻³. Since the residual is
dy/dt divided by the steady weights, this accepts rates below 1×10⁻³ weight
units per second, for example a tank draining through the leakage of a closed
valve over days. The method is then reported as
"pseudo-transient (quasi-steady)".

**Tanks.** A tank with `steady: hold` (the default) is a reservoir in a
steady-state solve: `steady::solve` adds its two states to
`Network::held_states`, whose rates are replaced by (value − y)/1 ms, so the
states stay at the initial or warm-start values and the rest of the network is
solved around them. A tank with `steady: solve` keeps its balance equations.
If the solve of a network with `solve` tanks fails, `tanks_held` repeats it
with those tanks held and accepts the result when no tank's relative mass rate
exceeds 1×10⁻⁴ 1/s (method "tanks held at their initial state, quasi-steady").
Before v0.5.1 all tanks were solved; a tank behind a closed, leaking valve then
converged only for some initial states, which differed between platforms.

**Dead-end volumes.** A volume without through-flow has no equation that fixes
its enthalpy at steady state. Pseudo-transient continuation, whose matrix
I/τ − J stays regular, leaves such states at their initial values.

**Steady weights** (`steady_weights`), used for all steady residual and step
norms:

| State kind | Weight |
|---|---|
| Pressure | 1×10⁻² Pa + 1×10⁻⁷ \|P\| |
| Enthalpy | 0.1 J/kg + 1×10⁻⁷ max(\|h\|, 2×10⁵ J/kg) |
| Mass flow | 1×10⁻⁹ kg/s + 1×10⁻⁷ \|ṁ\| |
| Shaft speed | 1×10⁻³ rad/s + 1×10⁻⁷ \|ω\| |
| Wall temperature | 1×10⁻⁴ K + 1×10⁻⁷ \|T_w\| |
| Mixture fraction Z | 1×10⁻⁸ |
| Valve position | 1×10⁻⁷ |

If every method fails, the error names the weighted residual and the state
with the largest scaled residual, and notes that the system may have no
steady state (a closed tank that keeps filling) or may oscillate.

### Warm start

`Network::apply_init` overwrites initial state values by state name. In the
Python API, `fluidnet.run(model, init=res)` passes the final state of an
earlier result. In the steady solver the warm-started flow states are
non-zero, so the flow guess leaves them alone; the shaft continuation of step 3
still runs, starting from the warm state with the free shaft speeds reset to
their guesses. A transient started with `init` begins from the warm state
directly.

## Parameters and outputs

Solver settings in the model's `simulation` block: `t_end`, `output_dt`,
`rtol`, `atol_scale`, `max_step`, `max_steps`, `start_from_steady`,
`steady_time` (defaults in [FNT-ARC-DOC-001](architecture.md)). The
environment variable `FLUIDNET_DEBUG` (native builds) prints step rejections
with the worst state, Newton failures and the progress of the steady methods
to standard error.

Transient statistics (`transient::Stats`, in `Results::stats`):

| Field | Meaning |
|---|---|
| `steps`, `rejected` | Accepted steps, error-test rejections |
| `newton_failures` | Stage iterations that failed |
| `rhs_evals` | Right-hand-side evaluations, including those for Jacobians |
| `jacobians`, `factorizations` | Jacobian evaluations, LU factorisations |
| `n_states`, `jacobian_colors`, `nonzeros`, `sparse` | Problem size, colours, structural non-zeros of M, sparse or dense LU |
| `h_min_used`, `h_max_used` | Smallest and largest accepted step, s |

Steady statistics (`steady::SteadyStats`, in `Results::steady`): `iterations`,
`rhs_evals`, `jacobians`, `residual_history` (weighted residual per
iteration), `converged`, and `method`, a description of the path that
succeeded, for example `newton`, `pseudo-transient`,
`time-marching + newton`, `shaft continuation (newton) + newton` or
`transient settling + pseudo-transient`.

## Limits and assumptions

- The integrator assumes f is continuous between breakpoints. Discontinuities
  that do not come from schedules (for example a property table edge) are
  handled only by step rejection.
- Newton iterates must keep all pressure states positive; other states are not
  bounded.
- The Jacobian is a forward difference with a relative perturbation of
  1×10⁻⁷; it is not exact at kinks of the property tables or of smoothed
  switching functions.
- Time-marching and settling use fixed internal settings (`rtol` 1×10⁻⁴, step
  limits above), not the model's `simulation` settings.
- The steady solver finds one solution. Systems with several operating points
  (for example a pump curve crossing a system curve twice) converge to the one
  nearest the start in the sense of the method that succeeds.

## Verification

- Every example runs as a transient and as a steady state:
  `all_examples_run_steady_and_transient`.
- A tank behind a closed valve gives the same steady state for perturbed
  initial states: `closed_tank_steady_state_is_robust_to_small_perturbations`.
- The expander start transient ends within 0.3 % of the steady solution in
  P_c, thrust, shaft speed and O/F: `expander_start_reaches_the_steady_design_point`.
  This checks that the transient and steady solvers agree on the same
  equations.
- Integrator accuracy against analytic transients: N₂ blowdown and water
  hammer (`n2_blowdown_matches_isentropic_ideal_gas`,
  `water_hammer_matches_joukowsky`).
- Dense LU with pivoting: `solves` in `src/solver/linalg.rs`.
- Steady solution from cold initial states with free shafts:
  `expander_cycle_conserves_energy_and_torque` and
  `rl10a33a_reproduces_test_point_1`.
- Case descriptions: [Analytic verification
  cases](../04-verification/analytic-cases.md) and
  [RL10A-3-3A validation](../04-verification/rl10.md).
- `fluidnet jaccheck MODEL [--t T] [--probe STATE]` compares the coloured
  Jacobian with a full finite difference.

## Code location

| Topic | Location |
|---|---|
| TR-BDF2, step control, dense output | `crates/fluidnet-core/src/solver/transient.rs`: `integrate`, `Ctx::newton` |
| Tolerance weights, perturbation magnitudes | `crates/fluidnet-core/src/network/mod.rs`: `Network::weights`, `Network::typical` |
| Jacobian and colouring | `crates/fluidnet-core/src/solver/jacobian.rs`: `JacobianBuilder::new`, `build` |
| Linear systems | `crates/fluidnet-core/src/solver/linsys.rs`: `LinSys::factor`, `solve`; `linalg.rs`: `Lu` |
| Steady sequence | `crates/fluidnet-core/src/solver/steady.rs`: `solve`, `solve_direct`, `newton`, `ptc`, `march`, `settle`, `steady_weights` |
| Flow guess, warm start | `crates/fluidnet-core/src/network/mod.rs`: `flow_guess`, `apply_init` |
| Callers | `crates/fluidnet-core/src/sim.rs`: `run_steady`, `run_transient` |

## Open items

- The former `docs/ARCHITECTURE.md` stated that a warm start skips the flow guess and the
  shaft continuation. In the code the flow guess is skipped only for non-zero
  flow states, and the shaft continuation always runs and resets the warm
  shaft speeds to their guesses. Decide whether a warm start should bypass the
  continuation.
- The transient settling step starts from the user's initial state, not from
  the flow-guessed state; this is probably intended (it replays the real start
  sequence) but is not stated in the code.
- The colour counts quoted above come from one `fluidnet bench` run on
  2026-11-02 and are not stored in a results file.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
| B | 2026-11-10 | v0.5.1: tanks held in steady-state solves; tanks-held quasi-steady fallback |
