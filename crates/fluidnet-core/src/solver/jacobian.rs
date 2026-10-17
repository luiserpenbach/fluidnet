//! Finite-difference Jacobian on the network sparsity pattern, with column
//! colouring (structurally independent columns perturbed together).

use crate::error::Result;
use crate::network::{Eval, Network};
use crate::solver::linsys::LinSys;

pub struct JacobianBuilder {
    n: usize,
    /// per column: (row, entry index into `vals`)
    col_entries: Vec<Vec<(usize, usize)>>,
    colors: Vec<Vec<usize>>,
    typical: Vec<f64>,
    yp: Vec<f64>,
    fp: Vec<f64>,
    fb: Vec<f64>,
    /// Jacobian values aligned with the linear system's entry list
    pub vals: Vec<f64>,
}

impl JacobianBuilder {
    /// Returns the builder and a matching (unfactored) linear system.
    pub fn new(net: &Network) -> (Self, LinSys) {
        let n = net.n_states;
        let mut cols = net.sparsity();
        // the iteration matrix always has a diagonal; include it in the colouring
        for (j, c) in cols.iter_mut().enumerate() {
            if !c.contains(&j) {
                c.push(j);
            }
        }
        let (ls, entries) = LinSys::new(n, &cols);
        let mut col_entries: Vec<Vec<(usize, usize)>> = vec![Vec::new(); n];
        for (k, &(r, c)) in entries.iter().enumerate() {
            col_entries[c].push((r, k));
        }
        let mut row_cols: Vec<Vec<usize>> = vec![Vec::new(); n];
        for (j, rows) in cols.iter().enumerate() {
            for &r in rows {
                row_cols[r].push(j);
            }
        }
        // greedy distance-2 colouring
        let mut color = vec![usize::MAX; n];
        let mut colors: Vec<Vec<usize>> = Vec::new();
        let mut forbidden = vec![usize::MAX; n + 1];
        for j in 0..n {
            for &r in &cols[j] {
                for &c in &row_cols[r] {
                    if color[c] != usize::MAX {
                        forbidden[color[c]] = j;
                    }
                }
            }
            let mut c = 0;
            while forbidden[c] == j {
                c += 1;
            }
            color[j] = c;
            if c == colors.len() {
                colors.push(Vec::new());
            }
            colors[c].push(j);
        }
        let typical = (0..n).map(|i| net.typical(i)).collect();
        let nnz = entries.len();
        (
            JacobianBuilder {
                n,
                col_entries,
                colors,
                typical,
                yp: vec![0.0; n],
                fp: vec![0.0; n],
                fb: vec![0.0; n],
                vals: vec![0.0; nnz],
            },
            ls,
        )
    }

    pub fn n_colors(&self) -> usize {
        self.colors.len()
    }

    /// Value of J[r][c] (testing / diagnostics).
    pub fn get(&self, r: usize, c: usize) -> f64 {
        self.col_entries[c]
            .iter()
            .find(|e| e.0 == r)
            .map(|e| self.vals[e.1])
            .unwrap_or(0.0)
    }

    /// Build J at (t, y). The base value f(t, y) is always evaluated here: a
    /// derived f (e.g. from an implicit stage) differs from f(y) by
    /// O(tolerance / h), which divided by the tiny perturbation would corrupt J.
    pub fn build(&mut self, net: &Network, t: f64, y: &[f64], ev: &mut Eval) -> Result<usize> {
        let _ = self.n;
        net.rhs(t, y, &mut self.fb, ev)?;
        let mut evals = 1;
        for group in &self.colors {
            self.yp.copy_from_slice(y);
            for &j in group {
                let d = 1e-7 * y[j].abs().max(self.typical[j]);
                self.yp[j] = y[j] + d;
            }
            net.rhs(t, &self.yp, &mut self.fp, ev)?;
            evals += 1;
            for &j in group {
                let d = self.yp[j] - y[j];
                for &(r, k) in &self.col_entries[j] {
                    self.vals[k] = (self.fp[r] - self.fb[r]) / d;
                }
            }
        }
        Ok(evals)
    }
}
