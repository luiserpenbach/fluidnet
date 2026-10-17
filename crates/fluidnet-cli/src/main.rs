//! fluidnet command-line runner.
//!
//!   fluidnet run MODEL.yaml [--steady] [--out FILE.csv|FILE.json] [--fluids DIR] [--print PATTERN]
//!   fluidnet check MODEL.yaml          compile only, list nodes, branches and states
//!   fluidnet catalog                   print the component catalog as JSON
//!   fluidnet props FLUID P_Pa T_K      property lookup

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::Instant;

use fluidnet_core::{catalog, run_steady, run_transient, FluidLibrary, Model, Network};

fn find_fluids(explicit: Option<&str>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(PathBuf::from(p));
    }
    if let Ok(p) = std::env::var("FLUIDNET_FLUIDS") {
        return Some(PathBuf::from(p));
    }
    let mut cands = vec![PathBuf::from("fluids")];
    if let Ok(exe) = std::env::current_exe() {
        let mut d = exe.parent().map(Path::to_path_buf);
        for _ in 0..4 {
            if let Some(dir) = d {
                cands.push(dir.join("fluids"));
                d = dir.parent().map(Path::to_path_buf);
            }
        }
    }
    cands.into_iter().find(|p| p.join("N2.fntb").exists())
}

fn usage() -> ExitCode {
    eprintln!(
        "usage:\n  fluidnet run MODEL.yaml [--steady] [--out FILE.csv|.json] [--fluids DIR] [--print SUBSTR]\n  fluidnet check MODEL.yaml\n  fluidnet catalog\n  fluidnet props FLUID P[Pa] T[K]"
    );
    ExitCode::from(2)
}

