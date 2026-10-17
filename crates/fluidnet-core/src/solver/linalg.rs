//! Dense LU factorisation with partial pivoting.

#[derive(Debug, Clone, Default)]
pub struct Lu {
    n: usize,
    a: Vec<f64>,
    piv: Vec<usize>,
}

impl Lu {
    /// Factor the row-major n x n matrix `m`. Returns None if singular.
    pub fn factor(n: usize, m: &[f64]) -> Option<Lu> {
        let mut a = m.to_vec();
        let mut piv: Vec<usize> = (0..n).collect();
        for k in 0..n {
            let mut p = k;
            let mut best = a[k * n + k].abs();
            for i in k + 1..n {
                let v = a[i * n + k].abs();
                if v > best {
                    best = v;
                    p = i;
                }
            }
            if best == 0.0 || !best.is_finite() {
                return None;
            }
            if p != k {
                for j in 0..n {
                    a.swap(k * n + j, p * n + j);
                }
                piv.swap(k, p);
            }
            let inv = 1.0 / a[k * n + k];
            for i in k + 1..n {
                let f = a[i * n + k] * inv;
                if f != 0.0 {
                    a[i * n + k] = f;
                    let (top, bot) = a.split_at_mut(i * n);
                    let rk = &top[k * n + k + 1..k * n + n];
                    let ri = &mut bot[k + 1..n];
                    for (x, y) in ri.iter_mut().zip(rk) {
                        *x -= f * y;
                    }
                } else {
                    a[i * n + k] = 0.0;
                }
            }
        }
        Some(Lu { n, a, piv })
    }

    /// Solve in place: b <- A^-1 b.
    pub fn solve(&self, b: &mut [f64]) {
        let n = self.n;
        let mut x: Vec<f64> = self.piv.iter().map(|&p| b[p]).collect();
        for i in 0..n {
            let mut s = x[i];
            let row = &self.a[i * n..i * n + i];
            for (j, l) in row.iter().enumerate() {
                s -= l * x[j];
            }
            x[i] = s;
        }
        for i in (0..n).rev() {
            let mut s = x[i];
            for j in i + 1..n {
                s -= self.a[i * n + j] * x[j];
            }
            x[i] = s / self.a[i * n + i];
        }
        b.copy_from_slice(&x);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn solves() {
        let m = [0.0, 2.0, 1.0, 1.0, 1.0, 1.0, 2.0, 1.0, 3.0];
        let lu = Lu::factor(3, &m).unwrap();
        // zero leading pivot forces a row swap
        let mut b = [0.0; 3];
        let x = [1.0, 1.0, 3.0];
        for i in 0..3 {
            b[i] = (0..3).map(|j| m[i * 3 + j] * x[j]).sum();
        }
        lu.solve(&mut b);
        for i in 0..3 {
            assert!((b[i] - x[i]).abs() < 1e-12);
        }
    }
}
