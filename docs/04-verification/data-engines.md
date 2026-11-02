# Literature validation data: engines and turbomachinery

FNT-VAL-REG-002 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

> Register of public literature data for validation (decision D-023: no own
> test data). Compiled from web research on 2026-11-02; every source was
> located online, and the entries state what was read in the document
> ("Seen") and what is judgement ("Inferred"). Numbers read from OCR text of
> old scans must be checked against the original before use. The reference
> problems built from these data sets are in section 3 of the
> [platform development plan](../01-project/platform-plan.md).

Catalogue for fluidnet validation beyond the RL10A-3-3A. Compiled 2026-11-02 from web research only.

## How this was checked

- Every source listed was located on NTRS, DTIC, JAXA, KoreaScience, DLR elib, university repositories or publisher pages, and its title, authors, year and report number were read from that page. The URLs given are the ones that were opened.
- PDFs could not be downloaded directly: the session proxy blocks ntrs.nasa.gov for curl. Full-text content was read through a web-fetch tool that extracts the PDF text and figure/table titles. Descriptions of content ("Table E-2 compares ...") come from that extracted text. The tool works from OCR on older scans, so numbers quoted here must be checked against the original before use.
- "Seen" means stated in the document text. "Inferred" means my engineering judgement and is marked as such.
- Where only an abstract or a metadata record could be opened, this is stated.

fluidnet capability baseline assumed: (P,h) nodes, inertial pipes, restrictions with cavitation/flashing, valves, tanks, centrifugal pumps and impulse turbines with tabulated maps, shafts, regenerative walls (Bartz), equilibrium chamber from Cantera tables, steady state and transients.

---

## 1. Ranked summary

Ranking weighs (a) real test data, (b) how much of the model input is published, (c) how far it pushes fluidnet beyond the RL10, and (d) effort. Model-to-model benchmarks are ranked on input completeness because they cannot replace test data but are the only fully specified large systems in the public domain.

| Rank | Data set | Cycle | Data type | Completeness for modelling | Effort | fluidnet capabilities likely missing |
|---|---|---|---|---|---|---|
| 1 | RL10-IIB breadboard low-thrust modes, NASA CR-174914 (+ OHE component tests CR-179487, CR-182159) | Expander (RL10 derivative) with H2/O2 heat exchanger; tank-head idle and pumped idle | Steady test vs cycle-deck tables; transient plots (start to THI, transition to PI, Pc oscillation); HX test tables | High: valve calibration curves, injector areas, HX geometry, damper volume, valve schedules given; RL10 base data already in hand | Low-medium (tables + some digitizing) | Two-stream liquid/gas heat exchanger with boiling oxygen; two-phase injector flow; cavitating venturi; very low-pressure two-phase H2 in jacket |
| 2 | SSME start with off-nominal inlet pressures, Bradley 1997, NASA CR-205359 / AIAA 97-2687 | Staged combustion, two fuel-rich preburners, 4 turbopumps | Start transient test traces vs Rocketdyne DTM to 80% RPL | Low: no maps, volumes, valve areas; sequence timing and prime times given | High (digitize; model must be calibrated from other sources) | Fuel-rich preburners, preburner/MCC priming and ignition, 4 shafts, boost pumps, hydraulic turbine (LPOTP), reaction turbines, closed-loop Pc/MR control, tank-head start |
| 3 | SSME low power level operation, Bradley 1997, NASA CR-205360 / AIAA 97-2685 | Staged combustion | Steady tables predicted vs actual at 17, 22, 27, 40 % RPL; pump test points on HPFTP/HPOTP/preburner-pump maps; throttle transient plots | Low-medium: steady states at many power levels plus measured pump operating points | Medium | Same as rank 2; low-speed pump/turbine map regions |
| 4 | OEPSS Data Book Vol. 8, Integrated BPM engine start dynamics (Kemp 1992), NASA CR-207443 | Gas generator LH2/LOX (STME), 8 chambers / 4 turbopumps, H2 spin start | Simulation only (Rocketdyne DTM): start, cutoff, chamber-out cases | High: line lengths/diameters, manifold dimensions, volumes, valve areas and area-vs-position, LOX and fuel pump maps | Medium (tables + digitize maps and results) | Gas generator (fuel-rich H2/O2), gas spin start supply, manifolded multi-chamber network; code-to-code only |
| 5 | Mark 48-F LH2 turbopump, NASA CR-159821, and Mark 48-O LOX turbopump, NASA CR-159509 | Turbopump component (Advanced Space Engine, staged combustion) | Component test: head-flow maps, efficiency, suction performance, turbine performance, speed histories | High for a component: geometry, design tables, maps | Medium (digitize maps) | Multi-stage pump treatment; axial reaction turbine maps; idle at zero NPSH |
| 6 | J-2 hydrogen pump zero-tank-NPSH tests, NASA TN D-6824 | Turbopump component (J-2 Mk 15-F, 7-stage axial pump) | Head coefficient vs inlet vapour fraction; start transients (speed, flow, discharge pressure) | Medium: feed-system diameters and tank volume; no pump map | Medium (digitize) | Pump performance degradation with two-phase inlet; axial multi-stage pump maps |
| 7 | SSME TTB-50 steady data with ROCETS predictions, Santi 1994 (NTRS 19950012587); PBM-based HPFTP subsystem data, Santi 1993, NASA CR-195253; SSME Orientation BC98-04 (104.5 % flow schematic) | Staged combustion | Steady tables (measured and predicted at 100 % RPL); design-balance values | Low-medium | Low (tables) | As rank 2 (steady only) |
| 8 | Nuclear rocket cold-flow bootstrap tests, NASA TM X-1917 (+ TM X-52139, SP-123, Run 11 TM) | Bootstrap start of an LH2 turbopump system, turbine driven by hydrogen warmed in nozzle/core | Measured start transients; pump head-flow lines incl. stall; tabulated peak values | Medium: valve Cv values, line sizes, pump characteristic lines; no volumes | Medium-high | Large solid thermal masses (core/reflector) as heat source; pump stall region; chilldown two-phase flow |
| 9 | LE-5 / LE-7 / LE-7A start transient papers (NAL TR-868; ISTS 1988; AIAA 89-2736; AIAA 2004-3850; JAXA-RR-04-010; JAXA-SP-03-002) | GG with bleed start (LE-5); fuel-rich staged combustion (LE-7/7A) | Simulation vs firing test (per abstracts) | Unknown; full text not accessed | High (Japanese for TR-868; AIAA paywall; likely normalised) | Fuel-rich preburner; GG; bleed-to-GG transition |
| 10 | Fastrac / MC-1 public set (LOX pump superscale water test, GG component tests, Ballard engine summary, Rocker stability) | GG LOX/RP-1, single shaft | Pump geometry + head coefficient; GG test temperatures; a few engine hot-fire Pc/MR values | Low at system level | Medium | Fuel-rich RP-1 GG (non-equilibrium gas properties); start igniters |
| 11 | STS-104 shutdown pressure surge, Greene and Kynard 2002, AIAA 2002-3581 | SSME low-pressure fuel feed during shutdown | Flight data vs lumped model | Low: no volumes or coefficients | Medium | Probably none (pump spindown, trapped liquid H2 volumes) |
| 12 | KARI 75 tf GG engine startup (Cho et al. 2010) and staged-combustion powerpack papers (Lee and Moon 2015, 2016; J. Aerosp. Eng. 2017) | GG LOX/kerosene; ox-rich staged combustion powerpack | One TP-GG closed-loop start comparison (rpm, GG Pc); otherwise simulation | Low: maps referred to internal reports | High (Korean text, digitizing) | Fuel-rich kerosene GG, ox-rich preburner, start turbine |
| 13 | CECE deep-throttling RL10 derivative (NASA MSFC-325, M10-0844, M10-0118) | Expander | Mostly qualitative / normalised throttle range, chug mitigation | Low | Low value | Combustion time-lag (chug) model; subcritical boiling in jacket |
| 14 | Apollo ascent/descent engine pressure-fed transients, NASA CR-114915 | Pressure-fed hypergolic | Start pressure transients vs test; appendices with data packages (not seen in extract) | Possibly medium | Medium | Line priming of empty lines, dissolved pressurant evolution |
| 15 | LUMEN (DLR), IAC-24 paper and FAR 2025 paper | Expander bleed LOX/LNG, 25 kN | One Pc/speed plot so far; component papers referenced | Low today | Watch list | Methane properties; bleed turbine exhaust |
| – | Not usable / no public data found | IPD (full-flow SC), J-2X system, Vulcain/Vulcain 2/HM7B/Vinci, NPSS rocket models, AETB, ROCETS TTBE model, SSME linear models, J-2 AEDC altitude reports (ITAR) | see section 4 | | | |

