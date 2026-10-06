// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Static-stability verdict of flights with a general (six-component) aero table.
//!
//! The simulator reports the static margin of every step from a single equivalent centre of
//! pressure of the transverse force. For a general table the pitch and yaw planes can differ, so
//! the verdict (`stable`, unstable counts, exit status 3) uses the separate pitch and yaw margins
//! of [`AeroTable::static_margins`], the smaller of the two. Pitch-yaw coupling is ignored.

use aero::table::TableKind;
use aero::AeroTable;
use sim::flight::{Phase, Sample};
use sim::Summary;

/// Smallest pitch and yaw static margins of a recorded flight [cal].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisMargins {
    /// Free flight before apogee with airspeed > 10 m/s (NaN when no sample qualifies).
    pub pitch_min: f64,
    pub yaw_min: f64,
    /// At rail exit (Mach and CG interpolated between the samples around it).
    pub rail_pitch: f64,
    pub rail_yaw: f64,
}

/// The pitch and yaw margins of a general table along the recorded `samples` of the flight `s`; None
/// for an axisymmetric table (its margin is the simulator's).
pub fn axis_margins(table: &AeroTable, s: &Summary, samples: &[Sample]) -> Option<AxisMargins> {
    if table.kind() != TableKind::General {
        return None;
    }
    let (mut pitch_min, mut yaw_min) = (f64::INFINITY, f64::INFINITY);
    let before_apogee = |t: f64| !s.apogee_time.is_finite() || t <= s.apogee_time;
    for smp in samples.iter().filter(|x| x.phase == Phase::Free && before_apogee(x.t) && x.airspeed > 10.0) {
        let aero::table::StaticMargins { pitch: p, yaw: y, .. } = table.static_margins(smp.mach, smp.x_cg);
        pitch_min = pitch_min.min(p);
        yaw_min = yaw_min.min(y);
    }
    let finite = |v: f64| if v.is_finite() { v } else { f64::NAN };
    let (rail_pitch, rail_yaw) = match rail_exit_state(s.rail_exit_time, samples) {
        Some((mach, x_cg)) => {
            let m = table.static_margins(mach, x_cg);
            (m.pitch, m.yaw)
        }
        None => (f64::NAN, f64::NAN),
    };
    Some(AxisMargins { pitch_min: finite(pitch_min), yaw_min: finite(yaw_min), rail_pitch, rail_yaw })
}

/// Mach and CG at time `t` interpolated linearly between the samples around it.
fn rail_exit_state(t: f64, samples: &[Sample]) -> Option<(f64, f64)> {
    if !t.is_finite() {
        return None;
    }
    let i = samples.iter().position(|x| x.t >= t)?;
    let b = &samples[i];
    if i == 0 || b.t == t {
        return Some((b.mach, b.x_cg));
    }
    let a = &samples[i - 1];
    let f = (t - a.t) / (b.t - a.t);
    Some((a.mach + (b.mach - a.mach) * f, a.x_cg + (b.x_cg - a.x_cg) * f))
}

/// Replaces the simulator's free-flight and rail-exit margins of `s` with the smaller of the pitch
/// and yaw margins.
pub fn apply(s: &mut Summary, m: &AxisMargins) {
    s.min_stability_cal = min_nan(m.pitch_min, m.yaw_min);
    s.stability_at_rail_exit = min_nan(m.rail_pitch, m.rail_yaw);
}

/// The smaller value; NaN when either is NaN (a plane without a margin gives no verdict).
fn min_nan(a: f64, b: f64) -> f64 {
    if a.is_nan() || b.is_nan() {
        f64::NAN
    } else {
        a.min(b)
    }
}

/// Largest difference between the pitch and the yaw centre of pressure of a general table over Mach
/// 0..`max_mach` [cal] (0 for an axisymmetric table, NaN when a plane has no normal-force slope).
pub fn max_pitch_yaw_split(table: &AeroTable, max_mach: f64) -> f64 {
    if table.kind() != TableKind::General {
        return 0.0;
    }
    let machs = &table.meta.machs;
    let top = if max_mach.is_finite() { max_mach } else { machs.last().copied().unwrap_or(0.0) };
    let mut split: f64 = 0.0;
    for m in machs.iter().copied().filter(|&m| m <= top).chain([top]) {
        let aero::table::StaticMargins { pitch: p, yaw: y, .. } = table.static_margins(m, 0.0);
        if p.is_nan() || y.is_nan() {
            return f64::NAN;
        }
        split = split.max((p - y).abs());
    }
    split
}

