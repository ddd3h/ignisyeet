// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Thrust curve in RASP (.eng) format.

use anyhow::{bail, Context, Result};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct Motor {
    pub name: String,
    /// Casing diameter \[m\].
    pub diameter: f64,
    /// Casing length \[m\].
    pub length: f64,
    pub propellant_mass: f64,
    /// Loaded mass from the .eng header \[kg\] (equals the propellant mass when built with `new`). Informational: the flight model uses dry mass + propellant mass.
    pub total_mass: f64,
    /// (time \[s\], thrust \[N\]) starting at t = 0.
    pub curve: Vec<(f64, f64)>,
    cumulative: Vec<f64>,
}

impl Motor {
    pub fn load(path: &Path) -> Result<Self> {
        // No path context here: the caller names the file once ("cannot read motor file ...").
        let text = std::fs::read_to_string(path)?;
        Self::parse(&text)
    }

    /// Non-fatal oddities of the thrust curve, for the caller to show to the user.
    pub fn warnings(&self) -> Vec<String> {
        let mut w = Vec::new();
        let last = self.curve.last().unwrap().1;
        if self.curve.len() == 2 && self.curve[0].1 == 0.0 {
            w.push("[IY-S500] the thrust curve has a single point: the thrust ramps linearly from 0 N at t = 0".to_string());
        }
        if last != 0.0 {
            w.push(format!("[IY-S501] thrust curve ends at {last} N instead of 0 N: the motor is cut off abruptly"));
        }
        if self.burn_time() < 0.05 {
            w.push(format!("[IY-S502] burn time {:.3} s is shorter than 0.05 s: the motor is an impulse; check the time column", self.burn_time()));
        }
        w
    }