---

## 2. Detailed entries

### 2.1 RL10-IIB breadboard low-thrust operating mode (rank 1)

- Citation: Kmiec, T. D.; Galler, D. E., "Breadboard RL10-11B Low Thrust Operating Mode", Pratt & Whitney FR-18683-2, NASA CR-174914, 1987. NTRS 19870005836 (accession 87N15269). Distribution: Public, "Work of the US Gov. Public Use Permitted".
- URLs opened: https://ntrs.nasa.gov/citations/19870005836 and https://ntrs.nasa.gov/api/citations/19870005836/downloads/19870005836.pdf
- Engine: RL10 derivative with an added hydrogen/oxygen heat exchanger (OHE) that vaporises LOX for injection. Tested February 1984 in tank-head idle (THI, ~1-2 % thrust, no pumping) and pumped idle (PI, ~10 % thrust).
- Seen in document:
  - Table E-2: THI test vs cycle-deck match (chamber pressure, mixture ratio, flows, pressures, temperatures, heat transfer, GOV pressure loss). Table E-3: PI test vs deck (adds thrust, Isp, fuel bypass flow, pump discharge pressure). Tables D-2/D-3: flow methods, MR, Isp for run 15.01.
  - Fig. D-25/D-26: fuel and oxidiser pump operating characteristics.
  - Transients: oxidiser pump cooldown (D-17, D-18, D-20), predicted start-to-THI (E-8 to E-14), predicted transition to PI (D-21), valve sequencing (D-22, E-20, E-24, E-28), measured chamber pressure for run 11.01 (D-23, E-29), predicted vs measured oscillation frequency and amplitude (E-30, E-31).
  - Model vs test: chamber/nozzle and HX heat transfer and HX pressure loss (D-1 to D-8), pump housing temperature (E-21, E-22).
  - Component data: HX geometry (Fig. B-1: plate counts, passage diameter, flow areas, 21.3 ft2 heat transfer area); valve calibration curves for turbine bypass, oxidiser control, cavitating venturi, gaseous oxidiser valves (A-1 to A-4); fuel vent valve area; injector effective areas (ox 0.760 in2, fuel 3.048 in2); gear ratio 2.118; damper volume about 270 in3; PI transient valve schedule (Table D-1).
  - No export marking found in extracted text; marked UNCLASSIFIED.
- Companion component data:
  - Kmiec, T.; Kanic, P., "Oxidizer Heat Exchanger Component Testing", P&W FR-19134-3, NASA CR-179487, 1986. NTRS 19860020429, Public. Opened https://ntrs.nasa.gov/citations/19860020429 and the PDF. Seen: tabulated steady run points (App. D, Table D-1) with GN2/LN2 surrogate fluids for two HX stages (flows, inlet T and p); HX passage dimensions and roughness (App. B); comparison with the CHEAP crossflow HX code (App. E). No transients.
  - Kanic, P. G., "Oxidizer heat exchanger component test", PW-FR-19602, NASA CR-182159, 1988. NTRS 19880020492. Only the citation page was opened: two HX designs tested in THI and PI conditions; low-heat-transfer unit reached O2 exit quality >= 0.95.
