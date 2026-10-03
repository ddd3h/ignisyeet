//! Environment models: wind, atmosphere, Earth/gravity and integrator selection.
//! The `*Config` types are deserialized from TOML; the runtime types are built from them
//! and return an error for variants that are not implemented yet.

use aero::atmosphere::G0;
pub use aero::atmosphere::AtmosphereModel;
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
    /// Speed at the reference height [m/s].
    pub speed: f64,
    /// Direction the wind blows FROM, clockwise from north [deg].
    pub direction_deg: f64,
    pub ref_height: f64,
    /// Power-law exponent n (power model).
    pub exponent: f64,
    /// Surface roughness length [m] (log model).
    pub roughness_length: f64,
    /// CSV with columns altitude_m,speed,direction_deg (profile model).
    pub profile: Option<PathBuf>,
}

impl Default for WindConfig {
    fn default() -> Self {
        Self { model: WindModel::Power, speed: 4.0, direction_deg: 0.0, ref_height: 2.0, exponent: 6.0, roughness_length: 0.03, profile: None }
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
/// evaluated from the table at `ref_height`; they are what the summary reports and what
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
    profile: Vec<ProfilePoint>,
}

/// Parses a wind profile CSV (`altitude_m,speed,direction_deg`, ascending altitude above ground).
pub fn parse_wind_profile(text: &str) -> Result<Vec<ProfilePoint>> {
    let mut lines = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#'));
    let header = lines.next().context("wind profile is empty")?;
    let cols: Vec<String> = header.split(',').map(|c| c.trim().to_ascii_lowercase()).collect();
    if cols != ["altitude_m", "speed", "direction_deg"] {
        bail!("wind profile header must be \"altitude_m,speed,direction_deg\", found \"{header}\"");
    }
    let mut pts: Vec<ProfilePoint> = Vec::new();
    for l in lines {
        let v: Vec<f64> = l
            .split(',')
            .map(|x| x.trim().parse::<f64>())
            .collect::<std::result::Result<_, _>>()
            .with_context(|| format!("bad wind profile row \"{l}\""))?;
        if v.len() != 3 || v.iter().any(|x| !x.is_finite()) {
            bail!("bad wind profile row \"{l}\": expected 3 finite numbers");
        }
        if v[1] < 0.0 {
            bail!("wind profile speed must not be negative (row \"{l}\")");
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
        if c.speed.is_nan() || c.speed < 0.0 {
            bail!("wind.speed must not be negative");
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
                if c.roughness_length >= c.ref_height {
                    bail!("wind.roughness_length ({}) must be smaller than wind.ref_height ({})", c.roughness_length, c.ref_height);
                }
            }
            WindModel::Profile => {
                let path = c.profile.as_deref().context("wind.profile (CSV path) is required when wind.model = \"profile\"")?;
                let text = std::fs::read_to_string(path).with_context(|| format!("cannot read wind profile {}", path.display()))?;
                let profile = parse_wind_profile(&text).with_context(|| format!("invalid wind profile {}", path.display()))?;
                let mut w = Self::with_profile(c, profile);
                let (e, n) = w.vector_at(c.ref_height);
                w.speed = e.hypot(n);
                w.direction_deg = if w.speed > 0.0 { direction_from(e, n) } else { 0.0 };
                return Ok(w);
            }
        }
        Ok(Self::with_profile(c, Vec::new()))
    }

    fn with_profile(c: &WindConfig, profile: Vec<ProfilePoint>) -> Self {
        Self {
            model: c.model,
            speed: c.speed,
            direction_deg: c.direction_deg,
            ref_height: c.ref_height,
            exponent: c.exponent,
            roughness_length: c.roughness_length,
            profile,
        }
    }

    /// Same wind field with a different reference speed and direction (dispersion cases).
    ///
    /// For the `profile` model every tabulated speed is multiplied by `speed / self.speed` and
    /// every direction is rotated by `direction_deg - self.direction_deg`, so the profile keeps its
    /// shape (speed ratios, veer) while the wind at `ref_height` becomes the requested one. If the
    /// profile is calm at `ref_height` it cannot be scaled; it is then replaced by a uniform wind
    /// of the requested speed and direction.
    pub fn with_speed_direction(&self, speed: f64, direction_deg: f64) -> Self {
        let mut w = self.clone();
        w.speed = speed;
        w.direction_deg = direction_deg;
        if self.model == WindModel::Profile {
            if self.speed > 0.0 {
                let k = speed / self.speed;
                let rot = direction_deg - self.direction_deg;
                for p in &mut w.profile {
                    p.speed *= k;
                    p.direction_deg = (p.direction_deg + rot).rem_euclid(360.0);
                }
            } else {
                w.model = WindModel::Constant;
                w.profile.clear();
            }
        }
        w
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
            WindModel::Power => self.speed * (h.max(0.0) / self.ref_height).powf(1.0 / self.exponent),
            WindModel::Log => {
                let z0 = self.roughness_length;
                self.speed * (h.max(z0) / z0).ln() / (self.ref_height / z0).ln()
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
    /// Adaptive Dormand-Prince using `rtol` / `atol`.
    Rk45,
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
        let k2 = k.with_speed_direction(3.0, 0.0);
        assert!((k2.at(10.0).y + 3.0).abs() < 1e-12 && (k2.speed, k2.direction_deg) == (3.0, 0.0));
    }

    #[test]
    fn log_wind() {
        let mut c = cfg(WindModel::Log);
        c.ref_height = 10.0;
        c.roughness_length = 0.1;
        let w = Wind::from_config(&c).unwrap();
        assert!((w.at(10.0).x - 4.0).abs() < 1e-12);
        assert_eq!(w.at(0.05).x, 0.0);
        assert_eq!(w.at(0.1).x, 0.0);
        let expect = 4.0 * (100.0f64).ln() / (100.0f64).ln();
        assert!((w.at(10.0).x - expect).abs() < 1e-12);
        // w(100) = 4 ln(1000)/ln(100) = 6
        assert!((w.at(100.0).x - 6.0).abs() < 1e-12);
        let w2 = w.with_speed_direction(2.0, 180.0);
        assert!((w2.at(100.0).y - 3.0).abs() < 1e-12 && w2.at(100.0).x.abs() < 1e-12);
        c.roughness_length = 10.0;
        assert!(Wind::from_config(&c).unwrap_err().to_string().contains("roughness_length"));
        c.ref_height = -1.0;
        assert!(Wind::from_config(&c).is_err());
    }

    #[test]
    fn profile_wind_interpolates_vectors() {
        let csv = "altitude_m,speed,direction_deg\n0,2,270\n100,6,270\n200,4,350\n300,4,10\n";
        let mut c = cfg(WindModel::Profile);
        c.ref_height = 50.0;
        let w = profile_wind(csv, c.clone());
        // linear in altitude, constant beyond the ends
        assert!((w.at(50.0).x - 4.0).abs() < 1e-12 && (w.at(-5.0).x - 2.0).abs() < 1e-12 && (w.at(1e4).norm() - 4.0).abs() < 1e-12);
        assert!((w.speed - 4.0).abs() < 1e-12 && (w.direction_deg - 270.0).abs() < 1e-9);
        // 350 and 10 deg average to 0 deg (wind from the north: blows towards -y), speed cos(10 deg) * 4
        let m = w.at(250.0);
        assert!(m.x.abs() < 1e-12 && (m.y + 4.0 * 10f64.to_radians().cos()).abs() < 1e-12, "{m:?}");
        // dispersion: speed at ref_height -> 8 m/s, direction at ref_height -> 90 deg, whole profile scaled/rotated
        let d = w.with_speed_direction(8.0, 90.0);
        assert!((d.speed - 8.0).abs() < 1e-12);
        let r = d.at(50.0);
        assert!((r.x + 8.0).abs() < 1e-9 && r.y.abs() < 1e-9, "{r:?}");
        assert!((d.at(0.0).norm() - 4.0).abs() < 1e-9 && (d.at(100.0).norm() - 12.0).abs() < 1e-9);
        // 350 deg point rotated by +180 -> 170
        let q = d.at(200.0);
        let from = (-q.x).atan2(-q.y).to_degrees().rem_euclid(360.0);
        assert!((from - 170.0).abs() < 1e-9, "{from}");
        // a calm profile at ref_height degrades to a uniform wind
        let calm = profile_wind("altitude_m,speed,direction_deg\n0,0,0\n10,0,0\n100,5,0\n", WindConfig { ref_height: 0.5, ..cfg(WindModel::Profile) });
        let u = calm.with_speed_direction(3.0, 180.0);
        assert!((u.at(1000.0).y - 3.0).abs() < 1e-12);
    }

    #[test]
    fn profile_validation() {
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,1,0\n0,2,0\n").unwrap_err().to_string().contains("ascending"));
        assert!(parse_wind_profile("alt,speed,dir\n0,1,0\n").unwrap_err().to_string().contains("header"));
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n").is_err());
        assert!(parse_wind_profile("altitude_m,speed,direction_deg\n0,x,0\n").is_err());
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
}
