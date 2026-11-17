# DEX1-SYS-DD-001: Dual Expander Engine Concept and Powerpack Preliminary Design

| Field | Value |
|---|---|
| Document | DEX1-SYS-DD-001 |
| Revision | A, v0.2 (draft for review; v0.2 adds the fluidnet network model and start sequence, section 8) |
| Date | 2026-11-10 |
| Program | DEX1 (program code proposed, see OP-01) |
| Configuration | DEX1-A, file `configs/dex1_baseline.yaml`, hash `d51bd6c0b1ff` |
| Toolchain | `dexfm` 0.2.0 with `fluidnet` 0.5.0, 12 physics gates passing (DEX1-SYS-AN-001) |
| Status of numbers | ⚙️ toolchain-generated unless marked ⚠️ ASSUMPTION |

## 1. Summary

This document starts the design of a liquid hydrogen (LH2) / liquid oxygen (LOX) engine for a large lunar lander. It derives the thrust class, trades the engine cycle and the number of thrust chambers and powerpacks, and defines a baseline with a preliminary design of the powerpack. A powerpack is one hydrogen turbopump plus one oxygen turbopump with their turbines and control valves.

The baseline, DEX1-A, is a 180 kN vacuum thrust propulsion system made of four 45 kN thrust chambers fed by two identical powerpacks, each powerpack feeding two diagonally opposite chambers. The cycle is a closed dual expander: hydrogen heated in the throat and nozzle region drives the fuel turbopump, and oxygen heated in the injector end of the chamber drives the oxygen turbopump. No drive gas of one propellant ever meets the pump of the other, so no interpropellant seal and no helium purge are needed. The design chamber pressure is 50 bar at a mixture ratio (oxidizer to fuel mass ratio) of 6.0, with a delivered vacuum specific impulse of 456 s.

Three results carry the design. First, splitting the thrust over several small chambers raises the heat picked up per kilogram of propellant from 332 kJ/kg (one chamber) to 568 kJ/kg (four chambers on two powerpacks), and this is what lets a dual expander close at a useful chamber pressure at all: with one 180 kN chamber the cycle closes only up to 28 bar, with four chambers on two powerpacks up to 58 bar. Second, sharing a powerpack between two chambers keeps the pumps large enough to be efficient; four independent 45 kN engines close only up to 46 bar. Third, two powerpacks give a system throttle range of 4.4 to 1 with every component inside its comfortable range (each powerpack between 45 % and 100 %), because one powerpack can be shut down.

A tank-head start that reaches mainstage in about 3.5 s was found in the fluidnet network transient with one closed-loop valve; open-loop schedules fail. The largest open risks are oxygen cooling below the critical pressure of oxygen during deep throttling, and the start from a cold liner, which does not bootstrap with the present sequence.

## 2. Mission basis and requirements

### 2.1 Thrust class

The user input is a large lunar lander with a multi-chamber engine. Two reference vehicles bound the requirement. Reference A is a cargo and crew lander of the Blue Moon Mark 2 class: more than 45 t launch mass and 16 t dry mass, flown with three BE-7 engines of 44.5 kN each [1][2]. Reference B is an Amphora propellant tanker that lifts off the lunar surface full and lands nearly empty. ⚠️ ASSUMPTION: 60 t lift-off mass and 14 t landing mass for Reference B; these are placeholders until the network manager model supplies tanker sizing.

| Case | Mass | Lunar weight | 180 kN, both packs | 90 kN, one pack |
|---|---|---|---|---|
| A, start of powered descent | 45 t | 73 kN | 4.0 m/s², thrust-to-weight 2.5 | 2.0 m/s², thrust-to-weight 1.2 |
| A, touchdown (about 29 t after a 2 km/s descent) | 29 t | 47 kN | hover at 26 % of total | hover at 52 % of one pack |
| B, surface lift-off | 60 t | 97 kN | thrust-to-weight 1.85 | 0.93 (no lift-off on one pack) |
| B, touchdown | 14 t | 23 kN | hover at 12.6 % of total | hover at 25 % of one pack |

A total of 180 kN gives Reference A a higher initial deceleration than the 133.5 kN of three BE-7 engines provide, keeps the descent flyable on one powerpack, and gives the full tanker a lift-off thrust-to-weight ratio of 1.85 in lunar gravity.

### 2.2 Requirements

