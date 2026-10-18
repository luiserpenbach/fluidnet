# Validation

All cases are in `tests_models/` or `examples/` and are checked by
`cargo test --release` (`crates/fluidnet-core/tests/validation.rs`). References
are analytic or computed with CoolProp 8.0.

| Case | Reference | Result |
|---|---|---|
| N2 tank blowdown, 10 bar → 1 bar through 3 mm orifice, choked | Isentropic ideal-gas blowdown | P within 0.1 % over 6 s (remainder is real-gas behaviour) |
| Water orifice, 10 → 5 bar | 0.7 A √(2ρΔP) = 0.43434 kg/s | 0.43434 kg/s |
| Water pipe, 10 m × 20 mm, 2 bar | Darcy–Colebrook 1.9452 kg/s | 1.9382 kg/s (Churchill vs Colebrook, −0.36 %) |
| Cavitating venturi, water 30 bar | 0.95 A √(2ρ(P₀−P_v)) = 0.23080 kg/s | 0.23080 kg/s, choked |
| Saturated LN2 at 5 bar through 2 mm orifice to 1 bar | HEM by isentropic integration, 0.02037 kg/s | 0.02051 kg/s (+0.7 %), choked |
| Water hammer, 100 m × 50 mm, 1.8 m/s, valve closed in 2 ms | Joukowsky plateau 66.24 bar; period 4L/a = 0.2687 s | 66.6 bar; 0.2690–0.2695 s |
| Heated water line, 100 kW/m², 0.1 kg/s | Δh = Q/ṁ = 62.83 kJ/kg | 62.73 kJ/kg (−0.16 %) |
| GN2 regulator, set 30 bar, band 2 bar | Outlet within band | 30.68 bar |
| Subcooled small water hammer exceeding static pressure | Column separation at P_v(20 °C) = 0.023 bar | Valve end settles at 0.0255 bar |
| Water pump at design speed against an orifice | Same affinity map solved in Python: 8.38489 kg/s, η 0.6229 | 8.3857 kg/s, η 0.6228 |
| Pressure-fed GH2/LOX chamber, 40 mm throat, ε 40 | Pc = ṁ c*/At with table c* and enthalpy correction: 45.4245 bar; ideal CF 1.9503, F 11,132.5 N | 45.4245 bar; 11,132.5 N |
| Expander steady state | Shaft torque balance; jacket ṁΔh = Q; pump ṁΔh = P; throat flow = injected flow | all to 10⁻⁶ |
| Expander start transient | Steady-state solver at the same inputs | Pc, thrust, speed, O/F within 0.3 % |

Combustion tables against CEA (LOX/GH2, 50 bar, O/F 6): c* 2329 m/s, Tc 3508 K
(CEA: about 2330 m/s and 3490–3510 K for gaseous hydrogen near room
temperature). Nozzle relations: exit Mach 3.2783 and vacuum CF 1.7423 at γ 1.2,
ε 10, matching an independent root solve. Copper Debye heat capacity: 82 J/kg/K
at 50 K (tabulated 98, -16 %) and 243 J/kg/K at 100 K (tabulated 254, -4 %); the
single-parameter Debye model underestimates metals at low temperature, which makes
chill-down slightly faster than reality.

Two-point omega against full HEM (CoolProp), restriction mass flux:

| Fluid, inlet | G_omega / G_HEM |
|---|---|
| N2, saturated, 5 bar | 1.008 |
| N2, saturated, 10 bar | 1.008 |
| O2, saturated, 5 bar | 1.007 |
| N2O, saturated, 50 bar | 1.008 |
| N2, 3 K subcooled, 5 bar | 0.994 |
| N2O, 5 K subcooled, 50 bar | 0.979 |

## RL10A-3-3A engine against test data

`examples/rl10a33a.yaml` models the Pratt & Whitney RL10A-3-3A closed expander
from the geometry and component design points in NASA TM-107318 and CR-190786.
Eight factors are calibrated at test point 1 of CR-190786 (fuel pump head scale,
turbine nozzle area and peak efficiency, hot-gas heat transfer factor, jacket
throat-region loss, fuel and LOX injector discharge, nozzle thrust efficiency).
Test points 2 to 5 are then predicted with only the bypass (TCV) and
mixture-ratio valve openings set to the measured thrust and O/F, as done for
ROCETS. Reproduce with `python validation/rl10/rl10.py` (8 s) and
`python validation/rl10/start.py` (95 s); `cargo test` checks the calibration
point and the start.

| Test points 2–5, 40 measured values | fluidnet | ROCETS (CR-190786) |
|---|---|---|
| Mean absolute error | 1.39 % | 1.26 % |
| RMS error | 2.19 % | 1.55 % |
| Largest error | 8.1 % (turbine inlet T, TP4) | 3.1 % |
| Within 1 % / 3 % | 65 % / 83 % | 55 % / 98 % |
| Mean error with each model's TP1 offset removed | 1.35 % | 0.38 % |

fluidnet is calibrated at TP1 and ROCETS is not, so the plain errors favour
fluidnet; with the TP1 offset removed (only the change between operating points
compared), ROCETS is clearly more accurate. Near TP1 (O/F 5.42, 5.55) 17 of 20
predictions are within 1 %. At O/F 4.99 and 4.67 the turbine inlet temperature
is 5.5 % and 8.1 % high and the TP4 pressures 2–3.5 % low. Causes:

- single-point affinity pump maps: the tested fuel pump head coefficient is
  nearly flat over the 10 % flow-coefficient range, the generic curve loses 2.6 %;
  the model then runs out of turbine power (TCV closed) at TP3 and TP4. Fitting
  the head curve to the measured slope (`rl10.py headcurve`, uses TP4 data)
  gives 1.02 % mean error, 95 % within 3 %;
- Bartz with frozen specific heat keeps the jacket heat constant across O/F,
  while the test data imply a 6 % drop.

Start transient (`examples/rl10a33a_start.yaml`, pressure-actuated OCV, FCV1 and
TCV): time from start signal to 200 psia is 1.84 s against 1.43–2.56 s measured
on nine starts (mean 1.93 s, CR-195478 Table 2); the start ends within 0.2 % of
the TP1 steady state. The unpublished start-valve settings move the result from
1.26 s to no start, so this is a plausibility check, not a validated prediction.

Published data that had to be resolved: the 2.47 in and 5.13 in chamber
dimensions are radii (the 19.19 in² throat area gives 4.94 in diameter); the
printed 1.05 in² venturi throat would choke at the TP1 fuel flow (1.5 in² used);
the published design point's pump heads are about 5 % above the tested pumps and
its turbine data deliver 12 % less power than its pump data absorb.