    /// Parse and validate a RASP .eng file. Errors name the (1-based) line.
    ///
    /// The header's total-mass field is parsed and checked (>= propellant mass) and stored in
    /// [`Motor::total_mass`]; the flight model uses the rocket's dry mass plus the propellant mass.
    pub fn parse(text: &str) -> Result<Self> {
        // A UTF-8 byte order mark (Excel "CSV UTF-8", some Windows editors) is not part of the header.
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let mut lines = text.lines().enumerate().map(|(i, l)| (i + 1, l.trim())).filter(|(_, l)| !l.is_empty() && !l.starts_with(';'));
        let (hn, hl) = lines.next().context("[IY-S001] empty .eng file")?;
        let header: Vec<&str> = hl.split_whitespace().collect();
        if header.len() < 7 {
            bail!("[IY-S003] line {hn}: malformed .eng header: expected 7 fields");
        }
        let num = |s: &str, what: &str| -> Result<f64> {
            let v = s.parse::<f64>().with_context(|| format!("[IY-S002] line {hn}: bad number '{s}' in .eng header ({what})"))?;
            if !v.is_finite() {
                bail!("[IY-S004] line {hn}: {what} must be finite, got '{s}'");
            }
            Ok(v)
        };
        let diameter = num(header[1], "diameter")? / 1000.0;
        let length = num(header[2], "length")? / 1000.0;
        let propellant = num(header[4], "propellant mass")?;
        let total = num(header[5], "total mass")?;
        if diameter <= 0.0 || length <= 0.0 {
            bail!("[IY-S005] line {hn}: diameter and length must be positive");
        }
        if propellant < 0.0 {
            bail!("[IY-S006] line {hn}: propellant mass must be non-negative, got {propellant}");
        }
        if total < propellant {
            bail!("[IY-S007] line {hn}: total mass {total} kg is smaller than the propellant mass {propellant} kg");
        }
        let mut curve = vec![(0.0, 0.0)];
        for (n, l) in lines {
            // A second header (name, then at least 6 more fields) is a second motor in the file.
            let fields: Vec<&str> = l.split_whitespace().collect();
            if fields.len() >= 7 && fields[0].parse::<f64>().is_err() {
                bail!("[IY-S008] line {n}: multiple motors in this file (second header '{l}'): keep one motor per file");
            }
            let v: Vec<f64> =
                l.split_whitespace().map(str::parse).collect::<Result<_, _>>().with_context(|| format!("[IY-S009] line {n}: bad thrust line '{l}'"))?;
            if v.len() < 2 {
                bail!("[IY-S009] line {n}: bad thrust line '{l}'");
            }
            let (t, f) = (v[0], v[1]);
            if !t.is_finite() || !f.is_finite() {
                bail!("[IY-S010] line {n}: time and thrust must be finite, got '{l}'");
            }
            if t < 0.0 {
                bail!("[IY-S011] line {n}: time must be non-negative, got {t}");
            }
            if f < 0.0 {
                bail!("[IY-S012] line {n}: thrust must be non-negative, got {f}");
            }
            if t == 0.0 && curve.len() == 1 {
                curve[0].1 = f;
                continue;
            }
            if t <= curve.last().unwrap().0 {
                bail!("[IY-S013] line {n}: thrust curve times must strictly increase, got {t} after {}", curve.last().unwrap().0);
            }
            curve.push((t, f));
        }
        if curve.len() < 2 {
            bail!("[IY-S014] thrust curve has no points");
        }
        let mut m = Self::new(header[0].to_string(), diameter, length, propellant, curve);
        if !(m.total_impulse().is_finite() && m.total_impulse() > 0.0) {
            bail!("[IY-S015] thrust curve must have a positive finite total impulse");
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

    /// End of the thrust \[s\]: the last thrust-curve knot after which the thrust stays zero (the
    /// curve end when it ends with thrust; a trailing zero-thrust stretch does not count).
    pub fn thrust_end(&self) -> f64 {
        let c = &self.curve;
        (1..c.len()).rev().find(|&i| c[i].1 > 0.0 || c[i - 1].1 > 0.0).map_or(c[0].0, |i| c[i].0)
    }

    /// First thrust-curve knot later than `t` (by more than 1e-9 s); None from the last knot on.
    pub fn next_knot(&self, t: f64) -> Option<f64> {
        self.curve.iter().map(|p| p.0).find(|&k| k > t + 1e-9)
    }

    /// Index `i` of the thrust-curve segment `[t_i, t_i+1]` that the motor is in just after `t`
    /// (a knot within 1e-9 s counts as passed, as in [`next_knot`](Self::next_knot)); None from
    /// burnout on.
    pub fn segment_after(&self, t: f64) -> Option<usize> {
        if t + 1e-9 >= self.burn_time() {
            return None;
        }
        Some(self.curve.partition_point(|p| p.0 <= t + 1e-9).saturating_sub(1).min(self.curve.len() - 2))
    }

    /// Thrust at `t` on the linear piece of segment `seg` (the piece is used slightly beyond its
    /// knots too, clamped at 0), 0 for None. With `seg = segment_after(t0)` this is the thrust
    /// over a step starting at `t0` that ends at the next knot: at the knot it is the left limit.
    pub fn thrust_on_segment(&self, t: f64, seg: Option<usize>) -> f64 {
        match seg {
            None => 0.0,
            Some(i) => {
                let ((t0, f0), (t1, f1)) = (self.curve[i], self.curve[i + 1]);
                (f0 + (f1 - f0) * (t - t0) / (t1 - t0)).max(0.0)
            }
        }
    }

    /// Whether the motor thrusts inside segment `seg` (the thrust is linear between knots, so
    /// it is positive inside when either end is).
    pub fn segment_powered(&self, seg: Option<usize>) -> bool {
        seg.is_some_and(|i| self.curve[i].1 > 0.0 || self.curve[i + 1].1 > 0.0)
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
        // A curve whose first node is (0, F) has its full thrust from ignition on.
        if t == 0.0 {
            return self.curve[0].1;
        }
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
        assert_eq!(m.thrust_end(), 1.1);
        assert_eq!(Motor::new("z".into(), 0.05, 0.3, 1.0, vec![(0.0, 10.0), (1.0, 10.0), (1.5, 0.0), (3.0, 0.0)]).thrust_end(), 1.5);
        assert_eq!(Motor::new("z".into(), 0.05, 0.3, 1.0, vec![(0.0, 10.0), (1.0, 10.0)]).thrust_end(), 1.0);
    }

    #[test]
    fn thrust_at_ignition_warnings_and_bom() {
        let m = Motor::parse("\u{feff}X 54 400 0 1.0 2.0 T\n0.0 50\n0.5 100\n1.0 0\n").unwrap();
        assert_eq!(m.thrust(0.0), 50.0);
        assert_eq!(m.thrust(-1.0), 0.0);
        assert!((m.thrust(0.25) - 75.0).abs() < 1e-12);
        let ramp = Motor::parse("X 54 400 0 1.0 2.0 T\n0.0 0\n0.5 100\n1.0 0\n").unwrap();
        assert_eq!(ramp.thrust(0.0), 0.0);
        assert!(ramp.warnings().is_empty());
        let cut = Motor::parse("X 54 400 0 1.0 2.0 T\n0.1 100\n1.0 20\n").unwrap();
        assert!(cut.warnings().iter().any(|w| w.contains("ends at 20 N instead of 0 N")));
        let single = Motor::parse("X 54 400 0 1.0 2.0 T\n0.03 100\n").unwrap();
        let w = single.warnings();
        assert!(w.iter().any(|w| w.contains("single point")) && w.iter().any(|w| w.contains("shorter than 0.05 s")), "{w:?}");
        let e = format!("{:#}", Motor::load(std::path::Path::new("nope.eng")).unwrap_err());
        assert!(!e.contains("nope.eng"), "{e}");
    }

    #[test]
    fn multi_motor_file_is_named_as_such() {
        let two = "A 54 400 0 1.0 2.0 T\n0.0 50\n0.5 100\n1.0 0\n\nM2 54 400 0 1.0 2.0 T\n0.0 50\n1.0 0\n";
        let e = format!("{:#}", Motor::parse(two).unwrap_err());
        assert!(e.contains("line 6") && e.contains("multiple motors in this file") && e.contains("keep one motor per file"), "{e}");
        assert!(!e.contains("invalid float"), "{e}");
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