| ID | Requirement | Value | Source |
|---|---|---|---|
| R-01 | Total vacuum thrust | 180 kN | section 2.1 |
| R-02 | Continuous system throttle range | 22.5 % to 100 % (4.4:1) with oxygen coolant supercritical | Reference A hover, section 7 |
| R-03 | Extended low thrust | down to 12.5 % (8:1) | Reference B hover; conditional on OP-03 |
| R-04 | Vacuum specific impulse | at least 450 s | ⚠️ ASSUMPTION |
| R-05 | Mixture ratio | 6.0, controllable 5.5 to 6.5 | ⚠️ ASSUMPTION |
| R-06 | Operation after loss of one powerpack | 50 % thrust, symmetric about the vehicle axis | fault tolerance |
| R-07 | No consumable purge gas in steady operation | no helium for seals | lunar refuelling: helium cannot be resupplied from lunar water |
| R-08 | Restarts and life | at least 50 starts, 3 h accumulated firing | ⚠️ ASSUMPTION, reusable lander |
| R-09 | Pump inlet conditions | LH2 2.5 bar at 21.5 K, LOX 3.0 bar at 91 K | ⚠️ ASSUMPTION |
| R-10 | Envelope | total nozzle exit area 3.0 m², engine length under 2.3 m | ⚠️ ASSUMPTION, lander base and ground clearance |

Each requirement must justify its existence at the next review or be deleted. R-03 and R-08 are the two most likely to change.

## 3. Cycle selection

### 3.1 Options

In a closed expander cycle the turbines are driven by propellant that has been heated in the cooling jacket and is then burned in the chamber. Three arrangements were modelled on the same four-chamber, two-powerpack system (total exit area 3.0 m², 10 % turbine bypass held as control margin, 750 K wall limit). "Characteristic length" is chamber volume divided by throat area and sets the chamber length.

| Arrangement | Characteristic length 1.2 m | 1.6 m | 2.0 m |
|---|---|---|---|
| Single expander: hydrogen cools everything and drives both pumps | 62.2 bar | 67.8 bar | 72.5 bar |
| Dual expander, oxygen cools the injector end of the cylinder | 42.5 bar | 55.6 bar | 58.4 bar |
| Dual expander, oxygen cools the nozzle from area ratio 3 to 40 | 45.3 bar | 45.3 bar | 45.3 bar |

Values are the maximum chamber pressure at which the power balance closes ⚙️. The best oxygen zone extent was 85 %, 70 % and 70 % of the cylinder length for the three chamber lengths.

The dual expander costs about 14 bar of closing pressure against a single expander on the same chamber. The reason is thermodynamic: heat put into oxygen yields less turbine work than heat put into hydrogen, because a large part goes into bringing oxygen from liquid to gas before it can expand usefully. The nozzle-cooled variant is limited by the oxygen side regardless of chamber length, since the nozzle heat flux is low; it would be the choice only if oxygen in the high-flux barrel proves unacceptable.

A fourth arrangement, in which oxygen is heated in a heat exchanger by the hydrogen turbine exhaust instead of by the chamber wall, removes oxygen from the hot wall but puts a static hydrogen-to-oxygen wall inside the engine. It is not modelled yet (OP-07).

### 3.2 Decision

The dual expander with an oxygen-cooled barrel section is selected, for four reasons. It needs no interpropellant seal package and no helium purge, which matters for a vehicle refuelled from lunar water (R-07). Each shaft runs at its own best speed without a gearbox: 106,400 rpm for hydrogen and 38,950 rpm for oxygen in the baseline. Both propellants reach the injector warm (hydrogen at 161 K, oxygen at 212 K), and section 7 shows that this keeps the injector pressure drop ratio from collapsing during throttling. Mixture ratio is controlled by the oxygen turbine alone, independent of thrust control. The price is 14 bar of chamber pressure, worth under 3 s of specific impulse at fixed exit area, and hot high-pressure oxygen in a turbine and in the liner (section 9). The BE-7 engine is a dual expander of the same thrust class as one DEX1 chamber, and its thrust chamber testing was reported as extracting energy from hydrogen-cooled and oxygen-cooled combustor segments [1][3], which supports the feasibility of the cooling split.

## 4. Number of chambers and powerpacks

All architectures below deliver 180 kN inside 3.0 m² of total exit area with a characteristic length of 2.0 m. The design chamber pressure is set to 80 % of the maximum closing pressure. Masses are rough ⚠️ ASSUMPTION-based estimates (areal densities and a two-point turbopump power law) and should be used only for ranking.

