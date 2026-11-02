# Literature validation data: fluid systems and thermal phenomena

FNT-VAL-REG-003 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

> Register of public literature data for validation (decision D-023: no own
> test data). Compiled from web research on 2026-11-02; every source was
> located online, and the entries state what was read in the document
> ("Seen") and what is judgement ("Inferred"). Numbers read from OCR text of
> old scans must be checked against the original before use. The reference
> problems built from these data sets are in section 3 of the
> [platform development plan](../01-project/platform-plan.md).

Compiled 2026-11-02 from web research only. Every source listed was located online and its landing page or full text was opened, unless the entry says otherwise. In each entry, **Seen** means read on the opened page or PDF. **Inferred** means my own judgement, not stated by the source. "Effort" estimates the work to turn the publication into a usable validation case.

Legend:
- Data type: **raw** = downloadable measured files; **tab** = numerical tables in the report; **plot** = time histories only as figures (must be digitised).
- Completeness: **A** = geometry and boundary conditions are published well enough to build a model without guessing; **B** = a few parameters must be assumed or taken from companion papers; **C** = major gaps.
- Effort: **L** (hours), **M** (days, mostly digitising), **H** (weeks, or needs extra physics or missing data).

fluidnet capability abbreviations used below: PIPE (inertia + friction), RESTR (restriction with cavitation/flashing, Leung omega), VALVE, CHECK, REG (regulator), TANK-H (rigid homogeneous tank), WALL (wall heat capacity + Gnielinski/Bartz), HEM (homogeneous-equilibrium two-phase). Planned: TANK-2F (pressurant over liquid), N2O-NEQ (non-equilibrium self-pressurising tank), DYER (non-equilibrium flashing), BOIL (boiling heat transfer for chill-down), ELAST (pipe wall elasticity / FSI), CTRL (controllers).

---

## 1. Ranked summary tables

### 1.1 Water hammer and column separation

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires (planned) |
|---|---|---|---|---|---|---|
| 1 | DLR PS-LN2-Set1, LN2 fluid hammer with cavitation (Klein, Traudt, Deeken 2025, Zenodo) + Klein et al. 2023 Exp. Fluids | raw (9 cases) | A/B (geometry in companion papers) | L-M | PIPE, VALVE, HEM cavitation / column separation, cryogenic properties | ELAST (optional, wave speed) |
| 2 | Bergant & Simpson 1999, Adelaide copper pipeline, column separation | plot | A | M | PIPE, VALVE, column separation, wave speed | ELAST (optional) |
| 3 | Bergant, Simpson & Vítkovský 2001, unsteady friction (same rig, no cavitation) | plot | A | M | PIPE friction (steady vs unsteady), VALVE | none |
| 4 | Tijsseling, Vardy & Fan 1996, single-elbow FSI with cavitation | plot (not opened) | B | H | column separation | ELAST + structural FSI |
| - | Bergant, Simpson & Tijsseling 2006 review | review | - | - | index to further data | - |

### 1.2 Priming of evacuated lines (storable and cryogenic)

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | DLR Lampoldshausen priming bench (Bombardieri, Traudt, Manfletti ~2014; Bombardieri et al. EUCASS 2019), water/ethanol, straight/T/L lines | plot + tables of geometry, test matrix, peak pressures | A | M | PIPE, VALVE (measured opening profile), peak pressure | non-condensable gas in line (two-component), gas desorption; not covered by single-fluid HEM |
| 2 | VKI priming facility, LN2 (Gouriet et al. SP2016) | plot + tables of initial conditions | A/B | M | as above, cryogenic | two-component gas/liquid; BOIL for the line wall |
| 3 | VKI priming, water/ethanol/acetaldehyde (Lema et al. 2015 Exp. Fluids; Lema et al. KU Leuven preprint) | plot | B | M | as above | gas desorption |
| 4 | Prickett, Mayer & Hermel 1992 (Hughes), water, line and network | 2 peak values quoted in NASA papers; original paywalled | B | M | peak-pressure check of a network | two-component |
| 5 | Moore et al. 2019 (Penn State), water, varied D, L, Cv, pretest pressure | not opened (paywalled); summary in NASA TFAWS 2019 slides | B | M | sensitivity trends | two-component |
| - | Lecourt & Steelant 2007 JPP 23(6) | not opened | ? | ? | - | - |

### 1.3 Line and transfer-line chill-down

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | Brennan, Brentari, Smith & Steward 1966, NBS 9264 / NASA CR-81338, 61 m copper line, LN2 and LH2 | plot | A/B | M (plus CRTech digitised summary) | WALL, PIPE, HEM, pressure surges during cool-down | BOIL |
| 2 | Darr et al. 2016 (Univ. Florida / NASA GRC), LN2 vertical tube, 55 tests | plot | A | M | WALL | BOIL |
| 3 | Hartwig et al. LH2 chill-down at high Re (NASA GRC SMiRF), 1.27 cm OD SS line | plot | B | M | WALL | BOIL |
| 4 | Hartwig et al. 2022 npj Microgravity, LN2 parabolic-flight chill-down, coated/bare tubes | plot | A | M | WALL | BOIL (reduced g) |

### 1.4 Tank self-pressurisation and pressurised discharge

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | Van Dresar & Stochl 1993, NASA TM-106427, K-site 4.89 m3 LH2 tank, GH2 ramp pressurisation and expulsion | tab (test matrix, mass/energy balance) + plot | A | M | TANK-H (limit), WALL | TANK-2F |
| 2 | Stochl, Masters, DeWitt & Maloy 1969, NASA TN D-5387, 3.96 m sphere, GH2 pressurant | tab (mass and energy balances) | A | L-M | WALL (tank wall heat sink) | TANK-2F |
| 3 | DeWitt, Johnson & Stochl 1966, NASA TN D-3458, 0.82 m3 cylinder, six diffuser types | tab | A | L-M | WALL | TANK-2F |
| 4 | DeWitt et al. 1969/1970 TN D-5336, D-5621, D-7019 (GH2 and GHe, 1.52 m and 3.96 m spheres) | tab (inferred from series; D-5387 checked) | A | L-M | WALL | TANK-2F |
| 5 | Johnson 1967, NASA TN D-4102, submerged He injection into LH2 | tab | A | M | - | TANK-2F + mass transfer to bubbles |
| 6 | Hasan, Lin & Van Dresar 1991, NASA TM-103804, K-site self-pressurisation, 83 % fill | plot | A | M | TANK-H (pressure rise vs homogeneous) | stratified tank (outside current scope) |
| 7 | Van Dresar, Hasan & Lin 1992, NASA TM-105411, same tank at 29 % and 49 % fill | plot | A | M | TANK-H | as above |
| 8 | Stochl, Van Dresar & Lacovic 1991, NASA TM-104516, submerged vapour injection | not opened beyond abstract | B | M | TANK-H | TANK-2F |
| 9 | MHTB (Hedayat et al. 2003, TVS spray bar, 18 m3 class LH2 tank) | plot (not opened beyond abstract) | B | H | - | TVS, stratification |
| 10 | Centaur pressurised feed system tests, NASA TN D-6876 (1972) | tab + plot | B | H | system-level | TANK-2F, autogenous bleed |
| 11 | Saturn AS-203 S-IVB LH2 low-g self-pressurisation (via Grayson et al. AIAA 2006-5258) | plot | C | H | - | low-g stratification |

