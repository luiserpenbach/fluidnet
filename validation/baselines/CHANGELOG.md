# Baseline changes

Every update of the reference-result baselines, with the reason.

| Date | Parent commit | Runs updated | Reason |
|---|---|---|---|
| 2026-11-10 | a4204c9 | cavitating_venturi.steady, cavitating_venturi.transient, expander_cycle.steady, expander_cycle.transient, expander_start.steady, expander_start.transient, gn2_blowdown.steady, gn2_blowdown.transient, lh2_heated_channel.steady, lh2_heated_channel.transient, lox_feed_startup.steady, lox_feed_startup.transient, regulated_pressurization.transient, rl10a33a.steady, rl10a33a.transient, rl10a33a_start.transient, water_hammer.steady, water_hammer.transient, t_actuated_valve.steady, t_actuated_valve.transient, t_blowdown_n2.steady, t_blowdown_n2.transient, t_cav_venturi.steady, t_cav_venturi.transient, t_chamber_pressure_fed.steady, t_chamber_pressure_fed.transient, t_heated_pipe.steady, t_ln2_flash.steady, t_ln2_flash.transient, t_pump_design.steady, t_pump_design.transient, t_regulator.steady, t_regulator.transient, t_water_hammer.steady, t_water_hammer.transient, t_water_orifice.steady, t_water_orifice.transient, t_water_pipe.steady, t_water_pipe.transient | Initial baselines at v0.5.0 (36580d1 behaviour, code formatted with rustfmt) |
| 2026-11-10 | 123eed2 | t_pump_design.transient | OI-020: pump stub volume now uses the pump design flow (was always 1e-5 m2 area); pump.out/in node dynamics change, steady state unchanged |
| 2026-11-10 | 123eed2 | t_heated_pipe_boundaries.steady, t_heated_pipe_boundaries.transient | New test model for OI-021 (end-wall heat at boundaries) |
| 2026-11-10 | 08ad737 | gn2_blowdown.steady, regulated_pressurization.steady, t_blowdown_n2.steady | v0.5.1: tanks are held as reservoirs in steady-state solves (gn2_blowdown, t_blowdown_n2 steady; regulated_pressurization steady added) |