| Architecture | Max. closing pressure | Jacket heat per kg propellant | Fuel pump efficiency | Engine length | Mass estimate | Thrust after one pack out | Turbopumps |
|---|---|---|---|---|---|---|---|
| 1 chamber, 1 pack | 28.4 bar | 332 kJ/kg | 0.790 | 3.51 m | 345 kg | 0 % | 2 |
| 2 chambers, 1 pack | 54.7 bar | 425 kJ/kg | 0.792 | 2.76 m | 318 kg | 0 % | 2 |
| 2 chambers, 2 packs | 50.0 bar | 424 kJ/kg | 0.759 | 2.75 m | 363 kg | 50 % | 4 |
| 3 chambers, 3 packs | 50.0 bar | 495 kJ/kg | 0.730 | 2.39 m | 411 kg | 67 % | 6 |
| 4 chambers, 1 pack | 64.1 bar | 576 kJ/kg | 0.783 | 2.19 m | 342 kg | 0 % | 2 |
| **4 chambers, 2 packs** | **58.4 bar** | **568 kJ/kg** | **0.748** | **2.19 m** | **388 kg** | **50 %** | **4** |
| 4 chambers, 4 packs | 46.2 bar | 541 kJ/kg | 0.714 | 2.18 m | 454 kg | 75 % | 8 |
| 6 chambers, 2 packs | 65.9 bar | 697 kJ/kg | 0.728 | 1.94 m | 421 kg | 50 % | 4 |
| 6 chambers, 3 packs | 58.4 bar | 673 kJ/kg | 0.711 | 1.93 m | 451 kg | 67 % | 6 |

Two effects compete. More chambers means smaller throats and more wetted area per unit of propellant flow, so more heat per kilogram and a higher closing pressure; engine length also falls with the square root of the chamber count because the same total exit area is split into smaller nozzles. More powerpacks means smaller pumps with lower efficiency, and the hydrogen pump runs into its bearing speed limit (DN, bearing bore in millimetres times speed in revolutions per minute) instead of its best specific speed. This is why three independent engines of the Blue Moon arrangement and four independent engines both close lower than four chambers on two packs.

Four chambers on two powerpacks is selected. It keeps nearly all of the heat advantage of the multi-chamber layout (568 against 576 kJ/kg), has redundancy in the turbomachinery where engine failures concentrate, and loses thrust symmetrically when a powerpack is shut down because each pack feeds a diagonal pair. Four chambers on one pack closes 6 bar higher and is 46 kg lighter but has no engine-out capability and half the throttle range. Six chambers on two packs gains 7 bar and 0.25 m of length for two more chambers, injectors and igniters; it is the growth option if the envelope tightens.

A chamber-out mode inside one pack (one of its two chambers isolated) is not analysed and is not planned as an operating mode (OP-08).

## 5. Baseline DEX1-A

### 5.1 Design point

The chamber is made long (characteristic length 2.0 m, 544 mm from injector face to throat) to buy heat margin. At this length the hardware closes up to 58.4 bar; the design point is set at 50 bar, which the model still closes with the full 10 % bypass margin if the real heat pickup is 30 % below prediction (section 6.2).

| Parameter | Value ⚙️ |
|---|---|
| Total vacuum thrust | 180 kN (4 × 45.0 kN) |
| Chamber pressure | 50.0 bar |
| Mixture ratio | 6.0 |
| Vacuum specific impulse, delivered | 456.0 s (ideal 474.8 s; efficiencies 0.985 and 0.975 ⚠️) |
| Characteristic velocity, ideal | 2,299 m/s |
| Chamber temperature | 3,452 K |
| Total propellant flow | 40.25 kg/s (hydrogen 5.75, oxygen 34.50) |
| Flow per powerpack | hydrogen 2.875 kg/s, oxygen 17.25 kg/s |
| Nozzle area ratio | 164.6 |
| Throat / chamber / exit diameter | 76.2 mm / 152.4 mm / 977 mm |
| Cylinder length / injector to throat | 462 mm / 544 mm |
| Nozzle length / engine length | 1.345 m / 2.19 m (0.30 m allowance for injector and mount ⚠️) |
| Regeneratively cooled up to area ratio | 12, radiatively cooled extension beyond (2.75 m² per chamber) |
| Heat pickup per chamber | 5.81 MW (hydrogen zone 3.26 MW, oxygen zone 2.54 MW) |
| Dry mass estimate | 391 kg, thrust-to-weight about 47 ⚠️ |

