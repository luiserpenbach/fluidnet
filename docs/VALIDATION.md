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
from the geometry and component data in NASA TM-107318 and CR-190786. Pump head,
efficiency and efficiency-speed-correction maps and the turbine efficiency and
flow-area maps are the P&W maps of TM-107318 figures 4.1.1–4.1.8, 4.1.16 and
4.1.17, digitized from the report scans (`validation/rl10/maps/`; check plots
`check_digitized.png`, `check_area.png`, `check_curves.png`). Eight factors are
calibrated at test point 1 of CR-190786 (fuel pump head scale, turbine nozzle
area and peak efficiency, hot-gas heat transfer factor, jacket throat-region
loss, fuel and LOX injector discharge, nozzle thrust efficiency). Reproduce with
`python validation/rl10/rl10.py` (2 min), the comparison cases
`rl10.py generic`, `rl10.py nominal_tcv`, `rl10.py frozen`, and
`python validation/rl10/start.py`; `cargo test` checks the calibration point
and the start.

Map digitization. The markers of each figure are located automatically after
deskewing the scan (0.4–1.4° rotation) and mapping the grid lines to the
printed ticks; 12 points per pump map, 14 per speed correction, 45 of the 52
turbine-area markers (the rest merge with grid lines or each other). Check
against the P&W typical operating point (tables 2.2.1 and 2.3.1, flow and head
coefficients from the definitions in section 10 of the report): the head maps
give the typical-point head within 0.3 % (fuel stage 1), 1.1 % (stage 2) and
0.8 % (LOX); the LOX efficiency map gives 0.639 against 0.642. The fuel-pump
efficiency maps (without P&W's 1.03 factor) lie 2–5 % below the typical-point
efficiencies; the model keeps the typical-point efficiency at the design flow
and uses the map shape. The turbine efficiency polynomial of figure 4.1.17 uses
the RL10 velocity ratio, which carries a √2 factor (section 10); with u/c0 =
X/√2 it gives 0.734 at the TP1 operating point, where the generic-map
calibration had found 0.731. In normalized form the pumps run left of their
best-efficiency flow at the design point (best efficiency at 1.2–1.4 × design
flow coefficient) and the fuel-pump head falls less steeply with flow than the
generic curve.

TCV area. Table 2.6.1 of TM-107318 gives the TCV area (0.0158 in², discharge
coefficient included) as a value "at nominal full-thrust condition (predicted by
model)", not at full open. fluidnet 0.4 used it as the full-open area and held
the valve fully open at TP1, which leaves no bypass to close at the low-O/F
points. The model now has 0.16 in² full open (unknown; any area that keeps the
valve from saturating gives the same predictions) and the valve half open at
TP1 (5.9 % bypass).

Two checks per test point:

- **At measured speed** (component-level): shaft speed prescribed at the
  measured value, mixture-ratio valve set to the measured O/F, TCV as at TP1;
  the other nine measured quantities, thrust and the turbopump power balance are
  outputs. Statistics cover TP2–TP5 (TP1 is the calibration point). The power
  balance is also evaluated with the measured pressures, turbine inlet
  temperature and flows in place of the predicted ones, which tests the
  turbomachinery maps directly.
- **Free running**: only the TCV and mixture-ratio valves are set, to the
  measured thrust and O/F, as done for ROCETS in CR-190786; speed follows from
  the power balance.

