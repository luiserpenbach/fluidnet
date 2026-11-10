//! Validation against analytic solutions and CoolProp references.
//! Reference values in comments were computed with CoolProp 8.0 (see
//! docs/04-verification/analytic-cases.md); run with `cargo test --release`.

use std::path::PathBuf;

use fluidnet_core::{run_model, FluidLibrary, Mode, Results};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn lib() -> FluidLibrary {
    FluidLibrary::load_dir(&root().join("fluids")).expect("fluid tables")
}

fn run(path: &str, mode: Mode) -> Results {
    let src = std::fs::read_to_string(root().join(path)).expect(path);
    let r = run_model(&src, &lib(), mode).unwrap_or_else(|e| panic!("{path}: {e}"));
    assert!(r.error.is_none(), "{path}: {:?}", r.error);
    r
}

fn rel(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs()
}

#[test]
fn n2_blowdown_matches_isentropic_ideal_gas() {
    let r = run("tests_models/blowdown_n2.yaml", Mode::Transient);
    let (g, rgas, t0, p0, v, a): (f64, f64, f64, f64, f64, f64) = (
        1.4,
        8.314462618 / 0.0280134,
        300.0,
        10e5,
        0.05,
        std::f64::consts::PI * 0.003f64.powi(2) / 4.0,
    );
    let c = (g - 1.0) / 2.0 * a / v
        * (g * rgas * t0).sqrt()
        * (2.0 / (g + 1.0)).powf((g + 1.0) / (2.0 * (g - 1.0)));
    let p = r.channel("tank.P").unwrap();
    for (i, t) in r.time.iter().enumerate() {
        let pi = p0 * (1.0 + c * t).powf(-2.0 * g / (g - 1.0));
        assert!(rel(p[i], pi) < 2e-3, "t = {t}: {} vs {}", p[i], pi);
    }
}

#[test]
fn liquid_orifice_matches_bernoulli() {
    let r = run("tests_models/water_orifice.yaml", Mode::Steady);
    // rho(10 bar, 293.15 K) = 998.6247 kg/m3 -> 0.7 A sqrt(2 rho dP)
    assert!(rel(r.last("orf.mdot").unwrap(), 0.434338) < 2e-4);
}

#[test]
fn pipe_friction_matches_darcy_colebrook() {
    let r = run("tests_models/water_pipe.yaml", Mode::Steady);
    // Colebrook: 1.94516 kg/s; Churchill differs by ~0.4 % in this regime
    assert!(rel(r.last("pipe.mdot_in").unwrap(), 1.94516) < 0.01);
}

#[test]
fn cavitating_venturi_chokes_at_vapour_pressure() {
    let r = run("tests_models/cav_venturi.yaml", Mode::Steady);
    // 0.95 A sqrt(2 rho (P0 - Pv)), water 300 K
    assert!(rel(r.last("cv.mdot").unwrap(), 0.230795) < 1e-3);
    assert_eq!(r.last("cv.choked").unwrap(), 1.0);
}

#[test]
fn saturated_ln2_flashing_matches_hem() {
    let r = run("tests_models/ln2_flash.yaml", Mode::Steady);
    // HEM by isentropic integration with CoolProp: 0.02037 kg/s
    assert!(rel(r.last("orf.mdot").unwrap(), 0.02037) < 0.03);
    assert_eq!(r.last("orf.choked").unwrap(), 1.0);
}

#[test]
fn heated_pipe_closes_energy_balance() {
    let r = run("tests_models/heated_pipe.yaml", Mode::Steady);
    let dh = r.last("orf.in.h").unwrap() - r.last("src.h").unwrap();
    let q = r.last("pipe.Q").unwrap();
    assert!(rel(q, std::f64::consts::PI * 0.01 * 2.0 * 1e5) < 1e-9);
    assert!(rel(dh, q / 0.1) < 5e-3, "dh {dh} vs {}", q / 0.1);
}

