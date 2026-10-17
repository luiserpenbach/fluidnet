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

Two-point omega against full HEM (CoolProp), restriction mass flux:

| Fluid, inlet | G_omega / G_HEM |
|---|---|
| N2, saturated, 5 bar | 1.008 |
| N2, saturated, 10 bar | 1.008 |
| O2, saturated, 5 bar | 1.007 |
| N2O, saturated, 50 bar | 1.008 |
| N2, 3 K subcooled, 5 bar | 0.994 |
| N2O, 5 K subcooled, 50 bar | 0.979 |