- Completeness (inferred): combined with the RL10A-3-3A data already used (TM-107318), this is close to a complete model. The cycle deck comparison tables let you separate component errors from system errors.
- Effort: tables are direct; transient comparisons need digitizing.
- Missing fluidnet capabilities: a counter/cross-flow two-stream heat exchanger node pair with boiling on one side; two-phase oxygen through injector; low-pressure (subcritical) hydrogen in the regen jacket at THI; a cavitating venturi (restriction with choked cavitation may already cover it).
- Why rank 1: smallest step from the validated RL10 model, but it exercises off-design, two-phase and heat-exchanger physics that larger engines also need.

### 2.2 SSME start with off-nominal propellant inlet pressures (rank 2)

- Citation: Bradley, M. A., "Space Shuttle Main Engine Start with Off-Nominal Propellant Inlet Pressures", AIAA 97-2687, 33rd JPC, Seattle, 1997. NASA CR-205359. NTRS 19970028362. Distribution: Public, public use permitted.
- URLs opened: https://ntrs.nasa.gov/citations/19970028362 and https://ntrs.nasa.gov/api/citations/19970028362/downloads/19970028362.pdf
- Engine: SSME Phase II, TTB engine 3001 at MSFC Technology Test Bed; tank-head start; tests 902-638 to 902-641 with LOX inlet pressure reduced from 80 to 50 psia and subcooled LOX (about 160 R).
- Seen in document:
  - Figs. 12-17: predicted (Rocketdyne Digital Transient Model) vs actual start transients to 80 % RPL for MCC chamber pressure, HPOTP discharge pressure, HPFTP speed, HPFTP discharge pressure, FPOV and OPOV positions.
  - Figs. 4-11: DTM analysis for test 902-641 (chamber pressure, LOX-side build-up, OPB combustion, OPOV/FPOV sequence modifications, MOV ramp rate).
  - Figs. 18-19: HPFTP turbine inlet temperature. Fig. 20: prime times vs LOX inlet temperature. Prime timing quoted in text (FPB ~1.3 s, MCC ~1.4 s, OPB ~1.6 s; mainstage at 2.4 s). Igniter orifice change 0.052 to 0.057 in.
  - Fig. 1 flow schematic; Fig. 2 specifications. No tables.
- Completeness: low for model building. No pump/turbine maps, line volumes, valve areas or injector resistances. The valve position traces are usable as boundary conditions.
- Effort: high. All comparison data is in plots; the model must be built from other public descriptions and calibrated on steady points (2.3, 2.7).
- Missing fluidnet capabilities: two fuel-rich H2/O2 preburners (equilibrium at MR around 0.6-0.9, near the lower validity limit of Cantera equilibrium tables for ignition timing); preburner and MCC priming (LOX dome fill, two-phase); ASI igniters; four shafts; LPOTP driven by a liquid-oxygen hydraulic turbine; HPOTP with a preburner boost pump on the same shaft (two pumps on one shaft); two-stage reaction turbines on HPFTP/HPOTP (tabulated maps would suffice if available); LPFTP turbine driven by warm hydrogen from the cooling circuits; closed-loop Pc and MR control during the ramp.
- Restrictions: none stated beyond normal AIAA copyright on the paper itself; NTRS lists it public.

### 2.3 SSME off-nominal low power level operation (rank 3)

- Citation: Bradley, M., "Space Shuttle Main Engine Off-Nominal Low Power Level Operation", AIAA 97-2685, 1997. NASA CR-205360. NTRS 19970028361. Distribution: Public.
- URLs opened: https://ntrs.nasa.gov/citations/19970028361 and https://ntrs.nasa.gov/api/citations/19970028361/downloads/19970028361.pdf
- Seen in document:
  - Fig. 5 (a table): "Low Power Level Prediction versus Actuals at 17 to 27 % RPL" (TTB-062/065): MCC Pc, engine MR, nozzle coolant flow, coolant discharge temperature, valve positions, pump speeds, pressures and temperatures.
  - Fig. 9 (a table): "Predictions versus Actuals at 40 % RPL" (engine 3001): pump speeds, HPOTP/HPFTP discharge temperatures, OPOV/FPOV positions, MR, flowmeter speed, with pretest prediction, actual, delta and 1-sigma.
  - Figs. 11-13: HPFTP, HPOTP main pump and HPOTP preburner pump performance curves with test points at various power levels.
  - Fig. 6: HPFTP flow coefficient during 17-27 % RPL; Figs. 3, 7, 8: power-level time histories including ramps 80 to 40 % RPL; Fig. 4: DTM results for TTB-065; Fig. 10: HPFTP first-rotor critical speed operation.
- Completeness: steady tables at multiple power levels are the most useful public SSME numbers I found. Pump curves with measured points give partial map information including the low-flow region.
- Effort: medium; tables need transcription, pump curves need digitizing.
- Missing capabilities: as 2.2; plus pump maps valid at low speed/low flow coefficient.

### 2.4 OEPSS Data Book Vol. 8: integrated booster propulsion module start dynamics (rank 4, model-to-model)

