// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Landing dispersion studies: wind speed x direction grid and Monte Carlo.

use crate::flight::{AeroScale, Descent, Simulation, Summary};
use crate::motor::Motor;
use anyhow::{bail, Result};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, StandardNormal};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DispersionMode {
    #[default]
    WindGrid,
    MonteCarlo,
}

/// 1-sigma perturbations for Monte Carlo runs; every perturbation is an independent normal draw
/// N(0, sigma) and a sigma of 0 disables it. "Relative" sigmas are fractions (0.03 = 3 %) and
/// multiply the nominal value by (1 + draw), clamped to at least 0.05; "absolute" sigmas are added.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct MonteCarloConfig {
    /// Number of samples (each one is run for every descent mode).
    pub samples: usize,
    /// Base seed; sample `i` uses the RNG seeded with `seed + i` (wrapping).
    pub seed: u64,
    /// Relative: thrust multiplier (total impulse scales, propellant mass does not).
    pub thrust_scale: f64,
    /// Relative: stretch of the thrust-curve time axis at constant impulse (burn time x (1 + draw)).
    pub burn_time_scale: f64,
    /// Relative: dry mass (the dry inertias scale with it).
    pub dry_mass: f64,
    /// Absolute [m]: shift of the dry centre of gravity (+ = aft).
    pub cg: f64,
    /// Relative: normal-force coefficient, its slope and the pitch damping (all proportional to CN_alpha).
    pub cn_scale: f64,
    /// Relative: axial-force (drag) coefficient, power on and off.
    pub ca_scale: f64,
    /// Absolute [deg]: launch rail elevation.
    pub elevation_deg: f64,
    /// Absolute [deg]: launch rail azimuth.
    pub azimuth_deg: f64,
    /// Absolute [m/s]: reference wind speed around the configured `[wind]` speed (clamped at 0).
    pub wind_speed: f64,
    /// Absolute [deg]: wind direction around the configured `[wind]` direction.
    pub wind_direction_deg: f64,
    /// Relative: parachute drag area Cd*S.
    pub parachute_cd_s_scale: f64,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            samples: 1000,
            seed: 1,
            thrust_scale: 0.03,
            burn_time_scale: 0.02,
            dry_mass: 0.1,
            cg: 0.01,
            cn_scale: 0.10,
            ca_scale: 0.15,
            elevation_deg: 0.5,
            azimuth_deg: 1.0,
            wind_speed: 1.0,
            wind_direction_deg: 15.0,
            parachute_cd_s_scale: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct DispersionConfig {
    pub mode: DispersionMode,
    pub wind_speeds: Vec<f64>,
    pub directions: usize,
    pub monte_carlo: MonteCarloConfig,
}

impl Default for DispersionConfig {
    fn default() -> Self {
        Self { mode: DispersionMode::WindGrid, wind_speeds: (1..=7).map(f64::from).collect(), directions: 8, monte_carlo: MonteCarloConfig::default() }
    }
}

/// One wind-grid case.
#[derive(Debug, Clone, Copy)]
pub struct Case {
    pub descent: Descent,
    pub wind_speed: f64,
    pub wind_direction_deg: f64,
}

/// Runs the wind-grid dispersion study around `base` (its wind model, with speed and direction
/// replaced per case); for `monte_carlo` use [`run_monte_carlo`]. Returns the cases and their summaries in matching order.
pub fn run(cfg: &DispersionConfig, base: &Simulation, recovery_enabled: bool) -> Result<(Vec<Case>, Vec<Summary>)> {
    run_with_progress(cfg, base, recovery_enabled, &|_, _| {})
}

/// Number of runs of the wind-grid study (cases x descent modes).
pub fn wind_grid_runs(cfg: &DispersionConfig, recovery_enabled: bool) -> usize {
    cfg.wind_speeds.len() * cfg.directions * if recovery_enabled { 2 } else { 1 }
}

