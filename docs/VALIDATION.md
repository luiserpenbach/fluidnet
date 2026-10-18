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
Reproduce with `python validation/rl10/rl10.py` (2.5 min including the
measured-speed check), `rl10.py frozen` and `rl10.py headcurve` (comparison
cases) and `python validation/rl10/start.py`; `cargo test` checks the
calibration point and the start.

Two checks per test point:

- **At measured speed** (component-level): the shaft speed is prescribed at the
  measured value and the mixture-ratio valve set to the measured O/F; the other
  nine measured quantities, thrust and the turbopump power balance are outputs.
  This tests the flow, heat-transfer and combustion models without the
  turbomachinery maps.
- **Free running**: only the bypass (TCV) and mixture-ratio valves are set, to the
  measured thrust and O/F, as done for ROCETS; speed follows from the power
  balance.

| | fluidnet 0.4 (enthalpy driving, contour) | fluidnet 0.3 (frozen c_p ΔT) | ROCETS |
|---|---|---|---|
| At measured speed, 45 values: mean / max error | 0.79 % / 3.6 % | 1.07 % / 7.2 % | – |
| At measured speed: turbine inlet T error, TP3 / TP4 | +0.7 % / +0.4 % | +5.4 % / +7.2 % | – |
| At measured speed: turbine power − pump power, TP3 / TP4 | −8.3 % / −12.8 % | −2.7 % / −4.8 % | – |
| Free running, TP2–5, 40 values: mean / max error | 2.92 % / 9.7 % | 1.39 % / 8.1 % | 1.26 % / 3.1 % |
| Free running, mean error with TP1 offset removed | 3.02 % | 1.35 % | 0.38 % |

The enthalpy driving potential (recombination heat at the wall) reproduces the
measured fall of turbine inlet temperature with mixture ratio; the frozen-c_p
form kept the jacket heat constant and overpredicted it by 5–7 % at O/F 4.99
and 4.67. That excess heat had hidden two thirds of a turbopump power
deficit: with the correct heat, the model's turbine delivers 8–13 % less power
than its pumps absorb at the low-O/F points, so running free it cannot reach
the test thrust there (TCV closed) and the TP3/TP4 pressures fall 4–10 % low.
The deficit lies in the generic single-design-point pump and turbine maps
(TM-107318's own design point shows a 12 % gap between its turbine and pump
data). Flattening the fuel pump head curve to the measured head coefficient
(`rl10.py headcurve`, uses TP4 data) moves the free-running mean error to
2.6 %; the measured maps (TM-107318 figures 4.1.1–4.1.17) are the next step.
fluidnet 0.4 keeps the enthalpy potential as the default because it is right
for the right reason.

Start transient (`examples/rl10a33a_start.yaml`, pressure-actuated OCV, FCV1 and
TCV): time from start signal to 200 psia is 1.51 s (fluidnet 0.3: 1.84 s)
against 1.43–2.56 s measured on nine starts (mean 1.93 s, CR-195478 Table 2);
the start ends within 0.3 % of the TP1 steady state. The unpublished
start-valve settings move the result between 1.20 and 2.34 s, so this is a
plausibility check, not a validated prediction.

Published data that had to be resolved: the 2.47 in and 5.13 in chamber
dimensions are radii (the 19.19 in² throat area gives 4.94 in diameter); the
printed 1.05 in² venturi throat would choke at the TP1 fuel flow (1.5 in² used);
the published design point's pump heads are about 5 % above the tested pumps and
its turbine data deliver 12 % less power than its pump data absorb.
