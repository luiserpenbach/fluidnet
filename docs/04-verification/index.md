# 04 Verification and Validation

Evidence that fluidnet computes what it claims: verification against analytic
solutions and reference codes, validation against engine test data, and the
register of which physical model is covered by which case.

> **Status (v0.5.0):** all analytic and conservation checks pass
> (`cargo test --release`: 14 unit tests, 16 validation tests). RL10A-3-3A
> steady state: mean 0.62 %, largest 2.05 % at four test points away from the
> calibration point. Start transient 0.3 s faster than the fastest measured
> start. Open point: calibrated turbine efficiency 6 % above the P&W map.

## Analyses

- [Verification against analytic and reference solutions](analytic-cases.md) — FNT-VAL-ANA-001. Orifice, pipe friction, cavitation, flashing, water hammer, heated line, regulator, pump, chamber, expander balances; unit tests.
- [RL10A-3-3A validation against test data](rl10.md) — FNT-VAL-ANA-002. Calibration at TP1, prediction at TP2–5 against CR-190786, comparison with ROCETS, start transient.
- [Digitization of the RL10A-3-3A turbomachinery maps](rl10-maps.md) — FNT-VAL-ANA-003. P&W pump and turbine maps from TM-107318.

## Registers

- [Validation matrix](validation-matrix.md) — FNT-VAL-REG-001. Every physical model with its verification and validation evidence and the gaps.
- [Literature validation data: engines and turbomachinery](data-engines.md) — FNT-VAL-REG-002. RL10-IIB, SSME, OEPSS gas-generator module, Mark 48 and J-2 turbopumps and further sets, ranked, with access and restriction notes.
- [Literature validation data: fluid systems and thermal phenomena](data-fluid-systems.md) — FNT-VAL-REG-003. Water hammer, priming, chill-down, tank pressurisation, N2O, flashing flow, venturis, blowdown, code benchmarks.

## How validation is run

| What | Command | Time |
|---|---|---|
| Analytic and regression suite | `cargo test --release` | about 2 min |
| RL10 steady validation and comparison cases | `python validation/rl10/rl10.py [all\|generic\|nominal_tcv\|frozen\|headcurve]` | about 2 min per case |
| RL10 start | `python validation/rl10/start.py` | under 1 min |
| Map digitization | see [FNT-VAL-ANA-003](rl10-maps.md) | under 1 min |

## Open items

See [open items](../01-project/open-items.md), category Validation.