/// [`run`] with a progress callback `progress(case, summary)` invoked (from worker threads, in
/// completion order) once per finished case.
pub fn run_with_progress(
    cfg: &DispersionConfig,
    base: &Simulation,
    recovery_enabled: bool,
    progress: &(dyn Fn(&Case, &Summary) + Sync),
) -> Result<(Vec<Case>, Vec<Summary>)> {
    match cfg.mode {
        DispersionMode::WindGrid => run_wind_grid(cfg, base, recovery_enabled, progress),
        DispersionMode::MonteCarlo => bail!("dispersion mode \"monte_carlo\" returns per-sample data: call run_monte_carlo"),
    }
}

/// The values drawn for one Monte Carlo sample: relative draws are fractions, absolute ones are in
/// the units of the corresponding [`MonteCarloConfig`] field.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Perturbation {
    pub thrust_scale: f64,
    pub burn_time_scale: f64,
    pub dry_mass: f64,
    pub cg: f64,
    pub cn_scale: f64,
    pub ca_scale: f64,
    pub elevation_deg: f64,
    pub azimuth_deg: f64,
    pub wind_speed: f64,
    pub wind_direction_deg: f64,
    pub parachute_cd_s_scale: f64,
}

/// Smallest allowed multiplicative factor.
const MIN_FACTOR: f64 = 0.05;

fn factor(e: f64) -> f64 {
    (1.0 + e).max(MIN_FACTOR)
}

impl Perturbation {
    /// Draws all perturbations of sample `index` from `ChaCha8Rng::seed_from_u64(seed + index)`.
    /// All eleven normals are always drawn (in field order), so switching one sigma on or off does
    /// not change the draws of the others.
    pub fn draw(cfg: &MonteCarloConfig, index: usize) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed.wrapping_add(index as u64));
        let mut n = |sigma: f64| {
            let z: f64 = StandardNormal.sample(&mut rng);
            sigma * z
        };
        Self {
            thrust_scale: n(cfg.thrust_scale),
            burn_time_scale: n(cfg.burn_time_scale),
            dry_mass: n(cfg.dry_mass),
            cg: n(cfg.cg),
            cn_scale: n(cfg.cn_scale),
            ca_scale: n(cfg.ca_scale),
            elevation_deg: n(cfg.elevation_deg),
            azimuth_deg: n(cfg.azimuth_deg),
            wind_speed: n(cfg.wind_speed),
            wind_direction_deg: n(cfg.wind_direction_deg),
            parachute_cd_s_scale: n(cfg.parachute_cd_s_scale),
        }
    }

    /// The perturbed thrust curve of `motor`.
    fn motor(&self, motor: &Motor) -> Motor {
        motor.perturbed(factor(self.thrust_scale), factor(self.burn_time_scale))
    }

    /// `base` with every perturbation applied; `motor` must come from [`Perturbation::motor`].
    fn simulation<'a>(&self, base: &Simulation<'a>, motor: &'a Motor) -> Simulation<'a> {
        let mut s = base.clone();
        s.motor = motor;
        let fm = factor(self.dry_mass);
        s.mass.dry_mass *= fm;
        s.mass.ixx_dry *= fm;
        s.mass.iyy_dry *= fm;
        s.mass.cg_dry += self.cg;
        s.aero_scale = AeroScale { cn: factor(self.cn_scale), ca: factor(self.ca_scale) };
        s.launch.elevation_deg += self.elevation_deg;
        s.launch.azimuth_deg += self.azimuth_deg;
        if self.wind_speed != 0.0 || self.wind_direction_deg != 0.0 {
            s.wind = base.wind.with_speed_direction((base.wind.speed + self.wind_speed).max(0.0), (base.wind.direction_deg + self.wind_direction_deg).rem_euclid(360.0));
        }
        s.recovery.cd_s *= factor(self.parachute_cd_s_scale);
        s
    }
}

/// One Monte Carlo run: a sample under one descent mode.
#[derive(Debug, Clone)]
pub struct McRow {
    pub sample: usize,
    pub descent: Descent,
    pub perturbation: Perturbation,
    /// The flight summary, or why the sample failed (e.g. never left the rail).
    pub result: std::result::Result<Summary, String>,
}