### 1.5 Self-pressurising N2O (and CO2) run tanks

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | Zimmerman 2015 Stanford PhD thesis (CC BY-NC 3.0), N2O and CO2 drains of transparent tanks | plot (full text not opened) | B | M-H | TANK-H (equilibrium limit), RESTR | N2O-NEQ |
| 2 | Song, Xu & Shen 2016 JPP 32(2), stratification in N2O tank | plot (abstract only) | B | M | - | N2O-NEQ, stratification |
| 3 | Zilliac & Karabeyoglu 2005, AIAA 2005-3549, N2O drain tests | not opened (paywalled); existence confirmed via RIT thesis | ? | M | TANK-H | N2O-NEQ |
| 4 | Yasuda, Nakata & Uchiumi (JAXA HR-2019-010), N2O run tank with liquid and vapour T | plot | C (tank volume not given) | M | TANK-H | N2O-NEQ |
| 5 | Zimmerman et al. IAC-13 C4.3.7 and Ow & Zimmerman IAC-14 | restricted (IAF) | - | - | - | - |
| - | AIAA 2013-4045 (Zimmerman, Waxman, Cantwell, Zilliac) | not accessible (AIAA site refused fetch) | - | - | - | - |

### 1.6 Two-phase flashing and choked flow

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | Hendricks, Simoneau & Barrows 1976, NASA TN D-8169, choked flow of subcooled LN2 and LOX in two C-D nozzles | tab (hundreds of points: P0, T0, Gmax, tap pressures) | A | L | RESTR (HEM, omega), real-fluid properties | DYER (for subcooled inlet) |
| 2 | BNL nozzle flashing, NUREG/CR-1864 Vol. 1 (Abuaf et al. ~1981), water, 49 pressure taps + void | tab (23 tables) per abstract; full text not opened | A | M | RESTR, HEM | DYER / delayed equilibrium |
| 3 | Palacz & Cieślik 2021, Aerospace 8(11):317, N2O injector critical flow, 5 injectors | tab (critical flow table) + plot, CC BY | A/B | L | RESTR with N2O | DYER |
| 4 | Waxman, Zimmerman, Cantwell & Zilliac ~2013 (NTRS 20190001326), N2O injector mass flow | plot | B | M | RESTR | DYER |
| 5 | Henry & Fauske 1971, J. Heat Transfer 93(2):179, DOI 10.1115/1.3449782 | model paper with data comparison (not opened) | B | M | RESTR | DYER-type |
| 6 | Super Moby Dick (CEA), OECD-NEA CSNI1017 package | restricted distribution, pressure only | - | H | - | - |
| 7 | Sozzi & Sutherland 1975, GE NEDO-13418 | not located online as full text | - | - | - | - |
| 8 | La Luna et al. 2022/2023 (PoliMi), N2O through 1 mm orifice | tab of averages; raw data only on request | B | L | RESTR with N2O | DYER |

### 1.7 Cavitating venturis

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | Hord, Anderson & Hall 1972, NASA CR-2054, LH2 cavitation in a quarter-round venturi | tab (Appendix A) | A/B | M | RESTR cavitation onset with thermal suppression | cavity thermodynamics (beyond HEM) |
| 2 | Edmonds & Hord 1969, NBS TN 374, LH2 and LN2 venturi | tab (partial) + plot | B | M | as above | as above |
| 3 | Yazıcı 2006 METU MSc thesis, water, 4 venturis (5 mm throat) | plot | A | M | RESTR in cavitating (choked) regime | - |
| 4 | Yoon & Ahn 2015 JKSPE 19(4), water, 2 mm throat, 4 angle variants | plot | A | M | RESTR choked regime, critical pressure ratio | - |
| 5 | Ulaş 2006, Flow Meas. Instrum. 17:93 | averages only; abstract seen | C | - | - | - |

### 1.8 Gas blowdown and regulator dynamics

| Rank | Data set | Data type | Completeness | Effort | Validates | Requires |
|---|---|---|---|---|---|---|
| 1 | Richardson & Saville 1992 (IChemE Symp. Ser. 130) / Haque et al. 1992 Trans IChemE B 70:10, Imperial College vessel blowdown (N2, CO2, hydrocarbons) | plot (ranges) | A/B | M | TANK-H, WALL (thick steel wall, inside/outside wall T), RESTR (choked gas) | - |
| 2 | Hauser & Quinn 2012, NASA TM-2012-217271, AA-1 cold-gas thruster system with regulator | plot | C (regulator internals not given) | H | REG, VALVE, PIPE | CTRL not needed; needs regulator dimensions |
| 3 | GFSSP Example 8 (tank blowdown) and Examples 16/17 (pressure/flow regulator) | analytic / code-to-code | A | L | TANK-H, REG | - |

### 1.9 Code benchmark collections

| Rank | Collection | Content | Use |
|---|---|---|---|
| 1 | GFSSP v6.0 user manual, NASA/TP-2016-218218 (30 examples) and NASA/TM-2013-217492 (25 examples) | fully specified inputs; most compared with analytic solutions or MOC, a few with test data | code-to-code verification, quick regression suite |
| 2 | NASA GFSSP validation papers (Bandyopadhyay, Majumdar & Holt 2017; LeClair, Boehm & Majumdar TFAWS 2019) | priming against Prickett, Lee, Moore | collects peak-pressure test numbers in one place |
| 3 | ESPSS / EcosimPro validation papers (Koppel et al. 2012, EUCASS) | abstract only seen | low; data not published in reusable form |

---

## 2. Data set details

### 2.1 Water hammer and column separation

#### WH-1. DLR PS-LN2-Set1: cryogenic pressure surge data set
- **Citation:** Klein, S., Traudt, T., Deeken, J. C. (2025). *Liquid Nitrogen Fluid Hammer: PS-LN2-Set1*, version 1.0.0. Zenodo. DOI 10.5281/zenodo.15526459. Also listed in DLR elib as "Cryogenic Pressure Surge Data Set: PS-LN2-Set1".
- **Companion papers:** Klein, S., Traudt, T., Oschwald, M. (2023). Comparison of water and cryogenic fluid hammer experiments for rocket engine feed line systems. *Experiments in Fluids* 64, 30. DOI 10.1007/s00348-023-03573-3 (CC BY 4.0). Klein, Traudt, Oschwald, "Comparison of pressure surge events in water and liquid nitrogen as a substitute fluid for rocket engine feed lines" (DLR elib 200679, Space Propulsion 2023 inferred from filename). Zenodo also lists AIAA paper DOI 10.2514/6.2025-2204 (not opened).
- **URLs opened:** https://zenodo.org/records/15526459 ; https://elib.dlr.de/214268 ; https://link.springer.com/article/10.1007/s00348-023-03573-3 ; https://elib.dlr.de/200679/1/Klein_PS_2023.pdf
- **Measured (Seen):** 9 test cases, LN2, fast-closing valve stops a steady flow; surge up to >100 bar at 13 m/s; cavitation near the valve. Single archive PS-LN2-Set1.7z, 96.4 MB, CC BY 4.0. Archive contents were not inspected. In the SP2023 paper: Kulite CTL-190S pressure at 10 kHz, type K thermocouples at 100 Hz, Coriolis mass flow and density (Emerson CMF50M).
- **Geometry and BCs (Seen):** FTTF-2 test section 9.29 m long (water configuration FTTF-1: 7.671 m), ID 19 mm, wall 1.5 mm, stainless 1.4541, 1.5-turn spiral inclined 0.62 deg, vacuum jacketed. HP tank (1.84 to 28.05 bar for TC1 to TC7 in the SP2023 paper) to LP tank near ambient. Axius axial valve closing in about 18 ms. Initial LN2 temperature 85.5 to 87.1 K.
- **Effort:** Low to medium. Raw data under CC BY. Valve closure law and exact sensor positions must be taken from the papers. Whether the Zenodo cases are identical to the paper's TC1 to TC7 was not checked.
- **Validates:** PIPE (Joukowsky amplitude, wave period, friction damping), VALVE closure, HEM with vapour formation and collapse (column separation analogue), cryogenic property evaluation, mass flow measurement for the initial steady state.
- **Requires:** Not strictly needed, but ELAST (or an effective wave speed) is needed to match the wave period. Cavity collapse peaks test the HEM/discrete-cavity treatment.
- **Note:** This is the only raw, openly licensed measured data set found in this search.

