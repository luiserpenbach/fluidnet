# 06 References

FNT-REF-REG-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

Literature and data sources that fluidnet's models, data and validation rest
on. Where a page uses a source, it cites it by the short key in the first
column.

## Engine data and validation

| Key | Reference | Used for |
|---|---|---|
| TM-107318 | Binder, M., Tomsik, T., Veres, J. P., *RL10A-3-3A Rocket Engine Modeling Project*, NASA TM-107318, 1997. https://ntrs.nasa.gov/citations/19970010379 | RL10 geometry, component data, typical operating point, P&W pump and turbine maps |
| CR-190786 | Binder, M., *An RL10A-3-3A Rocket Engine Model Using the Rocket Engine Transient Simulator (ROCETS) Software*, NASA CR-190786, 1993. https://ntrs.nasa.gov/citations/19950017370 | Test points TP1–TP5, ROCETS comparison |
| CR-195478 | NASA CR-195478 (RL10A-3-3A start transient data, table 2) | Nine measured start transients |

## Physical models

| Key | Reference | Used for |
|---|---|---|
| Churchill 1977 | Churchill, S. W., "Friction-factor equation spans all fluid-flow regimes", *Chemical Engineering* 84(24), 1977 | Pipe friction factor |
| Colebrook 1939 | Colebrook, C. F., "Turbulent flow in pipes …", *J. Inst. Civil Eng.* 11, 1939 | Verification of the friction factor |
| Leung 1986/1996 | Leung, J. C., "A generalized correlation for one-component homogeneous equilibrium flashing choked flow", *AIChE J.* 32(10), 1986; Leung, J. C., "Easily size relief devices and piping for two-phase flow", *Chem. Eng. Prog.*, 1996 | Omega method for two-phase and subcooled flashing flow |
| API 520 | API Standard 520 Part I, Annex C (two-phase sizing, two-point omega) | ω from a 90 % pressure flash |
| Gnielinski 1976 | Gnielinski, V., "New equations for heat and mass transfer in turbulent pipe and channel flow", *Int. Chem. Eng.* 16, 1976 | Coolant-side convection |
| Bartz 1957 | Bartz, D. R., "A simple equation for rapid estimation of rocket nozzle convective heat transfer coefficients", *Jet Propulsion* 27, 1957 | Hot-gas convection |
| Wylie & Streeter | Wylie, E. B., Streeter, V. L., *Fluid Transients in Systems*, Prentice Hall, 1993 | Water hammer (Joukowsky, wave period, column separation) |
| Debye | Debye model of the specific heat of solids (any solid-state physics text, e.g. Kittel, *Introduction to Solid State Physics*) | Wall heat capacity at cryogenic temperatures |
| Dyer 2007 | Dyer, J., Zilliac, G., et al., "Modeling feed system flow physics for self-pressurizing propellants", AIAA 2007-5702 | Planned non-equilibrium two-phase option (WP5) |
| Zimmerman 2013 | Zimmerman, J. E., Waxman, B. S., Cantwell, B. J., Zilliac, G. G., "Review and evaluation of models for self-pressurizing propellant tank dynamics", AIAA 2013-4045 | Planned N2O tank validation (WP5) |
| Suter | Four-quadrant pump characteristics in the Suter representation (Wylie & Streeter, chapter on pumps) | Planned four-quadrant pump maps (WP6) |

## Numerical methods

| Key | Reference | Used for |
|---|---|---|
| Bank 1985 | Bank, R. E., et al., "Transient simulation of silicon devices and circuits", *IEEE Trans. CAD* 4(4), 1985 | TR-BDF2 |
| Hosea & Shampine 1996 | Hosea, M. E., Shampine, L. F., "Analysis and implementation of TR-BDF2", *Appl. Numer. Math.* 20, 1996 | TR-BDF2 error estimate and step control |
| Hairer & Wanner | Hairer, E., Wanner, G., *Solving Ordinary Differential Equations II: Stiff and Differential-Algebraic Problems*, Springer, 1996 | Newton convergence-rate estimate |
| Curtis, Powell, Reid 1974 | Curtis, A. R., Powell, M. J. D., Reid, J. K., "On the estimation of sparse Jacobian matrices", *IMA J. Appl. Math.* 13, 1974 | Column coloring for finite-difference Jacobians |
| Kelley & Keyes 1998 | Kelley, C. T., Keyes, D. E., "Convergence analysis of pseudo-transient continuation", *SIAM J. Numer. Anal.* 35, 1998 | Steady-state solver |

## Software and data

| Key | Reference | Used for |
|---|---|---|
| CoolProp | Bell, I. H., Wronski, J., Quoilin, S., Lemort, V., "Pure and pseudo-pure fluid thermophysical property evaluation and the open-source thermophysical property library CoolProp", *Ind. Eng. Chem. Res.* 53(6), 2014; version 8.0.0 | Fluid property tables |
| Cantera | Goodwin, D. G., et al., *Cantera: An object-oriented software toolkit for chemical kinetics, thermodynamics, and transport processes*, https://www.cantera.org; version 3.2.0 | Combustion tables |
| CEA | Gordon, S., McBride, B. J., *Computer Program for Calculation of Complex Chemical Equilibrium Compositions and Applications*, NASA RP-1311, 1994 | Verification of the combustion tables |
| faer | faer linear algebra library for Rust, https://github.com/sarah-quinones/faer-rs | Sparse LU |

## Standards and conventions

| Key | Reference | Used for |
|---|---|---|
| NASA-STD-7009A | NASA Standard for Models and Simulations, 2016 | Intent of the credibility levels in FNT-PLT-PLN-001 |
| Hopper Start Here | Hopper Workspace, Propulsion Team, "Start Here" (document IDs and page kinds) | Document conventions of this documentation |
| HOPPER-PID-STD-001 | Hopper P&ID tag naming standard | Component ids in project models |

See also the [nomenclature](nomenclature.md).

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
