// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Landing dispersion studies: wind speed x direction grid and Monte Carlo.

use crate::flight::{AeroScale, Descent, Launch, Simulation, Summary};
use crate::motor::Motor;
use anyhow::{bail, Result};
use geom::Vec3;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use rand_distr::{Distribution, StandardNormal};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};

/// Fewest landed samples for which the P99.9 of the landing distance is reported: below it the percentile is
/// the largest sample or an interpolation next to it, not an estimate of the 0.1 % tail.
pub const P999_MIN_SAMPLES: usize = 1000;

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
    /// Absolute [deg]: rail pointing error. The rail axis is tilted away from its nominal
    /// direction by two independent normal angles with this sigma, one in the vertical plane of
    /// the rail (+ = towards higher elevation) and one perpendicular to it (+ = towards higher
    /// azimuth), so the tilt is isotropic about the nominal axis at any elevation, vertical
    /// included: the angle between the perturbed and the nominal axis is Rayleigh distributed
    /// with this sigma. The rocket turns with the rail, so its body axes stay within the tilt
    /// angle of their nominal directions (see [`tilt_launch`]).
    pub elevation_deg: f64,
    /// Absolute [deg]: azimuth setting error, a rotation of the (tilted) rail, with the rocket on
    /// it, about the vertical. It moves the axis sideways by sigma * cos(elevation); a vertical
    /// rail keeps its axis and only the rocket's roll about it changes.
    pub azimuth_deg: f64,
    /// Absolute [m/s]: reference wind speed around the configured `[wind]` speed (clamped at 0).
    pub wind_speed: f64,
    /// Absolute [deg]: wind direction around the configured `[wind]` direction.
    pub wind_direction_deg: f64,
    /// Relative: parachute drag area Cd*S.
    pub parachute_cd_s_scale: f64,
    /// Absolute [deg]: thrust misalignment; two orthogonal normals (y_b, z_b tilt components) are added
    /// to the nominal tilt vector, which gives the misalignment angle and its azimuth.
    pub thrust_misalignment_deg: f64,
    /// Absolute [m]: nozzle-exit point of the thrust line, one normal draw per body axis y and z.
    pub thrust_offset: f64,
    /// Absolute [deg]: fin cant added to the configured cant (the tabulated roll moment stays at the reference cant).
    pub fin_cant_deg: f64,
    /// Absolute [m]: lateral dry CG offset, one normal draw per body axis y and z.
    pub cg_lateral: f64,
    /// Relative: roll-moment coefficient Cl and its cant slope Cl_delta.
    pub cl_scale: f64,
    /// Relative: roll-damping derivative Clp.
    pub clp_scale: f64,
    /// Optional bound `k` of every standard-normal draw: each draw `z ~ N(0, 1)` is clipped to
    /// `[-k, k]` before it is scaled by its sigma (relative and absolute dispersions alike), so no
    /// perturbation exceeds `k` sigma. Clipping (not redrawing) keeps the number and order of the
    /// draws, so every other sample and dispersion keeps its value. `None` (the default): untruncated
    /// normals.
    pub truncate_sigma: Option<f64>,
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
            thrust_misalignment_deg: 0.0,
            thrust_offset: 0.0,
            fin_cant_deg: 0.0,
            cg_lateral: 0.0,
            cl_scale: 0.0,
            clp_scale: 0.0,
            truncate_sigma: None,
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
    /// Change of the rail elevation [deg], of the rail azimuth [deg, in (-180, 180]] and of the
    /// body roll about the rail [deg, in (-180, 180]] that the rail draws produce (see
    /// [`MonteCarloConfig::elevation_deg`] and [`tilt_launch`]): the perturbed launch has
    /// elevation `nominal + elevation_deg`, azimuth `nominal + azimuth_deg` and roll
    /// `nominal + roll_deg`. The roll change keeps the body axes turning with the rail, e.g. it
    /// is 180 deg when a small tilt of a vertical rail flips the azimuth.
    pub elevation_deg: f64,
    pub azimuth_deg: f64,
    pub roll_deg: f64,
    pub wind_speed: f64,
    pub wind_direction_deg: f64,
    pub parachute_cd_s_scale: f64,
    /// Thrust tilt added along y_b and z_b [deg].
    pub misalignment_y_deg: f64,
    pub misalignment_z_deg: f64,
    /// Nozzle-exit offset added along y_b and z_b [m].
    pub offset_y: f64,
    pub offset_z: f64,
    /// Fin cant added [deg].
    pub fin_cant_deg: f64,
    /// Lateral dry CG offset added along y_b and z_b [m].
    pub cg_lateral_y: f64,
    pub cg_lateral_z: f64,
    pub cl_scale: f64,
    pub clp_scale: f64,
}

/// Smallest allowed multiplicative factor.
const MIN_FACTOR: f64 = 0.05;

fn factor(e: f64) -> f64 {
    (1.0 + e).max(MIN_FACTOR)
}