#### WH-2. Bergant & Simpson, Adelaide copper pipeline, column separation flow regimes
- **Citation:** Bergant, A., Simpson, A. R. (1999). Pipeline column separation flow regimes. *J. Hydraulic Engineering* 125(8), 835–848. DOI 10.1061/(ASCE)0733-9429(1999)125:8(835) (DOI from search result URL). Open accepted manuscript: Adelaide handle 2440/995.
- **Related:** Simpson, A. R., Bergant, A. (1994). Developments in pipeline column separation experimentation. *J. Hydraulic Research* 32(2), 183–194. DOI 10.1080/00221686.1994.10750034 (Adelaide handle 2440/80918).
- **URLs opened:** https://digital.library.adelaide.edu.au/dspace/bitstream/2440/995/1/hdl_995.pdf ; https://thesis.library.adelaide.edu.au/dspace/handle/2440/80918
- **Measured (Seen):** pressure at five equidistant positions (piezoelectric Kistler 610B at the ends and midpoint, Druck PDCR 810 strain gauges at all five), 5 kHz sampling, water temperature, optical valve position. 60 runs in the global set (15 velocities 0.10 to 1.50 m/s, two static heads, upward and downward slope). Traces are given as plots only (Figs. 3 to 11).
- **Geometry and BCs (Seen):** copper pipe 37.23 m, ID 22.1 mm, wall 1.6 mm, slope ±3.2 deg (the manuscript text is inconsistent on slope; check), measured wave speed 1319 m/s, two pressurised tanks with heads regulated 20 to 620 kPa, ball valve at the downstream end with effective closure about 9 ms. Example runs: V0 = 0.30 and 1.40 m/s with Tank 2 head 22.0 m.
- **Effort:** Medium (digitise 2 to 4 traces). Long-established benchmark, so many published model results exist for comparison.
- **Validates:** PIPE, VALVE closure, column separation/vapour cavity, gravity term on a sloped line. **Requires:** nothing planned; the measured wave speed can be imposed directly.

#### WH-3. Bergant, Simpson & Vítkovský 2001: unsteady friction
- **Citation:** Bergant, A., Simpson, A. R., Vítkovský, J. (2001). Developments in unsteady pipe flow friction modelling. *J. Hydraulic Research* 39(3), 249–257. DOI 10.1080/00221680109499828. Open copy: Adelaide handle 2440/997.
- **URL opened:** https://digital.library.adelaide.edu.au/dspace/handle/2440/997 (abstract page; PDF link present, not read).
- **Measured (Seen in abstract):** laboratory laminar and low-Re turbulent water hammer, compared with quasi-steady, Zielke and Brunone models. Same apparatus as WH-2 (Inferred).
- **Effort:** Medium. **Validates:** damping by wall friction; shows the error of a quasi-steady friction model, which is what a lumped pipe element normally uses.

#### WH-4. Tijsseling, Vardy & Fan 1996: FSI and cavitation in a single-elbow pipe
- **Citation:** *J. Fluids and Structures* 10(4), 395–420. DOI 10.1006/jfls.1996.0025.
- **URL opened:** https://research.tue.nl/nl/publications/fluid-structure-interaction-and-cavitation-in-a-single-elbow-pipe/ (abstract only).
- **Content (Seen):** freely suspended horizontal pipe with one elbow, experiments and MOC with beam equations and lumped cavitation. Full text not accessed.
- **Requires:** ELAST plus axial/bending structural motion. Use only after the planned elasticity work, and only if structural coupling is a goal.

#### WH-review. Bergant, Simpson & Tijsseling 2006
- *J. Fluids and Structures* 22(2), 135–171, DOI 10.1016/j.jfluidstructs.2005.08.008. Accepted version open at Adelaide handle 2440/35874 (page opened). It is the best index of older column-separation experiments.

### 2.2 Priming of evacuated lines

#### PR-1. DLR Lampoldshausen priming test bench (water, ethanol)
- **Citations:**
  - Bombardieri, C., Traudt, T., Manfletti, C. *Effect of the dissolved pressurizing gas on the pressure surge during the filling process of spacecraft feedlines* (DLR; about 2014, year not printed; CORE copy).
  - Bombardieri, C., Traudt, T., Mangetti [sic, Manfletti], C. (2019). Experimental study of water hammer pressure surge. *Progress in Propulsion Physics* 11, 555–570, EUCASS. DOI 10.1051/eucass/201911555.
- **URLs opened:** https://core.ac.uk/download/31020608.pdf ; https://www.eucass-proceedings.eu/articles/eucass/pdf/2019/01/eucass2019_555.pdf
- **Measured (Seen):** six stations, each with a type K thermocouple (1 kHz), an absolute piezoresistive sensor (10 kHz) and a piezoelectric dynamic sensor (150 kHz, 30 kHz filter). Valve position encoder, dead-end accelerometer, high-speed video at 19.2 kfps through a quartz dead-end segment. 4 s per test.
- **Geometry and BCs (Seen):** 80 L run tank up to 50 bar (tests at 20 bar), GN2 or GHe pressurant. Upstream pipe 22 x 1.5 mm, 1023 mm from tank to valve. Coaxial fast-opening valve, 6 ms opening, 16 mm seat, loss coefficient 12.5. Test section: 2000 mm straight stainless 1.4541 tube, 19.05 x 1.25 mm (ID 16.56 mm), sloped about 1 deg, clamped at 5 points. Variants T, T2 (1000/500 mm branches) and L. Line vacuum 10 mbar (water) or 20 mbar (ethanol). Tables: bench dimensions, test matrix, peak pressures per geometry (2019 paper).
- **Effort:** Medium (digitise traces). Peak-pressure tables give a quick first check.
- **Validates:** PIPE, VALVE (the measured opening profile can be imposed), first-peak magnitude and timing.
- **Requires:** a two-component (liquid + non-condensable gas) capability to represent the residual gas cushion at the dead end, and optionally gas desorption. A single-component HEM network can only represent a vapour-filled line, which is closest to the lowest-pressure cases (Inferred).

#### PR-2. VKI priming facility with LN2 (Gouriet et al. 2016)
- **Citation:** Gouriet, J.-B., Huertas-Martinez, A., Buchlin, J.-M., Vetrano, M.-R., Steelant, J. (2016). Multiphase fluid hammer with cryogenic fluids. Space Propulsion 2016, Rome, paper SP2016_3124867.
- **URL opened:** https://lirias.kuleuven.be/retrieve/512997
- **Measured (Seen):** dead-end and reservoir pressure (Kulite CTL-190M), dead-end temperature (coaxial type E, calibrated 77 to 288 K), wall chill-down temperatures (Cernox). Tables of initial conditions with uncertainties. Time traces as plots.
- **Geometry and BCs (Seen):** vertical 2 m line, 5/8 in OD x 0.125 in wall stainless, dead end, liquid flowing upward. 6 L reservoir via a large-radius 90 deg bend. Cryogenic ball valve with 9.65 mm bore, about 100 ms opening. Cooling jacket for line pre-chill. Vacuum chamber below 100 Pa. Line gas GN2 or GHe.
- **Requires:** as PR-1, plus line-wall heat transfer if the line is not pre-chilled.
- **Note:** The task named an ESA/VKI "Fluid Structure Line (FSL)". None of the VKI or DLR documents opened uses that name. I could not verify that a facility of that name exists. The VKI priming facility (Lema; Gouriet) is the closest match.