- Citation: Kemp, V. R., "Operationally Efficient Propulsion System Study (OEPSS) Data Book: Integrated Booster Propulsion Module (BPM) Engine Start Dynamics, Volume 8", Rockwell International, RI/RD90-149-Vol-8, NASA/CR-92-207443, 1992. NTRS 19980041374. Distribution: Public.
- URLs opened: https://ntrs.nasa.gov/citations/19980041374 and https://ntrs.nasa.gov/api/citations/19980041374/downloads/19980041374.pdf
- System: eight STME thrust chambers fed by four turbopumps (each turbopump feeds two chambers through manifolds), gas generator cycle LH2/LOX, hydrogen spin-assisted GG start; balanced to a February 1990 design at 497 klbf.
- Seen in document: Table 2 line lengths and diameters (fuel and LOX feed), ring manifold dimensions, turbine line volumes, MCC volume (12,500 in3), throat area (132.5 in2), area ratio 39, pump inertias. Table 3 effective valve areas. Fig. 4 valve area vs position. Figs. 5-8 LOX and fuel pump maps. Table 1 model steady state vs design balance. Appendices A-H: start/cutoff and chamber-out transients with valve positions, chamber and GG pressures, MR, pump speeds, discharge pressures, flows, temperatures, pump flow coefficient, H2 spin flow.
- Not test data. Results are from Rocketdyne's digital transient model. Usable for code-to-code verification of a large GG network with fully specified inputs.
- Effort: medium (tables + digitizing maps and result curves).
- Missing capabilities: GG combustor (fuel-rich H2/O2 is well within equilibrium validity at these MR); gas spin-start supply (pressurised GH2 into turbine); turbine maps may not be fully given (only pump maps were listed in the extract; inferred that turbine characteristics are described elsewhere or simplified).

### 2.5 Mark 48-F and Mark 48-O turbopumps (rank 5, component)

- Citations:
  - Csomor, A.; Warren, D. J., "Small, High Pressure Liquid Hydrogen Turbopump", Rockwell/Rocketdyne RI/RD79-322, NASA CR-159821, 1980. NTRS 19800018163. Public.
  - Csomor, A., "Small, High-Pressure, Liquid Oxygen Turbopump", RI/RD78-278, NASA CR-159509, 1978. NTRS 19790009050. Public.
- URLs opened: https://ntrs.nasa.gov/citations/19800018163, https://ntrs.nasa.gov/api/citations/19800018163/downloads/19800018163.pdf, https://ntrs.nasa.gov/citations/19790009050, https://ntrs.nasa.gov/api/citations/19790009050/downloads/19790009050.pdf
- Seen: Mk 48-F is a three-stage centrifugal LH2 pump with a two-stage axial reaction turbine, 95,000 rpm design, 2.74 kg/s, 3140 N/cm2 discharge. Three test series (10 + 1 + 19 tests) with head-flow maps, efficiency, suction performance (Nss above ~100,000 US units), idle operation at zero NPSH, low-flow stall in test 019, turbine performance table (Table 14), stage-by-stage head data, design tables (Tables 1-3, 5), geometry figures and assembly drawings. Mk 48-O: LOX pump head-flow scaled to 70,000 rpm (Fig. 4, Fig. 59), efficiency vs flow (Figs. 56, 60), turbine total-to-static efficiency 51 % measured vs 59.8 % predicted, suction performance, speed vs time plots (Figs. 53-54), geometry.
- Use: verifying fluidnet's map handling (multi-stage pump, stall boundary, zero-NPSH idle), and turbine map interpolation at off-design velocity ratio.
- Missing capabilities: reaction (not impulse) turbine maps; if fluidnet's turbine element assumes impulse-type velocity-ratio efficiency, a generic tabulated map is needed.

### 2.6 J-2 hydrogen pump at zero tank NPSH (rank 6, component)

- Citation: Stinson, H. P.; Strickland, R. J., "Experimental Findings from Zero-Tank Net Positive Suction Head Operation of the J-2 Hydrogen Pump", NASA TN D-6824 (M-368), 1972. NTRS 19720022157. Public.
- URLs opened: https://ntrs.nasa.gov/citations/19720022157 and https://ntrs.nasa.gov/api/citations/19720022157/downloads/19720022157.pdf
- Seen: Mk 15-F pump in the MSFC J-2 turbopump facility with the S-IVB fuel feed system (sump and duct diameters, 136.3 m3 tank). Figs. 8-11: head coefficient vs percent vapour by volume at gimbal-duct inlet for several Q/N at 22-25 K. Fig. 12-13: vapour handling limit (~20-25 %). Figs. 14-19: start transients (speed, discharge flow, discharge pressure) at 24 and 25 K compared with normal pump and engine starts. Figs. 20-21: start vs steady cavitation performance.
- No pump map, line volumes or valve areas.
- Missing capabilities: pump head degradation as a function of inlet void fraction (fluidnet has cavitation in restrictions; pump-inlet two-phase degradation may need a separate head-ratio vs void-fraction table). J-2 pump is axial; tabulated maps should still work.

### 2.7 SSME steady data: TTB-50 table, PBM subsystem data, SSME Orientation (rank 7)

- Santi, L. M., "Evaluation of SSME test data reduction methods", 1994 NASA/ASEE Summer Faculty Fellowship report, NTRS 19950012587 (accession 95N19002). Public. Opened https://ntrs.nasa.gov/citations/19950012587 and the PDF. Seen: Table 2 TTB-50 test data at 100 % RPL (turbine temperatures, pump speeds, component pressures, flows) with ROCETS baseline predictions and adjusted data-reduction runs; Table 1 normalised sensitivities.
- Santi, L. M., "Integrated Model Development for Liquid Fueled Rocket Propulsion Systems", NASA CR-195253, 1993. NTRS 19940022663. Opened the PDF. Seen: HPFTP subsystem analysis started from Rocketdyne Power Balance Model values at 109 % RPL; Tables A1-A3 give branch flows, nodal pressures and temperatures, shaft speed. Model values, not test data.
- Rocketdyne Propulsion & Power, "Space Transportation System Training Data: Space Shuttle Main Engine Orientation", BC98-04, June 1998. Opened https://enginehistory.org/Rockets/SSME/SSMEorientation.pdf. Seen: propellant flow schematic with pressures, temperatures, flows and speeds at 104.5 % RPL (p. 19); turbopump key performance tables (speed, power, efficiency, flow, pressure) for LPOTP, LPFTP, HPOTP, HPFTP (pp. 53, 57, 59, 67); start and shutdown main valve position sequences (pp. 94-95). Restriction: every page marked "BOEING PROPRIETARY" and "Use this data for training purposes only". It is hosted by a private history site, not released by NASA. Recommend using it only for internal cross-checks and not redistributing or publishing values from it.
- Use: these give the calibration point set (100, 104.5, 109 % plus 17-40 % from 2.3) for the SSME model that 2.2 then tests transiently.