#[test]
fn heated_pipe_between_boundaries_keeps_end_heat() {
    // The end wall elements sit on boundary nodes; their heat must go into the
    // neighbouring internal volumes, not into the boundaries (OI-021).
    let r = run("tests_models/heated_pipe_boundaries.yaml", Mode::Steady);
    let mdot = r.last("pipe.s10.mdot").unwrap();
    let dh = r.last("pipe.n9.h").unwrap() - r.last("src.h").unwrap();
    let q = std::f64::consts::PI * 0.01 * 2.0 * 1e5;
    assert!(rel(mdot * dh, q) < 1e-3, "mdot dh {} vs Q {q}", mdot * dh);
}

#[test]
fn closed_tank_steady_state_is_robust_to_small_perturbations() {
    // A tank behind a closed (leaking) valve has no exact steady state. The
    // solver must still return the quasi-steady state for any initial state,
    // not only for the one in the example (CI on macOS failed on rounding
    // differences before the tanks-held fallback).
    let base = std::fs::read_to_string(root().join("examples/gn2_blowdown.yaml")).unwrap();
    let lib = lib();
    for (p0, t0) in [
        ("299.99999", "290"),
        ("301", "290"),
        ("305", "290.001"),
        ("300.00001", "290.001"),
    ] {
        let src = base
            .replace("P0: 300 bar", &format!("P0: {p0} bar"))
            .replace("T0: 290 K", &format!("T0: {t0} K"));
        let r =
            run_model(&src, &lib, Mode::Steady).unwrap_or_else(|e| panic!("{p0} bar {t0} K: {e}"));
        let p_line = r.last("vent_valve.in.P").unwrap();
        let p_tank: f64 = p0.parse::<f64>().unwrap() * 1e5;
        assert!(rel(p_line, p_tank) < 1e-6, "{p0} bar: line {p_line} Pa");
    }
}

#[test]
fn model_files_are_validated_strictly() {
    let base = std::fs::read_to_string(root().join("examples/gn2_blowdown.yaml")).unwrap();
    let lib = lib();
    let err = |src: String| {
        run_model(&src, &lib, Mode::Steady)
            .err()
            .map(|e| e.to_string())
            .unwrap_or_default()
    };
    // misspelt parameter: error with the closest name
    let e = err(base.replace("cd: 0.8, opening", "cdd: 0.8, opening"));
    assert!(
        e.contains("unknown parameter 'cdd'") && e.contains("did you mean 'cd'"),
        "{e}"
    );
    // misspelt simulation key and unknown top-level key
    assert!(err(base.replace("t_end: 60", "t_ned: 60")).contains("t_ned"));
    assert!(err(format!("{base}\nextra: 1\n")).contains("extra"));
    // a dimensional unit on a dimensionless parameter
    assert!(
        err(base.replace("cd: 0.8, opening", "cd: 0.8 bar, opening")).contains("dimensionless")
    );
    // file written for a newer version
    assert!(err(format!("fluidnet: \"99.0\"\n{base}")).contains("Update fluidnet"));
    // accepted: version of this build, time units, a fluid given by its label
    let ok = format!("fluidnet: \"0.5\"\n{base}")
        .replace(
            "t_end: 60, output_dt: 0.1",
            "t_end: 1 min, output_dt: 100 ms",
        )
        .replace("fluid: N2, P: 1 bar", "fluid: Nitrogen, P: 1 bar");
    let r = run_model(&ok, &lib, Mode::Transient).unwrap();
    assert!((r.time.last().unwrap() - 60.0).abs() < 1e-9);
}

#[test]
fn water_hammer_matches_joukowsky() {
    let r = run("examples/water_hammer.yaml", Mode::Transient);
    let t = &r.time;
    let p = r.channel("valve.in.P").unwrap();
    let m = r.channel("valve.mdot").unwrap();
    let (rho, a, area) = (
        999.986,
        1488.8,
        std::f64::consts::PI * 0.05f64.powi(2) / 4.0,
    ); // CoolProp, 40 bar 293.15 K
    let v0 = m[0] / (rho * area);
    let plateau = p[0] + rho * a * v0;
    let two_l_a = 200.0 / a;
    let mut win: Vec<f64> = t
        .iter()
        .zip(p)
        .filter(|(tt, _)| **tt > 0.03 && **tt < 0.02 + two_l_a - 0.005)
        .map(|(_, p)| *p)
        .collect();
    win.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median = win[win.len() / 2];
    assert!(
        rel(median, plateau) < 0.02,
        "plateau {median} vs Joukowsky {plateau}"
    );
    // period from crossings of the initial pressure
    let cross: Vec<f64> = (1..p.len())
        .filter(|&k| (p[k - 1] - p[0]) * (p[k] - p[0]) < 0.0 && t[k] > 0.025)
        .map(|k| t[k])
        .collect();
    assert!(rel(cross[2] - cross[0], 400.0 / a) < 0.01);
}