| | fluidnet 0.5 | generic maps, same TCV | P&W maps, 0.4 TCV | fluidnet 0.4 | ROCETS |
|---|---|---|---|---|---|
| Free running, TP2–5 (40 values): mean / max error | **0.62 % / 2.05 %** | 0.67 % / 3.93 % | 2.64 % / 8.91 % | 2.92 % / 9.71 % | 1.26 % / 3.13 % |
| Free running, within 1 % | 80 % | 78 % | 42 % | 42 % | 55 % |
| Free running, TP1 offset removed: mean / max | 0.66 % / 2.05 % | 0.62 % / 2.56 % | 2.78 % / 8.91 % | 3.02 % / 9.71 % | 0.38 % / 1.58 % |
| Free running, LOX pump discharge pressure, largest error | 1.25 % | 3.93 % | 6.20 % | 5.27 % | 2.78 % |
| At measured speed, TP2–5 (36 values): mean / max | 0.93 % / 2.17 % | 0.90 % / 3.57 % | 0.93 % / 2.17 % | 0.90 % / 3.57 % | – |
| Power balance with measured pressures, TP3 / TP4 | −1.0 % / −2.1 % | −2.9 % / −5.3 % | −0.9 % / −2.1 % | – | – |
| Power balance at measured speed, TCV as at TP1, TP3 / TP4 | −6.3 % / −9.7 % | −8.3 % / −12.8 % | −6.3 % / −9.6 % | −8.3 % / −12.8 % | – |
| Calibrated turbine peak efficiency (P&W map: 0.744) | 0.788 | 0.773 | 0.751 | 0.736 | – |
| TCV bypass TP1 → TP4 | 5.9 % → 2.0 % | 5.9 % → 1.0 % | 1.2 % → 0 (saturated) | 1.2 % → 0 (saturated) | – |

What the comparison shows:

1. The free-running error of fluidnet 0.4 at TP3 and TP4 came from the TCV
   assumption: with the published nominal area taken as full open, the valve
   saturates closed and the engine cannot reach the test thrust. With a valve
   that has the range to close, fluidnet's free-running predictions are within
   2.1 % at all points (mean 0.62 %), against 3.1 % and 1.26 % for ROCETS.
2. The P&W maps are consistent with the test data: with the measured pressures
   and temperatures the turbopump power balance closes within 2.1 % at TP4,
   against 5.3 % with the generic maps. They remove the largest single error
   of the generic model (LOX pump discharge pressure, 3.9 % → 1.25 %) and the
   largest measured-speed error (3.6 % → 2.2 %). On the mean error the maps
   change little, because the TCV absorbs a power-balance error when it is
   solved per point.
3. With the TCV held at its TP1 position the predicted turbine falls 6–10 %
   short of the pumps at TP3/TP4. Most of this is the predicted turbine
   pressure ratio (1.381 against 1.424 implied by the test pressures at TP4):
   the measured pressure drop venturi inlet → fuel manifold rises 19 % from
   TP1 to TP4 and the model's 11 %. Closing the TCV, as the solved openings do
   (5.9 % → 2.0 % bypass), raises the turbine flow and pressure ratio by this
   amount. The test points do not record the TCV position.
4. Open point: with 5.9 % bypass at TP1 the calibrated turbine peak efficiency
   is 0.788, 6 % above the P&W map (0.744); with the 1.2 % bypass of the 0.4
   assumption it is 0.751, 1 % above. The power level at TP1 and the trend from
   TP1 to TP4 are therefore not both consistent with the P&W turbine map at the
   published pump efficiencies; TM-107318 itself notes that its two turbine
   flow models disagree at the design point.

Start transient (`examples/rl10a33a_start.yaml`, pressure-actuated OCV, FCV1 and
TCV): time from start signal to 200 psia is 1.14 s against 1.43–2.56 s measured
on nine starts (mean 1.93 s, CR-195478 Table 2); the start ends within 0.1 % of
the TP1 chamber pressure. fluidnet 0.4 gave 1.51 s. The P&W turbine map gives
33 % more stall torque per unit efficiency than the generic parabola, and the
higher calibrated turbine efficiency adds to it. The start is sensitive to the
turbine efficiency, as TM-107318 reports: with the P&W peak efficiency (0.744)
the start takes 1.30 s, with 0.70 it takes 1.57 s. The efficiency speed
correction (+0.20 s) and the turbine flow-area map (+0.24 s) slow it. The
unpublished start-valve settings move it between 0.99 and 1.40 s. Below 20 %
speed the pumps still use the generic zero-speed curves; ROCETS used the
extended maps of figures 4.1.10–4.1.15 there.

Published data that had to be resolved: the 2.47 in and 5.13 in chamber
dimensions are radii (the 19.19 in² throat area gives 4.94 in diameter); the
printed 1.05 in² venturi throat would choke at the TP1 fuel flow (1.5 in² used);
the published design point's pump heads are about 5 % above the tested pumps;
the TCV area of table 2.6.1 is a nominal part-open value.
