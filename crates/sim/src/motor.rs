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
    /// Loaded mass from the .eng header [kg] (equals the propellant mass when built with `new`). Informational: the flight model uses dry mass + propellant mass.
    pub total_mass: f64,
    /// (time [s], thrust [N]) starting at t = 0.
    pub curve: Vec<(f64, f64)>,
    cumulative: Vec<f64>,
}

impl Motor {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("cannot read motor file {}", path.display()))?;
        Self::parse(&text).with_context(|| format!("invalid motor file {}", path.display()))
    }

    /// Parse and validate a RASP .eng file. Errors name the (1-based) line.
    ///
    /// The header's total-mass field is parsed and checked (>= propellant mass) and stored in
    /// [`Motor::total_mass`]; the flight model uses the rocket's dry mass plus the propellant mass.
    pub fn parse(text: &str) -> Result<Self> {
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim())).filter(|(_, l)| !l.is_empty() && !l.starts_with(';'));
        let (hn, hl) = lines.next().context("empty .eng file")?;
        let header: Vec<&str> = hl.split_whitespace().collect();
        if header.len() < 7 {
            bail!("line {hn}: malformed .eng header: expected 7 fields");
        }
        let num = |s: &str, what: &str| -> Result<f64> {
            let v = s.parse::<f64>().with_context(|| format!("line {hn}: bad number '{s}' in .eng header ({what})"))?;
            if !v.is_finite() {
                bail!("line {hn}: {what} must be finite, got '{s}'");
            }
            Ok(v)
        };
        let diameter = num(header[1], "diameter")? / 1000.0;
        let length = num(header[2], "length")? / 1000.0;
        let propellant = num(header[4], "propellant mass")?;
        let total = num(header[5], "total mass")?;
        if diameter <= 0.0 || length <= 0.0 {
            bail!("line {hn}: diameter and length must be positive");
        }
        if propellant < 0.0 {
            bail!("line {hn}: propellant mass must be non-negative, got {propellant}");
        }
        if total < propellant {
            bail!("line {hn}: total mass {total} kg is smaller than the propellant mass {propellant} kg");
        }
        let mut curve = vec![(0.0, 0.0)];
        for (n, l) in lines {
            let v: Vec<f64> = l.split_whitespace().map(str::parse).collect::<Result<_, _>>().with_context(|| format!("line {n}: bad thrust line '{l}'"))?;
            if v.len() < 2 {
                bail!("line {n}: bad thrust line '{l}'");
            }
            let (t, f) = (v[0], v[1]);
            if !t.is_finite() || !f.is_finite() {
                bail!("line {n}: time and thrust must be finite, got '{l}'");
            }
            if t < 0.0 {
                bail!("line {n}: time must be non-negative, got {t}");
            }
            if f < 0.0 {
                bail!("line {n}: thrust must be non-negative, got {f}");
            }
            if t == 0.0 && curve.len() == 1 {
                curve[0].1 = f;
                continue;
            }
            if t <= curve.last().unwrap().0 {
                bail!("line {n}: thrust curve times must strictly increase, got {t} after {}", curve.last().unwrap().0);
            }
            curve.push((t, f));
        }
        if curve.len() < 2 {
            bail!("thrust curve has no points");
        }
        let mut m = Self::new(header[0].to_string(), diameter, length, propellant, curve);
        if !(m.total_impulse().is_finite() && m.total_impulse() > 0.0) {
            bail!("thrust curve must have a positive finite total impulse");
        }
        m.total_mass = total;
        Ok(m)
    }

    pub fn new(name: String, diameter: f64, length: f64, propellant_mass: f64, curve: Vec<(f64, f64)>) -> Self {
        let mut cumulative = vec![0.0];
        for w in curve.windows(2) {
            let i = 0.5 * (w[0].1 + w[1].1) * (w[1].0 - w[0].0);
            cumulative.push(cumulative.last().unwrap() + i);
        }
        Self { name, diameter, length, propellant_mass, total_mass: propellant_mass, curve, cumulative }
    }

    /// Copy with the thrust multiplied by `thrust_scale` and the time axis stretched by
    /// `time_scale` at constant total impulse of the stretch: F'(t) = thrust_scale F(t / time_scale) / time_scale.
    /// The total impulse is `thrust_scale` times the original; the propellant mass is unchanged.
    pub fn perturbed(&self, thrust_scale: f64, time_scale: f64) -> Self {
        let curve = self.curve.iter().map(|&(t, f)| (t * time_scale, f * thrust_scale / time_scale)).collect();
        let mut m = Self::new(self.name.clone(), self.diameter, self.length, self.propellant_mass, curve);
        m.total_mass = self.total_mass;
        m
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

    #[test]
    fn parse_rejects_invalid_files() {
        let ok = "X 54 400 0 1.0 2.0 T\n0.0 0\n0.1 100\n1.0 0\n";
        let m = Motor::parse(ok).unwrap();
        assert_eq!(m.total_mass, 2.0);
        let bad = |t: &str, needle: &str| {
            let e = format!("{:#}", Motor::parse(t).unwrap_err());
            assert!(e.contains(needle), "{e}");
        };
        bad("X 54 400 0 -1.0 2.0 T\n0 0\n1 10\n", "line 1: propellant mass");
        bad("X 54 400 0 1.0 0.5 T\n0 0\n1 10\n", "total mass");
        bad("X 0 400 0 1.0 2.0 T\n0 0\n1 10\n", "positive");
        bad("X nan 400 0 1.0 2.0 T\n0 0\n1 10\n", "finite");
        bad("; c\nX 54 400 0 1.0 2.0 T\n0 0\n0.5 -3\n1 0\n", "line 4: thrust must be non-negative");
        bad("X 54 400 0 1.0 2.0 T\n0 0\nNaN 5\n1 0\n", "line 3: time and thrust must be finite");
        bad("X 54 400 0 1.0 2.0 T\n0 0\n-1 5\n", "time must be non-negative");
        bad("X 54 400 0 1.0 2.0 T\n0 0\n1 5\n1 5\n", "strictly increase");
        bad("X 54 400 0 1.0 2.0 T\n0 0\n1 inf\n", "finite");
        bad("X 54 400 0 1.0 2.0 T\n0 0\n1 0\n2 0\n", "total impulse");
    }
}