#[test]
fn regulator_holds_set_pressure_within_band() {
    let r = run("tests_models/regulator.yaml", Mode::Steady);
    let p = r.last("plenum.P").unwrap();
    assert!(p > 29e5 && p < 32e5, "{p}");
}

#[test]
fn all_examples_run_steady_and_transient() {
    for f in std::fs::read_dir(root().join("examples"))
        .unwrap()
        .flatten()
    {
        let p = f.path();
        if p.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }
        let rel_path = format!("examples/{}", p.file_name().unwrap().to_string_lossy());
        run(&rel_path, Mode::Transient);
        // tanks are reservoirs in a steady solve, so closed systems solve too
        run(&rel_path, Mode::Steady);
    }
}

#[test]
fn pump_matches_affinity_map_hand_calculation() {
    // water pump at design speed against an orifice; reference solved with
    // the same map in Python (scipy brentq): 8.38489 kg/s, efficiency 0.6229
    let r = run("tests_models/pump_design.yaml", Mode::Steady);
    assert!(rel(r.last("pump.mdot").unwrap(), 8.38489) < 2e-4);
    assert!((r.last("pump.eff").unwrap() - 0.6229).abs() < 5e-4);
}

#[test]
fn chamber_pressure_and_thrust_match_tables() {
    // independent Python check of Pc = mdot c*/At and the ideal CF at eps 40
    let r = run("tests_models/chamber_pressure_fed.yaml", Mode::Steady);
    let pc = r.last("tca.Pc").unwrap();
    assert!(rel(pc, 45.4245e5) < 1e-4, "{pc}");
    let f = r.last("tca.thrust").unwrap();
    assert!(rel(f, 11132.5) < 1e-3, "{f}");
    assert!((r.last("tca.OF").unwrap() - 5.5165).abs() < 1e-3);
}

#[test]
fn expander_cycle_conserves_energy_and_torque() {
    let r = run("examples/expander_cycle.yaml", Mode::Steady);
    let g = |n: &str| r.last(n).unwrap_or_else(|| panic!("{n}"));
    // shaft: turbine torque = fuel pump + geared LOX pump + bearing friction
    let w = g("turbopump.speed");
    let resid =
        g("turbine.torque") - g("fuel_pump.torque") - 0.3 * g("lox_pump.torque") - 0.05 * w.tanh();
    assert!(
        resid.abs() < 1e-6 * g("turbine.torque"),
        "torque residual {resid}"
    );
    // jacket: enthalpy rise of the hydrogen = heat into the coolant (the first
    // wall element heats the inlet manifold, so start from the pump outlet)
    let m = g("jacket.mdot_in");
    let dh = g("turbine_inlet.h") - g("fuel_pump.out.h");
    assert!(
        rel(m * dh, g("jacket.Q")) < 1e-6,
        "jacket energy {} vs {}",
        m * dh,
        g("jacket.Q")
    );
    // pump: all shaft power ends up in the fluid
    let mp = g("fuel_pump.mdot");
    assert!(
        rel(
            mp * (g("fuel_pump.out.h") - g("lh2_tank.h")),
            g("fuel_pump.power")
        ) < 1e-6
    );
    // mass: chamber throat flow = fuel + oxidizer injection
    assert!(rel(g("tca.mdot"), g("tca.mdot_fuel") + g("tca.mdot_ox")) < 1e-9);
    // design point reached by the steady solver with a free shaft
    assert!(rel(g("tca.Pc"), 40e5) < 0.01 && rel(g("tca.OF"), 5.8) < 0.01);
}