/// For a run flown without recorded samples (dispersion cases and samples) with a general table:
/// lowers the simulator's margins by [`max_pitch_yaw_split`] over the Mach range of the run. The
/// equivalent centre of pressure of a small transverse force lies between the pitch and the yaw
/// centre of pressure, so the result is a lower bound of the smaller of the pitch and yaw margins.
pub fn apply_bound(table: &AeroTable, s: &mut Summary) {
    if table.kind() != TableKind::General {
        return;
    }
    let split = max_pitch_yaw_split(table, s.max_mach);
    s.min_stability_cal -= split;
    s.stability_at_rail_exit -= split;
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::table::{angle_axis, AeroCoeffs, Coeffs6, TableMeta};
    use sim::Descent;

    fn meta() -> TableMeta {
        TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.5, machs: vec![0.0, 0.5, 1.0], alphas_deg: vec![0.0, 2.0, 10.0], ..Default::default() }
    }

    /// General table with the pitch centre of pressure at `xp` and the yaw one at `xy` [m].
    fn split_table(xp: f64, xy: f64) -> AeroTable {
        let meta = TableMeta { alphas_deg: angle_axis(180.0, 10.0, 1.0, 10.0), betas_deg: angle_axis(90.0, 10.0, 1.0, 10.0), ..meta() };
        AeroTable::general_from_fn(meta, move |_, a, b| {
            let (cz, cy) = (-0.2 * a.to_radians().sin() * 57.3, -0.2 * b.to_radians().sin() * 57.3);
            Coeffs6 { cx_on: -0.5, cx_off: -0.6, cz, cy, cm: xp * cz / 0.1, cn: -xy * cy / 0.1, ..Default::default() }
        })
    }

    fn sample(t: f64, phase: Phase, mach: f64, x_cg: f64) -> Sample {
        #[rustfmt::skip]
        let s = Sample {
            t, phase, east: 0.0, north: 0.0, up: 0.0, lat: 0.0, lon: 0.0, alt: 0.0, vel_e: 0.0, vel_n: 0.0, vel_u: 0.0,
            airspeed: mach * 340.0, mach, alpha_deg: 0.0, thrust: 0.0, mass: 1.0, x_cg, x_cp: 0.0, stability_cal: 0.0,
            dyn_pressure: 0.0, cn: 0.0, ca: 0.0, pitch_deg: 0.0, heading_deg: 0.0, wx: 0.0, wy: 0.0, wz: 0.0, beta_deg: 0.0,
            roll_rate: 0.0, cl: 0.0, cy: 0.0,
        };
        s
    }

    fn summary() -> Summary {
        let mut s = Summary::failed(Descent::Ballistic, 0.0, 0.0);
        s.rail_exit_time = 0.25;
        s.apogee_time = 10.0;
        s.max_mach = 0.8;
        s
    }

    #[test]
    fn axisymmetric_tables_keep_the_simulator_margin() {
        let t = AeroTable::from_fn(meta(), |_, a| AeroCoeffs { cn: 0.2 * a, xcp: 1.1, ..Default::default() });
        assert!(axis_margins(&t, &summary(), &[]).is_none());
        assert_eq!(max_pitch_yaw_split(&t, 0.8), 0.0);
        let mut s = summary();
        s.min_stability_cal = 1.5;
        apply_bound(&t, &mut s);
        assert_eq!(s.min_stability_cal, 1.5);
    }

    #[test]
    fn pitch_stable_yaw_unstable_flight_is_unstable() {
        // CG 0.9 m: pitch margin +3 cal, yaw -3 cal.
        let t = split_table(1.2, 0.6);
        let samples = [
            sample(0.0, Phase::Rail, 0.0, 0.9),
            sample(0.2, Phase::Rail, 0.05, 0.9),
            sample(0.3, Phase::Free, 0.1, 0.9),
            sample(5.0, Phase::Free, 0.6, 0.9),
            sample(11.0, Phase::Free, 0.3, 0.9),
        ];
        let mut s = summary();
        s.min_stability_cal = 2.0;
        s.stability_at_rail_exit = 2.0;
        let m = axis_margins(&t, &s, &samples).unwrap();
        assert!((m.pitch_min - 3.0).abs() < 1e-9 && (m.yaw_min + 3.0).abs() < 1e-9, "{m:?}");
        assert!((m.rail_pitch - 3.0).abs() < 1e-9 && (m.rail_yaw + 3.0).abs() < 1e-9, "{m:?}");
        apply(&mut s, &m);
        assert!((s.min_stability_cal + 3.0).abs() < 1e-9 && (s.stability_at_rail_exit + 3.0).abs() < 1e-9);
        // Without samples (dispersion runs): the simulator's margin lowered by the 6 cal split.
        assert!((max_pitch_yaw_split(&t, 0.8) - 6.0).abs() < 1e-9);
        let mut d = summary();
        d.min_stability_cal = 2.0;
        d.stability_at_rail_exit = 2.5;
        apply_bound(&t, &mut d);
        assert!((d.min_stability_cal + 4.0).abs() < 1e-9 && (d.stability_at_rail_exit + 3.5).abs() < 1e-9);
    }

    #[test]
    fn rail_exit_state_interpolates() {
        let samples = [sample(0.2, Phase::Rail, 0.04, 1.0), sample(0.3, Phase::Free, 0.08, 0.9)];
        let (m, x) = rail_exit_state(0.25, &samples).unwrap();
        assert!((m - 0.06).abs() < 1e-12 && (x - 0.95).abs() < 1e-12);
        assert!(rail_exit_state(f64::NAN, &samples).is_none());
    }
}