### 5.2 Cooling zones

Oxygen cools the 70 % of the cylinder next to the injector and flows toward the injector. Hydrogen enters at area ratio 12, cools the nozzle, throat, convergent and the remaining 30 % of the cylinder in counterflow.

| Parameter ⚙️ | Hydrogen zone | Oxygen zone |
|---|---|---|
| Axial extent from injector face | 333 to 706 mm | 0 to 320 mm |
| Hot-side area | 0.203 m² | 0.160 m² |
| Flow per chamber | 1.44 kg/s (all hydrogen) | 6.90 kg/s (80 % of oxygen) |
| Inlet state | 105.6 bar, 32.5 K | 86.8 bar, 94.8 K |
| Outlet state | 93.8 bar, 174 K | 81.3 bar, 294 K |
| Pressure drop | 10.9 bar | 4.7 bar |
| Heat flux, mean / peak | 16.1 / 49.9 MW/m² | 15.9 / 16.8 MW/m² |
| Hot-gas-side wall temperature, maximum | 750 K (at limit) | 746 K |
| Coolant-side wall temperature, maximum | 712 K | 709 K |
| Channels | 135 to 270, height 2.4 to 6.0 mm | 270, height 6.0 mm |
| Coolant velocity, maximum | 173 m/s (Mach 0.24) | 36 m/s |

Liner assumptions ⚠️: copper alloy of the GRCop-42 class with 320 W/m/K, 0.8 mm hot wall, 0.8 mm minimum rib, channel width 1.0 mm at the throat and at most 2.5 mm. The oxygen zone currently uses the full 6 mm channel height because the wall stays just under the limit there; a coolant-side wall at 709 K in 81 bar oxygen is not acceptable as it stands, and the oxygen zone needs its own lower wall temperature limit and shallower channels (OP-04).

### 5.3 State points of one powerpack

| Station ⚙️ | Hydrogen side | Oxygen side |
|---|---|---|
| Tank / pump inlet | 2.5 bar, 21.5 K | 3.0 bar, 91.0 K |
| Pump discharge | 113.5 bar, 32.5 K | 93.3 bar, 94.8 K |
| Cooling zone inlet | 105.6 bar | 86.8 bar (80 % of flow; 20 % direct to injector) |
| Turbine inlet | 93.8 bar, 174 K | 81.3 bar, 294 K |
| Turbine outlet | 59.2 bar, 160 K | 60.8 bar, 274 K |
| Injector manifold, after mixing with bypass and direct flow | 59.2 bar, 161 K | 60.8 bar, 212 K (126 kg/m³) |
| Chamber | 50.0 bar | 50.0 bar |

## 6. Powerpack preliminary design

### 6.1 Turbopumps

| Parameter ⚙️ | Fuel turbopump | Oxygen turbopump |
|---|---|---|
| Mass flow / volume flow | 2.875 kg/s / 41.3 l/s | 17.25 kg/s / 15.2 l/s |
| Pressure rise | 111.0 bar | 90.3 bar |
| Head / stages | 16,280 m / 2 | 810 m / 1 |
| Speed | 106,400 rpm | 38,950 rpm |
| Speed limited by | bearing DN (2.67 million with 25 mm bore) | suction specific speed (29,500 US units) |
| Stage specific speed nq | 25.2 | 31.6 |
| Impeller diameter / tip speed | 71.7 mm / 400 m/s | 61.8 mm / 126 m/s |
| Inducer tip diameter | 47.6 mm | 47.6 mm |
| Available net positive suction head | 176 m | 17 m |
| Pump efficiency | 0.740 | 0.710 |
| Shaft power / torque | 620 kW / 55.7 N·m | 193 kW / 47.3 N·m |
| Turbine type | axial, 2 stages | axial, 1 stage |
| Turbine flow (10 % bypassed) | 2.59 kg/s | 12.42 kg/s |
| Turbine pressure ratio | 1.58 | 1.34 |
| Turbine mean diameter / blade height | 56.6 mm / 8.1 mm | 54.2 mm / 11.3 mm |
| Mean blade speed / velocity ratio | 316 m/s / 0.55 | 111 m/s / 0.55 |
| Turbine efficiency / power | 0.762 / 649 kW | 0.793 / 199 kW |
| Rotor inertia, estimate ⚠️ | 5.0e-4 kg·m² | 2.1e-4 kg·m² |

