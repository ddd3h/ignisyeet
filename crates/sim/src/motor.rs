// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Thrust curve in RASP (.eng) format.

use anyhow::{bail, Context, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Motor {
    pub name: String,
    /// Casing diameter [m].
    pub diameter: f64,
    /// Casing length [m].
    pub length: f64,
    pub propellant_mass: f64,
    /// (time [s], thrust [N]) starting at t = 0.
    pub curve: Vec<(f64, f64)>,
    cumulative: Vec<f64>,
}

impl Motor {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("cannot read motor file {}", path.display()))?;
        Self::parse(&text)
    }

    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with(';'));
        let header: Vec<&str> = lines.next().context("empty .eng file")?.split_whitespace().collect();
        if header.len() < 7 {
            bail!("malformed .eng header: expected 7 fields");
        }
        let num = |s: &str| s.parse::<f64>().with_context(|| format!("bad number '{s}' in .eng header"));
        let mut curve = vec![(0.0, 0.0)];
        for l in lines {
            let v: Vec<f64> = l.split_whitespace().map(str::parse).collect::<Result<_, _>>().with_context(|| format!("bad thrust line '{l}'"))?;
            if v.len() < 2 {
                bail!("bad thrust line '{l}'");
            }
            if v[0] <= curve.last().unwrap().0 {
                if v[0] == 0.0 {
                    curve[0].1 = v[1];
                    continue;
                }
                bail!("thrust curve times must increase");
            }
            curve.push((v[0], v[1]));
        }
        if curve.len() < 2 {
            bail!("thrust curve has no points");
        }
        Ok(Self::new(header[0].to_string(), num(header[1])? / 1000.0, num(header[2])? / 1000.0, num(header[4])?, curve))
    }

    pub fn new(name: String, diameter: f64, length: f64, propellant_mass: f64, curve: Vec<(f64, f64)>) -> Self {
        let mut cumulative = vec![0.0];
        for w in curve.windows(2) {
            let i = 0.5 * (w[0].1 + w[1].1) * (w[1].0 - w[0].0);
            cumulative.push(cumulative.last().unwrap() + i);
        }
        Self { name, diameter, length, propellant_mass, curve, cumulative }
    }

    /// Copy with the thrust multiplied by `thrust_scale` and the time axis stretched by
    /// `time_scale` at constant total impulse of the stretch: F'(t) = thrust_scale F(t / time_scale) / time_scale.
    /// The total impulse is `thrust_scale` times the original; the propellant mass is unchanged.
    pub fn perturbed(&self, thrust_scale: f64, time_scale: f64) -> Self {
        let curve = self.curve.iter().map(|&(t, f)| (t * time_scale, f * thrust_scale / time_scale)).collect();
        Self::new(self.name.clone(), self.diameter, self.length, self.propellant_mass, curve)
    }

    pub fn burn_time(&self) -> f64 {
        self.curve.last().unwrap().0
    }

    pub fn total_impulse(&self) -> f64 {
        *self.cumulative.last().unwrap()
    }

    fn segment(&self, t: f64) -> Option<(usize, f64)> {
        if t <= 0.0 || t >= self.burn_time() {
            return None;
        }
        let i = self.curve.partition_point(|p| p.0 <= t) - 1;
        let (t0, t1) = (self.curve[i].0, self.curve[i + 1].0);
        Some((i, (t - t0) / (t1 - t0)))
    }

    pub fn thrust(&self, t: f64) -> f64 {
        match self.segment(t) {
            Some((i, s)) => self.curve[i].1 + (self.curve[i + 1].1 - self.curve[i].1) * s,
            None => 0.0,
        }
    }

    /// Impulse delivered up to time t.
    pub fn impulse(&self, t: f64) -> f64 {
        match self.segment(t) {
            Some((i, s)) => {
                let (t0, t1) = (self.curve[i].0, self.curve[i + 1].0);
                let f0 = self.curve[i].1;
                let ft = self.thrust(t);
                self.cumulative[i] + 0.5 * (f0 + ft) * s * (t1 - t0)
            }
            None if t <= 0.0 => 0.0,
            None => self.total_impulse(),
        }
    }

    /// Remaining propellant mass, assuming consumption proportional to delivered impulse.
    pub fn propellant_left(&self, t: f64) -> f64 {
        let total = self.total_impulse();
        if total <= 0.0 {
            return 0.0;
        }
        self.propellant_mass * (1.0 - self.impulse(t) / total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_and_integrate() {
        let m = Motor::parse("; c\nX 54 400 0 1.0 2.0 Test\n0.0 0\n0.1 100\n1.0 100\n1.1 0\n").unwrap();
        assert!((m.diameter - 0.054).abs() < 1e-12);
        assert!((m.total_impulse() - 100.0).abs() < 1e-9);
        assert!((m.thrust(0.5) - 100.0).abs() < 1e-9);
        assert!((m.impulse(0.05) - 1.25).abs() < 1e-9);
        assert!((m.propellant_left(1.1) - 0.0).abs() < 1e-12);
        let p = m.perturbed(1.1, 1.5);
        assert!((p.burn_time() - 1.65).abs() < 1e-12 && (p.total_impulse() - 110.0).abs() < 1e-9);
        assert!((p.thrust(0.5 * 1.5) - 110.0 / 1.5).abs() < 1e-9 && p.propellant_mass == m.propellant_mass);
        assert!((m.propellant_left(0.55) - 0.5).abs() < 1e-9);
    }
}