#### PR-3. VKI priming, inert storable simulants (Lema et al.)
- **Citations:**
  - Lema, M., López Peña, F., Rambaud, P., Buchlin, J.-M., Steelant, J. (2015). Fluid hammer with gas desorption in a liquid-filling tube: experiments with three different liquids. *Experiments in Fluids* 56, 180. DOI 10.1007/s00348-015-2043-2 (paywalled; abstract page opened).
  - Lema, M., López Peña, F., Buchlin, J.-M., Steelant, J. *Experiments on water hammer induced by fast opening valve during priming: effect of gas desorption* (preprint, https://lirias.kuleuven.be/retrieve/512999, opened).
  - VKI PhD summary, M. Lema (https://www.vki.ac.be/download/phdthesis/phdsum_Lema.pdf, opened).
- **Content (Seen):** 4 L spherical tank with membrane, 10 L deaeration vessel, ball valve with 6 mm orifice opening in 45 to 50 ms, titanium line 6.35 mm ID, 0.4 mm wall, straight 2 m. Water, ethanol, acetaldehyde. 2 MPa tank, 1 kPa line. Deaerated or N2-saturated liquid. Results mainly qualitative, with pressure traces as plots.
- **Effort:** Medium. Useful to bracket the effect of dissolved gas, which fluidnet does not model.

#### PR-4. Prickett, Mayer & Hermel 1992 (via NASA papers)
- **Citation:** Prickett, R. P., Mayer, E., Hermel, J. (1992). Water hammer in spacecraft propellant feed systems. *J. Propulsion and Power* 8(3), 592–597 (AIAA-88-2920). Original not opened. Citation and numbers taken from NASA papers.
- **URLs opened:** https://ntrs.nasa.gov/api/citations/20170008981/downloads/20170008981.pdf (Bandyopadhyay, Majumdar & Holt, "Fluid Transient Analysis during Priming of Evacuated Line") ; https://ntrs.nasa.gov/api/citations/20190030443/downloads/20190030443.pdf (LeClair, Boehm & Majumdar, TFAWS 2019).
- **Content (Seen in NASA papers):** straight line, 12 in of 3/8 in upstream of a latch valve and 96 in of 1/4 in downstream, supply 30 to 120 psia, water. Measured peak 2350 psia at 0.17 s for 120 psia. Network case: reservoirs of 3.55, 14.55, 5.68 and 4.68 in3, filter with dp = 15.92 (GPM)^1.18 psia, 240 psia supply, node 28 peak 1800 psia.
- **Effort:** Medium. Only two or three numbers, but the network case is a good test of network topology handling.

#### PR-5. Moore et al. 2019 (Penn State)
- **Citation:** Moore, J. D., Gilbert, J., Brindle, B., Risha, G. A. (2019). Priming event peak pressures in liquid propulsion systems. *J. Spacecraft and Rockets* 56(3), 899–909. DOI 10.2514/1.A34393.
- **URL opened:** https://pure.psu.edu/en/publications/priming-event-peak-pressures-in-liquid-propulsion-systems/ (abstract). Parameter ranges are from the NASA TFAWS 2019 slides: reservoir 1.5 to 2.9 MPa, L = 0.51 or 2.0 m, D = 6.5, 9.5 or 12.7 mm, Cv = 0.037, 1.5 or 4.0, initial air 4 to 101 kPa, peaks 4.5 to 28 MPa. Distilled water as a hydrazine simulant. Includes cavitating-venturi mitigation.
- **Full text not accessed.**

#### PR-6. Lecourt & Steelant 2007
- Citation as given in Gouriet et al. 2016: "Experimental investigation of waterhammer in simplified feed lines of satellite propulsion system", *J. Propulsion and Power* 23(6), 1214–1224. **Not opened. DOI not verified.**

### 2.3 Line chill-down

#### CD-1. Brennan et al. 1966, NBS cool-down of cryogenic transfer lines
- **Citation:** Brennan, J. A., Brentari, E. G., Smith, R. V., Steward, W. G. (1966). *Cooldown of Cryogenic Transfer Lines: An Experimental Report*. NBS Report 9264 / NASA-CR-81338. NTRS 19670007291.
- **URLs opened:** https://ntrs.nasa.gov/citations/19670007291 (record; PDF linked) ; https://www.crtech.com/sites/default/files/files/NBS1966CryoLineCooldownStudy.pdf (C&R Technologies re-analysis, opened).
- **Measured (Seen, from CRTech summary):** pressure and temperature histories at 4 stations (6.1, 24.4, 43, 60.4 m from the supply tank). Supply pressure recorded, with uncertain accuracy. Flow rate not measured. Sampling not stated. Data are plots only, and CRTech digitised them. Known data issues: station 2 erratic, station 3 sometimes warmer than station 4, time offsets of 1 to 2 s.
- **Geometry and BCs (Seen):** vacuum-jacketed copper line, ID 1.59 cm, OD 1.90 cm, 61 m. 300 L supply dewar. Exhaust to ambient at about 0.82 atm (Boulder). Test matrix: LH2 at 2.5 to 11 atm supply (saturated and subcooled), LN2 at 2.5 to 5.9 atm (saturated and subcooled). Eleven usable runs (NBS Figs. 2 to 7 and 10 to 14).
- **Effort:** Medium. Digitise the NBS figures or follow the CRTech paper's reconstruction. The CRTech paper also gives an independent model comparison.
- **Validates:** WALL heat capacity, PIPE with a strongly varying density, HEM along a long line, pressure oscillations during cool-down.
- **Requires:** BOIL (film/transition/nucleate boiling). Without BOIL only a single-phase or Gnielinski-HEM approximation can be shown, which will not reproduce the timing.

#### CD-2. Darr et al. 2016 (University of Florida / NASA), LN2 terrestrial chill-down
- **Citation:** Darr, S. R., Hu, H., Glikin, N., Hartwig, J. W., Majumdar, A. K., LeClair, A. C., Chung, J. N. (2016). An experimental study on terrestrial cryogenic transfer line chilldown, I and II. *Int. J. Heat Mass Transfer* (Elsevier PII S0017931016302575 seen in a catalogue result; the catalogue page was bot-blocked, so volume, pages and DOI were not verified).
- **URLs opened (secondary descriptions):** https://ntrs.nasa.gov/api/citations/20150011066/downloads/20150011066.pdf (Darr et al., "Numerical Simulation of Liquid Nitrogen Chilldown of a Vertical Tube", AIAA) ; https://ntrs.nasa.gov/api/citations/20230016607/downloads/Scitech2024-Chilldown-paper%20v3.pdf (Ahuja et al., uses Darr Test 40).
- **Geometry and BCs (Seen):** SS304 tube 57.2 cm long, OD 1.27 cm, wall 0.0508 cm, downward flow, in vacuum. Six T-type outer-wall thermocouples at two stations (14.9 and 40.1 cm), two pressure transducers (18 and 37.1 cm), flow measured as gas after a downstream evaporator. 55 tests: G = 61 to 1150 kg/m2 s, p_in = 175 to 817 kPa, subcooling 0 to 14 K, initial wall about 293 K. Example Test 40: p_in = 325.93 kPa, T_in = 86.61 K, G = 327.82 kg/m2 s.
- **Effort:** Medium (digitise wall-temperature curves). A short, well-defined geometry makes it a good single-pipe BOIL validation.
- **Requires:** BOIL.

#### CD-3. Hartwig et al., LH2 chill-down at NASA GRC SMiRF
- **Citations:**
  - Hartwig, J., Hu, H., Styborski, J., Chung, J. N. (2015). Comparison of cryogenic flow boiling in liquid nitrogen and liquid hydrogen chilldown experiments. *IJHMT* 88. DOI 10.1016/j.ijheatmasstransfer.2015.04.102 (NTRS 20190007959; no download on NTRS).
  - Rame, E., Hartwig, J. W., McQuillen, J. B. Flow visualization of liquid hydrogen line chilldown tests (AIAA paper, NTRS 20140008545, opened).
  - Darr, S. R., Hartwig, J. W. (2019). Two-phase convection heat transfer correlations for liquid hydrogen pipe chilldown. NTRS 20190029114 (record opened). Provides correlations rather than data.
  - "Liquid hydrogen line chilldown experiments at high Reynolds numbers I/II", *Cryogenics* (about 2020): seen only as catalogue entries; not opened.
- **Content (Seen):** vertical 0.5 in OD (0.402 in ID) stainless line about 1.62 m (Umemura et al., NTRS 20190032460) or 2.26 m horizontal line (Rame et al.). Silicon diode skin and stream temperatures, pressures, Coriolis flow, sight glass. Inlet 20.3 to 24.2 K, 101 to 276 kPa, 0.0023 to 0.036 kg/s, He-pressurised supply. Wall temperature in one cited case was recorded only every 0.71 s.
- **Completeness:** B (pipe wall mass is given as total line mass including valves).
- **Requires:** BOIL.

#### CD-4. Hartwig et al. 2022, LN2 chill-down in reduced gravity
- **Citation:** Hartwig, J., Chung, J. N., Dong, J., Han, B., Wang, H., Darr, S., Taliaferro, M., Jain, S., Doherty, M. (2022). Nitrogen flow boiling and chilldown experiments in microgravity using pulse flow and low-thermally conductive coatings. *npj Microgravity* 8, 33. DOI 10.1038/s41526-022-00220-9. CC BY 4.0.
- **URL opened:** https://pmc.ncbi.nlm.nih.gov/articles/PMC9363417
- **Content (Seen):** 28 tests, three SS304 tubes 0.914 m long, OD 1.27 cm, wall 0.051 cm, bare or with 4 or 7 FEP layers. 15 thermocouples at 5 stations, inlet and outlet pressure, gas flow meters. No public raw data.
- **Relevance:** lower priority. Low gravity and coatings add physics beyond the planned BOIL model. The bare-tube cases are usable.

### 2.4 Tank pressurisation and pressurised discharge

#### TK-1. Van Dresar & Stochl 1993: pressurisation and expulsion of a flightweight LH2 tank (K-site)
- **Citation:** Van Dresar, N. T., Stochl, R. J. (1993). *Pressurization and Expulsion of a Flightweight Liquid Hydrogen Tank*. NASA TM-106427 / AIAA-93-1966.
- **URL opened:** https://ntrs.nasa.gov/api/citations/19940015704/downloads/19940015704.pdf
- **Geometry and BCs (Seen):** 4.89 m3 ellipsoidal tank (axis ratio 1.2, major diameter 2.2 m), internal area 14.0 m2, mass 149 kg, 2219 Al mostly 2.08 mm thick, MLI, vacuum chamber with 295 K shroud. Ambient-temperature GH2 through a conical top diffuser. Ramp pressurisation from 1 atm to 205 or 275 kPa at 50 % and 87 % fill (0.23 to 8.2 min). Expulsions from 87 % to 5 % at constant 205 or 275 kPa (12 to 33 min).
- **Measured (Seen):** vertical and radial diode rakes (0.64 cm spacing near the interface), wall diodes, inlet gas temperature, orifice pressurant flow, capacitance level probe, venturi outflow. Table 1 test matrix, Table 2 mass and energy balance. The paper warns that some Table 2 entries are likely erroneous.
- **Effort:** Medium. Integral quantities (pressurant mass, collapse factor) are tabulated. Ullage temperature profiles must be digitised.
- **Validates:** WALL (aluminium wall as a heat sink), mass and energy bookkeeping. **Requires:** TANK-2F with an ullage that is not in equilibrium with the liquid. Stratification will limit what a single ullage node can match. Comparing pressurant mass is the fair test.

#### TK-2. Stochl, Masters, DeWitt & Maloy 1969: GH2 pressurant requirements, 3.96 m sphere
- **Citation:** NASA TN D-5387 (NTRS 19690023833).
- **URLs opened:** https://ntrs.nasa.gov/citations/19690023833 ; PDF https://ntrs.nasa.gov/api/citations/19690023833/downloads/19690023833.pdf
- **Geometry and BCs (Seen):** 3.96 m sphere, twelve 2219-T87 gores, wall 1.95 cm (weld lands), 1.27 cm (midlands), 0.89 cm (membrane). 347 stainless lid. Inside a 7.61 m vacuum chamber. Two injectors (hemisphere screen 928 cm2, straight pipe 17.15 cm2). Inlet gas 173 K or 300 K. Tank pressure 35, 50 or 75 psia. Outflow 1.73 to 4.32 kg/s. Ramp 0.376 to 1.47 psi/s. Initial ullage 5 to 74 %. Expulsion from 5 % to 95 % ullage.
- **Measured (Seen):** ullage thermopile rakes, platinum sensors, 17-sensor interface rake, 18 wall temperatures, turbine outflow meter, orifice pressurant flow. Tables I to IV: experimental and analytical mass balance, energy balance, deviations, ramp-period pressurant.
- **Effort:** Low to medium (tables). **Validates:** WALL, energy split between ullage, wall and liquid. **Requires:** TANK-2F.

#### TK-3. DeWitt, Johnson & Stochl 1966: pressurant diffusers, 0.82 m3 cylinder
- **Citation:** NASA TN D-3458 (NTRS 19660017895).
- **URLs opened:** https://ntrs.nasa.gov/citations/19660017895 ; PDF opened.
- **Content (Seen):** 29 ft3 vacuum-jacketed cylinder, 27 in ID x 82 in cylindrical length, 5/16 in 304 SS wall. LH2 with GH2 pressurant at about 160 psia. Inlet 508 to 558 R. Outflow 0.20 to 1.17 lb/s. Six diffuser types. Tables I to IV: operating parameters and results, errors, heat balances, floating-barrier mass balances.
- **Effort:** Low to medium. A cylindrical geometry is simpler to model than a sphere.

#### TK-4. Remaining Lewis 1969–1970 series (same rig family)
- DeWitt, Malloy, Masters, Stochl 1969, *Gaseous hydrogen requirements for the discharge of liquid hydrogen from a 1.52 m (5 ft) diameter spherical tank*, NASA TN D-5336 (NTRS 19690022940, record opened).
- De Witt, Maloy, Masters, Stochl 1970, *Gaseous-helium requirements for the discharge of liquid hydrogen from a 1.52 m (5 ft) spherical tank*, NASA TN D-5621 (NTRS 19700007064, record opened).
- De Witt, Maloy, Masters, Stochl 1970, *Gaseous-helium requirements ... 3.96 m (13 ft) spherical tank*, NASA TN D-7019 (NTRS 19710004571, record opened).
- Only the record pages were opened. That they contain tables like TN D-5387 is inferred from the series. The He-pressurant cases are the direct test of a two-component ullage (He + H2 vapour).
- Analysis program for the series: Masters 1974, NASA TN D-7504 (RAMP and expulsion codes, NTRS 19740020126). It is a useful reference implementation.

#### TK-5. Johnson 1967: submerged helium injection into LH2
- NASA TN D-4102 (PDF https://ntrs.nasa.gov/api/citations/19670024112/downloads/19670024112.pdf, opened). 0.357 m3 cylinder, 21.4 in ID, 304 SS 0.188 in wall, porous plate He bubbling, 21 to 33 psia, 60 to 300 s expulsions. Table I compares experimental and ideal He requirements and ullage saturation. It requires bubble-liquid mass transfer, which is beyond the planned scope.

#### TK-6. Hasan, Lin & Van Dresar 1991: K-site self-pressurisation at low heat flux
- **Citation:** Hasan, M. M., Lin, C. S., Van Dresar, N. T. (1991). Self-pressurization of a flightweight liquid hydrogen storage tank subjected to low heat flux. NASA TM-103804 (ASME/AIChE NHTC 1991). NTRS 19910011011.
- **Related:** Van Dresar, Hasan, Lin (1992). Self-pressurization of a flightweight LH2 tank: effects of fill level at low wall heat flux. NASA TM-105411 / AIAA 92-0818 (NTRS 19920009200). Lin, Van Dresar, Hasan (1991). A pressure control analysis of cryogenic storage systems. **This is NASA TM-104409** (NTRS 19910015147, AIAA 91-2405). It is an analysis paper, not the experiment. The task brief attached TM-104409 to the experiment. The experiment is TM-103804.
- **URLs opened:** NTRS record pages for 19910011011, 19920009200, 19910015147.
- **Content (Seen in abstracts):** same 4.89 m3 tank as TK-1. Wall heat flux 0.35, 2.0 and 3.5 W/m2. Fill 83 to 84 % (TM-103804) and 29 % and 49 % (TM-105411). Pressure rise rate and stratification measured. At the highest flux the rise rate is more than 3 times the homogeneous rate, and close to homogeneous at the lowest flux.
- **Validates:** TANK-H plus heat leak. A homogeneous tank should match the 0.35 W/m2 case and fall short by about 3x at 3.5 W/m2. That is a clear, quantitative statement of model limits.
- **Effort:** Medium (digitise pressure vs time; PDFs available on NTRS, not read in full).

#### TK-7. MHTB thermodynamic vent system tests
- Hedayat, A., Bailey, J. W., Hastings, L. J., Flachbart, R. H. (2003). Test data analysis of a spray bar zero-gravity liquid hydrogen vent system for upper stages (NTRS 20030106051, record opened). 20 and 50 W heat leak, 90/50/25 % fill, pressure held within about 7 kPa, destratification. Lock-up self-pressurisation segments were also run. Low priority until TVS and stratification are in scope.

#### TK-8. Centaur pressurised propellant feed system tests
- NASA TN D-6876 (1972), Lewis Research Center (Szabo; Groesbeck). PDF https://ntrs.nasa.gov/api/citations/19720025206/downloads/19720025206.pdf (opened, first pages). Full-scale Centaur 5C tank at Plum Brook B-2, He and engine-bleed GH2 pressurisation, tables of test conditions and LOX tank pressurisation performance. A system-level case, best used late.

#### TK-9. Saturn AS-203 S-IVB low-gravity self-pressurisation
- Known through Grayson, G. D., Lopez, A., Chandler, F. O., Hastings, L. J., Tucker, S. P. (2006). Cryogenic tank modeling for the Saturn AS-203 experiment. AIAA 2006-5258 (NTRS 20060047496, record opened). The paper calls it the only well-instrumented low-g cryogenic self-pressurisation test. The original flight data report was not located. Low priority.

#### TK-10. GFSSP LN2 flight-tank self-pressurisation (for reference)
- Bandyopadhyay, Majumdar, LeClair, Valenzuela (2019). Multi-node modeling of cryogenic tank pressurization system using GFSSP. AIAA Propulsion and Energy 2019 (NTRS 20190030427). A ULA-supplied flight tank about 10 ft x 10 ft, LN2, 75 % and 45 % fill, ullage pressure and temperature at several heights. Test data appear only as plots in the paper. Hardware details are probably incomplete (Inferred).

### 2.5 Self-pressurising N2O / CO2 tanks

#### N2O-1. Zimmerman 2015 Stanford PhD thesis
- **Citation:** Zimmerman, J. E. (2015). *Self-pressurizing propellant tank dynamics*. PhD thesis, Stanford University. https://purl.stanford.edu/ds271zh2987. Licence CC BY-NC 3.0.
- **URL opened:** the PURL landing page. The embedded viewer needs JavaScript, so **the full text was not read**.
- **Content (Seen in abstract):** small transparent pressure vessels drained under hybrid-rocket-like conditions. High-speed video. CO2 shown to be an accurate simulant. Flow rate, fill level, temperature and scale varied. Two regimes: an initial pressure drop and recovery, then a linear decline. A lumped-node model plus a 1-D bubble-population (DQMOM) model reproduces both.
- **Related:** Zimmerman, Waxman, Cantwell, Zilliac (2013). Self-pressurizing propellant tank dynamics. IAC-13 C4.3.7 (IAF page opened; full text restricted). Ow & Zimmerman, IAC-14 C4.5 (abstract: acrylic tank, orifice-controlled drain times 1 to 500 s). Zimmerman, Waxman, Cantwell, Zilliac, AIAA 2013-4045 "Review and evaluation of models for self-pressurizing propellant tank dynamics": **the AIAA page refused the fetch, so neither the paper nor the data sets it evaluates could be confirmed.** Two of the data sets it compares are probably the Stanford small-scale tests and the Zilliac & Karabeyoglu 2005 N2O drain data, but that is Inferred and unverified.
- **Validates:** TANK-H equilibrium limit, RESTR outlet. **Requires:** N2O-NEQ (the pressure dip and recovery is the target).
- **Effort:** Medium to high. Get the PDF from Stanford SearchWorks (record 11513046), then digitise. The licence permits non-commercial reuse. Check whether that is compatible with the intended use of fluidnet.

#### N2O-2. Song, Xu & Shen 2016
- *J. Propulsion and Power* 32(2), 337–344. DOI 10.2514/1.B35849. Abstract opened via catalogue record. Axially spaced temperature sensors and load cells during fill and drain. Liquid stratification is small enough for one node, while vapour is always stratified. The saturation temperature from pressure is slightly above the liquid temperature. Full text not opened (paywalled). It directly informs how many nodes N2O-NEQ needs.

#### N2O-3. Zilliac & Karabeyoglu 2005
- Zilliac, G., Karabeyoglu, M. A. (2005). Modeling of propellant tank pressurization. AIAA 2005-3549. Existence confirmed through the RIT thesis of M. M. Fernandez (2009), *Propellant tank pressurization modeling for a hybrid rocket* (https://repository.rit.edu/theses/7111, opened). That thesis validates against the Zilliac & Karabeyoglu N2O drain data, and its PDF is downloadable (not opened). The AIAA paper itself was not accessed. The RIT thesis may contain digitised or re-plotted pressure histories (Inferred).

#### N2O-4. JAXA / Muroran N2O run tank
- Yasuda, K., Nakata, D., Uchiumi, M. *Evaluation of filling, discharging and flow characteristics of nitrous oxide*. JAXA-HR-2019-010 (Japanese). https://jaxa.repo.nii.ac.jp/record/20746/files/SA6000143011.pdf (opened). Tank pressure, top (vapour) and bottom (liquid) thermocouples, injector upstream p and T, tank weight, 6 s runs, some with CO2. Tank volume not given, figures only. Completeness C.

#### N2O-5. Other N2O sources checked and set aside
- La Luna et al. 2022, *Aerospace* 9(12):828, DOI 10.3390/aerospace9120828, and La Luna, Zuin, Maggi, EUCASS 2023 (DOI 10.13009/EUCASS2023-728). 10 L inverted bottle, 1 mm orifice, Coriolis flow, tables of averaged results. Raw data only on request ("sensitive"). Useful for orifice mass flow (section 2.6), less so for tank dynamics.
- Wei et al. 2025, *Aerospace* 12(5):372 (Nytrox hot fires, 2.8 L tank): tank p, T and mass are plotted, but it is coupled to combustion.
- McGill Rocket Team report (arXiv 2302.06725): HF4 and HF5 hot-fire tank data, internal team data with no separate publication.
- Muttangi & Jangaon, PoliMi thesis 2017 (literature survey, full text not accessible).

### 2.6 Two-phase flashing and choked flow

#### FL-1. Hendricks, Simoneau & Barrows 1976: choked flow of subcooled O2 and N2
- **Citation:** Hendricks, R. C., Simoneau, R. J., Barrows, R. F. (1976). *Two-phase choked flow of subcooled oxygen and nitrogen*. NASA TN D-8169 (E-8499). NTRS 19760011288.
- **URLs opened:** record page and PDF https://ntrs.nasa.gov/api/citations/19760011288/downloads/19760011288.pdf
- **Content (Seen):** two C-D nozzles in two facilities. Conical: throat 0.3555 cm, convergence 6.79 deg, divergence 3.78 deg, throat L/d 3.20. Elliptical: throat 0.2934 cm, 2:1 elliptical inlet, divergence 4.00 deg, L/d 2.10. Tables IV to VII list P0, T0, G_max and tap pressures per run for O2 and N2, grouped by reduced temperature (about 0.61 to 1.03). Roughly several hundred points (OCR made exact counts impossible).
- **Companion:** Simoneau 1975, NASA TM X-71762 (opened): axial pressure profiles in the conical nozzle with LN2, 95 to 130 K, 60 to 680 N/cm2. It refers to a TN with the extensive tables (not located).
- **Effort:** Low (OCR or retype tables).
- **Validates:** RESTR choked mass flux. HEM will underpredict G_max for strongly subcooled inlet. The omega method with subcooled extension and the planned DYER model are the natural comparison. Real-fluid N2/O2 properties near the critical point are also tested.

#### FL-2. BNL flashing nozzle (water)
- **Citation:** Brookhaven National Laboratory (~1981). *Study of nonequilibrium flashing of water in a converging-diverging nozzle, Vol. 1: Experimental*. NUREG/CR-1864 Vol. 1 / BNL-NUREG-51317. OSTI 6221124. The authors (Abuaf, Wu, Zimmer, Saha) are not listed on the OSTI page.
- **URL opened:** https://www.osti.gov/biblio/6221124 (record; no full-text link seen).
- **Content (Seen):** steady water loop, C-D nozzle with 49 pressure taps, void fraction distributions, 163 figures and 23 tables. NRC ADAMS may hold a scan (not checked).
- **Validates:** axial pressure profile through a flashing nozzle, which separates inception delay from equilibrium. **Requires:** DYER or a relaxation model.

#### FL-3. Palacz & Cieślik 2021: N2O injectors
- *Aerospace* 8(11), 317. DOI 10.3390/aerospace8110317. CC BY 4.0. https://www.mdpi.com/2226-4310/8/11/317/html opened.
- 0.28 L run tank (40 mm ID x 220 mm) on a load cell, 4 mm feed tubing, five injectors with geometry in Table 1, critical-flow results in Table 2. Initial state about 294 K, 52 bar. No temperature time histories.
- **Validates:** RESTR with N2O (HEM vs omega vs DYER), real-fluid N2O properties.

#### FL-4. Waxman et al. ~2013: N2O injector mass flow and isolation
- Waxman, B. S., Zimmerman, J. E., Cantwell, B. J., Zilliac, G. G. *Mass flow rate and isolation characteristics of injectors for use with self-pressurizing oxidizers in hybrid rockets* (AIAA 2013, NTRS 20190001326; PDF opened). Upstream and downstream p and T at 500 Hz to 5 kHz, venturi flow measurement, several injector geometries in Table 1. Results are plotted. The broader data set is in Waxman's 2014 PhD thesis (https://purl.stanford.edu/ng346xh6244, CC BY-NC 3.0; full text not read).

#### FL-5. Henry & Fauske 1971
- *J. Heat Transfer* 93(2), 179–187. DOI 10.1115/1.3449782 (confirmed via scilit/sciencegate listing). Not opened. It is the origin of the non-equilibrium critical flow model and includes comparisons with earlier water data.

#### FL-6. Super Moby Dick
- OECD-NEA package CSNI1017/01 (test SMD 12R305C, CEA): https://oecd-nea.org/tools/abstract/detail/csni1017/ (opened). Listed as "Report only", pressures only (no mass flow for that test), distributed under NEA end-user rules. Not freely downloadable.

#### FL-7. Sozzi & Sutherland 1975
- GE report NEDO-13418. Cited in the CCPS TPHEM data file list (https://www.chem.mtu.edu/org/aiches&h/pdf/CCPS_Part_IV_TPHEM_Datafile.pdf, opened). No online copy of the report was found.

### 2.7 Cavitating venturis

#### CV-1. Hord, Anderson & Hall 1972: cavitation in liquid cryogens, Part I venturi
- NASA CR-2054 (NBS Boulder). PDF https://ntrs.nasa.gov/api/citations/19720016713/downloads/19720016713.pdf (opened). LH2 in a transparent quarter-round venturi. Appendix A tabulates developed-cavity data (inlet 19.9 to 23.0 K, 30 to 59 m/s), with cavity pressures and temperatures, cavity length and mass flow.
- Companions: Edmonds & Hord 1969, NBS TN 374 (LH2 and LN2, PDF opened). Hord 1973, NASA CR-2242 (ogives, record opened).
- **Relevance:** quantifies thermodynamic suppression of cavitation (vapour pressure depression inside the cavity) in cryogens. This matters for the cavitation onset criterion in RESTR. These are not choked-venturi flow-limiting tests.

#### CV-2. Water cavitating venturis (choked regime)
- Yazıcı, B. (2006). *Numerical and experimental investigation of flow through a cavitating venturi*. MSc thesis, METU (https://open.metu.edu.tr/bitstream/handle/11511/16448/index.pdf, opened). Four venturis (inlet 10 mm, throat 5 mm, throat length 3 mm, angles 30/7, 15/15, 15/30, 15/60 deg). Inlet and outlet pressure, turbine flow meter. Figures only.
- Yoon, W., Ahn, K. (2015). An experimental study on flow characteristics of cavitation venturi. *J. Korean Soc. Propulsion Engineers* 19(4), 1–7 (https://www.koreascience.or.kr/article/JAKO201528652395035.pdf, opened). Throat 2 mm, inlet and outlet 4.82 mm, angles 15/5, 30/5, 45/5, 30/10 deg, upstream 3 to 10 bar, water at 10 C. Cd and critical pressure ratio plotted.
- Ulaş, A. (2006). Passive flow control in liquid-propellant rocket engines with cavitating venturi. *Flow Meas. Instrum.* 17, 93–97. DOI 10.1016/j.flowmeasinst.2005.10.003 (abstract opened). Average flow rates only.
- **Validates:** RESTR in the choked-cavitating regime (mass flow independent of back pressure up to the critical pressure ratio), and the diffuser recovery that sets the critical ratio. Simple and cheap to set up.

### 2.8 Gas blowdown and regulators

#### GB-1. Imperial College vessel blowdown
- Richardson, S. M., Saville, G. (1992). Blowdown of vessels and pipelines. IChemE Symposium Series 130. https://www.icheme.org/media/25273/major-hazards-onshore-and-offshore-icheme-symposium-series-130-1992-16-richardson.pdf (opened). Journal companion: Haque, M. A., Richardson, S. M., Saville, G., Chamberlain, G., Shirvill, L. (1992). Blowdown of pressure vessels II: experimental validation of computer model and case studies. *Trans. IChemE Part B* 70, 10–17 (not opened).
- **Content (Seen):** Vessel A: 1.524 m long, ID 0.273 m, 25 mm wall, N2 and CO2 at about 150 bar, about 100 s blowdowns, 15 test sets. Vessel B: 2.25 m T/T, ID 1.13 m, 59 mm wall, CH4/C2H6/C3H8 mixtures and N2 at about 120 bar, about 1500 s, 18 sets. Chokes such as 6.35 mm (A) and 10 mm (B). Pressure, fluid temperature, inside and outside wall temperature. Plots show experimental ranges as hatched bands.
- **Validates:** TANK-H, WALL (thick wall, free convection inside), RESTR choked gas flow, real-gas properties (N2 at 150 bar). The CO2 cases cross the saturation line and test HEM in a tank. **Effort:** medium. Wall material is not stated on the pages read (carbon steel is likely, Inferred).

#### GB-2. Hauser & Quinn 2012: cold-gas thruster system with regulator
- NASA TM-2012-217271 / AIAA 2011-5769. NTRS 20120003741 (record and PDF opened). Four N2 tanks at 3600 psig, direct-acting regulator set at 600 psig, 110 in3 downstream volume, pilot-operated thruster valve, 150 lbf nozzle. Test traces of regulator inlet and outlet pressure and chamber pressure under 400 to 2500 ms pulses (Figs. 2, 3, 9, 10). Regulator spring rates, areas and line geometry are **not** given, and the sense orifice was tuned. Marked Unclassified-Unlimited, no export statement found.
- **Use:** qualitative check of regulator droop and recovery under pulsing only. It cannot be rebuilt without assumptions.

#### GB-3. Regulator data: gap
- No public regulator data set with full internal parameters and measured step responses was found. Most regulator modelling papers found (METU, ITU, Kharkiv, Iranian JCARME) are model studies with limited test data. If REG needs quantitative validation, the realistic options are GFSSP Examples 16 and 17 (code-to-code) or a bench test.

### 2.9 Code benchmark collections

#### BM-1. GFSSP user manuals
- Majumdar, A. K., LeClair, A. C., Moore, R., Schallhorn, P. A. (2016). *Generalized Fluid System Simulation Program, Version 6.0*. NASA/TP-2016-218218 (NTRS 20160005401), 30 example problems.
- Earlier: NASA/TM-2013-217492 (NTRS 20140003178), 25 examples.
- **URLs opened:** both records and PDFs (table of contents).
- **Relevant examples (Seen in TOC):** 1 pump-valve-pipe; 3 C-D nozzle; 8 tank blowdown (vs analytic); 10 propellant tank pressurisation; 11 turbopump power balance; 12 He pressurisation of LOX and RP-1 tanks; 14 chill-down of a short cryogenic line (vs analytic solution of Cross et al.); 15 valve-closure transient (vs MOC); 16 pressure regulator downstream of a tank; 17 flow regulator; 18 Fanno; 19 Rayleigh; 24 relief valve; 26 fluid transient after sudden valve opening; 28 no-vent tank chill and fill; 29 tank self-pressurisation by boil-off. Only Examples 6, 7 and 21 are labelled in the TOC as compared with experimental data, and those (rotating disk, squeeze-film damper, turbopump internal flow) are outside fluidnet's scope.
- **Use:** fully specified inputs (Appendix input files). Code-to-code verification of every fluidnet component class, including REG. They are not experimental validation.

#### BM-2. ESPSS / EcosimPro
- Koppel, C. R., Moral, J., De Rosa, M., Vara, R. P., Steelant, J., Omaly, P. (2012). Satellite propulsion modeling with EcosimPro: comparison between simulation and ground tests. EUCASS Progress in Propulsion Physics, DOI 10.1051/eucass/201102743 (abstract page opened; test cases not named).
- An EcosimPro transient-flow thesis (CORE 14702006, opened) describes a water-hammer validation and a P2/AESTUS case, without reusable data.
- ESA ESPSS validation material is distributed with the licensed tool. No public test-case collection was found.

---

## 3. Recommended first set (in order)

1. **DLR PS-LN2-Set1 (WH-1) + Klein et al. 2023.** Raw, CC BY, cryogenic, with cavitation. Start with the low-velocity cases (no cavitation) to calibrate wave speed and friction, then the high-velocity cases with vapour formation. Tests PIPE, VALVE, HEM and LN2 properties in one rig.
2. **Bergant & Simpson 1999 (WH-2).** Fully specified classic water benchmark with a measured wave speed. Use one non-cavitating and one column-separation run. Many published model comparisons exist to compare against.
3. **Hendricks, Simoneau & Barrows 1976, TN D-8169 (FL-1).** Tabulated choked LN2/LOX mass flux over a wide range of subcooling. Lowest effort, and it directly shows where HEM and omega fail and what DYER must fix.
4. **Brennan et al. 1966, NBS 9264 (CD-1).** A long-line LN2/LH2 cool-down with pressure and temperature at four stations. It is the reference case for WALL + BOIL in a network context. Use the CRTech re-analysis to resolve data quirks. Add **Darr et al. 2016 Test 40 (CD-2)** as the clean single-tube check of the boiling correlations.
5. **Stochl et al. 1969, TN D-5387 (TK-2) and Van Dresar & Stochl 1993, TM-106427 (TK-1).** Tabulated pressurant mass and energy balances for GH2 pressurisation and expulsion of LH2 tanks with known wall mass. These are the primary validation cases for the planned two-fluid tank. Add TN D-7019 (He pressurant) once the ullage is multi-component.
6. **Hasan, Lin & Van Dresar 1991, TM-103804 (TK-6).** Self-pressurisation at three heat fluxes in the same K-site tank. The homogeneous tank should match the lowest flux and miss the highest by about 3x. That documents the limit of TANK-H quantitatively.
7. **Imperial College vessel blowdown (GB-1).** N2 and CO2 blowdowns with inside/outside wall temperatures. Tests TANK-H + WALL + choked RESTR, and the CO2 cases cross into two-phase.
8. **Zimmerman 2015 thesis (N2O-1)**, after getting the PDF. It is the best-documented public N2O/CO2 tank drain data and the target for N2O-NEQ. Pair with **Palacz & Cieślik 2021 (FL-3)** for N2O orifice mass flow. Check that the CC BY-NC licence fits the intended use.

Deferred: priming data (PR-1, PR-2) until fluidnet can carry a non-condensable gas in a liquid line. Pipe-wall FSI (WH-4) until ELAST exists. MHTB, Centaur and AS-203 until stratification and TVS are in scope. Regulator dynamics has no adequate public data (GB-3), so use GFSSP Examples 16 and 17 for code-to-code checks.

## 4. Access and licence notes
- CC BY 4.0: DLR PS-LN2-Set1 (Zenodo), Klein et al. 2023 Exp. Fluids, Hartwig et al. 2022 npj Microgravity, Palacz & Cieślik 2021, La Luna et al. 2022.
- CC BY-NC 3.0: Zimmerman 2015 and Waxman 2014 Stanford theses.
- NASA NTRS documents opened are public. The Hartwig et al. 2015 IJHMT record is marked "Public Use Permitted". The Hauser & Quinn TM is "Unclassified-Unlimited". No export-control markings were seen in the NASA documents opened. Only first pages or records were read for most, so markings deeper in a document cannot be excluded.
- Paywalled or not accessed: AIAA 2013-4045, AIAA 2005-3549, Dyer et al. AIAA 2007-5702, Lecourt & Steelant 2007, Prickett et al. 1992, Moore et al. 2019, Song et al. 2016, Lema et al. 2015, Haque et al. 1992, Tijsseling et al. 1996, Henry & Fauske 1971.
- Restricted distribution: Super Moby Dick (OECD-NEA package), IAC-13/IAC-14 papers (IAF).

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