The fuel turbopump is a small, very fast machine: two shrouded impellers of 72 mm on a shaft turning at 106,400 rpm, with a two-stage turbine of 57 mm mean diameter on the same shaft. Its speed is set by the bearing limit, not by hydraulics. Lifting the DN limit (hydrostatic or foil bearings fed with hydrogen, which also suit R-08) would raise specific speed and efficiency, and is the first trade for the detailed design (OP-05). Full admission is retained in both turbines; blade heights of 8 mm and 11 mm are manufacturable.

The oxygen turbopump is deliberately gentle. The pump tip speed is 126 m/s and the turbine blade speed 111 m/s, far below structural limits, which keeps rub and particle-impact ignition energy low in 81 bar oxygen at 294 K. Speed is set by suction performance at 1.9 bar of net positive suction pressure; a higher tank pressure or a boost stage is not needed for the cycle to close.

### 6.2 Margins of the power balance

| Case ⚙️ (hardware of the baseline) | Maximum closing chamber pressure |
|---|---|
| Baseline | 58.4 bar |
| Heat transfer 20 % below prediction | 54.7 bar |
| Heat transfer 30 % below prediction | 50.9 bar |
| Heat transfer 40 % below prediction | 44.4 bar |
| Heat transfer 20 % above prediction | 59.4 bar |
| Pump efficiency 5 points lower | 56.6 bar |
| Turbine efficiency 5 points lower | 56.6 bar |
| Wall temperature limit 650 K / 850 K | 55.6 bar / 60.3 bar |
| Bypass margin 5 % / 20 % | 62.2 bar / 52.8 bar |
| Mixture ratio 5.5 / 6.5 | 58.4 bar / 59.4 bar |

The design point of 50 bar is below every single-parameter case except heat pickup more than 30 % under prediction. The heat transfer model is calibrated on one engine and is quoted at plus or minus 20 % (DEX1-SYS-AN-001), so the design holds margin against that uncertainty with the control bypass untouched.

### 6.3 Valves and lines

Flow areas are effective areas (discharge coefficient times area) of one powerpack; equivalent bores assume a discharge coefficient of 0.7.

| Component ⚙️ | Function | Effective area at design | Equivalent bore | Range needed for throttling to 15 % |
|---|---|---|---|---|
| Fuel turbine bypass valve | thrust control | 31 mm² | 7.6 mm | up to 16 times design area |
| Oxygen turbine bypass valve | mixture ratio control | 65 mm² | 10.8 mm | up to 15 times design area |
| Oxidizer control valve (direct liquid path) | holds heated oxygen fraction | 45 mm² | 9.0 mm | down to 0.47 of design area |
| Oxygen back-pressure valve (downstream of oxygen turbine) | keeps oxygen coolant supercritical | open | n/a | closes to take up to 18 bar at 45 % |
| Fuel turbine nozzles | fixed | 282 mm² | 22.7 mm | fixed |
| Oxygen turbine nozzles | fixed | 581 mm² | 32.5 mm | fixed |
| Fuel injector (two chambers) | fixed | 733 mm² | n/a | fixed |
| Oxygen injector (two chambers) | fixed | 1,046 mm² | n/a | fixed |

Main fuel and main oxidizer shut-off valves sit between pump discharge and the cooling zones. Preliminary line bores from velocity limits ⚠️: LH2 pump inlet 81 mm (8 m/s) and discharge 51 mm (20 m/s), hydrogen gas to the turbine 71 mm (60 m/s), LOX pump inlet 62 mm (5 m/s) and discharge 40 mm (12 m/s), oxygen gas to the turbine 63 mm (40 m/s).

Each powerpack carries four modulating valves. All four are needed: two for thrust and mixture ratio, one because a fixed orifice on the direct oxygen path lets the heated oxygen fraction fall during throttling (in an earlier model run with a fixed orifice the oxygen turbine inlet passed 550 K at 20 % thrust), and one for the supercritical floor.

## 7. Throttling

The throttle solver freezes the baseline hardware and finds shaft speeds, flows and valve positions for a commanded chamber pressure at mixture ratio 6.0.

