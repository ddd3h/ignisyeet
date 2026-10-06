// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Environment models: wind, atmosphere, Earth/gravity and integrator selection.
//! The `*Config` types are deserialized from TOML; the runtime types are built from them
//! and return an error for variants that are not implemented yet.

pub use aero::atmosphere::AtmosphereModel;
use aero::atmosphere::G0;
use anyhow::{bail, Context, Result};
use geom::Vec3;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub(crate) const R_EARTH: f64 = 6_371_000.0;

/// True for finite-or-infinite numbers greater than zero (false for NaN).
pub fn is_pos(x: f64) -> bool {
    x > 0.0
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WindModel {
    /// Speed independent of height.
    Constant,
    /// Power law v = v_ref (h / h_ref)^(1/n).
    #[default]
    Power,
    /// Logarithmic boundary-layer profile.
    Log,
    /// Tabulated altitude profile read from a CSV file.
    Profile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct WindConfig {
    pub model: WindModel,
    /// Speed at the reference height [m/s] (constant, power and log models; ignored by the
    /// profile model, whose speeds come from the CSV).
    pub speed: f64,
    /// Direction the wind blows FROM, clockwise from north [deg] (ignored by the profile model).
    pub direction_deg: f64,
    pub ref_height: f64,
    /// Profile model: mean speed [m/s] the profile is scaled to (`k = v / W`, see
    /// [`Wind::with_speed_direction`]). None keeps the CSV speeds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_mean_speed: Option<f64>,
    /// Profile model: direction [deg] the profile's mean wind vector is rotated to (every row is
    /// rotated by `psi - Psi`). None keeps the CSV directions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_mean_direction_deg: Option<f64>,
    /// Power-law exponent n (power model).
    pub exponent: f64,
    /// Surface roughness length [m] (log model).
    pub roughness_length: f64,
    /// Height below which the power/log speed profile is blended to zero with a cubic Hermite
    /// polynomial [m]. Must be positive and smaller than `ref_height`.
    ///
    /// The blend removes the infinite (power) or discontinuous (log) derivative of the profile at
    /// the ground, which would otherwise force the adaptive integrators to take vanishingly small
    /// steps near touchdown. For the log model the value must also exceed the roughness length
    /// `roughness_length` (`z0`) so that the analytic derivative at the blend height exists; a
    /// configured value at or below `z0` is read as `max(value, 2 z0)`.
    pub ground_blend_height: f64,
    /// CSV with columns altitude_m,speed,direction_deg (profile model).
    pub profile: Option<PathBuf>,
}

impl Default for WindConfig {
    fn default() -> Self {
        Self {
            model: WindModel::Power,
            speed: 4.0,
            direction_deg: 0.0,
            ref_height: 2.0,
            profile_mean_speed: None,
            profile_mean_direction_deg: None,
            exponent: 6.0,
            roughness_length: 0.03,
            ground_blend_height: 1.0,
            profile: None,
        }
    }
}

/// One row of a wind profile: height above ground [m], speed [m/s], direction the wind blows FROM [deg].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProfilePoint {
    pub height: f64,
    pub speed: f64,
    pub direction_deg: f64,
}

impl ProfilePoint {
    /// Wind velocity (east, north) [m/s].
    fn vector(&self) -> (f64, f64) {
        let d = self.direction_deg.to_radians();
        (-self.speed * d.sin(), -self.speed * d.cos())
    }
}

/// Runtime wind field in the local ENU frame.
///
/// For the `profile` model `speed` and `direction_deg` are not taken from the configuration but
/// are the profile's representative wind: the mean speed over the profile's altitude range and
/// the direction of its altitude-averaged wind vector (see [`Wind::profile_mean`]), or the
/// targets `profile_mean_speed` / `profile_mean_direction_deg` of the configuration, to which
/// the profile is then scaled and rotated. They are what the summary reports and what
/// [`Wind::with_speed_direction`] changes.
#[derive(Debug, Clone)]
pub struct Wind {
    pub model: WindModel,
    /// Speed at the reference height [m/s].
    pub speed: f64,
    /// Direction the wind blows FROM, clockwise from north [deg].
    pub direction_deg: f64,
    pub ref_height: f64,
    /// Power-law exponent n in v = v_ref (h / h_ref)^(1/n).
    pub exponent: f64,
    /// Roughness length z0 of the logarithmic profile [m].
    pub roughness_length: f64,
    /// Cubic Hermite ground blend height [m] (power and log models); effective log value is
    /// `max(configured, 2 z0)`.
    pub ground_blend_height: f64,
    profile: Vec<ProfilePoint>,
    /// Profile model: factor the tabulated profile has been multiplied by relative to the CSV
    /// (1 for the unscaled profile and for every other model).
    pub profile_scale: f64,
}

/// A profile whose mean speed is below this [m/s] cannot be scaled to another mean speed.
pub const CALM_PROFILE_SPEED: f64 = 0.1;

/// Parses a wind profile CSV (`altitude_m,speed,direction_deg`, ascending altitude above ground).
pub fn parse_wind_profile(text: &str) -> Result<Vec<ProfilePoint>> {
    // A UTF-8 byte order mark (Excel "CSV UTF-8") is not part of the header.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
    let header = lines.next().context("wind profile is empty")?;
    let cols: Vec<String> = header.split(',').map(|c| c.trim().to_ascii_lowercase()).collect();
    if cols != ["altitude_m", "speed", "direction_deg"] {
        bail!("wind profile header must be \"altitude_m,speed,direction_deg\", found \"{header}\"");
    }
    let mut pts: Vec<ProfilePoint> = Vec::new();
    for l in lines {
        let v: Vec<f64> =
            l.split(',').map(|x| x.trim().parse::<f64>()).collect::<std::result::Result<_, _>>().with_context(|| format!("bad wind profile row \"{l}\""))?;
        if v.len() != 3 || v.iter().any(|x| !x.is_finite()) {
            bail!("bad wind profile row \"{l}\": expected 3 finite numbers");
        }
        if v[1] < 0.0 {
            bail!("wind profile speed must not be negative (row \"{l}\")");
        }
        if !(0.0..=360.0).contains(&v[2]) {
            bail!("wind profile direction must be between 0 and 360 degrees (row \"{l}\")");
        }
        if v[0] < 0.0 {
            bail!("wind profile altitude must be ≥ 0 m above the launch site (row \"{l}\"): drop levels below the launch site and add a 0 m row");
        }
        if let Some(p) = pts.last() {
            if v[0] <= p.height {
                bail!("wind profile altitudes must be strictly ascending (row \"{l}\")");
            }
        }
        pts.push(ProfilePoint { height: v[0], speed: v[1], direction_deg: v[2] });
    }
    if pts.is_empty() {
        bail!("wind profile has no data rows");
    }
    Ok(pts)
}

fn direction_from(e: f64, n: f64) -> f64 {
    (-e).atan2(-n).to_degrees().rem_euclid(360.0)
}

impl Wind {
    pub fn from_config(c: &WindConfig) -> Result<Self> {
        if !is_pos(c.ref_height) {
            bail!("wind.ref_height must be positive");
        }
        if !(c.speed.is_finite() && c.speed >= 0.0) {
            bail!("wind.speed must be a finite number >= 0, found {}", c.speed);
        }
        if !c.direction_deg.is_finite() {
            bail!("wind.direction_deg must be a finite number, found {}", c.direction_deg);
        }
        if matches!(c.model, WindModel::Power | WindModel::Log) && !(is_pos(c.ground_blend_height) && c.ground_blend_height < c.ref_height) {
            bail!("wind.ground_blend_height must be positive and smaller than wind.ref_height");
        }
        match c.model {
            WindModel::Constant => {}
            WindModel::Power => {
                if !is_pos(c.exponent) {
                    bail!("wind.exponent must be positive");
                }
            }
            WindModel::Log => {
                if !is_pos(c.roughness_length) {
                    bail!("wind.roughness_length must be positive");
                }
                // The blend height is raised to at least 2 z0 (see `with_profile`) and must stay below
                // ref_height, where the configured speed applies.
                if 2.0 * c.roughness_length >= c.ref_height {
                    bail!("wind.roughness_length ({}) must be smaller than half of wind.ref_height ({}) for the log profile", c.roughness_length, c.ref_height);
                }
            }
            WindModel::Profile => {
                let path = c.profile.as_deref().context("wind.profile (CSV path) is required when wind.model = \"profile\"")?;
                let text = std::fs::read_to_string(path).with_context(|| format!("cannot read wind profile {}", path.display()))?;
                let profile = parse_wind_profile(&text).with_context(|| format!("invalid wind profile {}", path.display()))?;
                let mut w = Self::with_profile(c, profile);
                let (speed, direction_deg) = w.profile_mean();
                w.speed = speed;
                w.direction_deg = direction_deg;
                if c.profile_mean_speed.is_none() && c.profile_mean_direction_deg.is_none() {
                    return Ok(w);
                }
                let target = c.profile_mean_speed.unwrap_or(speed);
                if !(target.is_finite() && target >= 0.0) {
                    bail!("wind.profile_mean_speed must be a finite number >= 0, found {target}");
                }
                let dir = c.profile_mean_direction_deg.unwrap_or(direction_deg);
                if !(dir.is_finite() && (0.0..=360.0).contains(&dir)) {
                    bail!("wind.profile_mean_direction_deg must be between 0 and 360 degrees, found {dir}");
                }
                if speed < CALM_PROFILE_SPEED {
                    bail!(
                        "wind profile is calm (mean speed {speed:.3} m/s): it cannot be scaled to wind.profile_mean_speed / rotated to wind.profile_mean_direction_deg"
                    );
                }
                // The same scaling and rotation as a wind-grid case with this speed and direction.
                return w.with_speed_direction(target, dir);
            }
        }
        Ok(Self::with_profile(c, Vec::new()))
    }

    fn with_profile(c: &WindConfig, profile: Vec<ProfilePoint>) -> Self {
        let ground_blend_height = if c.model == WindModel::Log && c.ground_blend_height <= c.roughness_length {
            c.ground_blend_height.max(2.0 * c.roughness_length)
        } else {
            c.ground_blend_height
        };
        Self {
            model: c.model,
            speed: c.speed,
            direction_deg: c.direction_deg,
            ref_height: c.ref_height,
            exponent: c.exponent,
            roughness_length: c.roughness_length,
            ground_blend_height,
            profile,
            profile_scale: 1.0,
        }
    }

    /// Representative wind of the profile: `(W, Psi)` with `W` the mean speed over the profile's
    /// altitude range (trapezoidal average of the interpolated vector magnitude `|w(h)|` from the
    /// lowest to the highest row; the speed of the row for a single-row profile) and `Psi` the
    /// direction the altitude-averaged wind vector blows FROM [deg]. If that mean vector is ~0 the
    /// direction at `ref_height` is used (0 when that wind is calm too). Other models:
    /// `(speed, direction_deg)`.
    pub fn profile_mean(&self) -> (f64, f64) {
        let p = &self.profile;
        if self.model != WindModel::Profile || p.is_empty() {
            return (self.speed, self.direction_deg);
        }
        let fallback = || {
            let (e, n) = self.vector_at(self.ref_height);
            if e.hypot(n) > 1e-9 {
                direction_from(e, n)
            } else {
                0.0
            }
        };
        let (h0, h1) = (p[0].height, p[p.len() - 1].height);
        if p.len() == 1 || h1 <= h0 {
            return (p[0].speed, if p[0].speed > 0.0 { p[0].direction_deg } else { fallback() });
        }
        // |w| is not linear between rows (veer), so each segment is integrated with 64
        // trapezoids; the vector components are linear, so their mean is exact.
        const SUB: usize = 64;
        let (mut mag, mut me, mut mn) = (0.0, 0.0, 0.0);
        for seg in p.windows(2) {
            let (a, b) = (seg[0], seg[1]);
            let dh = b.height - a.height;
            let mut prev: Option<f64> = None;
            for i in 0..=SUB {
                let (e, n) = self.vector_at(a.height + dh * i as f64 / SUB as f64);
                let m = e.hypot(n);
                if let Some(q) = prev {
                    mag += 0.5 * (q + m) * dh / SUB as f64;
                }
                prev = Some(m);
            }
            let ((ae, an), (be, bn)) = (a.vector(), b.vector());
            me += 0.5 * (ae + be) * dh;
            mn += 0.5 * (an + bn) * dh;
        }
        let range = h1 - h0;
        let (w, me, mn) = (mag / range, me / range, mn / range);
        let dir = if me.hypot(mn) > 1e-9 { direction_from(me, mn) } else { fallback() };
        (w, dir)
    }

    /// Ok when this wind can be scaled to another speed for a dispersion study: always, except for
    /// a calm `profile` (mean speed below [`CALM_PROFILE_SPEED`]).
    pub fn check_scalable(&self) -> Result<()> {
        self.check_scalable_for(true)
    }

    /// [`check_scalable`](Self::check_scalable) with the error worded for the wind grid (`grid`)
    /// or for the Monte Carlo wind perturbations.
    pub fn check_scalable_for(&self, grid: bool) -> Result<()> {
        if self.model == WindModel::Profile && self.speed < CALM_PROFILE_SPEED {
            if grid {
                bail!(
                    "wind profile is calm (mean speed {:.3} m/s): it cannot be scaled for dispersion; use wind.model = \"constant\"/\"power\" for a wind grid",
                    self.speed
                );
            }
            bail!(
                "wind profile is calm (mean speed {:.3} m/s): the Monte Carlo wind perturbations cannot be applied to it; set dispersion.monte_carlo.wind_speed and wind_direction_deg to 0, or use a profile with wind",
                self.speed
            );
        }
        Ok(())
    }

    /// Same wind field with a different reference speed and direction (dispersion cases).
    ///
    /// For the `profile` model `speed` means the profile's mean speed `W` and `direction_deg` the
    /// direction of its mean wind vector `Psi` (see [`profile_mean`](Self::profile_mean)): every
    /// tabulated speed is multiplied by `k = speed / W` and every direction is rotated by
    /// `direction_deg - Psi`, so the profile keeps its shape (speed ratios, veer) while its mean
    /// becomes the requested one. The applied factor accumulates in `profile_scale`. A calm
    /// profile (`W < CALM_PROFILE_SPEED`) is an error: it is never silently replaced by a
    /// uniform wind. Monte Carlo draws use the same call with `speed = W + delta`, i.e. `k` is
    /// `(W + delta) / W` clamped to >= 0. Other models only change `speed` and `direction_deg`
    /// (the values at `ref_height`).
    pub fn with_speed_direction(&self, speed: f64, direction_deg: f64) -> Result<Self> {
        if !(speed.is_finite() && direction_deg.is_finite()) {
            bail!("wind speed and direction must be finite numbers, found {speed} m/s from {direction_deg} deg");
        }
        self.check_scalable()?;
        let mut w = self.clone();
        if self.model == WindModel::Profile {
            let k = speed.max(0.0) / self.speed;
            let rot = direction_deg - self.direction_deg;
            for p in &mut w.profile {
                p.speed *= k;
                p.direction_deg = (p.direction_deg + rot).rem_euclid(360.0);
            }
            w.profile_scale *= k;
        }
        w.speed = speed;
        w.direction_deg = direction_deg;
        Ok(w)
    }

    /// Largest tabulated speed [m/s] of the (scaled) profile; None for the other models.
    pub fn profile_max_speed(&self) -> Option<f64> {
        (self.model == WindModel::Profile).then(|| self.profile.iter().map(|p| p.speed).fold(0.0, f64::max))
    }

    /// Horizontal wind (east, north) at height `h` above ground, from the profile table: linear
    /// interpolation of the vector components, constant beyond the ends.
    fn vector_at(&self, h: f64) -> (f64, f64) {
        let p = &self.profile;
        let i = p.partition_point(|q| q.height <= h);
        if i == 0 {
            return p[0].vector();
        }
        if i == p.len() {
            return p[i - 1].vector();
        }
        let (a, b) = (p[i - 1], p[i]);
        let f = (h - a.height) / (b.height - a.height);
        let (ae, an) = a.vector();
        let (be, bn) = b.vector();
        (ae + f * (be - ae), an + f * (bn - an))
    }

    /// Unscaled power/log speed profile `w(h) / speed`. The `constant` and `profile` models do
    /// not use it.
    fn model_shape(&self, h: f64) -> f64 {
        match self.model {
            WindModel::Power => (h / self.ref_height).powf(1.0 / self.exponent),
            WindModel::Log => {
                let z0 = self.roughness_length;
                (h / z0).ln() / (self.ref_height / z0).ln()
            }
            _ => 1.0,
        }
    }

    /// Analytic `d/dh` of [`model_shape`](Self::model_shape).
    fn model_shape_deriv(&self, h: f64) -> f64 {
        match self.model {
            WindModel::Power => (h / self.ref_height).powf(1.0 / self.exponent) / (self.exponent * h),
            WindModel::Log => 1.0 / (h * (self.ref_height / self.roughness_length).ln()),
            _ => 0.0,
        }
    }

    /// Cubic Hermite blend of the power/log speed profile on `[0, ground_blend_height]`. The value
    /// matches the model at the blend height `h_b`; at the ground the speed is zero with bounded
    /// slope `w(h_b) / h_b`. The end slopes (in units of `w(h_b) / h_b`) are `alpha = 1` at the
    /// ground and `beta = h_b w'(h_b) / w(h_b)` at `h_b`; when `alpha^2 + beta^2 > 9` (a log
    /// profile with `h_b` close to `z0`) both are scaled onto that circle (Fritsch-Carlson), which
    /// keeps the blend monotonic and non-negative at the cost of a slope kink at `h_b`. Otherwise
    /// (every power law with exponent >= 0.36, every log profile with `h_b >= 1.43 z0`) the blend is
    /// the plain C1 Hermite curve.
    fn blend_shape(&self, h: f64) -> f64 {
        let hb = self.ground_blend_height;
        let wb = self.model_shape(hb);
        let d1 = self.model_shape_deriv(hb);
        let t = h / hb;
        let (t2, t3) = (t * t, t * t * t);
        let beta = hb * d1 / wb;
        let r2 = 1.0 + beta * beta;
        if r2 <= 9.0 {
            return wb * (-t3 + t2 + t) + hb * d1 * (t3 - t2);
        }
        let tau = 3.0 / r2.sqrt();
        // Hermite basis: h10 = t^3 - 2t^2 + t (slope at 0), h01 = -2t^3 + 3t^2, h11 = t^3 - t^2.
        wb * (tau * (t3 - 2.0 * t2 + t) + (-2.0 * t3 + 3.0 * t2) + tau * beta * (t3 - t2))
    }

    /// Horizontal wind at `ref_height` as `(speed [m/s], direction it blows FROM [deg])`, 0 deg when it is calm.
    pub fn at_ref_height(&self) -> (f64, f64) {
        let v = self.at(self.ref_height);
        let s = v.x.hypot(v.y);
        (s, if s > 1e-9 { direction_from(v.x, v.y) } else { 0.0 })
    }

    /// Wind velocity at height `h` above ground.
    pub fn at(&self, h: f64) -> Vec3 {
        if self.model == WindModel::Profile {
            let (e, n) = self.vector_at(h);
            return Vec3::new(e, n, 0.0);
        }
        if self.speed == 0.0 {
            return Vec3::ZERO;
        }
        let v = match self.model {
            WindModel::Power | WindModel::Log => {
                if h < 0.0 {
                    0.0
                } else if h < self.ground_blend_height {
                    self.speed * self.blend_shape(h)
                } else {
                    // The model itself, evaluated exactly as before the blend was introduced.
                    match self.model {
                        WindModel::Power => self.speed * (h / self.ref_height).powf(1.0 / self.exponent),
                        WindModel::Log => {
                            let z0 = self.roughness_length;
                            self.speed * (h / z0).ln() / (self.ref_height / z0).ln()
                        }
                        _ => unreachable!(),
                    }
                }
            }
            _ => self.speed,
        };
        let d = self.direction_deg.to_radians();
        Vec3::new(-v * d.sin(), -v * d.cos(), 0.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AtmosphereKind {
    #[default]
    Us1976,
    Constant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AtmosphereConfig {
    pub model: AtmosphereKind,
    /// Temperature offset from the standard atmosphere [K] (us1976 only).
    pub temperature_offset: f64,
    /// Density [kg/m^3] (constant only).
    pub density: f64,
    /// Speed of sound [m/s] (constant only).
    pub sound_speed: f64,
    /// Dynamic viscosity [Pa s] (constant only).
    pub viscosity: f64,
}

impl Default for AtmosphereConfig {
    fn default() -> Self {
        Self { model: AtmosphereKind::Us1976, temperature_offset: 0.0, density: 1.225, sound_speed: 340.29, viscosity: 1.789e-5 }
    }
}

impl AtmosphereConfig {
    /// Runtime atmosphere (defined in the aero crate, which uses it for Reynolds numbers).
    pub fn build(&self) -> Result<AtmosphereModel> {
        match self.model {
            AtmosphereKind::Us1976 => AtmosphereModel::us1976(self.temperature_offset),
            AtmosphereKind::Constant => AtmosphereModel::constant(self.density, self.sound_speed, self.viscosity),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EarthModel {
    /// Flat local ENU frame.
    #[default]
    Flat,
    /// Rotating ellipsoidal Earth in ECEF.
    Ecef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GravityModel {
    /// g0 at all heights.
    Constant,
    /// g0 (R / (R + h))^2 along the local vertical (the geodetic down in the ECEF frame).
    #[default]
    InverseSquare,
    J2,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct EarthConfig {
    pub model: EarthModel,
    pub gravity: GravityModel,
}

/// WGS84 gravitational parameter [m^3/s^2].
pub const GM: f64 = 3.986_004_418e14;
/// WGS84 second zonal harmonic.
pub const J2: f64 = 1.082_626_68e-3;
/// WGS84 equatorial radius [m].
pub const A_EQ: f64 = 6_378_137.0;
/// Earth rotation rate [rad/s].
pub const OMEGA_EARTH: f64 = 7.292_115e-5;

/// Runtime Earth model: integration frame and gravity.
#[derive(Debug, Clone, Copy)]
pub struct Earth {
    pub model: EarthModel,
    pub gravity: GravityModel,
    /// Rotation rate of the Earth-fixed frame [rad/s] (ecef only).
    pub omega: f64,
}

impl Earth {
    pub fn from_config(c: &EarthConfig) -> Result<Self> {
        if c.model == EarthModel::Flat && c.gravity == GravityModel::J2 {
            bail!("gravity model \"j2\" needs a spherical/ellipsoidal Earth: set earth.model = \"ecef\" (or use another gravity model with earth.model = \"flat\")");
        }
        Ok(Self { model: c.model, gravity: c.gravity, omega: OMEGA_EARTH })
    }

    /// Gravity [m/s^2] in the flat ENU frame at `h` m above a site at `base_alt` m MSL.
    pub fn flat_gravity(&self, base_alt: f64, h: f64) -> Vec3 {
        match self.gravity {
            GravityModel::InverseSquare => {
                let r = R_EARTH / (R_EARTH + base_alt + h);
                Vec3::new(0.0, 0.0, -G0 * r * r)
            }
            _ => Vec3::new(0.0, 0.0, -G0),
        }
    }
}

/// Gravitational acceleration of the WGS84 point mass + J2 model at ECEF position `r` [m/s^2]
/// (gravitation only, without the centrifugal term).
pub fn j2_gravity(r: Vec3) -> Vec3 {
    let rn = r.norm();
    let k = 1.5 * J2 * (A_EQ / rn).powi(2);
    let zz = (r.z / rn).powi(2);
    let c = -GM / (rn * rn * rn);
    Vec3::new(c * r.x * (1.0 + k * (1.0 - 5.0 * zz)), c * r.y * (1.0 + k * (1.0 - 5.0 * zz)), c * r.z * (1.0 + k * (3.0 - 5.0 * zz)))
}

impl Default for Earth {
    fn default() -> Self {
        Self { model: EarthModel::Flat, gravity: GravityModel::InverseSquare, omega: OMEGA_EARTH }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum IntegratorKind {
    /// Fixed-step classical Runge-Kutta.
    #[default]
    Rk4,
    /// Adaptive Dormand-Prince 5(4) using `rtol` / `atol`.
    Rk45,
    /// Adaptive Dormand-Prince 8(5,3) (DOP853) using `rtol` / `atol`.
    Dop853,
}

impl IntegratorKind {
    /// Whether the step size is controlled by `rtol` / `atol`.
    pub fn is_adaptive(self) -> bool {
        self != IntegratorKind::Rk4
    }
}

/// How the attitude quaternion is advanced.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttitudeKind {
    /// Quaternion treated as 4 ordinary state components and renormalised after every step.
    #[default]
    Normalize,
    /// Runge-Kutta-Munthe-Kaas on SO(3): `q <- q (x) exp(Theta)`, exactly unit-norm.
    LieGroup,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_toml_names() {
        let e: EarthConfig = toml::from_str("model = \"flat\"\ngravity = \"inverse_square\"").unwrap();
        assert_eq!(e.gravity, GravityModel::InverseSquare);
        let a: AtmosphereConfig = toml::from_str("model = \"us1976\"").unwrap();
        assert_eq!(a.model, AtmosphereKind::Us1976);
        assert_eq!(a.build().unwrap(), AtmosphereModel::default());
        let c = AtmosphereConfig { model: AtmosphereKind::Constant, ..Default::default() };
        assert!(matches!(c.build().unwrap(), AtmosphereModel::Constant { .. }));
        assert_eq!(serde_json::from_str::<IntegratorKind>("\"rk45\"").unwrap(), IntegratorKind::Rk45);
        assert_eq!(serde_json::from_str::<IntegratorKind>("\"dop853\"").unwrap(), IntegratorKind::Dop853);
        assert_eq!(serde_json::from_str::<AttitudeKind>("\"lie_group\"").unwrap(), AttitudeKind::LieGroup);
    }

    fn cfg(model: WindModel) -> WindConfig {
        WindConfig { model, speed: 4.0, direction_deg: 270.0, ..Default::default() }
    }

    fn profile_wind(csv: &str, mut c: WindConfig) -> Wind {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let path = std::env::temp_dir().join(format!("ignisyeet_wind_{}_{n}.csv", std::process::id()));
        std::fs::write(&path, csv).unwrap();
        c.model = WindModel::Profile;
        c.profile = Some(path);
        Wind::from_config(&c).unwrap()
    }

    #[test]
    fn power_and_constant_wind() {
        let mut c = cfg(WindModel::Power);
        let p = Wind::from_config(&c).unwrap();
        assert!((p.at(2.0).x - 4.0).abs() < 1e-12);
        assert!(p.at(128.0).x > 4.0 * 1.9);
        c.model = WindModel::Constant;
        let k = Wind::from_config(&c).unwrap();
        assert!((k.at(2000.0).x - 4.0).abs() < 1e-12 && k.at(0.0).y.abs() < 1e-12);
        let k2 = k.with_speed_direction(3.0, 0.0).unwrap();
        assert!((k2.at(10.0).y + 3.0).abs() < 1e-12 && (k2.speed, k2.direction_deg) == (3.0, 0.0));
    }

    #[test]
    fn non_finite_wind_speed_and_direction_are_rejected() {
        for (speed, dir) in [(f64::INFINITY, 0.0), (f64::NAN, 0.0), (-1.0, 0.0)] {
            let e = Wind::from_config(&WindConfig { speed, direction_deg: dir, ..cfg(WindModel::Power) }).unwrap_err().to_string();
            assert!(e.starts_with("wind.speed must be a finite number >= 0, found"), "{e}");
        }
        for dir in [f64::NAN, f64::INFINITY] {
            let e = Wind::from_config(&WindConfig { direction_deg: dir, ..cfg(WindModel::Constant) }).unwrap_err().to_string();
            assert!(e.starts_with("wind.direction_deg must be a finite number, found"), "{e}");
        }
        let w = Wind::from_config(&cfg(WindModel::Power)).unwrap();
        for (speed, dir) in [(f64::NAN, 0.0), (f64::INFINITY, 0.0), (3.0, f64::NAN), (3.0, f64::NEG_INFINITY)] {
            let e = w.with_speed_direction(speed, dir).unwrap_err().to_string();
            assert!(e.starts_with("wind speed and direction must be finite numbers"), "{e}");
        }
        let p = profile_wind("altitude_m,speed,direction_deg\n0,2,90\n1000,6,120\n", cfg(WindModel::Profile));
        assert!(p.with_speed_direction(f64::NAN, 0.0).is_err());
    }

    #[test]
    fn log_wind() {
        let mut c = cfg(WindModel::Log);
        c.ref_height = 10.0;
        c.roughness_length = 0.1;
        let w = Wind::from_config(&c).unwrap();
        assert!((w.at(10.0).x - 4.0).abs() < 1e-12);
        // Below the blend height the profile is transitional, but it starts at zero and grows.
        assert_eq!(w.at(0.0).x, 0.0);
        assert!(w.at(0.05).x > 0.0 && w.at(0.1).x > 0.0);
        assert!(w.at(0.05).x < w.at(1.0).x && w.at(0.1).x < w.at(1.0).x);
        let expect = 4.0 * (100.0f64).ln() / (100.0f64).ln();
        assert!((w.at(10.0).x - expect).abs() < 1e-12);
        // w(100) = 4 ln(1000)/ln(100) = 6
        assert!((w.at(100.0).x - 6.0).abs() < 1e-12);
        let w2 = w.with_speed_direction(2.0, 180.0).unwrap();
        assert!((w2.at(100.0).y - 3.0).abs() < 1e-12 && w2.at(100.0).x.abs() < 1e-12);
        c.roughness_length = 10.0;
        assert!(Wind::from_config(&c).unwrap_err().to_string().contains("roughness_length"));
        c.ref_height = -1.0;
        assert!(Wind::from_config(&c).is_err());
    }

    /// Pre-blend power/log speed magnitude, used to check that `h >= h_b` is unchanged bit for bit.
    fn legacy_speed(w: &Wind, h: f64) -> f64 {
        match w.model {
            WindModel::Power => w.speed * (h.max(0.0) / w.ref_height).powf(1.0 / w.exponent),
            WindModel::Log => w.speed * (h.max(w.roughness_length) / w.roughness_length).ln() / (w.ref_height / w.roughness_length).ln(),
            _ => w.speed,
        }
    }

    fn blent(model: WindModel) -> Wind {
        let mut c = cfg(model);
        c.ref_height = 2.0;
        c.roughness_length = 0.05;
        c.ground_blend_height = 1.0;
        Wind::from_config(&c).unwrap()
    }

    #[test]
    fn ground_blend_is_continuous_and_has_continuous_slope() {
        for model in [WindModel::Power, WindModel::Log] {
            let w = blent(model);
            let hb = w.ground_blend_height;
            // Value continuity across the blend height.
            let eps = 1e-10;
            let (below, above) = (w.at(hb - eps).x, w.at(hb + eps).x);
            assert!((below - above).abs() < 1e-9, "{model:?}: value jump {}", below - above);
            // Slope continuity: one-sided finite differences agree to 1e-5 relative.
            let d = 1e-7;
            let dl = (w.at(hb).x - w.at(hb - d).x) / d;
            let dr = (w.at(hb + d).x - w.at(hb).x) / d;
            let rel = (dl - dr).abs() / dl.abs().max(dr.abs()).max(f64::MIN_POSITIVE);
            assert!(rel < 1e-5, "{model:?}: slopes {dl} vs {dr} (rel {rel:e})");
        }
    }

    #[test]
    fn ground_blend_starts_at_zero_with_bounded_slope() {
        for model in [WindModel::Power, WindModel::Log] {
            let w = blent(model);
            let hb = w.ground_blend_height;
            assert_eq!(w.at(0.0), Vec3::ZERO, "{model:?}");
            assert_eq!(w.at(-1.0), Vec3::ZERO, "{model:?}");
            // The initial slope is w(h_b)/h_b, so the finite-difference derivative is finite.
            let d = 1e-9;
            let fd = (w.at(d).x - w.at(0.0).x) / d;
            let expect = w.at(hb).x / hb;
            assert!(fd.is_finite() && (fd / expect - 1.0).abs() < 1e-5, "{model:?}: {fd} vs {expect}");
            assert!(w.at(0.5).x > 0.0 && w.at(0.5).x < w.at(hb).x, "{model:?}");
        }
    }

    #[test]
    fn ground_blend_leaves_profile_above_blend_height_unchanged() {
        for model in [WindModel::Power, WindModel::Log] {
            // Direction 0 so that `-at(h).y` is exactly the speed magnitude.
            let mut c = cfg(model);
            c.ref_height = 2.0;
            c.roughness_length = 0.05;
            c.ground_blend_height = 1.0;
            c.direction_deg = 0.0;
            let w = Wind::from_config(&c).unwrap();
            for h in [w.ground_blend_height, 1.5, 2.0, 10.0, 1000.0] {
                assert_eq!(-w.at(h).y, legacy_speed(&w, h), "{model:?} at {h}");
            }
            // with_speed_direction only scales the speed, so the blend scales with it.
            let scaled = w.with_speed_direction(9.0, 0.0).unwrap();
            for h in [w.ground_blend_height, 1.5, 2.0, 10.0] {
                assert_eq!(-scaled.at(h).y, legacy_speed(&scaled, h), "{model:?} scaled at {h}");
            }
            for h in [0.0, 0.25, 0.5, 1.0, 3.0] {
                let (a, b) = (scaled.at(h).norm(), w.at(h).norm());
                assert!((a - 2.25 * b).abs() <= 1e-12 * a.max(1.0), "{model:?} scale at {h}: {a} vs {b}");
            }
        }
    }

    #[test]
    fn ground_blend_validation_and_log_clamp() {
        let mut c = cfg(WindModel::Power);
        c.ground_blend_height = 0.0;
        assert!(Wind::from_config(&c).unwrap_err().to_string().contains("ground_blend_height"));
        c.ground_blend_height = c.ref_height;
        assert!(Wind::from_config(&c).is_err());
        // The constant and profile models ignore the field, so any value is accepted.
        c.model = WindModel::Constant;
        c.ground_blend_height = -1.0;
        assert!(Wind::from_config(&c).is_ok());
        // Log: a value at or below z0 is raised to 2 z0 so the analytic slope exists.
        let mut c = cfg(WindModel::Log);
        c.ref_height = 10.0;
        c.roughness_length = 0.2;
        c.ground_blend_height = 0.1;
        let w = Wind::from_config(&c).unwrap();
        assert!((w.ground_blend_height - 0.4).abs() < 1e-12);
        c.ground_blend_height = 0.5;
        assert!((Wind::from_config(&c).unwrap().ground_blend_height - 0.5).abs() < 1e-12);
    }

    #[test]
    fn profile_wind_interpolates_vectors() {
        let csv = "altitude_m,speed,direction_deg\n0,2,270\n100,6,270\n200,4,350\n300,4,10\n";
        let mut c = cfg(WindModel::Profile);
        c.ref_height = 50.0;
        let w = profile_wind(csv, c.clone());
        // linear in altitude, constant beyond the ends
        assert!((w.at(50.0).x - 4.0).abs() < 1e-12 && (w.at(-5.0).x - 2.0).abs() < 1e-12 && (w.at(1e4).norm() - 4.0).abs() < 1e-12);
        // 350 and 10 deg average to 0 deg (wind from the north: blows towards -y), speed cos(10 deg) * 4
        let m = w.at(250.0);
        assert!(m.x.abs() < 1e-12 && (m.y + 4.0 * 10f64.to_radians().cos()).abs() < 1e-12, "{m:?}");
        // speed and direction are the profile's mean speed and the direction of its mean vector
        let (mean, psi) = w.profile_mean();
        assert!((w.speed - mean).abs() < 1e-12 && (w.direction_deg - psi).abs() < 1e-12);
        assert!(mean > 3.5 && mean < 4.5 && w.profile_scale == 1.0);
        // dispersion: the whole profile is scaled by speed / W and rotated by direction - Psi
        let d = w.with_speed_direction(2.0 * mean, psi + 180.0).unwrap();
        assert!((d.profile_scale - 2.0).abs() < 1e-12 && (d.speed - 2.0 * mean).abs() < 1e-12);
        assert!((d.at(0.0).norm() - 4.0).abs() < 1e-9 && (d.at(100.0).norm() - 12.0).abs() < 1e-9);
        // the 350 deg row rotated by the (270 deg -> psi + 180) offset
        let rot = (psi + 180.0) - psi;
        let q = d.at(200.0);
        let from = (-q.x).atan2(-q.y).to_degrees().rem_euclid(360.0);
        assert!((from - (350.0 + rot).rem_euclid(360.0)).abs() < 1e-9, "{from}");
        // the mean of the rescaled profile is the requested one
        let (m2, _) = d.profile_mean();
        assert!((m2 - 2.0 * mean).abs() < 1e-9, "{m2}");
    }

    #[test]
    fn profile_mean_targets_scale_and_rotate_like_a_wind_grid_case() {
        let csv = "altitude_m,speed,direction_deg\n0,2,270\n100,6,270\n200,4,350\n300,4,10\n";
        let base = profile_wind(csv, cfg(WindModel::Profile));
        let (mean, _) = base.profile_mean();
        let target = WindConfig { profile_mean_speed: Some(15.6), profile_mean_direction_deg: Some(123.0), ..cfg(WindModel::Profile) };
        let w = profile_wind(csv, target);
        assert_eq!((w.speed, w.direction_deg), (15.6, 123.0));
        assert!((w.profile_scale - 15.6 / mean).abs() < 1e-12);
        // bit for bit the wind of the wind-grid case with the same speed and direction
        let grid = base.with_speed_direction(15.6, 123.0).unwrap();
        for i in 0..400 {
            let h = i as f64 - 20.0;
            assert_eq!(w.at(h), grid.at(h), "{h}");
        }
        // dispersion cases from the targeted wind equal those of the plain profile
        let (a, b) = (w.with_speed_direction(9.0, 45.0).unwrap(), base.with_speed_direction(9.0, 45.0).unwrap());
        for h in [0.0, 50.0, 150.0, 250.0, 500.0] {
            assert!((a.at(h) - b.at(h)).norm() < 1e-12, "{h}");
        }
        // only the direction: the speeds stay those of the CSV
        let d = profile_wind(csv, WindConfig { profile_mean_direction_deg: Some(90.0), ..cfg(WindModel::Profile) });
        assert_eq!(d.direction_deg, 90.0);
        assert!((d.speed - mean).abs() < 1e-12 && (d.profile_scale - 1.0).abs() < 1e-15);
        // wind.speed and wind.direction_deg do not change a profile
        let ignored = profile_wind(csv, WindConfig { speed: 30.0, direction_deg: 10.0, ..cfg(WindModel::Profile) });
        assert_eq!((ignored.speed, ignored.direction_deg), (base.speed, base.direction_deg));
    }

    #[test]
    fn profile_mean_targets_are_checked() {
        let path = std::env::temp_dir().join(format!("ignisyeet_wind_target_{}.csv", std::process::id()));
        std::fs::write(&path, "altitude_m,speed,direction_deg\n0,0,270\n100,0,270\n").unwrap();
        let c = |speed, dir| WindConfig {
            model: WindModel::Profile,
            profile: Some(path.clone()),
            profile_mean_speed: speed,
            profile_mean_direction_deg: dir,
            ..Default::default()
        };
        let calm = Wind::from_config(&c(Some(5.0), None)).unwrap_err().to_string();
        assert!(calm.contains("calm") && calm.contains("profile_mean_speed"), "{calm}");
        assert!(Wind::from_config(&c(None, None)).is_ok());
        std::fs::write(&path, "altitude_m,speed,direction_deg\n0,3,270\n100,5,270\n").unwrap();
        assert!(Wind::from_config(&c(Some(-1.0), None)).unwrap_err().to_string().contains("profile_mean_speed"));
        assert!(Wind::from_config(&c(None, Some(400.0))).unwrap_err().to_string().contains("profile_mean_direction_deg"));
        let zero = Wind::from_config(&c(Some(0.0), None)).unwrap();
        assert_eq!(zero.at(50.0).norm(), 0.0);
        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn profile_mean_is_over_the_altitude_range() {
        // linear 2 -> 6 m/s over 0..100 m: mean 4 from 270 deg
        let w = profile_wind("altitude_m,speed,direction_deg\n0,2,270\n100,6,270\n", cfg(WindModel::Profile));
        let (m, psi) = w.profile_mean();
        assert!((m - 4.0).abs() < 1e-9 && (psi - 270.0).abs() < 1e-9, "{m} {psi}");
        // single row: that row
        let one = profile_wind("altitude_m,speed,direction_deg\n10,3,90\n", cfg(WindModel::Profile));
        assert_eq!(one.profile_mean(), (3.0, 90.0));
        // opposite winds: the mean vector is ~0, the direction at ref_height is used
        let opp = profile_wind("altitude_m,speed,direction_deg\n0,5,0\n100,5,180\n", WindConfig { ref_height: 1.0, ..cfg(WindModel::Profile) });
        assert!(opp.direction_deg < 1e-6 || opp.direction_deg > 360.0 - 1e-6, "{}", opp.direction_deg);
        assert!(opp.speed > 2.0);
    }

    #[test]
    fn calm_surface_profile_keeps_the_winds_aloft() {
        // calm at ref_height (2 m), 10-14 m/s aloft: the profile used to become a uniform wind
        let csv = "altitude_m,speed,direction_deg\n0,0,270\n500,10,270\n2000,14,270\n";
        let w = profile_wind(csv, cfg(WindModel::Profile));
        let (mean, _) = w.profile_mean();
        assert!(mean > 8.0, "{mean}");
        let d = w.with_speed_direction(mean, 270.0).unwrap();
        assert!((d.profile_scale - 1.0).abs() < 1e-12 && d.at(0.0).norm() == 0.0 && (d.at(2000.0).norm() - 14.0).abs() < 1e-9);
        let d = w.with_speed_direction(mean * 0.5, 90.0).unwrap();
        assert!((d.at(2000.0).norm() - 7.0).abs() < 1e-9 && d.at(2000.0).x < 0.0);
        // a zero-speed case zeroes the profile (k = 0) without an error
        assert_eq!(w.with_speed_direction(0.0, 0.0).unwrap().at(1000.0).norm(), 0.0);
    }

    #[test]
    fn near_calm_surface_does_not_amplify_the_profile() {
        let csv = "altitude_m,speed,direction_deg\n0,0.02,270\n500,10,270\n2000,14,270\n";
        let w = profile_wind(csv, WindConfig { ref_height: 2.0, ..cfg(WindModel::Profile) });
        let (mean, _) = w.profile_mean();
        // a 9 m/s case scales by 9 / mean (about 1), not by 9 / 0.02 m/s
        let d = w.with_speed_direction(9.0, 270.0).unwrap();
        assert!((d.profile_scale - 9.0 / mean).abs() < 1e-12 && d.profile_scale < 1.5);
        assert!(d.at(2000.0).norm() < 20.0);
    }

    #[test]
    fn calm_profile_cannot_be_scaled() {
        let w = profile_wind("altitude_m,speed,direction_deg\n0,0.01,0\n100,0.05,0\n", cfg(WindModel::Profile));
        let e = w.with_speed_direction(3.0, 90.0).unwrap_err().to_string();
        assert!(
            e.contains("wind profile is calm (mean speed 0.030") && e.contains("cannot be scaled for dispersion") && e.contains("\"constant\"/\"power\""),
            "{e}"
        );
        assert!(w.check_scalable().is_err());
        // the flight itself is not affected: the wind is simply weak
        assert!(w.at(50.0).norm() < 0.1);
        // other models are always scalable
        assert!(Wind::from_config(&cfg(WindModel::Power)).unwrap().with_speed_direction(0.0, 0.0).is_ok());
    }

    #[test]
    fn profile_validation() {
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,1,0\n0,2,0\n").unwrap_err().to_string().contains("ascending"));
        assert_eq!(parse_wind_profile("\u{feff}altitude_m,speed,direction_deg\n0,1,0\n100,2,90\n").unwrap().len(), 2);
        assert!(parse_wind_profile("alt,speed,dir\n0,1,0\n").unwrap_err().to_string().contains("header"));
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n").is_err());
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,x,0\n").is_err());
        let e = parse_wind_profile("altitude_m,speed,direction_deg\n0,1,400\n").unwrap_err().to_string();
        assert!(e.contains("between 0 and 360") && e.contains("0,1,400"), "{e}");
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,1,-1\n").is_err());
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,1,0\n100,1,360\n").is_ok());
        let mut c = cfg(WindModel::Profile);
        assert!(Wind::from_config(&c).unwrap_err().to_string().contains("wind.profile"));
        c.profile = Some("/nonexistent/wind.csv".into());
        assert!(Wind::from_config(&c).unwrap_err().to_string().contains("cannot read"));
    }

    #[test]
    fn j2_flat_is_rejected() {
        let e = EarthConfig { gravity: GravityModel::J2, ..Default::default() };
        assert!(Earth::from_config(&e).unwrap_err().to_string().contains("ecef"));
        assert!(Earth::from_config(&EarthConfig { model: EarthModel::Ecef, gravity: GravityModel::J2 }).is_ok());
    }

    #[test]
    fn j2_gravity_magnitudes_and_direction() {
        use crate::geo::lla_to_ecef;
        let eq = lla_to_ecef(0.0, 30.0, 0.0);
        let g = j2_gravity(eq);
        assert!((g.norm() - 9.814).abs() < 0.002, "equator {}", g.norm());
        assert!(g.dot(eq) < 0.0 && g.cross(eq).norm() < 1e-9 * g.norm() * eq.norm());
        let pole = lla_to_ecef(90.0, 0.0, 0.0);
        let gp = j2_gravity(pole);
        assert!((gp.norm() - 9.832).abs() < 0.01, "pole {}", gp.norm());
        assert!(gp.x.abs() < 1e-9 && gp.z < 0.0);
        // geocentric latitude 45 deg: J2 attraction has a component towards the equator (south)
        let (sn, cs) = 45f64.to_radians().sin_cos();
        let p = Vec3::new(cs, 0.0, sn) * 7.0e6;
        let gm = j2_gravity(p);
        let north = Vec3::new(-sn, 0.0, cs);
        let along = gm.dot(north);
        assert!(along < -0.01 && along > -0.02, "{along}"); // ~ -3 J2 g sin cos (Re/r)^2
        assert!(gm.y.abs() < 1e-12);
    }

    #[test]
    fn ground_blend_is_non_negative_and_monotonic() {
        // Log profile with the blend just above z0: the model slope at h_b is steep
        // (h_b w'(h_b) / w(h_b) = 1 / ln(1.1) = 10.5) and the plain cubic Hermite undershot to
        // -0.142 m/s at 0.055 m.
        let mut c = cfg(WindModel::Log);
        c.speed = 10.0;
        c.direction_deg = 0.0;
        c.ref_height = 10.0;
        c.roughness_length = 0.1;
        c.ground_blend_height = 0.11;
        let w = Wind::from_config(&c).unwrap();
        let speed = |h: f64| -w.at(h).y;
        let hb = w.ground_blend_height;
        let mut prev = 0.0;
        for i in 0..=1000 {
            let h = hb * i as f64 / 1000.0;
            let v = speed(h);
            assert!(v >= 0.0 && v >= prev - 1e-15, "h {h}: {v} after {prev}");
            prev = v;
        }
        assert!(speed(0.055) >= 0.0);
        assert!((speed(hb - 1e-12) - speed(hb)).abs() < 1e-9, "value continuity at h_b");
        // A power law (gentle slope at h_b) keeps the C1 blend: unchanged.
        let w = blent(WindModel::Power);
        let hb = w.ground_blend_height;
        let (wb, d1) = (w.model_shape(hb), w.model_shape_deriv(hb));
        let t: f64 = 0.3;
        let plain = w.speed * (wb * (-t.powi(3) + t * t + t) + hb * d1 * (t.powi(3) - t * t));
        assert!((w.at(0.3 * hb).norm() - plain).abs() <= 1e-14 * plain, "{} vs {plain}", w.at(0.3 * hb).norm());
    }

    #[test]
    fn log_profile_needs_twice_the_roughness_below_the_reference_height() {
        let mut c = cfg(WindModel::Log);
        c.ref_height = 10.0;
        c.roughness_length = 6.0;
        c.ground_blend_height = 1.0;
        let e = Wind::from_config(&c).unwrap_err().to_string();
        assert!(e.contains("must be smaller than half of wind.ref_height"), "{e}");
        c.roughness_length = 4.9;
        assert!(Wind::from_config(&c).is_ok());
    }

    #[test]
    fn wind_profile_rejects_levels_below_the_launch_site() {
        let e = parse_wind_profile("altitude_m,speed,direction_deg\n-30,3,270\n0,4,270\n500,8,280\n").unwrap_err().to_string();
        assert!(e.contains("wind profile altitude must be ≥ 0 m above the launch site (row \"-30,3,270\")"), "{e}");
        assert!(e.contains("drop levels below the launch site and add a 0 m row"), "{e}");
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,4,270\n500,8,280\n").is_ok());
    }
}