/// Landing-point statistics (east/north relative to the launch site [m]).
#[derive(Debug, Clone, Copy, Serialize)]
pub struct Ellipse {
    /// Semi-major axis [m].
    pub semi_major: f64,
    /// Semi-minor axis [m].
    pub semi_minor: f64,
    /// Direction of the major axis, clockwise from north, in [0, 180) [deg].
    pub major_axis_bearing_deg: f64,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct PointStats {
    pub n: usize,
    pub mean_east: f64,
    pub mean_north: f64,
    /// Sample covariance [[ee, en], [en, nn]] [m^2].
    pub covariance: [[f64; 2]; 2],
    pub ellipse_1sigma: Ellipse,
    pub ellipse_3sigma: Ellipse,
}

/// Mean, sample covariance (n - 1) and k-sigma error ellipses of (east, north) points.
/// With no points everything is NaN; with one point the covariance is zero.
pub fn landing_statistics(points: &[(f64, f64)]) -> PointStats {
    let n = points.len();
    let nf = n as f64;
    let (me, mn) = (points.iter().map(|p| p.0).sum::<f64>() / nf, points.iter().map(|p| p.1).sum::<f64>() / nf);
    let (mut see, mut sen, mut snn) = (0.0, 0.0, 0.0);
    for &(e, v) in points {
        see += (e - me) * (e - me);
        sen += (e - me) * (v - mn);
        snn += (v - mn) * (v - mn);
    }
    let d = if n > 1 { nf - 1.0 } else { 1.0 };
    let (a, b, c) = (see / d, sen / d, snn / d);
    let half = 0.5 * (a + c);
    let r = (0.25 * (a - c) * (a - c) + b * b).sqrt();
    let (l1, l2) = ((half + r).max(0.0), (half - r).max(0.0));
    // Major axis at angle theta from east (counter-clockwise) -> bearing clockwise from north.
    let theta = 0.5 * (2.0 * b).atan2(a - c);
    let bearing = (90.0 - theta.to_degrees()).rem_euclid(180.0);
    let ell = |k: f64| Ellipse { semi_major: k * l1.sqrt(), semi_minor: k * l2.sqrt(), major_axis_bearing_deg: bearing };
    PointStats { n, mean_east: me, mean_north: mn, covariance: [[a, b], [b, c]], ellipse_1sigma: ell(1.0), ellipse_3sigma: ell(3.0) }
}

#[derive(Debug, Clone, Serialize)]
pub struct DescentStats {
    pub descent: Descent,
    /// Samples that failed (excluded from every statistic).
    pub failed: usize,
    #[serde(flatten)]
    pub landing: PointStats,
    pub max_landing_distance: f64,
    /// Sample index of the farthest landing.
    pub max_landing_distance_sample: Option<usize>,
    pub apogee_mean: f64,
    pub apogee_std: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct McSummary {
    pub samples: usize,
    pub seed: u64,
    pub descents: Vec<DescentStats>,
}

#[derive(Debug, Clone)]
pub struct McResult {
    /// Ordered by sample index, then descent mode (ballistic first).
    pub rows: Vec<McRow>,
    pub summary: McSummary,
}

/// Statistics over the successful rows of each descent mode present in `rows`.
pub fn summarize(rows: &[McRow], samples: usize, seed: u64) -> McSummary {
    let mut modes: Vec<Descent> = Vec::new();
    for r in rows {
        if !modes.contains(&r.descent) {
            modes.push(r.descent);
        }
    }
    let descents = modes
        .into_iter()
        .map(|m| {
            let mine: Vec<&McRow> = rows.iter().filter(|r| r.descent == m).collect();
            let ok: Vec<(usize, &Summary)> = mine.iter().filter_map(|r| r.result.as_ref().ok().map(|s| (r.sample, s))).collect();
            let pts: Vec<(f64, f64)> = ok.iter().map(|(_, s)| (s.landing_east, s.landing_north)).collect();
            let (mut dmax, mut imax) = (f64::NAN, None);
            for &(i, s) in &ok {
                if imax.is_none() || s.landing_distance > dmax {
                    dmax = s.landing_distance;
                    imax = Some(i);
                }
            }
            let nf = ok.len() as f64;
            let am = ok.iter().map(|(_, s)| s.apogee).sum::<f64>() / nf;
            let var = ok.iter().map(|(_, s)| (s.apogee - am).powi(2)).sum::<f64>() / if ok.len() > 1 { nf - 1.0 } else { 1.0 };
            DescentStats {
                descent: m,
                failed: mine.len() - ok.len(),
                landing: landing_statistics(&pts),
                max_landing_distance: dmax,
                max_landing_distance_sample: imax,
                apogee_mean: am,
                apogee_std: var.sqrt(),
            }
        })
        .collect();
    McSummary { samples, seed, descents }
}

/// Monte Carlo landing dispersion around `base`. Every sample draws its own perturbations (see
/// [`Perturbation::draw`]) and is flown with each descent mode (ballistic always, parachute if
/// `recovery_enabled`) using the same draw. Samples run in parallel; the result does not depend on
/// scheduling. A failing sample is recorded as such and excluded from the statistics.
pub fn run_monte_carlo(cfg: &MonteCarloConfig, base: &Simulation, recovery_enabled: bool) -> Result<McResult> {
    run_monte_carlo_with_progress(cfg, base, recovery_enabled, &|_| {})
}

/// [`run_monte_carlo`] with a progress callback invoked (from worker threads, in completion
/// order) once per finished flight, i.e. `samples` x number of descent modes times.
pub fn run_monte_carlo_with_progress(cfg: &MonteCarloConfig, base: &Simulation, recovery_enabled: bool, progress: &(dyn Fn(&McRow) + Sync)) -> Result<McResult> {
    if cfg.samples == 0 {
        bail!("dispersion.monte_carlo.samples must be at least 1");
    }
    let sigmas = [
        ("thrust_scale", cfg.thrust_scale),
        ("burn_time_scale", cfg.burn_time_scale),
        ("dry_mass", cfg.dry_mass),
        ("cg", cfg.cg),
        ("cn_scale", cfg.cn_scale),
        ("ca_scale", cfg.ca_scale),
        ("elevation_deg", cfg.elevation_deg),
        ("azimuth_deg", cfg.azimuth_deg),
        ("wind_speed", cfg.wind_speed),
        ("wind_direction_deg", cfg.wind_direction_deg),
        ("parachute_cd_s_scale", cfg.parachute_cd_s_scale),
    ];
    for (name, v) in sigmas {
        if !(v.is_finite() && v >= 0.0) {
            bail!("dispersion.monte_carlo.{name} must be a finite number >= 0 (1-sigma), found {v}");
        }
    }
    let mut modes = vec![Descent::Ballistic];
    if recovery_enabled {
        modes.push(Descent::Parachute);
    }
    let rows: Vec<McRow> = (0..cfg.samples)
        .into_par_iter()
        .flat_map_iter(|i| {
            let p = Perturbation::draw(cfg, i);
            let motor = p.motor(base.motor);
            let sim = p.simulation(base, &motor);
            modes
                .iter()
                .map(|&descent| {
                    let result = match sim.run(descent, false) {
                        Ok(r) if r.summary.landing_east.is_finite() && r.summary.landing_north.is_finite() => Ok(r.summary),
                        Ok(_) => Err("did not land within sim.max_time".to_string()),
                        Err(e) => Err(format!("{e:#}")),
                    };
                    let row = McRow { sample: i, descent, perturbation: p, result };
                    progress(&row);
                    row
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let summary = summarize(&rows, cfg.samples, cfg.seed);
    Ok(McResult { rows, summary })
}

fn run_wind_grid(cfg: &DispersionConfig, base: &Simulation, recovery_enabled: bool, progress: &(dyn Fn(&Case, &Summary) + Sync)) -> Result<(Vec<Case>, Vec<Summary>)> {
    let n = cfg.directions;
    let mut modes = vec![Descent::Ballistic];
    if recovery_enabled {
        modes.push(Descent::Parachute);
    }
    let cases: Vec<Case> = modes
        .iter()
        .flat_map(|&m| {
            cfg.wind_speeds.iter().flat_map(move |&v| (0..n).map(move |k| Case { descent: m, wind_speed: v, wind_direction_deg: 360.0 * k as f64 / n as f64 }))
        })
        .collect();
    let summaries = cases
        .par_iter()
        .map(|c| {
            let mut s = base.clone();
            s.wind = base.wind.with_speed_direction(c.wind_speed, c.wind_direction_deg);
            s.run(c.descent, false).map(|r| {
                progress(c, &r.summary);
                r.summary
            })
        })
        .collect::<Result<_>>()?;
    Ok((cases, summaries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{AtmosphereModel, Earth, IntegratorKind, Wind, WindConfig, WindModel};
    use crate::flight::{Launch, MassProperties, Recovery, Settings};
    use aero::table::TableMeta;
    use aero::{AeroCoeffs, AeroTable, Extrapolation};

    fn table() -> AeroTable {
        let meta = TableMeta {
            source_hash: String::new(),
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.0,
            machs: vec![0.0, 5.0],
            alphas_deg: vec![0.0, 90.0],
            extrapolation: Extrapolation::Clamp,
        };
        AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, ca_on: 0.5, ca_off: 0.5, ..Default::default() })
    }

    fn motor() -> Motor {
        Motor::new("c".into(), 0.05, 0.3, 0.5, vec![(0.0, 400.0), (2.0, 400.0)])
    }

    /// Vertical launch, constant 5 m/s wind from the west, parachute recovery.
    fn base<'a>(table: &'a AeroTable, motor: &'a Motor) -> Simulation<'a> {
        Simulation {
            table,
            mass: MassProperties { dry_mass: 10.0, cg_dry: 0.5, ixx_dry: 0.01, iyy_dry: 1.0 },
            motor,
            motor_aft_x: 1.0,
            launch: Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: 90.0, azimuth_deg: 0.0 },
            recovery: Recovery { enabled: true, cd_s: 1.0, delay: 1.0 },
            wind: Wind::from_config(&WindConfig { model: WindModel::Constant, speed: 5.0, direction_deg: 270.0, ..Default::default() }).unwrap(),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: Settings { integrator: IntegratorKind::Rk45, max_time: 2000.0, ..Default::default() },
            aero_scale: AeroScale::default(),
        }
    }

    fn zero() -> MonteCarloConfig {
        MonteCarloConfig {
            samples: 6,
            seed: 7,
            thrust_scale: 0.0,
            burn_time_scale: 0.0,
            dry_mass: 0.0,
            cg: 0.0,
            cn_scale: 0.0,
            ca_scale: 0.0,
            elevation_deg: 0.0,
            azimuth_deg: 0.0,
            wind_speed: 0.0,
            wind_direction_deg: 0.0,
            parachute_cd_s_scale: 0.0,
        }
    }

    fn landings(r: &McResult) -> Vec<(f64, f64)> {
        r.rows.iter().map(|x| x.result.as_ref().map(|s| (s.landing_east, s.landing_north)).unwrap_or((f64::NAN, f64::NAN))).collect()
    }

    #[test]
    fn seed_reproducibility() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let cfg = MonteCarloConfig { samples: 8, seed: 3, ..Default::default() };
        let a = run_monte_carlo(&cfg, &b, true).unwrap();
        let a2 = run_monte_carlo(&cfg, &b, true).unwrap();
        assert_eq!(a.rows.len(), 16);
        assert_eq!(landings(&a), landings(&a2));
        let c = run_monte_carlo(&MonteCarloConfig { seed: 4, ..cfg.clone() }, &b, true).unwrap();
        assert_ne!(landings(&a), landings(&c));
        // sample i of seed s is sample i + 1 of seed s - 1: draws depend only on seed + index
        assert_eq!(Perturbation::draw(&cfg, 1), Perturbation::draw(&MonteCarloConfig { seed: 2, ..cfg.clone() }, 2));
    }

    #[test]
    fn zero_sigma_reproduces_nominal() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let r = run_monte_carlo(&zero(), &b, true).unwrap();
        for row in &r.rows {
            let nom = b.run(row.descent, false).unwrap().summary;
            let s = row.result.as_ref().unwrap();
            assert_eq!((s.landing_east, s.landing_north, s.apogee, s.landing_time), (nom.landing_east, nom.landing_north, nom.apogee, nom.landing_time));
            assert_eq!(row.perturbation, Perturbation::default());
        }
        let st = &r.summary.descents[1];
        assert_eq!((st.landing.n, st.failed), (6, 0));
        assert!(st.landing.covariance[0][0] < 1e-18);
    }

    #[test]
    fn wind_direction_spreads_in_angle() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let cfg = MonteCarloConfig { samples: 40, wind_direction_deg: 20.0, ..zero() };
        let r = run_monte_carlo(&cfg, &b, true).unwrap();
        let pts: Vec<(f64, f64)> = r
            .rows
            .iter()
            .filter(|x| x.descent == Descent::Parachute)
            .map(|x| {
                let s = x.result.as_ref().unwrap();
                (s.landing_east, s.landing_north)
            })
            .collect();
        let rad: Vec<f64> = pts.iter().map(|p| p.0.hypot(p.1)).collect();
        let ang: Vec<f64> = pts.iter().map(|p| p.1.atan2(p.0)).collect();
        let std = |v: &[f64]| {
            let m = v.iter().sum::<f64>() / v.len() as f64;
            (v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / (v.len() as f64 - 1.0)).sqrt()
        };
        let mean_r = rad.iter().sum::<f64>() / rad.len() as f64;
        let (cv_r, sd_ang) = (std(&rad) / mean_r, std(&ang));
        println!("radial cv {cv_r}, angular std {sd_ang} rad (sigma {} rad)", 20f64.to_radians());
        assert!(cv_r < 0.02, "radial cv {cv_r}");
        assert!((sd_ang - 20f64.to_radians()).abs() < 0.1, "angular std {sd_ang}");
        let st = &r.summary.descents[1].landing;
        assert!(st.ellipse_1sigma.semi_major > 3.0 * st.ellipse_1sigma.semi_minor);
    }

    #[test]
    fn statistics_of_known_covariance() {
        // Points on a rotated rectangle grid: +-a along u (bearing 30 deg), +-b along v, symmetric.
        let (a, b) = (30.0, 10.0);
        let brg = 30f64.to_radians();
        let u = (brg.sin(), brg.cos()); // (east, north) of bearing 30 deg
        let v = (brg.cos(), -brg.sin());
        let mut pts = Vec::new();
        for &(s, t) in &[(1.0, 1.0), (1.0, -1.0), (-1.0, 1.0), (-1.0, -1.0)] {
            pts.push((100.0 + s * a * u.0 + t * b * v.0, -50.0 + s * a * u.1 + t * b * v.1));
        }
        let st = landing_statistics(&pts);
        // sample covariance (n - 1 = 3): variance along u = 4 a^2 / 3, along v = 4 b^2 / 3
        let (vu, vv) = (4.0 * a * a / 3.0, 4.0 * b * b / 3.0);
        assert!((st.mean_east - 100.0).abs() < 1e-9 && (st.mean_north + 50.0).abs() < 1e-9);
        let c = st.covariance;
        let ee = vu * u.0 * u.0 + vv * v.0 * v.0;
        let en = vu * u.0 * u.1 + vv * v.0 * v.1;
        let nn = vu * u.1 * u.1 + vv * v.1 * v.1;
        assert!((c[0][0] - ee).abs() < 1e-9 && (c[0][1] - en).abs() < 1e-9 && (c[1][1] - nn).abs() < 1e-9 && c[0][1] == c[1][0]);
        let e1 = st.ellipse_1sigma;
        assert!((e1.semi_major - vu.sqrt()).abs() < 1e-9 && (e1.semi_minor - vv.sqrt()).abs() < 1e-9);
        assert!((e1.major_axis_bearing_deg - 30.0).abs() < 1e-9, "{}", e1.major_axis_bearing_deg);
        assert!((st.ellipse_3sigma.semi_major - 3.0 * vu.sqrt()).abs() < 1e-9);
        // major axis east-west -> bearing 90; north-south -> 0
        let ew = landing_statistics(&[(-2.0, 0.0), (2.0, 0.0), (0.0, 1.0), (0.0, -1.0)]);
        assert!((ew.ellipse_1sigma.major_axis_bearing_deg - 90.0).abs() < 1e-9);
        let ns = landing_statistics(&[(-1.0, 0.0), (1.0, 0.0), (0.0, 2.0), (0.0, -2.0)]);
        assert!(ns.ellipse_1sigma.major_axis_bearing_deg.abs() < 1e-9);
    }

    #[test]
    fn failed_samples_are_marked_and_excluded() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        // Huge thrust sigma: draws below about -0.74 give less thrust than weight (~103 N).
        let cfg = MonteCarloConfig { samples: 30, thrust_scale: 2.0, ..zero() };
        let r = run_monte_carlo(&cfg, &b, true).unwrap();
        let bad: Vec<_> = r.rows.iter().filter(|x| x.descent == Descent::Ballistic && x.result.is_err()).collect();
        assert!(!bad.is_empty() && bad.len() < 30, "{} failures", bad.len());
        assert!(bad.iter().all(|x| x.result.as_ref().unwrap_err().contains("rail") && x.perturbation.thrust_scale < -0.7));
        let st = &r.summary.descents[0];
        assert_eq!(st.failed, bad.len());
        assert_eq!(st.landing.n, 30 - bad.len());
        assert!(st.landing.mean_east.is_finite() && st.max_landing_distance.is_finite());
        // the parachute rows of the same samples fail identically
        let pf = r.summary.descents[1].failed;
        assert_eq!(pf, bad.len());
    }