| Pack thrust ⚙️ | Chamber pressure | Shaft speed fuel / oxygen | Turbine bypass flow fuel / oxygen | Turbine inlet temperature fuel / oxygen | Pump flow coefficient relative to design, fuel / oxygen | Injector pressure drop over chamber pressure, fuel / oxygen | Oxygen circuit outlet pressure |
|---|---|---|---|---|---|---|---|
| 100 % | 50.0 bar | 100 / 100 % | 0.11 / 0.10 | 175 / 294 K | 1.00 / 1.00 | 0.185 / 0.216 | 81 bar |
| 80 % | 40.0 bar | 85 / 86 % | 0.22 / 0.22 | 178 / 305 K | 0.95 / 0.93 | 0.187 / 0.235 | 63 bar |
| 60 % | 30.0 bar | 69 / 71 % | 0.34 / 0.34 | 183 / 324 K | 0.87 / 0.84 | 0.191 / 0.263 | 46 bar |
| 50 % | 25.0 bar | 61 / 64 % | 0.40 / 0.39 | 188 / 333 K | 0.82 / 0.79 | 0.195 / 0.277 | 38 bar |
| 40 % | 20.0 bar | 53 / 56 % | 0.46 / 0.45 | 193 / 347 K | 0.76 / 0.72 | 0.200 / 0.295 | 30 bar |
| 25 % | 12.5 bar | 40 / 42 % | 0.56 / 0.54 | 208 / 380 K | 0.64 / 0.60 | 0.214 / 0.336 | 19 bar |
| 15 % | 7.5 bar | 29 / 31 % | 0.64 / 0.62 | 226 / 421 K | 0.53 / 0.49 | 0.231 / 0.383 | 11 bar |

The cycle has surplus power everywhere below the design point: bypass flow rises steadily and turbine inlet temperatures rise because heat flux falls more slowly than flow. Both injector pressure drop ratios increase as thrust falls, since both propellants are injected as low-density fluids that get warmer; this is the benefit of the dual expander for throttling stability. Pump flow coefficients stay above half of design down to 15 %. Hot wall temperatures stay within 45 K of the limit at all points.

The limiting item is the oxygen cooling circuit. Without action its pressure falls below the critical pressure of oxygen (50.4 bar) at about 65 % thrust, and oxygen then boils in the channels at a heat flux of several MW/m². With the back-pressure valve the circuit is held at 55 bar down to 45 % thrust, at which point the oxygen turbine bypass is nearly closed (0.08) and no power is left to hold the floor lower.

| Operating mode | Thrust range | Oxygen coolant |
|---|---|---|
| Two powerpacks, 45 % to 100 % each | 81 to 180 kN | supercritical |
| One powerpack, 45 % to 100 % | 40.5 to 90 kN | supercritical |
| One powerpack, 15 % to 45 % | 13.5 to 40.5 kN | sub-critical, boiling in the liner |

The two ranges overlap between 81 and 90 kN, so thrust is continuous from 40.5 to 180 kN (22.5 % to 100 %, R-02) with supercritical oxygen. Reference A hovers at touchdown inside this range on one powerpack. The empty tanker of Reference B needs 23 kN, which lies in the sub-critical band; the model converges there, but its oxygen heat transfer is not credible in that regime (OP-03). Constant combustion and nozzle efficiencies are assumed across the throttle range ⚠️.

## 8. Start

### 8.1 Network model

One powerpack was rebuilt as a `fluidnet` model (two shafts, two cooled lines heated by one equivalent chamber, both turbine bypass valves, the oxidizer control valve, pump discharge lines and manifolds; about 100 states). The model file is generated from the dexfm design point. Thirteen parameters that the two tools model differently (flow areas, hot-gas heat transfer factors, channel hydraulic diameters, turbine peak efficiencies) were fitted so that the fluidnet steady state reproduces the design point; the residual is 2e-5. The fitted turbine efficiencies are 0.746 (fuel) and 0.796 (oxygen) against 0.762 and 0.793 from the dexfm correlations, so two independent turbine and flow models agree on the power balance within two points. With both shafts free the network settles at 49.95 bar and mixture ratio 6.00.

### 8.2 Start sequence that works

| Event ⚙️ | Time |
|---|---|
| Main fuel valve opens (liner at 250 K, tanks at 2.5 and 3.0 bar) | 0 to 0.2 s |
| Main oxidizer valve opens | 0.35 to 0.6 s |
| Ignition | 0.5 s |
| Fuel turbine bypass at 3 times design area, then ramped to design | held to 1.0 s, ramp to 3.0 s |
| Oxidizer control valve (direct liquid path) opens | 0.6 to 2.5 s |
| Oxygen turbine bypass | closed loop from t = 0 |

