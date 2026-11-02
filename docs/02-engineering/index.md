# 02 Engineering

This section describes how fluidnet is built and which physical models it
solves. Each page is a design description: it states the equations as they are
implemented, the numerical treatment, the assumptions and limits, where the
model lives in the code and how it is verified. A new team member who wants to
understand or change the solver should read the architecture page first, then
the network formulation and solver pages, and then the component model that
is being changed.

> **Status.** All pages are released at Rev A and checked against the code of
> fluidnet v0.5.0 (2026-11-02). Where the code and the earlier summaries in
> `README.md` and the former `docs/ARCHITECTURE.md` differ, these pages follow
> the code.
> Points that could not be settled from the code are listed under "Open items"
> on each page.

## Design descriptions

- [Software architecture](architecture.md) (FNT-ARC-DOC-001): crates, data flow from model file to results, component catalog, units, errors, result transport.
- [Network formulation](network-formulation.md) (FNT-SLV-DOC-001): nodes and branches, state variables, conservation equations, initial conditions and the Jacobian sparsity pattern.
- [Transient and steady-state solvers](solvers.md) (FNT-SLV-DOC-002): TR-BDF2 integrator, finite-difference Jacobian, linear algebra, steady-state solution sequence.
- [Fluid property tables](fluid-properties.md) (FNT-FLD-DOC-001): CoolProp table generation, file format and the saturation-aware (P, h) lookup.
- [Flow elements](flow-elements.md) (FNT-CMP-DOC-001): pipes, orifices and venturis, valves, check valves, regulators, losses, mass-flow sources, boundaries, tanks and junctions.
- [Pumps, turbines and shafts](turbomachinery.md) (FNT-TRB-DOC-001): generic and tabulated pump maps, impulse turbine, nozzle area map, shaft torque balance.
- [Heat transfer and walls](heat-transfer.md) (FNT-HTX-DOC-001): prescribed-temperature and heat-flux walls, walls with heat capacity, coolant-side and Bartz hot-gas-side heat transfer.
- [Combustion chamber](combustion.md) (FNT-CMB-DOC-001): chamber states, equilibrium tables, injection-enthalpy correction, ignition, throat flow and thrust.

## Open items

Open items found while writing these pages are listed at the end of each page
and collected in the project register, [Open items](../01-project/open-items.md).