impl Perturbation {
    /// Draws all perturbations of sample `index` from `ChaCha8Rng::seed_from_u64(seed + index)`.
    /// All normals are always drawn (in field order: the eleven original dispersions from
    /// `thrust_scale` to `parachute_cd_s_scale`, then the nine of the thrust line, fin cant,
    /// lateral CG and roll coefficients from `misalignment_y_deg` to `clp_scale`, the cross-plane
    /// rail tilt last; `roll_deg` is not drawn but follows from the rail draws), so
    /// switching one sigma on or off does not change the draws of the others. The rail draws
    /// (in-plane tilt, rotation about the vertical, cross-plane tilt) are turned into the
    /// elevation, azimuth and roll changes of the launch `launch` (see [`tilt_launch`]).
    pub fn draw(cfg: &MonteCarloConfig, index: usize, launch: &Launch) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(cfg.seed.wrapping_add(index as u64));
        let k = cfg.truncate_sigma.unwrap_or(f64::INFINITY);
        let mut n = |sigma: f64| {
            let z: f64 = StandardNormal.sample(&mut rng);
            sigma * z.clamp(-k, k) + 0.0
        };
        let mut p = Self {
            thrust_scale: n(cfg.thrust_scale),
            burn_time_scale: n(cfg.burn_time_scale),
            dry_mass: n(cfg.dry_mass),
            cg: n(cfg.cg),
            cn_scale: n(cfg.cn_scale),
            ca_scale: n(cfg.ca_scale),
            elevation_deg: n(cfg.elevation_deg),
            azimuth_deg: n(cfg.azimuth_deg),
            // Set from the rail draws below.
            roll_deg: 0.0,
            wind_speed: n(cfg.wind_speed),
            wind_direction_deg: n(cfg.wind_direction_deg),
            parachute_cd_s_scale: n(cfg.parachute_cd_s_scale),
            misalignment_y_deg: n(cfg.thrust_misalignment_deg),
            misalignment_z_deg: n(cfg.thrust_misalignment_deg),
            offset_y: n(cfg.thrust_offset),
            offset_z: n(cfg.thrust_offset),
            fin_cant_deg: n(cfg.fin_cant_deg),
            cg_lateral_y: n(cfg.cg_lateral),
            cg_lateral_z: n(cfg.cg_lateral),
            cl_scale: n(cfg.cl_scale),
            clp_scale: n(cfg.clp_scale),
        };
        let cross = n(cfg.elevation_deg);
        let (el, az, roll) = tilt_launch(launch, p.elevation_deg, cross, p.azimuth_deg);
        p.elevation_deg = el - launch.elevation_deg;
        p.azimuth_deg = wrap180(az - launch.azimuth_deg);
        p.roll_deg = wrap180(roll - launch.roll_deg);
        p
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
        s.mass.izz_dry = s.mass.izz_dry.map(|z| z * fm);
        s.mass.ixy_dry *= fm;
        s.mass.ixz_dry *= fm;
        s.mass.iyz_dry *= fm;
        s.mass.cg_dry += self.cg;
        s.mass.cg_lateral_dry[0] += self.cg_lateral_y;
        s.mass.cg_lateral_dry[1] += self.cg_lateral_z;
        s.aero_scale = AeroScale { cn: factor(self.cn_scale), ca: factor(self.ca_scale), cl: factor(self.cl_scale), clp: factor(self.clp_scale) };
        s.fin_cant_deg += self.fin_cant_deg;
        s.thrust_line.offset[0] += self.offset_y;
        s.thrust_line.offset[1] += self.offset_z;
        if self.misalignment_y_deg != 0.0 || self.misalignment_z_deg != 0.0 {
            // Add the draws to the nominal tilt vector (eps cos phi, eps sin phi).
            let t = base.thrust_line;
            let (sp, cp) = t.misalignment_azimuth_deg.to_radians().sin_cos();
            let (y, z) = (t.misalignment_deg * cp + self.misalignment_y_deg, t.misalignment_deg * sp + self.misalignment_z_deg);
            s.thrust_line.misalignment_deg = y.hypot(z);
            s.thrust_line.misalignment_azimuth_deg = z.atan2(y).to_degrees();
        }
        // `elevation_deg` is the difference of the `tilt_launch` elevation (in [-90, 90] deg) from the
        // nominal one, so the sum is that elevation again.
        s.launch.elevation_deg += self.elevation_deg;
        s.launch.azimuth_deg += self.azimuth_deg;
        s.launch.roll_deg += self.roll_deg;
        if self.wind_speed != 0.0 || self.wind_direction_deg != 0.0 {
            // For a profile `speed` is the mean speed W: k = (W + delta) / W, clamped to >= 0.
            // `run_monte_carlo` has already rejected a calm profile, so this does not fail.
            if let Ok(w) = base
                .wind
                .with_speed_direction((base.wind.speed + self.wind_speed).max(0.0), (base.wind.direction_deg + self.wind_direction_deg).rem_euclid(360.0))
            {
                s.wind = w;
            }
        }
        let k = factor(self.parachute_cd_s_scale);
        s.recovery.cd_s *= k;
        s.recovery.main_cd_s = s.recovery.main_cd_s.map(|c| c * k);
        s
    }
}

/// Rail elevation, azimuth and body roll [deg] (see [`Launch`]) of the launch attitude of
/// `launch` after rotating the whole body: first a tilt of the rail axis by `d_in` in its
/// vertical plane (+ = up; past the zenith the azimuth flips) and by `d_cross` perpendicular to
/// that plane (+ = towards increasing azimuth), both [deg], as one rotation by
/// `hypot(d_in, d_cross)` about the nominal axis' normal, then a rotation about the vertical by
/// `d_az` [deg] (clockwise seen from above). For small angles the axis moves to `el + d_in`,
/// `az + d_cross / cos(el) + d_az`; for a vertical rail the in-plane direction is that of the
/// nominal azimuth. The body axes turn with the rail, so they stay within the rotation angle of
/// their nominal directions: a tiny tilt of a vertical rail that flips the azimuth by 180 deg
/// flips the roll by 180 deg as well. The returned azimuth and roll are in (-180, 180].
pub fn tilt_launch(launch: &Launch, d_in: f64, d_cross: f64, d_az: f64) -> (f64, f64, f64) {
    let theta = d_in.hypot(d_cross);
    if theta == 0.0 {
        // A rotation about the vertical keeps the elevation and the roll.
        return (launch.elevation_deg, wrap180(launch.azimuth_deg + d_az), wrap180(launch.roll_deg));
    }
    let mut axes = launch.body_axes_enu();
    let (se, ce) = launch.elevation_deg.to_radians().sin_cos();
    let (sa, ca) = launch.azimuth_deg.to_radians().sin_cos();
    // Unit vectors (east, north, up) towards higher elevation and higher azimuth.
    let ue = Vec3::new(-se * sa, -se * ca, ce);
    let uc = Vec3::new(ca, -sa, 0.0);
    let u = ue * (d_in / theta) + uc * (d_cross / theta);
    // Rotation by theta about k = d0 x u, which turns the axis d0 towards u (Rodrigues).
    let k = axes[0].cross(u);
    let k = k * (1.0 / k.norm());
    let (st, ct) = theta.to_radians().sin_cos();
    axes = axes.map(|v| v * ct + k.cross(v) * st + k * (k.dot(v) * (1.0 - ct)));
    let (sz, cz) = d_az.to_radians().sin_cos();
    let axes = axes.map(|v| Vec3::new(v.x * cz + v.y * sz, -v.x * sz + v.y * cz, v.z));
    let x = axes[0];
    let h = x.x.hypot(x.y);
    let el = x.z.atan2(h).to_degrees();
    let az = if h == 0.0 { launch.azimuth_deg + d_az } else { x.x.atan2(x.y).to_degrees() };
    // Roll of the turned y_b from the unrolled y_b of the new elevation and azimuth.
    let [_, y0, z0] = Launch { elevation_deg: el, azimuth_deg: az, roll_deg: 0.0, ..launch.clone() }.body_axes_enu();
    let roll = axes[1].dot(z0).atan2(axes[1].dot(y0)).to_degrees();
    (el, wrap180(az), wrap180(roll))
}

