// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Butcher tableaux of explicit Runge-Kutta methods, shared by the vector-space
//! integrators and the Lie-group (RKMK) attitude integrator.

/// Explicit Runge-Kutta tableau: `a` is strictly lower triangular (row `i` has `i` entries).
#[derive(Debug, Clone)]
pub struct RkTableau {
    pub c: Vec<f64>,
    pub a: Vec<Vec<f64>>,
    pub b: Vec<f64>,
    /// Classical order of the propagating solution.
    pub order: u32,
}

impl RkTableau {
    pub fn stages(&self) -> usize {
        self.b.len()
    }

    /// Classical fourth-order Runge-Kutta.
    pub fn rk4() -> Self {
        RkTableau {
            c: vec![0.0, 0.5, 0.5, 1.0],
            a: vec![vec![], vec![0.5], vec![0.0, 0.5], vec![0.0, 0.0, 1.0]],
            b: vec![1.0 / 6.0, 1.0 / 3.0, 1.0 / 3.0, 1.0 / 6.0],
            order: 4,
        }
    }

    /// Check consistency: sum_j a_ij = c_i and sum b = 1.
    pub fn check(&self) -> Result<(), String> {
        let s = self.stages();
        if self.c.len() != s || self.a.len() != s {
            return Err("tableau dimensions do not match".into());
        }
        for (i, row) in self.a.iter().enumerate() {
            if row.len() != i {
                return Err(format!("row {i} of a must have {i} entries"));
            }
            let sum: f64 = row.iter().sum();
            if (sum - self.c[i]).abs() > 1e-12 {
                return Err(format!("row {i}: sum a = {sum} != c = {}", self.c[i]));
            }
        }
        let sb: f64 = self.b.iter().sum();
        if (sb - 1.0).abs() > 1e-12 {
            return Err(format!("sum b = {sb} != 1"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rk4_is_consistent() {
        RkTableau::rk4().check().unwrap();
    }
}