    #[test]
    fn thread_count_does_not_change_results() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let cfg = MonteCarloConfig { samples: 12, seed: 5, ..Default::default() };
        let dcfg = DispersionConfig { wind_speeds: vec![2.0, 6.0], directions: 4, ..Default::default() };
        let go = |n: usize| {
            rayon::ThreadPoolBuilder::new().num_threads(n).build().unwrap().install(|| {
                let mc = run_monte_carlo(&cfg, &b, true).unwrap();
                let (_, grid) = run(&dcfg, &b, true).unwrap();
                (landings(&mc), format!("{grid:?}"))
            })
        };
        let one = go(1);
        assert_eq!(one, go(4));
        assert_eq!(one, go(7));
    }

    #[test]
    fn perturbations_act_as_documented() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let p = Perturbation { thrust_scale: 0.1, burn_time_scale: 0.25, dry_mass: 0.2, cg: 0.01, cn_scale: 0.1, ca_scale: -2.0, elevation_deg: -1.0, azimuth_deg: 2.0, wind_speed: -9.0, wind_direction_deg: 100.0, parachute_cd_s_scale: 0.5 };
        let pm = p.motor(&m);
        let s = p.simulation(&b, &pm);
        assert!((pm.total_impulse() - 1.1 * m.total_impulse()).abs() < 1e-9 && (pm.burn_time() - 2.5).abs() < 1e-12);
        assert_eq!(pm.propellant_mass, m.propellant_mass);
        assert!((s.mass.dry_mass - 12.0).abs() < 1e-12 && (s.mass.iyy_dry - 1.2).abs() < 1e-12 && (s.mass.cg_dry - 0.51).abs() < 1e-12);
        assert_eq!(s.aero_scale, AeroScale { cn: 1.1, ca: 0.05 });
        assert_eq!((s.launch.elevation_deg, s.launch.azimuth_deg), (89.0, 2.0));
        assert_eq!((s.wind.speed, s.wind.direction_deg), (0.0, 10.0));
        assert!((s.recovery.cd_s - 1.5).abs() < 1e-12);
    }
}