fn fmt_si(v: f64, unit: &str) -> String {
    match unit {
        "Pa" => format!("{:.4} bar", v / 1e5),
        "K" => format!("{:.2} K", v),
        "kg/s" => format!("{:.5} kg/s", v),
        _ => format!("{:.5e} {unit}", v),
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return usage();
    }
    let flag = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .map(|s| s.as_str())
    };
    let has = |name: &str| args.iter().any(|a| a == name);

    if args[0] == "catalog" {
        println!(
            "{}",
            serde_json::to_string_pretty(&catalog::catalog_json()).unwrap()
        );
        return ExitCode::SUCCESS;
    }

    let fluids_dir = match find_fluids(flag("--fluids")) {
        Some(d) => d,
        None => {
            eprintln!("cannot find fluid tables; pass --fluids DIR or set FLUIDNET_FLUIDS");
            return ExitCode::from(2);
        }
    };
    let lib = match FluidLibrary::load_dir(&fluids_dir) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };

    if args[0] == "propsbatch" {
        // stdin: "P h" per line; stdout: "rho T s a"
        use std::io::BufRead;
        let fl = lib.get(&args[1]).expect("fluid");
        for line in std::io::stdin().lock().lines().map_while(|l| l.ok()) {
            let v: Vec<f64> = line
                .split_whitespace()
                .filter_map(|x| x.parse().ok())
                .collect();
            if v.len() == 2 {
                let pr = fl.props_ph(v[0], v[1]);
                println!("{:.12e} {:.12e} {:.12e} {:.12e}", pr.rho, pr.t, pr.s, pr.a);
            }
        }
        return ExitCode::SUCCESS;
    }
    if args[0] == "props" {
        if args.len() < 4 {
            return usage();
        }
        let fl = match lib.get(&args[1]) {
            Some(f) => f,
            None => {
                eprintln!("unknown fluid {}", args[1]);
                return ExitCode::from(1);
            }
        };
        let p: f64 = args[2].parse().unwrap_or(1e5);
        let t: f64 = args[3].parse().unwrap_or(300.0);
        match fl.h_pt(p, t) {
            Ok(h) => println!("{:#?}", fl.props_ph(p, h)),
            Err(e) => eprintln!("{e}"),
        }
        return ExitCode::SUCCESS;
    }

    if args.len() < 2 {
        return usage();
    }
    let src = match std::fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{}: {e}", args[1]);
            return ExitCode::from(1);
        }
    };
    let model = match Model::parse(&src) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    let t0 = Instant::now();
    let net = match Network::compile(&model, &lib) {
        Ok(n) => n,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    for w in &net.warnings {
        eprintln!("warning: {w}");
    }

    if args[0] == "check" {
        println!(
            "{}: {} nodes, {} branches, {} states",
            model.name,
            net.nodes.len(),
            net.branches.len(),
            net.n_states
        );
        for n in &net.nodes {
            println!(
                "  node {:24} fluid {:5} {:?}",
                n.name,
                net.fluids[n.fluid].key,
                match &n.kind {
                    fluidnet_core::network::NodeKind::Volume { vol, .. } =>
                        format!("V = {:.4e} m3", vol),
                    _ => "boundary".to_string(),
                }
            );
        }
        for b in &net.branches {
            println!(
                "  branch {:22} {} -> {}",
                b.name, net.nodes[b.a].name, net.nodes[b.b].name
            );
        }
        return ExitCode::SUCCESS;
    }

    if args[0] == "bench" {
        use fluidnet_core::network::Eval;
        use fluidnet_core::solver::jacobian::JacobianBuilder;
        let n = net.n_states;
        let y = net.y0.clone();
        let mut ev = Eval::new(&net);
        let mut f = vec![0.0; n];
        let reps = 2000;
        let t1 = Instant::now();
        for _ in 0..reps {
            net.rhs(0.0, &y, &mut f, &mut ev).unwrap();
        }
        let rhs_us = t1.elapsed().as_secs_f64() / reps as f64 * 1e6;
        let (mut jb, mut ls) = JacobianBuilder::new(&net);
        let t2 = Instant::now();
        for _ in 0..50 {
            jb.build(&net, 0.0, &y, &mut ev).unwrap();
        }
        let jac_us = t2.elapsed().as_secs_f64() / 50.0 * 1e6;
        let t3 = Instant::now();
        for _ in 0..50 {
            assert!(ls.factor(1.0, 1e-4, &jb.vals));
        }
        let lu_us = t3.elapsed().as_secs_f64() / 50.0 * 1e6;
        let mut b = f.clone();
        let t4 = Instant::now();
        for _ in 0..reps {
            b.copy_from_slice(&f);
            ls.solve(&mut b);
        }
        let sol_us = t4.elapsed().as_secs_f64() / reps as f64 * 1e6;
        let mut pr = 0.0;
        let fl = &net.fluids[0];
        let t5 = Instant::now();
        for i in 0..200000 {
            pr += fl.props_ph(1e5 + i as f64, 1e5).rho;
        }
        let props_ns = t5.elapsed().as_secs_f64() / 200000.0 * 1e9;
        println!("{n} states ({} colors, nnz {}, {}): rhs {rhs_us:.1} us, jacobian {jac_us:.1} us, factor {lu_us:.1} us, solve {sol_us:.1} us, props_ph {props_ns:.0} ns ({pr:.0})",
            jb.n_colors(), ls.nnz(), if ls.is_sparse() {"sparse"} else {"dense"});
        return ExitCode::SUCCESS;
    }
    if args[0] == "jaccheck" {
        use fluidnet_core::network::Eval;
        use fluidnet_core::solver::jacobian::JacobianBuilder;
        let n = net.n_states;
        let y = if net.sim.start_from_steady {
            fluidnet_core::solver::steady::solve(&net, &net.y0, 0.0, 600)
                .map(|r| r.0)
                .unwrap_or(net.y0.clone())
        } else {
            net.y0.clone()
        };
        let t: f64 = flag("--t").and_then(|s| s.parse().ok()).unwrap_or(0.0);
        let mut ev = Eval::new(&net);
        let mut f0 = vec![0.0; n];
        net.rhs(t, &y, &mut f0, &mut ev).unwrap();
        let (mut jb, _ls) = JacobianBuilder::new(&net);
        jb.build(&net, t, &y, &mut ev).unwrap();
        let mut worst = (0.0, 0, 0, 0.0, 0.0);
        let mut fp = vec![0.0; n];
        for j in 0..n {
            let mut yp = y.clone();
            let d = 1e-7 * y[j].abs().max(net.typical(j));
            yp[j] += d;
            net.rhs(t, &yp, &mut fp, &mut ev).unwrap();
            for i in 0..n {
                let full = (fp[i] - f0[i]) / d;
                let jcv = jb.get(i, j);
                let err = (full - jcv).abs() / (full.abs().max(jcv.abs()).max(1e-30));
                if err > worst.0 && (full.abs() > 1e-6 || jcv.abs() > 1e-6) {
                    worst = (err, i, j, full, jcv);
                }
            }
        }
        if let Some(name) = flag("--probe") {
            let j = net
                .state_names
                .iter()
                .position(|s| s == name)
                .expect("state");
            println!("sweep of {name} (y = {:.9e}):", y[j]);
            for e in [-6i32, -4, -2, 0, 1, 2, 3, 4] {
                let d = 10f64.powi(e)
                    * if net.state_names[j].ends_with(".P") {
                        1.0
                    } else {
                        1e-3
                    };
                let mut yp = y.clone();
                yp[j] += d;
                net.rhs(t, &yp, &mut fp, &mut ev).unwrap();
                println!(
                    "  d = {d:9.1e}  df/dy for own row = {:.6e}   ({} row)",
                    (fp[j] - f0[j]) / d,
                    net.state_names[j]
                );
            }
        }
        println!("{} colors for {n} states; worst rel diff {:.3e} at d({})/d({}): full {:.4e} colored {:.4e}", jb.n_colors(), worst.0, net.state_names[worst.1], net.state_names[worst.2], worst.3, worst.4);
        return ExitCode::SUCCESS;
    }
    if args[0] != "run" {
        return usage();
    }
    let res = if has("--steady") {
        run_steady(&net, 0.0)
    } else {
        run_transient(&net, &mut |_| true)
    };
    let elapsed = t0.elapsed().as_secs_f64();
    let res = match res {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::from(1);
        }
    };
    eprintln!(
        "{} [{}]: {} states, solved in {:.1} ms",
        model.name,
        res.mode,
        res.n_states,
        elapsed * 1e3
    );
    if let Some(e) = &res.error {
        eprintln!(
            "  STOPPED EARLY at t = {:.6e} s: {e}",
            res.time.last().copied().unwrap_or(0.0)
        );
    }
    if let Some(s) = &res.stats {
        eprintln!(
            "  steps {} (rejected {}), rhs evals {}, jacobians {} ({} colors), LU {} ({}, nnz {}), h in [{:.2e}, {:.2e}] s",
            s.steps, s.rejected, s.rhs_evals, s.jacobians, s.jacobian_colors, s.factorizations, if s.sparse { "sparse" } else { "dense" }, s.nonzeros, s.h_min_used, s.h_max_used
        );
    }
    if let Some(s) = &res.steady {
        eprintln!(
            "  steady: {} iterations, {} rhs evals, converged {}",
            s.iterations, s.rhs_evals, s.converged
        );
    }
    let pat = flag("--print");
    for c in &res.channels {
        let show = match pat {
            Some(p) => p.split(',').any(|q| c.name.contains(q)),
            None => c.name.ends_with(".P") || c.name.ends_with(".mdot") || c.name.ends_with(".T"),
        };
        if show && !c.name.contains(".n") && !c.name.contains(".s") || show && pat.is_some() {
            if let Some(v) = c.data.last() {
                println!("  {:28} {}", c.name, fmt_si(*v, c.unit));
            }
        }
    }
    if let Some(out) = flag("--out") {
        let body = if out.ends_with(".json") {
            res.to_json()
        } else {
            res.to_csv()
        };
        if let Err(e) = std::fs::write(out, body) {
            eprintln!("{out}: {e}");
            return ExitCode::from(1);
        }
        eprintln!("  wrote {out}");
    }
    ExitCode::SUCCESS
}
