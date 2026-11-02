# Release history

FNT-PLT-REG-001 · Rev A · Released · 2026-11-02 · Owner: Luis Erpenbach

| Version | Commit | Content | Evidence |
|---|---|---|---|
| 0.1.0 | 33d4f7f | Fluid network simulator: Rust core, (P, h) formulation, CoolProp tables, pipes, restrictions, valves, tanks, TR-BDF2, steady solver; CLI, WebAssembly, web UI | Analytic cases: blowdown, orifice, pipe friction, cavitating venturi, HEM flashing, water hammer, heated line, regulator |
| (0.1.x) | 8600773 | Turbomachinery (pump, turbine, shaft), regenerative walls, combustion chamber; expander-cycle examples | Pump hand calculation, chamber Pc and thrust, expander energy and torque balances, start against steady state |
| 0.2.0 | 005c65c | UI for engine components, Python bindings, documentation | – |
| 0.3.0 | 1a73b72 | RL10A-3-3A model and validation, pressure-actuated valves, robust steady cold start (flow guess, shaft continuation) | RL10 steady (generic maps) and start |
| (0.3.x) | 6fd6ad2 | RL10 data correction (TP3 fuel manifold ROCETS error −0.69 %) | – |
| 0.4.0 | ae553b4 | Enthalpy-based, contour-resolved hot-gas heat transfer; tabulated turbomachinery maps | RL10 turbine inlet temperature within 0.7 % |
| (0.4.x) | 5d0f895 | RL10 measured-speed statistics on predicted points only; documentation corrections | – |
| 0.5.0 | 5a52681 | P&W turbomachinery maps digitized from TM-107318; TCV range; pump efficiency speed correction; turbine flow-area map | RL10 free running 0.62 % mean / 2.05 % max (FNT-VAL-ANA-002) |
| (0.5.x) | 3eb65b3 | UI: table parameters with more than two columns | – |
| (0.5.x) | 36580d1 | Review fixes: pump speed correction held at table edges and applied only to absorbed power; TCV assumption and start-test bound documented | 30 tests pass |
| (unreleased) | – | Documentation set (this tree), component-reference generator, Notion export script | – |

Versions in brackets are commits between releases. From WP1 of the
[platform plan](platform-plan.md) on, every release gets a Git tag and a
changelog entry here.

## Revision history

| Rev | Date | Change |
|---|---|---|
| A | 2026-11-02 | First release |