#[test]
fn expander_start_reaches_the_steady_design_point() {
    let start = run("examples/expander_start.yaml", Mode::Transient);
    let steady = run("examples/expander_cycle.yaml", Mode::Steady);
    for ch in ["tca.Pc", "tca.thrust", "turbopump.speed", "tca.OF"] {
        let a = start.last(ch).unwrap();
        let b = steady.last(ch).unwrap();
        assert!(rel(a, b) < 0.003, "{ch}: start ends at {a}, steady {b}");
    }
}

#[test]
fn pressure_actuated_valves_follow_and_latch() {
    let r = run("tests_models/actuated_valve.yaml", Mode::Transient);
    let t = &r.time;
    let at = |name: &str, tt: f64| {
        let i = t.iter().position(|&x| x >= tt - 1e-9).unwrap();
        r.channel(name).unwrap()[i]
    };
    // closed below the set range, fully open above it
    assert!(at("v_free.opening", 0.2) < 1e-3);
    assert!(at("v_free.opening", 1.0) > 0.999);
    // on the way down the free valve closes again, the latching one stays open
    assert!(at("v_free.opening", 2.0) < 1e-3);
    assert!(at("v_latch.opening", 2.0) > 0.999);
    // between the set points the commanded position is linear in the signal;
    // on a ramp a first-order lag of 20 ms trails the command by 20 ms
    let p = at("upstream.P", 0.48);
    let cmd = ((p - 4e5) / 2e5).clamp(0.0, 1.0);
    assert!(
        (at("v_free.opening", 0.5) - cmd).abs() < 0.03,
        "{} vs {cmd}",
        at("v_free.opening", 0.5)
    );
}

/// RL10A-3-3A at test point 1 of NASA CR-190786 (the calibration point):
/// the calibrated model must reproduce the measured values it was fitted to,
/// and stay within the stated accuracy on the values it predicts.
#[test]
fn rl10a33a_reproduces_test_point_1() {
    let r = run("examples/rl10a33a.yaml", Mode::Steady);
    let g = |n: &str| r.last(n).unwrap();
    let psi = 6894.757293;
    let rank = 5.0 / 9.0;
    let fitted = [
        ("tca.Pc", 472.35 * psi),
        ("tca.thrust", 16603.0 * 4.4482216152605),
        ("tca.OF", 5.63),
        ("lox_pump.speed", 12697.0 * std::f64::consts::PI / 30.0),
        ("fuel_pump_discharge.P", 1061.40 * psi),
        ("venturi_inlet.P", 800.45 * psi),
        ("turbine_inlet.T", 439.32 * rank),
        ("fuel_manifold.P", 534.19 * psi),
        ("lox_plenum.P", 532.50 * psi),
    ];
    for (n, v) in fitted {
        assert!(rel(g(n), v) < 2e-3, "{n}: {} vs {v}", g(n));
    }
    // predicted at TP1: LOX pump discharge (published LOX pump design point) and inlet flows
    assert!(rel(g("lox_discharge.P"), 613.80 * psi) < 0.02);
    let gpm = 3.785411784e-3 / 60.0;
    assert!(rel(g("fuel_pump_1.mdot") / g("lh2_tank.rho"), 580.60 * gpm) < 0.015);
    assert!(rel(g("lox_pump.mdot") / g("lox_tank.rho"), 205.82 * gpm) < 0.015);
}

/// Start transient: time to accelerate (MES to Pc = 200 psia) against the
/// range measured on ground tests and flights (1.43 to 2.56 s, NASA CR-195478),
/// and the start ends on the steady test point 1 solution. With the P&W
/// turbine map and the TP1-calibrated turbine efficiency the model starts in
/// 1.14 s, faster than the fastest measured start (docs/04-verification/rl10.md); the
/// bound checks that it stays within 30 % of the measured range.
#[test]
fn rl10a33a_start_accelerates_to_mainstage() {
    let r = run("examples/rl10a33a_start.yaml", Mode::Transient);
    let psi = 6894.757293;
    let pc = r.channel("tca.Pc").unwrap();
    let i = pc
        .iter()
        .position(|&p| p >= 200.0 * psi)
        .expect("reaches 200 psia");
    let tta = r.time[i];
    assert!((1.0..=2.56).contains(&tta), "time to accelerate {tta}");
    assert!(rel(*pc.last().unwrap(), 472.35 * psi) < 0.005);
}