The oxygen turbine bypass valve is pressure-actuated: its opening is proportional to the oxygen pump discharge pressure minus the fuel pump discharge pressure, fully closed at minus 22 bar and fully open (16 times design area) at plus 10 bar. At the design point the difference is minus 20 bar and the valve sits at its design area. The effect is that the oxygen shaft can only run up as far as the fuel shaft has already gone, which keeps the mixture fuel-rich during bootstrap.

| Result ⚙️ | Value |
|---|---|
| Chamber pressure at ignition | 0.9 bar |
| 90 % of design chamber pressure | 2.8 s |
| Within 1 % of design chamber pressure | about 3.5 s |
| End state at 6 s | 50.1 bar, mixture ratio 6.03, both shafts at 100 % |
| Mixture ratio after ignition | 1.4 minimum, 6.7 peak at 0.70 s |
| Maximum shaft speed | 100 % fuel, 100.2 % oxygen (no overspeed) |
| Steepest pressure rise | 6.6 to 32.9 bar between 0.6 and 0.7 s |

The pressure rise just after ignition is too steep for hardware and the mixture ratio peak of 6.7 coincides with it; both come from the main oxidizer valve opening fully within 0.25 s and need shaping (OP-02).

### 8.3 What fails

| Case ⚙️ | Outcome |
|---|---|
| Fuel turbine bypass at design area from t = 0 (open loop) | fuel shaft at 111 % by 0.78 s with oxygen shaft at 9 %; cold-flow chamber pressure blocks and reverses oxygen flow |
| Fuel bypass 8 times open, oxygen bypass closed (open loop) | oxygen shaft at 100 % with fuel shaft at 25 %; fuel flow reverses after 1 s |
| Liner at 290 K | starts, mixture ratio peak 7.7 |
| Liner at 150 K or 100 K (restart after a long coast) | no bootstrap: stays at 3 bar fuel-rich idle, fuel shaft 44 to 64 %, oxygen shaft under 16 % |
| Hot-gas heat transfer 20 % lower | settles at 37.5 bar and mixture ratio 5.2 with valves at design position |
| Hot-gas heat transfer 20 % higher | settles at 60 bar with shafts at 113 % and 117 % |

The two open-loop failures are the same two modes the lumped dexfm start model had shown: a dual expander has two independent bootstraps with shaft time constants of a few tenths of a second, and one of them must be slaved to the other. The last three rows show what is still missing. The fuel turbine bypass is on a fixed schedule, so the end state follows the available heat; it needs chamber pressure feedback. The cold-liner case needs its own sequence, because the present one relies on about 1.2 MJ of heat stored in the liners of each chamber between 250 K and 60 K ⚠️.

The network model matches heat pickup and pressure drop of the cooling zones but not their wall temperatures (it reports 310 K and 535 K where dexfm designs to 750 K), because it represents the channels as round tubes. Liner temperatures from the start runs are therefore not quoted.

## 9. Risk register

| ID | Risk | Assessment | Mitigation or decision path |
|---|---|---|---|
| RK-01 | Boiling oxygen in the liner below 45 % pack thrust | open, could remove R-03 | heated-tube tests in oxygen; alternative of moving oxygen cooling to the nozzle (closes to 45 bar) or the heat exchanger arrangement |
| RK-02 | Start sequencing of two coupled bootstraps | warm-liner start found in the network model; cold-liner start open | chamber pressure feedback on the fuel bypass, separate cold-start sequence, start tests on a powerpack rig before engine level |
| RK-03 | Hot-gas heat pickup differs from prediction | design holds to minus 30 % | calorimeter or subscale chamber before liner freeze; chamber length is the adjustment |
| RK-04 | Hot high-pressure oxygen in turbine, ducts and liner (294 K at design, 420 K at 15 %) | manageable by design | burn-resistant alloys, low blade speed already chosen, lower wall limit in oxygen zone (OP-04), cleanliness as a system requirement |
| RK-05 | Fuel turbopump at 106,400 rpm and DN 2.67 million | development risk | bearing trade (OP-05), rotordynamics at preliminary design |
| RK-06 | Liner life at 50 MW/m² and 750 K wall for 50 starts | not analysed | strain range and ratchetting with `channel_fem` on the throat section |
| RK-07 | Leak between oxygen channel and hot gas | failure mode to analyse | published dual-cooling studies report benign behaviour for small leaks [5]; needs own failure analysis |
| RK-08 | Uneven flow split between the two chambers of a pack | not analysed | trim orifices per chamber, check in the network model |