### 2.8 Nuclear-rocket cold-flow bootstrap start tests (rank 8)

- Petrik, E. J.; Reardon, J. E., "Cold-Flow Startup Test Summary of a Full-Scale Nuclear Rocket Engine Using a Radial Turbopump", NASA TM X-1917, 1969. NTRS 19700003018. Public. Opened https://ntrs.nasa.gov/citations/19700003018 and PDF. Seen: unfuelled KIWI-B1-B-type reactor simulator at Plum Brook B-3, radial pump with inducer and two-stage axial turbine on one shaft, turbine driven by hydrogen bled from the nozzle; valve Cv values in Fig. 1 (PSOV 3500, PMDV 710, TPCV 710, LHBV 105); 8 in inlet, 4 in discharge and bleed lines; pump head-rise vs inlet flow with constant-speed lines for radial and axial pumps (Figs. 20-21); measured inlet flow, speed, pressures, temperatures vs time (Figs. 5-21); Tables II-V with peak flow, pressure rise, speed, time to peak (e.g. 35 psia wet bootstrap: 32.6 lb/s, 164 psi, 9050 rpm at 6.5 s). No model comparison, no volumes.
- Related (opened, smaller value): Colmery and Powers, "Flow System Startup of a Full-Scale Simulated Nuclear Rocket Engine", NASA TM X-52139, 1965 (https://ntrs.nasa.gov/api/citations/19650025799/downloads/19650025799.pdf); "Nuclear Rocket Simulator Tests: Flow Initiation with Turbine Accelerated, Tank Pressure 50 psia, Run 11", Lewis TM, 1964, report number not legible in the scan (https://ntrs.nasa.gov/api/citations/19660002339/downloads/19660002339.pdf); "Nuclear Rocket Technology Conference", NASA SP-123, 1966, includes bootstrap pump speed vs time for several tank pressures (https://ntrs.nasa.gov/api/citations/19730061727/downloads/19730061727.pdf).
- Inferred value: bootstrap start of a hydrogen turbopump driven by heat from large solid masses is physically close to an expander start, but at larger scale and with an axial or radial pump. Good test of start-up with tank-pressure-only initial drive and pump stall.
- Missing capabilities: large lumped solid heat sources (core, reflector) coupled to the flow; chilldown with film boiling; pump stall region in maps.

### 2.9 Japanese LE-5 / LE-7 / LE-7A transient papers (rank 9, access-limited)

- Kanmuri, A.; Wakamatsu, Y.; Shimura, T.; Toki, K.; Torii, Y., "Start Transient Analysis of Turbopump-fed LOX/LH2 Rocket Engine (LE-5)", NAL TR-868, 1985 (Japanese with English abstract). Record https://jaxa.repo.nii.ac.jp/records/45126 (also NTIS N8619371). Abstract: GG cycle with turbopump initially driven by coolant bleed, then switched to GG; simulated start compared with firing tests. A PDF (naltr00868.pdf, 2.0 MB) is listed as open, but the repository blocked automated retrieval, so the content was not checked.
- Kanmuri, A.; Kanda, T.; Wakamatsu, Y.; Torii, Y.; Kagawa, E.; Hasegawa, K., "Start Transient Analysis of LOX/LH2 Rocket Engine (LE-7)", Proc. 16th ISTS, Sapporo, 1988, vol. 1, pp. 309-314. Record https://jaxa.repo.nii.ac.jp/records/38504 (metadata only).
- Same authors, "Transient analysis of LOX/LH2 rocket engine (LE-7)", AIAA 89-2736, 25th JPC, 1989. Record https://jaxa.repo.nii.ac.jp/records/38480 (metadata only).
- Yamanishi, N.; Kimura, T.; Takahashi, M.; Okita, K.; Negishi, H.; Atsumi, M., "Transient Analysis of the LE-7A Rocket Engine Using the Rocket Engine Dynamic Simulator (REDS)", AIAA 2004-3850, DOI 10.2514/6.2004-3850. Record https://jaxa.repo.nii.ac.jp/records/39006 (metadata only; AIAA paywall).
- Kimura, T. et al., "Rocket Engine Dynamic Simulator (REDS)", JAXA-RR-04-010, 2004; abstract states verification against LE-7A combustion test data. Record https://jaxa.repo.nii.ac.jp/records/2386 (full text availability not determined). Shorter version JAXA-SP-03-002, pp. 107-112, PDF listed as public (https://jaxa.repo.nii.ac.jp/records/6621), not retrieved.
- Inferred: JAXA publications typically show normalised axes; component data likely absent. Worth obtaining manually (browser download) because LE-7A is the only other staged-combustion start transient with published test comparison besides the SSME.
- Missing capabilities: fuel-rich preburner, staged combustion; for LE-5, GG plus bleed-start mode switching.

### 2.10 Fastrac / MC-1 public material (rank 10)

- Skelley, S.; Zoladz, T., "Water Flow Performance of a Superscale Model of the Fastrac Liquid Oxygen Pump", 10th Thermal and Fluids Analysis Workshop, 2001. NTRS 20020050420. Public. Opened citation and PDF. Seen: Table 1 inducer/impeller/diffuser/volute geometry (e.g. 3+6 blades, 5.177 in inlet tip diameter, 7.056 in exit tip diameter, 26.5 deg exit blade angle, 6.660 in2 volute throat), Table 2 design conditions, head coefficient vs flow (80-120 % design) at 5000 rpm vs prediction (Fig. 5), head vs suction specific speed (Figs. 6-7), discharge flow angle (Fig. 10). No efficiency. Normalised performance with absolute geometry.
- Zoladz, T., "Observations on Rotating Cavitation and Cavitation Surge From the Development of the Fastrac Engine Turbopump", AIAA 2000-3403. Opened https://ntrs.nasa.gov/api/citations/20000067660/downloads/20000067660.pdf. Dynamics only.
- Garcia, R.; Williams, R.; Fears, S., "Hydrodynamic Design of the Fastrac Turbopump", NASA/TM-1998-207922. NTRS 19990009133 (citation page only).
- Dennis, H. J.; Sanders, T., "NASA Fastrac Engine Gas Generator Component Test Program and Results", AIAA 2000-3401. NTRS 20000064017 (citation), presentation 20000085971 (citation). 37 GG hot-fire tests.
- Nesman, T. E.; Dennis, J., "Fastrac Gas Generator Testing", TFAWS 2001. NTRS 20020050413 (citation only).
- Nguyen, H., "Thermal Analysis and Testing of Fastrac Gas Generator Design", AIAA 98-3366. NTRS 19980236008 (citation only).
- Ballard, R. O.; Olive, T.; Turner, J. E., "Development Status of the NASA MC-1 (Fastrac) Engine", AIAA 2000-3898. NTRS 20000097369, PDF opened. Seen: Table 3 hot-fire summary with Pc, turbine inlet temperature, MR for selected tests normalised to design inlet conditions (MR flagged uncertain due to flowmeter problems). No transients, no turbopump data.
- Rocker, M.; Jones, P., "Characterization of Low-Frequency Combustion Stability of the Fastrac Engine", JANNAF 2002. NTRS 20020050537, PDF opened. Seen: Table 1 mainstage test results (tests 1-26: duration, dominant frequency, Pc, dPc/Pc); test-23 Pc time histories for start, mainstage and shutdown; feedline geometry of test stand 116; chamber dimensions.
- Assessment: the public record does not contain a Fastrac engine system data set (no start transient traces, no turbopump maps beyond the LOX pump water model). Do not plan on Fastrac as a system validation case.
- Missing capabilities: fuel-rich RP-1 GG (equilibrium is wrong for kerosene GG at MR ~0.3; need an empirical temperature/property table); TEA/pyrotechnic ignition.

### 2.11 SSME STS-104 shutdown pressure surge (rank 11)

- Citation: Greene, W. D.; Kynard, M. H., "Understanding and Resolution of the Block 2 SSME, STS-104 Engine Shutdown Pressure Surge In-Flight Anomaly", AIAA 2002-3581, 38th JPC, 2002. NTRS 20030005845.
- URL opened: https://ntrs.nasa.gov/api/citations/20030005845/downloads/20030005845.pdf
- Seen: STS-104 and STS-108 flight data for low-pressure fuel duct and 17-inch manifold pressure after shutdown (Figs. 1, 5, 9); LPFP/HPFP spindown traces as model inputs; MFV, pre-valve, disconnect and dump valve timing; lumped-volume model vs flight data; mitigation simulations (Fig. 7). No numerical volumes or coefficients.
- Use: a small, well-defined feed-system transient (trapped LH2 between closing valves, pump spindown, heat leak). Geometry must be estimated.

### 2.12 KARI engine and powerpack papers (rank 12)

- Cho, W. K.; Park, S. Y.; Moon, Y. W.; Nam, C. H.; Kim, C. W.; Seol, W. S., "Liquid Rocket Engine System of Korean Launch Vehicle" (Korean), J. Korean Soc. Propulsion Eng. 14(1):56-64, 2010. Opened https://koreascience.kr/article/JAKO201029335105966.pdf. Seen: 75 tf GG LOX/kerosene engine; startup simulation compared with a turbopump-GG coupled closed-loop test (30 tf hardware) for turbopump rpm and GG chamber pressure. Component maps referred to internal KARI reports.
- Lee, S.; Moon, I., "Simulator Development for Startup Analysis of Staged Combustion Cycle Engine Powerpack" (Korean), JKSPE 19(5):62-70, 2015. Opened https://koreascience.kr/article/JAKO201534168451616.pdf. Seen: ox-rich preburner (MR 60), single shaft, two-stage fuel pump, start turbine; no test data overlaid; no coefficients published.
- Lee, S.; Moon, I., "Startup Analysis of Staged Combustion Cycle Engine Powerpack" (Korean), JKSPE 20(3):1-8, 2016, DOI 10.6108/KSPE.2016.20.3.001. Opened https://koreascience.or.kr/article/JAKO201629453670009.page?lang=en. Monte Carlo sequence study; no test comparison stated.
- "Startup Analysis Simulator for a Small Staged Combustion-Cycle Engine Powerpack", J. Aerospace Engineering 30(6), 2017. Opened https://ascelibrary.com/doi/10.1061/%28ASCE%29AS.1943-5525.0000750; authors not shown on the page I could load. Abstract states validation against combustion tests of an 8 tf-class powerpack. Full text not accessed.
- Assessment: one usable start comparison (2010) and possibly the 2017 paper; low completeness.

### 2.13 CECE deep throttling (rank 13)

- Giuliano, V. J.; Leonard, T. G.; Adamski, W. M.; Kim, T. S., "CECE: A Deep Throttling Demonstrator Cryogenic Engine for NASA's Lunar Lander", MSFC-325, 2007. NTRS 20090028814 (citation opened).
- Giuliano, V. J.; Leonard, T. G.; Lyda, R. T.; Kim, T. S., "CECE: Expanding the Envelope of Deep Throttling Technology ...", M10-0844, 46th JPC 2010. Opened https://ntrs.nasa.gov/archive/nasa/casi.ntrs.nasa.gov/20100032918.pdf and https://ntrs.nasa.gov/api/citations/20100032962/downloads/20100032962.pdf. Text only, normalised thrust (104 % to 5.9 %), no figures of data in extract.
- Devine, M. K.; Hulka, J.; Adamski, W.; Brown, C., "Throttling Characteristics of the RL10 Derivative CECE - Demo 1.6 and 1.7 Test Results", JANNAF 2010, M10-0118. NTRS 20100022021 (citation). Abstract describes a 1 Hz fuel-side oscillation caused by subcritical hydrogen boiling in the cooling jacket, suppressed with reduced-area gas venturis.
- Use: qualitative target only (reproduce a low-frequency jacket boiling oscillation at deep throttle). Not a quantitative data set.

### 2.14 Apollo pressure-fed engine transients (rank 14)

- Boehnlein, J. J., "Generalized Propulsion System Model for NASA Manned Spacecraft Center", Rocketdyne R-8445, NASA CR-114915, 1971. NTRS 19710010698. Public. Opened https://ntrs.nasa.gov/citations/19710010698 and PDF.
- Seen: water-hammer feedline model with priming of empty lines and dissolved-gas evolution; comparisons with test data for Apollo Ascent Engine (test 006 run 1: fuel, oxidiser, chamber pressure) and Descent Engine start (LTA-5) and an Apollo 9 pressurant bubble event; appendices A and B contain the ascent and descent engine data packages (not visible in the extract).
- Use: feed system priming and water hammer, which fluidnet will need for any start from dry lines. Not turbomachinery.

### 2.15 LUMEN (DLR) (rank 15, watch list)

- Traudt, T. et al., "LUMEN: Liquid Upper Stage Demonstrator Engine - A Versatile Test Bed for Rocket Engine Components: Hot-Fire Test Results", IAC-24-C4.1.5.x86850, 2024. Opened https://elib.dlr.de/213229/1/IAC-24%2CC4%2C1%2C5%2Cx86850%20manuscript.pdf. Seen: one plot (Fig. 5) of combustor pressure and turbopump speeds for a start; nominal 60 bar, ROF 3.4, 25 kN; no tables, no maps.
- Dresia, K. et al., "Hot-Fire Testing and System Analysis of the LUMEN Liquid Upper Stage Demonstrator Engine", FAR 2025, https://elib.dlr.de/219029 (abstract): 38-78 bar throttling; EcosimPro model being validated.
- Gulczyński, M. T. et al., Aerospace 11(10):808, 2024, DOI 10.3390/aerospace11100808 (CC BY): turbine blade fatigue; not system data.
- Inferred: the referenced LUMEN component papers (turbopump off-design validation, cavitation, chamber ignition transients) may hold usable component data; check them later. Today the system data is insufficient.

---

## 3. Supporting references (methods, not data)

- Seymour, D. C.; Martin, M. A.; Nguyen, H. H.; Greene, W. D., "Transient Mathematical Modeling for Liquid Rocket Engine Systems: Methods, Capabilities, and Experience", 5th Int. Symp. Liquid Space Propulsion, 2003. NTRS 20040000363 (preprint) and 20050217133. Opened both citations and the 20050217133 PDF. Seen: SSME model-only plots of MCC Pc and HPFTP speed during start with degraded HPOTP (Figs. 3-4), statement that the SSME model is anchored to test data but no comparison shown; MC-1 valve "jerking" reproduced (no data). Useful for MSFC modelling practice (property tables, numerics), not for data.
- Binder, M., NASA CR-190786 (1993) and NASA CR-195478 / AIAA 95-2968 (1995): RL10A-3-3A models already used. Opened https://ntrs.nasa.gov/citations/19950017370 and https://ntrs.nasa.gov/api/citations/19950022693/downloads/19950022693.pdf. CR-195478 mentions RL10A-4 only as a later configuration; no RL10A-4 or RL10B-2 test data is cited. I found no public RL10A-4 or RL10B-2 data set.
- Di Matteo, F.; De Rosa, M.; Onofri, M., "Start-up transient simulation of a liquid rocket engine", AIAA 2011-6032, DOI 10.2514/6.2011-6032 (https://iris.uniroma1.it/handle/11573/405703). ESPSS model validated against the RL10 start, i.e. the same data fluidnet already uses. Useful as a cross-code comparison of the RL10 case.

---

## 4. Candidates investigated with no usable public data

- Integrated Powerhead Demonstrator (full-flow staged combustion). Guidos and Seymour, "Transient Simulation of the IPD Rocket Engine", 2003/2004 (NTRS 20040075665) is abstract only. Jones et al., "Integrated Powerhead Demonstration: Full Flow Cycle Development", AFRL-PR-ED-TP-1998-003 (https://apps.dtic.mil/sti/pdfs/ADA397910.pdf) has design points (20.7 MPa, MR 6.7, 1.11 MN) and planned tests, no test data. Stennis facility papers (NTRS 20040084662, 20050041816 abstract only; 20040129712 PDF opened) show only facility characterisation (feed loss curve, turbine drive control). No engine or turbopump performance data found.
- J-2X. Buzzell 2010 (NTRS 20100021053), Kynard 2011 (NTRS 20120014523): program overviews. Hulka et al. 2010 (M10-0078, NTRS 20100022010), Morgan et al. 2015 (M15-4338, NTRS 20150016306) and Morgan, Hulka, Casiano 2015 "Evaluation of Start Transient Oscillations with the J-2X Engine Gas Generator Assembly" (M15-4316, https://repository.exst.jaxa.jp/dspace/handle/a-is/553306): combustion stability only. No powerpack or engine system performance data found.
- J-2 (Saturn). AEDC altitude test reports exist with start transient data, e.g. Kunz, C. H., "Altitude Developmental Testing of the J-2 Rocket Engine in Propulsion Engine Test Cell (J-4) (Test J4-1801-16)", AEDC-TR-68-43, 1968 (https://apps.dtic.mil/sti/pdfs/AD0829984.pdf). The cover carries "Distribution authorized to DoD only" and the foreword states the information is embargoed under ITAR, while the DTIC record says approved for public release (1974 limitation change). Because the markings conflict and ITAR is explicitly stated, I recommend not using this series.
- Vulcain / Vulcain 2 / HM7B / Vinci / Aestus. ESPSS Phase 3 executive summary (https://nebula.esa.int/sites/default/files/neb_tec_study/2283/public/24383.pdf) mentions HM7B industrial evaluation cases with CNES data but publishes no data. A Pisa/core.ac.uk thesis on AESTUS in EcosimPro (https://core.ac.uk/download/14702006.pdf) validates against P2 bench water-hammer tests with tank and line data tables, but no engine data was visible. Koppel et al., EUCASS 2011, DOI 10.1051/eucass/201102743, covers satellite propulsion. Patrao, "System Analysis of a Staged Combustion Rocket Engine", Chalmers MSc 2013 (https://odr.chalmers.se/items/66f84799-c566-4de8-bfc6-6ca4d494f288/full) models the SSME from literature; its reference data source is not named in the abstract. No public European engine transient data set found.
- NPSS rocket models. Parker et al., NASA/TM-2004-212945 (turbofan only); Lavelle, NASA/TM-2003-211896 (COBRA, Titan stage 2, SSME NPSS models validated against ROCETS, not test). No test data.
- ROCETS TTBE model. Mason and Southwick, NASA CR-184099 (1991, NTRS 19910011919) and Southwick and Mason, NASA CR-183780 (1989, NTRS 19910015027): state lists and schematics of an SSME-based TTBE model; initial guesses from the SSME DTM; no numerical inputs or test comparisons in the extracted text.
- SSME linearised models. Duyar, Guo, Merrill, NASA TM-101982 (1989), TM-106007 (1991), TM-104421 (1991): identified against a nonlinear simulation, not test data.
- Advanced Expander Test Bed. NASA CR-187082 (1991), CR-189130 (1992, AD-A247789), CR-191049 (1993). Design and cycle-deck tables only; no engine test data. CR-191049 carries DTIC number AD-B171638L, a limited-distribution series; check before use.
- Other SSME items opened that are not system data: Childs, HPOTP rotordynamics (NASA CR-140455, NTRS 19740025890); Wang and Dumas nozzle CFD (NTRS 19950017014); Wyle dynamic pressure database (NTRS 19860014168); George, shutdown chugging (NTRS 19860015053); Bartrand, preburner chug thesis (NTRS 19880017841); Koelbl, instrumented HPOTP on engine 3001 (AIAA 93-1907, NTRS 19930065779, citation only, may contain measured internal pressures/temperatures and is worth a later look).

---

## 5. Recommended order

1. **RL10-IIB low-thrust modes (CR-174914, with CR-179487 and CR-182159).** Extend the existing RL10A-3-3A model with the oxidiser heat exchanger, two-phase oxygen injection and tank-head idle. Validate steady points against Tables E-2/E-3 first, then the THI-to-PI transition. This adds the heat exchanger element and low-pressure two-phase hydrogen without new combustion devices.
2. **Turbomachinery components: Mark 48-F (CR-159821), Mark 48-O (CR-159509), J-2 Mk 15-F (TN D-6824).** Test the pump and turbine elements in isolation: multi-stage pump maps, stall boundary, zero-NPSH idle, head loss with inlet vapour, reaction turbine efficiency at off-design. Add a pump two-phase-inlet degradation model if needed.
3. **OEPSS Vol. 8 STME gas generator system (CR-207443).** Build the fully specified GG network and compare start/cutoff and chamber-out transients code-to-code against Rocketdyne's DTM output. This adds the gas generator, spin start and a manifolded multi-element network at large scale, with complete inputs.
4. **SSME steady balance: Bradley low power (CR-205360) + Santi TTB-50 table + PBM data (CR-195253), with the SSME Orientation schematic as an internal cross-check only.** Build the staged-combustion model with two fuel-rich preburners, four shafts, boost pumps and the LPOTP hydraulic turbine, and calibrate unknown resistances, volumes and map scalings to steady points from 17 % to 109 % RPL. Requires the preburner, hydraulic turbine and multi-pump-shaft capabilities.
5. **SSME start transient (CR-205359).** With the calibrated model, use the measured OPOV/FPOV positions as inputs and compare MCC Pc, HPOTP discharge pressure, HPFTP speed and discharge pressure to 80 % RPL, including the reduced LOX inlet pressure cases. This is the most complex public engine-system transient comparison I found. Expect the priming and ignition timing of the preburners to dominate the error.

Optional additions after step 5: obtain the LE-7/LE-7A and LE-5 papers manually (section 2.9) for a second staged-combustion start; use the nuclear rocket cold-flow bootstrap tests (2.8) and the STS-104 shutdown surge (2.11) as smaller targeted transient checks.

## 6. Restrictions summary

- NTRS documents cited above are listed as "Public" with "Work of the US Gov. Public Use Permitted" unless noted. AIAA/ASCE/JSASS/KSPE papers carry publisher copyright; digitizing for internal validation is fine, republishing figures is not.
- SSME Orientation BC98-04: "BOEING PROPRIETARY", training use only. Internal cross-check only.
- J-2 AEDC altitude test reports: explicit ITAR embargo statement on the document. Avoid.
- AETB CR-191049: DTIC AD-B (limited) number. Check before use.
- No ITAR or export marking was found in the extracted text of CR-174914, CR-179487, CR-205359, CR-205360, CR-207443, CR-159821, CR-159509, TN D-6824 or TM X-1917; this should be confirmed on the cover pages of the originals.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
