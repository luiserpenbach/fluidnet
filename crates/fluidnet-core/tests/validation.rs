//! Validation against analytic solutions and CoolProp references.
//! Reference values in comments were computed with CoolProp 8.0 (see
//! docs/VALIDATION.md); run with `cargo test --release`.

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
        // a system that is closed at t = 0 (tanks behind closed valves) has no unique steady state
        if !rel_path.contains("regulated_pressurization") {
            run(&rel_path, Mode::Steady);
        }
    }
}