## 10. Kill criteria and next steps

The baseline is abandoned or reworked if any of the following is found. The measured jacket heat pickup is more than 30 % under prediction and the chamber cannot be lengthened within the envelope. Oxygen cooling of the barrel cannot be made stable above 45 % pack thrust. No start is found for a cold liner, if restart after a long coast is required. The lander study shows that the tanker hover case (R-03) is a hard requirement and RK-01 remains open.

The next design loop, in order: wire `h2cea` (OP-M2 in DEX1-SYS-AN-001); shape the start, add chamber pressure feedback and find a cold-liner sequence (OP-02); add an oxygen-specific wall limit and coolant correlation and redo the zone split (OP-03, OP-04); mean-line design of both turbopumps with the bearing trade (OP-05); injector concept for warm gas-like oxygen and hydrogen; liner thermo-structural check of the throat.

## 11. Open points

| ID | Open point | Owner | Decision milestone |
|---|---|---|---|
| OP-01 | Program code and document tree for the engine (proposed: DEX1, this document as DD-001, model note as AN-001) | Luis | before next issue |
| OP-02 | Start: soften the pressure rise after ignition, chamber pressure feedback on the fuel bypass, cold-liner (150 K) sequence, shutdown | propulsion | concept review |
| OP-03 | Sub-critical oxygen cooling: heat transfer, stability, whether R-03 survives | propulsion | concept review |
| OP-04 | Separate wall temperature limit and channel design for the oxygen zone | propulsion | next toolchain cycle |
| OP-05 | Fuel turbopump bearing type and speed (rolling element at DN 2.7 million against hydrostatic or foil) | turbomachinery | powerpack preliminary design review |
| OP-06 | Lander masses for Reference B from the network manager; confirm 180 kN | systems | next issue |
| OP-07 | Model the heat exchanger arrangement for the oxygen side | propulsion | if RK-01 or RK-04 does not close |
| OP-08 | Chamber-out operation inside one pack: needed or excluded | systems | requirements review |
| OP-09 | Nozzle: fixed radiatively cooled extension material and attachment area ratio | propulsion | concept review |
| OP-10 | Replace placeholder combustion and nozzle efficiencies; confirm 456 s | propulsion | when delivered-performance stack exists |
| OP-11 | Mass model with real component estimates | systems | preliminary design review |
| OP-12 | Igniter concept for four chambers (torch per chamber, shared feed) | propulsion | concept review |

## 12. References

1. Blue Origin, BE-7 engine page: dual expander cycle, 44.5 kN vacuum thrust. https://www.blueorigin.com/engines/be-7
2. Blue Moon (spacecraft), Wikipedia: Mark 2 launch mass above 45 t, dry mass 16 t, three BE-7 engines. https://en.wikipedia.org/wiki/Blue_Moon_(spacecraft)
3. "A Closer Look At Blue Origin's BE-7 Engine", The Space Bucket: thrust chamber test statement on hydrogen and oxygen cooled combustor segments. https://thespacebucket.com/?p=9382
4. Glassman, A. J., Jones, S. M., NASA TM-4275 (1991): full, split and dual expander cycle definitions. https://ntrs.nasa.gov/api/citations/19910010893/downloads/19910010893.pdf
5. Kacynski, K. J., Kazaroff, J. M., Jankovsky, R. S., "A Dual-Cooled Hydrogen-Oxygen Rocket Engine Heat Transfer Analysis", NASA TM-104430 (1991). https://ntrs.nasa.gov/api/citations/19910014989/downloads/19910014989.pdf
6. Binder, M. et al., NASA TM-107318 and NASA CR-195478: RL10A-3-3A model data used for calibration. https://ntrs.nasa.gov/citations/19970010379
7. Vinci (rocket engine), Wikipedia: 180 kN, 60.8 bar, expander cycle. https://en.wikipedia.org/wiki/Vinci_(rocket_engine)
8. DEX1-SYS-AN-001, Engine Foundation Model, Description and Verification.
