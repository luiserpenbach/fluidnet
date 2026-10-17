//! Linear systems M = a I - b J for the implicit solvers, with J stored on a
//! fixed sparsity pattern. Small systems use dense LU; larger ones use faer's
//! sparse LU (partial pivoting, fill-reducing ordering), reusing the symbolic
//! analysis because the pattern never changes during a run.

use faer::prelude::*;
use faer::sparse::linalg::solvers::{Lu as SpLu, SymbolicLu};
use faer::sparse::{SparseColMat, Triplet};
use faer::MatMut;

use crate::solver::linalg::Lu;

/// Below this size dense LU is faster than sparse bookkeeping.
const DENSE_MAX: usize = 48;

pub struct LinSys {
    n: usize,
    /// (row, col) of each structural entry of J; the diagonal is always present.
    entries: Vec<(usize, usize)>,
    diag: Vec<usize>,
    dense: Option<Lu>,
    sym: Option<SymbolicLu<usize>>,
    sparse: Option<SpLu<usize, f64>>,
    use_sparse: bool,
    trip: Vec<Triplet<usize, usize, f64>>,
}

impl LinSys {
    /// `cols[j]` lists the rows depending on column j.
    pub fn new(n: usize, cols: &[Vec<usize>]) -> (LinSys, Vec<(usize, usize)>) {
        let mut entries = Vec::new();
        let mut diag = vec![usize::MAX; n];
        for (j, rows) in cols.iter().enumerate() {
            let mut has_diag = false;
            for &r in rows {
                if r == j {
                    has_diag = true;
                }
            }
            if !has_diag {
                diag[j] = entries.len();
                entries.push((j, j));
            }
            for &r in rows {
                if r == j {
                    diag[j] = entries.len();
                }
                entries.push((r, j));
            }
        }
        let use_sparse = n > DENSE_MAX;
        let ls = LinSys {
            n,
            entries: entries.clone(),
            diag,
            dense: None,
            sym: None,
            sparse: None,
            use_sparse,
            trip: Vec::with_capacity(entries.len()),
        };
        (ls, entries)
    }

    pub fn is_sparse(&self) -> bool {
        self.use_sparse
    }

    pub fn nnz(&self) -> usize {
        self.entries.len()
    }

    /// Factor M = a I - b J, with `jac` aligned to the entry list. Returns false if singular.
    pub fn factor(&mut self, a: f64, b: f64, jac: &[f64]) -> bool {
        let n = self.n;
        self.dense = None;
        self.sparse = None;
        if !self.use_sparse {
            let mut m = vec![0.0; n * n];
            for (k, &(r, c)) in self.entries.iter().enumerate() {
                m[r * n + c] -= b * jac[k];
            }
            for i in 0..n {
                m[i * n + i] += a;
            }
            self.dense = Lu::factor(n, &m);
            return self.dense.is_some();
        }
        self.trip.clear();
        for (k, &(r, c)) in self.entries.iter().enumerate() {
            let mut v = -b * jac[k];
            if self.diag[c] == k {
                v += a;
            }
            self.trip.push(Triplet::new(r, c, v));
        }
        let m = match SparseColMat::<usize, f64>::try_new_from_triplets(n, n, &self.trip) {
            Ok(m) => m,
            Err(_) => return false,
        };
        if self.sym.is_none() {
            match SymbolicLu::try_new(m.symbolic()) {
                Ok(s) => self.sym = Some(s),
                Err(_) => return false,
            }
        }
        match SpLu::try_new_with_symbolic(self.sym.clone().unwrap(), m.as_ref()) {
            Ok(lu) => {
                self.sparse = Some(lu);
                true
            }
            Err(_) => false,
        }
    }

    pub fn is_factored(&self) -> bool {
        self.dense.is_some() || self.sparse.is_some()
    }

    /// x <- M^-1 x
    pub fn solve(&self, x: &mut [f64]) {
        if let Some(lu) = &self.dense {
            lu.solve(x);
        } else if let Some(lu) = &self.sparse {
            let n = self.n;
            let view = MatMut::from_column_major_slice_mut(x, n, 1);
            lu.solve_in_place(view);
        }
    }
}