/// `a` [deg] wrapped into (-180, 180].
fn wrap180(a: f64) -> f64 {
    let w = (a + 180.0).rem_euclid(360.0) - 180.0;
    if w == -180.0 {
        180.0
    } else {
        w
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
/// The mean needs one point; the covariance and the ellipses need two. Statistics without enough
/// points are NaN (null in the JSON output).
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
    let d = if n > 1 { nf - 1.0 } else { f64::NAN };
    let (a, b, c) = (see / d, sen / d, snn / d);
    let half = 0.5 * (a + c);
    let r = (0.25 * (a - c) * (a - c) + b * b).sqrt();
    // f64::max would turn NaN into 0, so the n < 2 case is handled explicitly.
    let clip = |x: f64| if x.is_nan() { x } else { x.max(0.0) };
    let (l1, l2) = (clip(half + r), clip(half - r));
    // Major axis at angle theta from east (counter-clockwise) -> bearing clockwise from north.
    let theta = 0.5 * (2.0 * b).atan2(a - c);
    let bearing = (90.0 - theta.to_degrees()).rem_euclid(180.0);
    let ell = |k: f64| Ellipse { semi_major: k * l1.sqrt(), semi_minor: k * l2.sqrt(), major_axis_bearing_deg: bearing };
    PointStats { n, mean_east: me, mean_north: mn, covariance: [[a, b], [b, c]], ellipse_1sigma: ell(1.0), ellipse_3sigma: ell(3.0) }
}

#[derive(Debug, Clone, Serialize)]
pub struct DescentStats {
    pub descent: Descent,
    /// Samples that failed (excluded from every statistic); includes those that did not land.
    pub failed: usize,
    /// Samples that did not reach the ground within `sim.max_time` (a subset of `failed`).
    pub not_landed: usize,
    /// False when any sample failed: the statistics then describe only the landed samples and
    /// the true spread is larger or unknown.
    pub complete: bool,
    /// The maximum landing distance is only a lower bound (some samples did not land).
    pub max_landing_distance_is_lower_bound: bool,
    /// Empirical percentiles of the landing distance of the landed samples [m] (linear interpolation).
    pub landing_distance_p50: f64,
    pub landing_distance_p90: f64,
    pub landing_distance_p99: f64,
    /// P99.9 of the landing distance [m]; None (null) with fewer than [`P999_MIN_SAMPLES`] landed samples.
    pub landing_distance_p999: Option<f64>,
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
    /// `dispersion.monte_carlo.truncate_sigma`: the bound of the clipped standard-normal draws (null: untruncated).
    pub truncate_sigma: Option<f64>,
    pub descents: Vec<DescentStats>,
}

#[derive(Debug, Clone)]
pub struct McResult {
    /// Ordered by sample index, then descent mode (ballistic first).
    pub rows: Vec<McRow>,
    pub summary: McSummary,
}

/// Message of a sample that did not reach the ground within `sim.max_time`.
pub const NOT_LANDED: &str = "did not land within sim.max_time";

/// Empirical percentile `q` in [0, 100] of `sorted` (ascending) by linear interpolation; NaN when empty.
pub fn percentile(sorted: &[f64], q: f64) -> f64 {
    match sorted.len() {
        0 => f64::NAN,
        1 => sorted[0],
        n => {
            let x = (q / 100.0).clamp(0.0, 1.0) * (n - 1) as f64;
            let (i, f) = (x.floor() as usize, x.fract());
            sorted[i] + (sorted[(i + 1).min(n - 1)] - sorted[i]) * f
        }
    }
}

/// Statistics over the successful rows of each descent mode present in `rows`; `samples`, `seed`
/// and `truncate_sigma` (the bound of the clipped draws, None: untruncated) are recorded as given.
pub fn summarize(rows: &[McRow], samples: usize, seed: u64, truncate_sigma: Option<f64>) -> McSummary {
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
            let mut dist: Vec<f64> = ok.iter().map(|(_, s)| s.landing_distance).collect();
            dist.sort_by(f64::total_cmp);
            let not_landed = mine.iter().filter(|r| r.result.as_ref().err().is_some_and(|e| e == NOT_LANDED)).count();
            let failed = mine.len() - ok.len();
            let nf = ok.len() as f64;
            let am = ok.iter().map(|(_, s)| s.apogee).sum::<f64>() / nf;
            let var = ok.iter().map(|(_, s)| (s.apogee - am).powi(2)).sum::<f64>() / if ok.len() > 1 { nf - 1.0 } else { f64::NAN };
            DescentStats {
                descent: m,
                failed,
                not_landed,
                complete: failed == 0,
                max_landing_distance_is_lower_bound: failed > 0,
                landing_distance_p50: percentile(&dist, 50.0),
                landing_distance_p90: percentile(&dist, 90.0),
                landing_distance_p99: percentile(&dist, 99.0),
                landing_distance_p999: (dist.len() >= P999_MIN_SAMPLES).then(|| percentile(&dist, 99.9)),
                landing: landing_statistics(&pts),
                max_landing_distance: dmax,
                max_landing_distance_sample: imax,
                apogee_mean: am,
                apogee_std: var.sqrt(),
            }
        })
        .collect();
    McSummary { samples, seed, truncate_sigma, descents }
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
pub fn run_monte_carlo_with_progress(
    cfg: &MonteCarloConfig,
    base: &Simulation,
    recovery_enabled: bool,
    progress: &(dyn Fn(&McRow) + Sync),
) -> Result<McResult> {
    if cfg.samples == 0 {
        bail!("dispersion.monte_carlo.samples must be at least 1");
    }
    let sigmas = [
        ("thrust_misalignment_deg", cfg.thrust_misalignment_deg),
        ("thrust_offset", cfg.thrust_offset),
        ("fin_cant_deg", cfg.fin_cant_deg),
        ("cg_lateral", cfg.cg_lateral),
        ("cl_scale", cfg.cl_scale),
        ("clp_scale", cfg.clp_scale),
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
    if let Some(k) = cfg.truncate_sigma.filter(|k| !(k.is_finite() && *k > 0.0)) {
        bail!("dispersion.monte_carlo.truncate_sigma must be a finite number > 0 (the bound of the clipped draws in sigma, e.g. 3.0), found {k}");
    }
    if cfg.wind_speed != 0.0 || cfg.wind_direction_deg != 0.0 {
        base.wind.check_scalable_for(false)?;
    }
    let mut modes = vec![Descent::Ballistic];
    if recovery_enabled {
        modes.push(Descent::Parachute);
    }
    let rows: Vec<McRow> = (0..cfg.samples)
        .into_par_iter()
        .flat_map_iter(|i| {
            let p = Perturbation::draw(cfg, i, &base.launch);
            let motor = p.motor(base.motor);
            let sim = p.simulation(base, &motor);
            let el = sim.launch.elevation_deg;
            let invalid = !(el > 0.0 && el <= 90.0);
            modes
                .iter()
                .map(|&descent| {
                    let result =
                        match if invalid { Err(anyhow::anyhow!("perturbed rail elevation {el:.3} deg is outside (0, 90]")) } else { sim.run(descent, false) } {
                            Ok(r) if r.summary.landing_east.is_finite() && r.summary.landing_north.is_finite() => Ok(r.summary),
                            Ok(_) => Err(NOT_LANDED.to_string()),
                            Err(e) => Err(format!("{e:#}")),
                        };
                    let row = McRow { sample: i, descent, perturbation: p, result };
                    progress(&row);
                    row
                })
                .collect::<Vec<_>>()
        })
        .collect();
    let summary = summarize(&rows, cfg.samples, cfg.seed, cfg.truncate_sigma);
    Ok(McResult { rows, summary })
}

fn run_wind_grid(
    cfg: &DispersionConfig,
    base: &Simulation,
    recovery_enabled: bool,
    progress: &(dyn Fn(&Case, &Summary) + Sync),
) -> Result<(Vec<Case>, Vec<Summary>)> {
    let (cases, results) = run_wind_grid_detailed(cfg, base, recovery_enabled, progress)?;
    let summaries = cases.iter().zip(results).map(|(c, r)| r.unwrap_or_else(|_| Summary::failed(c.descent, c.wind_speed, c.wind_direction_deg))).collect();
    Ok((cases, summaries))
}

/// Cases and per-case results (the error text of a failed case) of a wind-grid study.
pub type WindGridRuns = (Vec<Case>, Vec<std::result::Result<Summary, String>>);

/// Wind-grid study keeping the error of every failed case (the other cases are not affected by it).
/// Cases and results are in matching order; `progress` is also called for failed cases, with a
/// NaN [`Summary::failed`]. Errors for a calm wind profile, which cannot be scaled to the grid.
pub fn run_wind_grid_detailed(
    cfg: &DispersionConfig,
    base: &Simulation,
    recovery_enabled: bool,
    progress: &(dyn Fn(&Case, &Summary) + Sync),
) -> Result<WindGridRuns> {
    // A calm wind profile cannot be scaled to the grid speeds: stop before any case runs.
    base.wind.check_scalable()?;
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
    let results = cases
        .par_iter()
        .map(|c| {
            let mut s = base.clone();
            // For a profile the case speed and direction are its mean speed and mean-wind direction.
            s.wind = match base.wind.with_speed_direction(c.wind_speed, c.wind_direction_deg) {
                Ok(w) => w,
                Err(e) => {
                    progress(c, &Summary::failed(c.descent, c.wind_speed, c.wind_direction_deg));
                    return Err(format!("{e:#}"));
                }
            };
            match s.run(c.descent, false) {
                Ok(r) => {
                    progress(c, &r.summary);
                    Ok(r.summary)
                }
                Err(e) => {
                    progress(c, &Summary::failed(c.descent, c.wind_speed, c.wind_direction_deg));
                    Err(format!("{e:#}"))
                }
            }
        })
        .collect();
    Ok((cases, results))
}

/// Flies again the wind-grid cases of `cases` without an entry in `errors`, discarding the results. Used
/// after a study with failed cases to rebuild the shared aero-table lookup statistics from the
/// runs that completed (a diverged run leaves out-of-range lookups behind that do not describe
/// the flight): reset the statistics, then call this.
pub fn replay_wind_grid_ok(base: &Simulation, cases: &[Case], errors: &[Option<String>]) {
    cases.par_iter().zip(errors.par_iter()).filter(|(_, e)| e.is_none()).for_each(|(c, _)| {
        let mut s = base.clone();
        if let Ok(w) = base.wind.with_speed_direction(c.wind_speed, c.wind_direction_deg) {
            s.wind = w;
            let _ = s.run(c.descent, false);
        }
    });
}

/// [`replay_wind_grid_ok`] for Monte Carlo rows.
pub fn replay_monte_carlo_ok(base: &Simulation, rows: &[McRow]) {
    rows.par_iter().filter(|r| r.result.is_ok()).for_each(|r| {
        let motor = r.perturbation.motor(base.motor);
        let _ = r.perturbation.simulation(base, &motor).run(r.descent, false);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::env::{AtmosphereModel, Earth, IntegratorKind, Wind, WindConfig, WindModel};
    use crate::flight::{Launch, MassProperties, Recovery, Settings, ThrustLine};
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
            ..Default::default()
        };
        AeroTable::from_fn(meta, |_, _| AeroCoeffs { xcp: 0.5, cna: 2.0, ca_on: 0.5, ca_off: 0.5, ..Default::default() })
    }

    fn motor() -> Motor {
        Motor::new("c".into(), 0.05, 0.3, 0.5, vec![(0.0, 400.0), (2.0, 400.0)])
    }

    /// Vertical launch, constant 5 m/s wind from the west, parachute recovery.
    fn base<'a>(table: &'a AeroTable, motor: &'a Motor) -> Simulation<'a> {
        Simulation {
            table,
            mass: MassProperties::axisymmetric(10.0, 0.5, 0.01, 1.0),
            motor,
            motor_aft_x: 1.0,
            launch: Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: 90.0, azimuth_deg: 0.0, roll_deg: 0.0 },
            recovery: Recovery { enabled: true, cd_s: 1.0, delay: 1.0, main_cd_s: None, main_altitude: None },
            wind: Wind::from_config(&WindConfig { model: WindModel::Constant, speed: 5.0, direction_deg: 270.0, ..Default::default() }).unwrap(),
            atmosphere: AtmosphereModel::default(),
            earth: Earth::default(),
            settings: Settings { integrator: IntegratorKind::Rk45, max_time: 2000.0, ..Default::default() },
            aero_scale: AeroScale::default(),
            thrust_line: ThrustLine::default(),
            fin_cant_deg: 0.0,
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
            thrust_misalignment_deg: 0.0,
            thrust_offset: 0.0,
            fin_cant_deg: 0.0,
            cg_lateral: 0.0,
            cl_scale: 0.0,
            clp_scale: 0.0,
            truncate_sigma: None,
        }
    }

    fn profile_wind(tag: &str, csv: &str) -> Wind {
        let path = std::env::temp_dir().join(format!("ignisyeet_disp_{}_{tag}.csv", std::process::id()));
        std::fs::write(&path, csv).unwrap();
        let w = Wind::from_config(&WindConfig { model: WindModel::Profile, profile: Some(path.clone()), ..Default::default() }).unwrap();
        let _ = std::fs::remove_file(path);
        w
    }

    const CALM_SURFACE: &str = "altitude_m,speed,direction_deg\n0,0,270\n50,6,270\n3000,12,270\n";

    #[test]
    fn calm_surface_profile_wind_grid_keeps_the_winds_aloft() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.wind = profile_wind("grid", CALM_SURFACE);
        let mean = b.wind.speed;
        assert!(mean > 8.0, "{mean}");
        let dcfg = DispersionConfig { wind_speeds: vec![mean, 2.0 * mean], directions: 4, ..Default::default() };
        let (cases, res) = run_wind_grid_detailed(&dcfg, &b, true, &|_, _| {}).unwrap();
        let landing = |v: f64, d: f64| {
            let i = cases.iter().position(|c| c.descent == Descent::Parachute && c.wind_speed == v && c.wind_direction_deg == d).unwrap();
            res[i].as_ref().unwrap().clone()
        };
        // wind grid direction 270 = the profile's own direction: k = 1, drift towards the east
        let s1 = landing(mean, 270.0);
        assert!((s1.wind_scale.unwrap() - 1.0).abs() < 1e-9 && s1.landing_east > 50.0, "{s1:?}");
        let s2 = landing(2.0 * mean, 270.0);
        assert!((s2.wind_scale.unwrap() - 2.0).abs() < 1e-9 && s2.landing_east > 1.5 * s1.landing_east);
        // the other models report no scale
        let c = base(&t, &m).run(Descent::Ballistic, false).unwrap().summary;
        assert_eq!(c.wind_scale, None);
    }

    #[test]
    fn calm_surface_profile_monte_carlo_keeps_the_winds_aloft() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.wind = profile_wind("mc", CALM_SURFACE);
        let mean = b.wind.speed;
        let cfg = MonteCarloConfig { samples: 6, wind_speed: 1.0, ..zero() };
        let r = run_monte_carlo(&cfg, &b, true).unwrap();
        for row in &r.rows {
            let s = row.result.as_ref().unwrap();
            let expect = ((mean + row.perturbation.wind_speed).max(0.0)) / mean;
            assert!((s.wind_scale.unwrap() - expect).abs() < 1e-9, "{} vs {expect}", s.wind_scale.unwrap());
            assert!(s.wind_scale.unwrap() > 0.5 && s.wind_scale.unwrap() < 2.0);
        }
        // the mean landing stays far downwind (the profile used to collapse to a uniform wind
        // of the 2 m value, i.e. none)
        assert!(r.summary.descents[1].landing.mean_east > 50.0, "{:?}", r.summary.descents[1].landing);
    }

    #[test]
    fn calm_profile_is_an_error_for_dispersion() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.wind = profile_wind("calm", "altitude_m,speed,direction_deg\n0,0,0\n100,0.04,0\n");
        let e = run_wind_grid_detailed(&DispersionConfig::default(), &b, false, &|_, _| {}).unwrap_err().to_string();
        assert!(e.contains("wind profile is calm") && e.contains("cannot be scaled for dispersion"), "{e}");
        let e = run_monte_carlo(&MonteCarloConfig { samples: 2, wind_speed: 1.0, ..zero() }, &b, false).unwrap_err().to_string();
        assert!(e.contains("wind profile is calm") && e.contains("Monte Carlo") && !e.contains("wind grid"), "{e}");
        // without a wind sigma nothing is scaled, so the Monte Carlo study runs
        assert!(run_monte_carlo(&MonteCarloConfig { samples: 2, ..zero() }, &b, false).is_ok());
    }

    #[test]
    fn replay_counts_only_the_completed_cases() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let cfg = DispersionConfig { wind_speeds: vec![2.0], directions: 4, ..Default::default() };
        t.reset_out_of_range_stats();
        let (cases, res) = run_wind_grid_detailed(&cfg, &b, false, &|_, _| {}).unwrap();
        let full = t.out_of_range_stats().lookups;
        assert!(full > 0);
        let none: Vec<Option<String>> = vec![None; cases.len()];
        t.reset_out_of_range_stats();
        replay_wind_grid_ok(&b, &cases, &none);
        assert_eq!(t.out_of_range_stats().lookups, full);
        let mut half = none.clone();
        half[0] = Some("diverged".into());
        t.reset_out_of_range_stats();
        replay_wind_grid_ok(&b, &cases, &half);
        let part = t.out_of_range_stats().lookups;
        assert!(part > 0 && part < full, "{part} {full}");
        let all = vec![Some("x".to_string()); cases.len()];
        t.reset_out_of_range_stats();
        replay_wind_grid_ok(&b, &cases, &all);
        assert_eq!(t.out_of_range_stats().lookups, 0);
        assert_eq!(res.len(), cases.len());
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
        assert_eq!(Perturbation::draw(&cfg, 1, &b.launch), Perturbation::draw(&MonteCarloConfig { seed: 2, ..cfg.clone() }, 2, &b.launch));
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
    fn wind_grid_records_errored_cases_instead_of_aborting() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        // An absurd wind speed (1e12 m/s) makes its cases diverge; the finite ones are unaffected.
        let dcfg = DispersionConfig { wind_speeds: vec![2.0, 1.0e12], directions: 2, ..Default::default() };
        let (cases, res) = run_wind_grid_detailed(&dcfg, &b, true, &|_, _| {}).unwrap();
        assert_eq!((cases.len(), res.len()), (8, 8));
        for (c, r) in cases.iter().zip(&res) {
            if c.wind_speed > 1e6 {
                assert!(r.is_err(), "{r:?}");
            } else {
                assert!(r.as_ref().unwrap().landed());
            }
        }
        let (cases, sums) = run(&dcfg, &b, true).unwrap();
        assert_eq!(sums.len(), cases.len());
        assert_eq!(sums.iter().filter(|s| !s.landed()).count(), 4);
    }

    #[test]
    fn parachute_cd_s_scale_applies_to_both_canopies() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.recovery.main_cd_s = Some(4.0);
        b.recovery.main_altitude = Some(100.0);
        let p = Perturbation { parachute_cd_s_scale: 0.5, ..Default::default() };
        let s = p.simulation(&b, &m);
        assert_eq!((s.recovery.cd_s, s.recovery.main_cd_s, s.recovery.main_altitude), (1.5, Some(6.0), Some(100.0)));
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
        let p = Perturbation {
            thrust_scale: 0.1,
            burn_time_scale: 0.25,
            dry_mass: 0.2,
            cg: 0.01,
            cn_scale: 0.1,
            ca_scale: -2.0,
            elevation_deg: -1.0,
            azimuth_deg: 2.0,
            wind_speed: -9.0,
            wind_direction_deg: 100.0,
            parachute_cd_s_scale: 0.5,
            ..Default::default()
        };
        let pm = p.motor(&m);
        let s = p.simulation(&b, &pm);
        assert!((pm.total_impulse() - 1.1 * m.total_impulse()).abs() < 1e-9 && (pm.burn_time() - 2.5).abs() < 1e-12);
        assert_eq!(pm.propellant_mass, m.propellant_mass);
        assert!((s.mass.dry_mass - 12.0).abs() < 1e-12 && (s.mass.iyy_dry - 1.2).abs() < 1e-12 && (s.mass.cg_dry - 0.51).abs() < 1e-12);
        assert_eq!(s.aero_scale, AeroScale { cn: 1.1, ca: 0.05, cl: 1.0, clp: 1.0 });
        assert_eq!((s.launch.elevation_deg, s.launch.azimuth_deg), (89.0, 2.0));
        assert_eq!((s.wind.speed, s.wind.direction_deg), (0.0, 10.0));
        assert!((s.recovery.cd_s - 1.5).abs() < 1e-12);
    }

    #[test]
    fn old_seeds_reproduce_the_old_draws_and_new_sigmas_do_not_move_them() {
        let base = MonteCarloConfig { samples: 4, seed: 11, ..Default::default() };
        let launch = Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: 85.0, azimuth_deg: 270.0, roll_deg: 0.0 };
        let all = MonteCarloConfig {
            thrust_misalignment_deg: 0.3,
            thrust_offset: 0.002,
            fin_cant_deg: 0.1,
            cg_lateral: 0.001,
            cl_scale: 0.2,
            clp_scale: 0.2,
            ..base.clone()
        };
        for i in 0..4 {
            let (a, b) = (Perturbation::draw(&base, i, &launch), Perturbation::draw(&all, i, &launch));
            // The first eleven draws are the original ones, independent of the new sigmas.
            assert_eq!(
                (
                    a.thrust_scale,
                    a.burn_time_scale,
                    a.dry_mass,
                    a.cg,
                    a.cn_scale,
                    a.ca_scale,
                    a.elevation_deg,
                    a.azimuth_deg,
                    a.wind_speed,
                    a.wind_direction_deg,
                    a.parachute_cd_s_scale
                ),
                (
                    b.thrust_scale,
                    b.burn_time_scale,
                    b.dry_mass,
                    b.cg,
                    b.cn_scale,
                    b.ca_scale,
                    b.elevation_deg,
                    b.azimuth_deg,
                    b.wind_speed,
                    b.wind_direction_deg,
                    b.parachute_cd_s_scale
                )
            );
            // Zero sigmas draw exact zeros for the new terms.
            assert_eq!((a.misalignment_y_deg, a.offset_z, a.fin_cant_deg, a.cg_lateral_y, a.cl_scale, a.clp_scale), (0.0, 0.0, 0.0, 0.0, 0.0, 0.0));
            assert!(
                b.misalignment_y_deg != 0.0 && b.offset_y != 0.0 && b.fin_cant_deg != 0.0 && b.cg_lateral_z != 0.0 && b.cl_scale != 0.0 && b.clp_scale != 0.0
            );
        }
        // Frozen first draw of seed 11 (guards the draw order of the original eleven).
        let p = Perturbation::draw(&base, 0, &launch);
        let mut rng = ChaCha8Rng::seed_from_u64(11);
        let z: f64 = StandardNormal.sample(&mut rng);
        assert_eq!(p.thrust_scale, base.thrust_scale * z);
    }

    #[test]
    fn new_perturbations_act_on_the_simulation() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.thrust_line = ThrustLine { misalignment_deg: 1.0, misalignment_azimuth_deg: 0.0, offset: [0.001, 0.0] };
        b.fin_cant_deg = 0.5;
        let p = Perturbation {
            misalignment_z_deg: 1.0,
            offset_z: 0.002,
            fin_cant_deg: 0.25,
            cg_lateral_y: 0.003,
            cl_scale: 0.5,
            clp_scale: -0.5,
            ..Default::default()
        };
        let pm = p.motor(&m);
        let s = p.simulation(&b, &pm);
        // Nominal tilt (1, 0) deg plus (0, 1) deg.
        assert!((s.thrust_line.misalignment_deg - 2f64.sqrt()).abs() < 1e-12 && (s.thrust_line.misalignment_azimuth_deg - 45.0).abs() < 1e-9);
        assert_eq!(s.thrust_line.offset, [0.001, 0.002]);
        assert_eq!((s.fin_cant_deg, s.mass.cg_lateral_dry), (0.75, [0.003, 0.0]));
        assert_eq!((s.aero_scale.cl, s.aero_scale.clp), (1.5, 0.5));
        // Zero perturbation leaves the simulation exactly nominal.
        let z = Perturbation::default().simulation(&b, &m);
        assert_eq!((z.thrust_line, z.fin_cant_deg, z.aero_scale), (b.thrust_line, b.fin_cant_deg, AeroScale::default()));
        assert!(z.mass.is_axisymmetric());
    }

    fn landed_summary(d: f64) -> Summary {
        Summary {
            descent: Descent::Ballistic,
            wind_speed: 0.0,
            wind_direction_deg: 0.0,
            rail_exit_time: 0.1,
            rail_exit_speed: 20.0,
            burnout_time: 1.0,
            max_speed: 100.0,
            max_mach: 0.3,
            max_dyn_pressure: 1000.0,
            apogee_time: 10.0,
            apogee: 500.0,
            apogee_east: 0.0,
            apogee_north: 0.0,
            min_stability_cal: 2.0,
            stability_at_rail_exit: 2.0,
            static_margin_pitch_min_cal: 2.0,
            static_margin_yaw_min_cal: 2.0,
            static_margin_min_t: 1.0,
            static_margin_min_mach: 0.3,
            stability_unknown: None,
            broadside_time: 0.0,
            landing_time: 30.0,
            landing_east: d,
            landing_north: 0.0,
            landing_lat: 35.0,
            landing_lon: 139.0,
            landing_distance: d,
            landing_speed: 10.0,
            main_deploy_time: None,
            main_deploy_altitude: None,
            drogue_deploy_time: None,
            deploy_delayed: None,
            drogue_deploy_speed: None,
            drogue_deploy_q: None,
            main_deploy_speed: None,
            main_deploy_q: None,
            wind_scale: None,
        }
    }

    #[test]
    fn percentiles_interpolate() {
        let v = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert_eq!(percentile(&v, 50.0), 3.0);
        assert_eq!(percentile(&v, 0.0), 1.0);
        assert_eq!(percentile(&v, 100.0), 5.0);
        assert!((percentile(&v, 90.0) - 4.6).abs() < 1e-12);
        assert!(percentile(&[], 50.0).is_nan());
    }

    #[test]
    fn non_landing_samples_make_statistics_incomplete() {
        let p = Perturbation::default();
        let mk = |sample, result| McRow { sample, descent: Descent::Ballistic, perturbation: p, result };
        let rows = vec![
            mk(0, Ok(landed_summary(100.0))),
            mk(1, Ok(landed_summary(200.0))),
            mk(2, Ok(landed_summary(300.0))),
            mk(3, Err(NOT_LANDED.to_string())),
            mk(4, Err("never left the rail".to_string())),
        ];
        let st = &summarize(&rows, 5, 1, None).descents[0];
        assert_eq!((st.failed, st.not_landed, st.landing.n), (2, 1, 3));
        assert!(!st.complete && st.max_landing_distance_is_lower_bound);
        assert_eq!(st.max_landing_distance, 300.0);
        assert_eq!((st.landing_distance_p50, st.landing_distance_p90), (200.0, 280.0));
        // Three landed samples: no P99.9.
        assert_eq!(st.landing_distance_p999, None);
        let many: Vec<McRow> = (0..1000).map(|i| mk(i, Ok(landed_summary(i as f64)))).collect();
        let st = &summarize(&many, 1000, 1, None).descents[0];
        assert!((st.landing_distance_p999.unwrap() - 998.001).abs() < 1e-6, "{:?}", st.landing_distance_p999);
        let few = summarize(&many[..999], 999, 1, None);
        assert_eq!(few.descents[0].landing_distance_p999, None);
        let all = summarize(&rows[..3], 3, 1, None);
        assert!(all.descents[0].complete && !all.descents[0].max_landing_distance_is_lower_bound);
    }

    #[test]
    fn too_few_samples_give_nan_statistics() {
        let st0 = landing_statistics(&[]);
        assert!(st0.mean_east.is_nan() && st0.covariance[0][0].is_nan() && st0.ellipse_1sigma.semi_major.is_nan());
        let st1 = landing_statistics(&[(3.0, 4.0)]);
        assert_eq!((st1.mean_east, st1.mean_north), (3.0, 4.0));
        assert!(st1.covariance[0][0].is_nan() && st1.ellipse_3sigma.semi_major.is_nan() && st1.ellipse_1sigma.semi_minor.is_nan());
        let mk = |sample, result| McRow { sample, descent: Descent::Ballistic, perturbation: Perturbation::default(), result };
        let none = summarize(&[mk(0, Err("never left the rail".into()))], 1, 1, None);
        let d = &none.descents[0];
        assert!(d.apogee_mean.is_nan() && d.apogee_std.is_nan() && d.landing.ellipse_1sigma.semi_major.is_nan());
        assert!(serde_json::to_string(d).unwrap().contains("\"apogee_std\":null"));
        let one = summarize(&[mk(0, Ok(landed_summary(100.0)))], 1, 1, None);
        assert!(one.descents[0].apogee_mean.is_finite() && one.descents[0].apogee_std.is_nan());
    }

    #[test]
    fn short_max_time_gives_unlanded_summaries() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.settings.max_time = 5.0;
        let r = b.run(Descent::Ballistic, false).unwrap();
        assert!(!r.summary.landed() && r.summary.landing_distance.is_nan());
        b.settings.max_time = 2000.0;
        assert!(b.run(Descent::Ballistic, false).unwrap().summary.landed());
    }

    #[test]
    fn new_monte_carlo_sigmas_are_validated() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        for (name, mutate) in [
            ("thrust_misalignment_deg", (|c: &mut MonteCarloConfig| c.thrust_misalignment_deg = -1.0) as fn(&mut MonteCarloConfig)),
            ("thrust_offset", |c| c.thrust_offset = f64::NAN),
            ("fin_cant_deg", |c| c.fin_cant_deg = -0.1),
            ("cg_lateral", |c| c.cg_lateral = f64::INFINITY),
            ("cl_scale", |c| c.cl_scale = -0.1),
            ("clp_scale", |c| c.clp_scale = -0.1),
        ] {
            let mut c = zero();
            mutate(&mut c);
            let e = run_monte_carlo(&c, &b, false).err().unwrap().to_string();
            assert!(e.contains(name), "{name}: {e}");
        }
    }

    #[test]
    fn unknown_config_keys_are_rejected() {
        use crate::flight::{Launch, Recovery};
        let l = r#"latitude = 1.0
longitude = 2.0
altitude = 0.0
rail_length = 3.0
elevation_deg = 85.0
azimuth_deg = 0.0"#;
        assert!(toml::from_str::<Launch>(l).is_ok());
        let e = toml::from_str::<Launch>(&format!("{l}\nrol_deg = 5.0")).unwrap_err().to_string();
        assert!(e.contains("rol_deg"), "{e}");
        let r = "enabled = true\ncd_s = 1.0\ndelay = 1.0";
        assert!(toml::from_str::<Recovery>(r).is_ok());
        let e = toml::from_str::<Recovery>(&format!("{r}\ndeploy_altitude = 100.0")).unwrap_err().to_string();
        assert!(e.contains("deploy_altitude"), "{e}");
    }

    #[test]
    fn rail_elevation_outside_the_valid_range_fails_the_sample() {
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.launch.elevation_deg = 2.0;
        let cfg = MonteCarloConfig { samples: 20, elevation_deg: 3.0, ..zero() };
        let r = run_monte_carlo(&cfg, &b, true).unwrap();
        let bad: Vec<&McRow> = r.rows.iter().filter(|x| x.result.as_ref().is_err_and(|e| e.contains("perturbed rail elevation"))).collect();
        assert!(!bad.is_empty(), "no sample drew an elevation <= 0");
        for x in &bad {
            let el = 2.0 + x.perturbation.elevation_deg;
            assert!(el <= 0.0, "{el}");
            assert_eq!(x.result.as_ref().unwrap_err(), &format!("perturbed rail elevation {el:.3} deg is outside (0, 90]"));
        }
        assert!(r.rows.iter().any(|x| x.result.is_ok()));
        assert_eq!(r.summary.descents[0].failed, bad.len() / 2);
    }

    fn site(el: f64, az: f64, roll: f64) -> Launch {
        Launch { latitude: 35.0, longitude: 139.0, altitude: 0.0, rail_length: 3.0, elevation_deg: el, azimuth_deg: az, roll_deg: roll }
    }

    /// Rail elevation and azimuth after `tilt_launch`.
    fn tilt_rail(el: f64, az: f64, d_in: f64, d_cross: f64, d_az: f64) -> (f64, f64) {
        let (e, a, _) = tilt_launch(&site(el, az, 0.0), d_in, d_cross, d_az);
        (e, a)
    }

    #[test]
    fn rail_tilt_rotates_the_axis() {
        let close = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() < 1e-9 && (a.1 - b.1).abs() < 1e-9;
        // Vertical rail: an in-plane tilt leans it towards the back azimuth, a cross-plane tilt
        // towards azimuth + 90 deg; both give the same elevation.
        assert!(close(tilt_rail(90.0, 0.0, 1.0, 0.0, 0.0), (89.0, 180.0)), "{:?}", tilt_rail(90.0, 0.0, 1.0, 0.0, 0.0));
        assert!(close(tilt_rail(90.0, 0.0, -1.0, 0.0, 0.0), (89.0, 0.0)));
        assert!(close(tilt_rail(90.0, 0.0, 0.0, 1.0, 0.0), (89.0, 90.0)), "{:?}", tilt_rail(90.0, 0.0, 0.0, 1.0, 0.0));
        // Inclined rail: in-plane adds to the elevation, the azimuth rotation adds to the azimuth.
        assert!(close(tilt_rail(45.0, 30.0, 1.0, 0.0, 0.0), (46.0, 30.0)));
        assert!(close(tilt_rail(45.0, 30.0, 0.0, 0.0, 2.0), (45.0, 32.0)));
        // A small cross-plane tilt d changes the azimuth by d / cos(elevation) and the elevation
        // only to second order; the angle between the axes is d.
        let (el, az) = tilt_rail(45.0, 30.0, 0.0, 0.01, 0.0);
        assert!((az - 30.0 - 0.01 / 45f64.to_radians().cos()).abs() < 1e-6 && (el - 45.0).abs() < 1e-5, "{el} {az}");
        let (d0, d1) = (site(45.0, 30.0, 0.0).rail_dir_enu(), site(el, az, 0.0).rail_dir_enu());
        let ang = d0.dot(d1).clamp(-1.0, 1.0).acos().to_degrees();
        assert!((ang - 0.01).abs() < 1e-9, "{ang}");
    }

    /// Largest angle [deg] between corresponding body axes of two launch attitudes.
    fn axes_angle(a: &Launch, b: &Launch) -> f64 {
        let (a, b) = (a.body_axes_enu(), b.body_axes_enu());
        (0..3).map(|i| a[i].dot(b[i]).clamp(-1.0, 1.0).acos().to_degrees()).fold(0.0, f64::max)
    }

    #[test]
    fn small_rail_tilt_moves_the_body_axes_only_a_little() {
        // A tilt of a vertical rail flips the azimuth; the body must not flip with it, so thrust
        // offsets, lateral CG offsets, fin cant and asymmetric aerodynamics keep their direction.
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        for (el, az, roll) in [(90.0, 0.0, 0.0), (90.0, 270.0, 30.0), (85.0, 270.0, 0.0), (85.0, 0.0, -60.0), (45.0, 30.0, 10.0)] {
            let nominal = site(el, az, roll);
            for (d_in, d_cross, d_az) in [
                (1e-3, 0.0, 0.0),
                (-1e-3, 0.0, 0.0),
                (0.0, 1e-3, 0.0),
                (0.0, -1e-3, 0.0),
                (7e-4, -7e-4, 0.0),
                (0.5, 0.3, 0.0),
                (0.0, 0.0, 1e-3),
                (2e-4, 1e-4, 3e-4),
            ] {
                let (e, a, r) = tilt_launch(&nominal, d_in, d_cross, d_az);
                let ang = axes_angle(&nominal, &site(e, a, r));
                let bound = d_in.hypot(d_cross) + f64::abs(d_az);
                assert!(ang <= bound * (1.0 + 1e-6) + 1e-9, "rail ({el}, {az}, {roll}) tilt ({d_in}, {d_cross}, {d_az}): axes moved {ang} deg");
                // The recorded perturbation reproduces the turned attitude in the simulation.
                let p = Perturbation { elevation_deg: e - el, azimuth_deg: wrap180(a - az), roll_deg: wrap180(r - roll), ..Default::default() };
                let mut bb = b.clone();
                bb.launch = nominal.clone();
                let s = p.simulation(&bb, &m);
                let ang = axes_angle(&nominal, &s.launch);
                assert!(ang <= bound * (1.0 + 1e-6) + 1e-9, "rail ({el}, {az}, {roll}) tilt ({d_in}, {d_cross}, {d_az}): flown axes moved {ang} deg");
            }
        }
        // The flip of a vertical rail: azimuth 180 deg, roll 180 deg, the elevation and azimuth
        // columns still give the tilted rail.
        let (e, a, r) = tilt_launch(&site(90.0, 0.0, 0.0), 1e-3, 0.0, 0.0);
        assert!((e - (90.0 - 1e-3)).abs() < 1e-9 && (a - 180.0).abs() < 1e-9 && (r.abs() - 180.0).abs() < 1e-9, "{e} {a} {r}");
        // A pure azimuth rotation of a vertical rail rolls the rocket with the launcher.
        let (e, a, r) = tilt_launch(&site(90.0, 10.0, 5.0), 0.0, 0.0, 2.0);
        assert_eq!((e, a, r), (90.0, 12.0, 5.0));
    }

    /// Fingerprint of the default draws (sum of the drawn fields over samples 0..200), fixed so that a change
    /// of the draw order or of the default distribution shows up.
    fn draw_fingerprint(cfg: &MonteCarloConfig) -> [f64; 3] {
        let launch = site(85.0, 270.0, 0.0);
        let mut s = [0.0; 3];
        for i in 0..200 {
            let p = Perturbation::draw(cfg, i, &launch);
            s[0] += p.thrust_scale + p.ca_scale + p.cn_scale;
            s[1] += p.wind_speed + p.wind_direction_deg;
            s[2] += p.elevation_deg + p.azimuth_deg;
        }
        s
    }

    #[test]
    fn default_draws_are_unchanged_and_truncation_clips_the_standard_normal() {
        let base = MonteCarloConfig::default();
        let f = draw_fingerprint(&base);
        println!("fingerprint {:?}", f.map(|v| format!("{v:.15e}")));
        let expect = [1.089_999_041_445_204, 300.304_481_347_577_6, -129.571_375_225_135];
        for k in 0..3 {
            assert!((f[k] - expect[k]).abs() < 1e-9 * expect[k].abs().max(1.0), "{k}: {} vs {}", f[k], expect[k]);
        }
        assert_eq!(base.truncate_sigma, None);
        // A bound beyond every draw changes nothing.
        let wide = MonteCarloConfig { truncate_sigma: Some(1e6), ..base.clone() };
        let launch = site(85.0, 270.0, 0.0);
        for i in 0..200 {
            assert_eq!(Perturbation::draw(&wide, i, &launch), Perturbation::draw(&base, i, &launch));
        }
        // Clipping at 1 sigma: every relative draw within +-sigma, the ones inside unchanged, the others on the bound.
        let one = MonteCarloConfig { truncate_sigma: Some(1.0), ..base.clone() };
        let (mut clipped, mut kept) = (0, 0);
        for i in 0..200 {
            let (a, b) = (Perturbation::draw(&base, i, &launch), Perturbation::draw(&one, i, &launch));
            for (sigma, x, y) in
                [(base.ca_scale, a.ca_scale, b.ca_scale), (base.thrust_scale, a.thrust_scale, b.thrust_scale), (base.wind_speed, a.wind_speed, b.wind_speed)]
            {
                assert!(y.abs() <= sigma * (1.0 + 1e-12));
                if x.abs() <= sigma {
                    assert_eq!(x, y);
                    kept += 1;
                } else {
                    assert!((y - sigma * x.signum()).abs() < 1e-15);
                    clipped += 1;
                }
            }
        }
        assert!(clipped > 50 && kept > 300, "{clipped} {kept}");
    }

    #[test]
    fn drawn_rail_perturbation_turns_the_body_with_the_rail() {
        // Draws with a tiny tilt sigma: the flown body axes stay within a few sigma of nominal.
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        let cfg = MonteCarloConfig { samples: 50, elevation_deg: 1e-3, azimuth_deg: 1e-3, ..zero() };
        for i in 0..50 {
            let p = Perturbation::draw(&cfg, i, &b.launch);
            let ang = axes_angle(&b.launch, &p.simulation(&b, &m).launch);
            assert!(ang < 1e-2, "sample {i}: body axes moved {ang} deg ({p:?})");
        }
    }

    #[test]
    fn perturbed_elevation_stays_within_90_deg() {
        // Rails near the vertical tilted through it by large draws: the perturbed elevation is the
        // `tilt_launch` one, never above 90 deg.
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        for el in [90.0, 89.99, 89.0, 85.0, 30.0] {
            b.launch.elevation_deg = el;
            let cfg = MonteCarloConfig { samples: 400, elevation_deg: 3.0, azimuth_deg: 5.0, ..zero() };
            for i in 0..400 {
                let p = Perturbation::draw(&cfg, i, &b.launch);
                let e = p.simulation(&b, &m).launch.elevation_deg;
                assert!((-90.0..=90.0).contains(&e), "el {el}, sample {i}: {e}");
            }
        }
    }

    #[test]
    fn rail_tilt_spread_is_isotropic_for_a_vertical_rail() {
        // Vertical launch in calm air with only the rail tilt dispersed: the ballistic landing
        // points spread equally in every direction.
        let (t, m) = (table(), motor());
        let mut b = base(&t, &m);
        b.wind = Wind::from_config(&WindConfig { model: WindModel::Constant, speed: 0.0, ..Default::default() }).unwrap();
        let cfg = MonteCarloConfig { samples: 200, elevation_deg: 2.0, azimuth_deg: 5.0, ..zero() };
        let r = run_monte_carlo(&cfg, &b, false).unwrap();
        let e = r.summary.descents[0].landing.ellipse_1sigma;
        assert!(e.semi_minor > 0.7 * e.semi_major, "1-sigma ellipse {} x {} m", e.semi_major, e.semi_minor);
        // The recorded elevation/azimuth changes reproduce the flown rail.
        for row in r.rows.iter().take(5) {
            let s = row.perturbation.simulation(&b, &m);
            assert!(s.launch.elevation_deg <= 90.0 && s.launch.elevation_deg > 80.0);
        }
    }

    #[test]
    fn monte_carlo_validates_and_reports_truncate_sigma() {
        let (t, m) = (table(), motor());
        let b = base(&t, &m);
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let cfg = MonteCarloConfig { samples: 1, truncate_sigma: Some(bad), ..Default::default() };
            let e = run_monte_carlo(&cfg, &b, false).err().map(|e| e.to_string()).unwrap_or_default();
            assert!(e.contains("dispersion.monte_carlo.truncate_sigma must be a finite number > 0"), "{bad}: {e}");
        }
        let r = run_monte_carlo(&MonteCarloConfig { samples: 2, truncate_sigma: Some(2.5), ..Default::default() }, &b, false).unwrap();
        assert_eq!(r.summary.truncate_sigma, Some(2.5));
        // summarize() carries the bound it is given.
        assert_eq!(summarize(&r.rows, 2, 1, Some(2.5)).truncate_sigma, Some(2.5));
        assert_eq!(summarize(&r.rows, 2, 1, None).truncate_sigma, None);
    }

    #[test]
    fn every_dispersion_run_reports_its_stability() {
        // A table with a normal force: the static margin is that of the force the flight applies
        // (the fixture `table()` has CN = 0, whose margin is unknown).
        let t =
            AeroTable::from_fn(table().meta, |_, a| AeroCoeffs { cn: 2.0 * a.to_radians(), xcp: 0.5, cna: 2.0, ca_on: 0.5, ca_off: 0.5, ..Default::default() });
        let m = motor();
        let b = base(&t, &m);
        let r = run_monte_carlo(&MonteCarloConfig { samples: 4, ..Default::default() }, &b, true).unwrap();
        for row in &r.rows {
            let s = row.result.as_ref().unwrap();
            assert!(s.min_stability_cal.is_finite() && s.stability_at_rail_exit.is_finite(), "{s:?}");
        }
        let dcfg = DispersionConfig { wind_speeds: vec![2.0], directions: 2, ..Default::default() };
        let (_, sums) = run(&dcfg, &b, true).unwrap();
        assert!(sums.iter().all(|s| s.min_stability_cal.is_finite() && s.stability_at_rail_exit.is_finite()));
    }
}
