// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! TOML configuration file. Relative paths are resolved against the config file directory.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub output: OutputCfg,
    pub rocket: RocketCfg,
    pub motor: MotorCfg,
    pub launch: sim::Launch,
    pub recovery: sim::Recovery,
    #[serde(default)]
    pub aero: AeroCfg,
    #[serde(default)]
    pub earth: sim::EarthConfig,
    #[serde(default)]
    pub atmosphere: sim::AtmosphereConfig,
    #[serde(default)]
    pub wind: sim::WindConfig,
    #[serde(default)]
    pub sim: SimCfg,
    #[serde(default)]
    pub dispersion: sim::DispersionConfig,
    #[serde(default)]
    pub resources: ResourcesCfg,
    #[serde(skip)]
    pub base_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputCfg {
    pub dir: PathBuf,
    /// Also write the Google Earth file ignisyeet.kml (flight and dispersion results).
    #[serde(default = "yes")]
    pub kml: bool,
}

impl Default for OutputCfg {
    fn default() -> Self {
        Self { dir: "out".into(), kml: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RocketCfg {
    pub stl: PathBuf,
    /// Factor converting STL units to metres (0.001 for millimetres).
    #[serde(default = "one")]
    pub stl_scale: f64,
    /// "auto" or one of "+x", "-x", "+y", "-y", "+z", "-z": direction the nose points in the STL.
    #[serde(default = "auto")]
    pub nose_direction: String,
    pub dry_mass: f64,
    pub cg_dry: f64,
    pub ixx_dry: f64,
    pub iyy_dry: f64,
    /// Dry yaw inertia Izz [kg m^2]; defaults to `iyy_dry`.
    #[serde(default)]
    pub izz_dry: Option<f64>,
    /// Products of the inertia tensor about the dry CG in body axes (x forward, y/z of the
    /// geometry.json body basis). These are the tensor matrix elements, `Ixy = -∫ x y dm`.
    #[serde(default)]
    pub ixy_dry: f64,
    #[serde(default)]
    pub ixz_dry: f64,
    #[serde(default)]
    pub iyz_dry: f64,
    /// Lateral offset (y, z) of the dry CG from the axis in body axes [m].
    #[serde(default)]
    pub cg_lateral_dry: [f64; 2],
    /// Fin cant angle [deg]; rolls the rocket (positive = positive roll about +x_b). `fin_override.cant_deg` takes precedence.
    #[serde(default)]
    pub fin_cant_deg: f64,
    #[serde(default = "default_roughness")]
    pub roughness: f64,
    /// "rounded" or "sharp".
    #[serde(default = "rounded")]
    pub fin_le: String,
    /// "square" or "tapered".
    #[serde(default = "square")]
    pub fin_te: String,
    #[serde(default)]
    pub extra_cd: f64,
    #[serde(default)]
    pub fin_override: Option<FinOverride>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FinOverride {
    pub count: Option<usize>,
    pub root_chord: Option<f64>,
    pub tip_chord: Option<f64>,
    pub span: Option<f64>,
    pub sweep: Option<f64>,
    pub thickness: Option<f64>,
    pub x_le_root: Option<f64>,
    /// Fin cant [deg]; replaces `rocket.fin_cant_deg`.
    pub cant_deg: Option<f64>,
}

impl RocketCfg {
    /// Configured fin cant [deg].
    pub fn cant_deg(&self) -> f64 {
        self.fin_override.as_ref().and_then(|o| o.cant_deg).unwrap_or(self.fin_cant_deg)
    }

    pub fn mass_properties(&self) -> sim::MassProperties {
        sim::MassProperties {
            dry_mass: self.dry_mass,
            cg_dry: self.cg_dry,
            ixx_dry: self.ixx_dry,
            iyy_dry: self.iyy_dry,
            izz_dry: self.izz_dry,
            ixy_dry: self.ixy_dry,
            ixz_dry: self.ixz_dry,
            iyz_dry: self.iyz_dry,
            cg_lateral_dry: self.cg_lateral_dry,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MotorCfg {
    pub eng: PathBuf,
    /// Aft end of the motor, aft of the nose tip [m]. Defaults to the body length.
    pub aft_x: Option<f64>,
    #[serde(default)]
    pub nozzle_exit_diameter: f64,
    /// Overrides the propellant mass in the .eng header [kg].
    pub propellant_mass: Option<f64>,
    /// Angle between the thrust line and the body axis [deg].
    #[serde(default)]
    pub misalignment_deg: f64,
    /// Direction of the misalignment, measured from +y_b towards +z_b [deg].
    #[serde(default)]
    pub misalignment_azimuth_deg: f64,
    /// Nozzle-exit point (y, z) of the thrust line in body axes [m].
    #[serde(default)]
    pub misalignment_offset: [f64; 2],
}

impl MotorCfg {
    pub fn thrust_line(&self) -> sim::ThrustLine {
        sim::ThrustLine { misalignment_deg: self.misalignment_deg, misalignment_azimuth_deg: self.misalignment_azimuth_deg, offset: self.misalignment_offset }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AeroMethod {
    #[default]
    Barrowman,
    Panel,
    Table,
    Cfd,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AeroCfg {
    pub method: AeroMethod,
    /// External coefficient table (CSV with JSON sidecar); required for method "table".
    pub table: Option<PathBuf>,
    pub panel: panel::PanelOptions,
    /// Settings of the SU2 CFD method (only used when method = "cfd").
    pub cfd: cfd::CfdOptions,
    pub mach_min: f64,
    pub mach_max: f64,
    pub mach_step: f64,
    pub alpha_max_deg: f64,
    pub alpha_step_deg: f64,
    pub n_slices: usize,
    pub fin_threshold: f64,
    pub extrapolation: aero::Extrapolation,
    /// "axisymmetric" (coefficients of the total angle of attack) or "general" (six body-axis
    /// coefficients of alpha and beta, expanded from the axisymmetric table).
    pub table_kind: aero::table::TableKind,
    /// Mach step of a general Barrowman table.
    pub general_mach_step: f64,
}

impl Default for AeroCfg {
    fn default() -> Self {
        Self {
            method: AeroMethod::Barrowman,
            table: None,
            panel: panel::PanelOptions::default(),
            cfd: cfd::CfdOptions::default(),
            mach_min: 0.0,
            mach_max: 3.0,
            mach_step: 0.02,
            alpha_max_deg: 30.0,
            alpha_step_deg: 1.0,
            n_slices: 600,
            fin_threshold: 0.05,
            extrapolation: aero::Extrapolation::Linear,
            table_kind: aero::table::TableKind::Axisymmetric,
            general_mach_step: 0.05,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SimCfg {
    /// "rk4" (fixed step `dt`), "rk45" (adaptive Dormand-Prince 5(4)) or "dop853" (adaptive 8th order).
    pub integrator: sim::IntegratorKind,
    /// Attitude update: "normalize" (quaternion renormalised after each step) or "lie_group"
    /// (Runge-Kutta-Munthe-Kaas steps on SO(3); the quaternion stays unit-norm by construction).
    pub attitude: sim::AttitudeKind,
    /// Step of the rk4 integrator.
    pub dt: f64,
    /// Tolerances of the adaptive integrators (rk45, dop853).
    pub rtol: f64,
    pub atol: f64,
    /// Flight time limit [s] (default 3600). A flight still airborne at this time did not land: `sim`
    /// warns, writes `landed = false` to summary.json and exits with status 3.
    pub max_time: f64,
    /// Interval between trajectory samples [s].
    pub output_interval: f64,
    /// Descent mode for `sim`: "parachute" or "ballistic". Defaults to parachute when recovery is enabled.
    pub descent: Option<sim::Descent>,
}

impl Default for SimCfg {
    fn default() -> Self {
        Self {
            integrator: sim::IntegratorKind::Rk4,
            attitude: sim::AttitudeKind::Normalize,
            dt: 0.002,
            rtol: 1e-7,
            atol: 1e-6,
            max_time: 3600.0,
            output_interval: 0.05,
            descent: None,
        }
    }
}

/// Compute resources IgnisYeet may use. They never change results, so they are not part of the aero hash.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ResourcesCfg {
    /// Maximum number of CPU threads for everything; 0 = all available.
    pub threads: usize,
    /// Memory budget in GB for heavy steps (CFD cases, the panel dense matrix); 0 = no limit.
    pub memory_gb: f64,
    /// Process priority lowering, 0..=19 (0 = unchanged).
    pub nice: i32,
}

/// [`ResourcesCfg`] with defaults and clamping applied.
#[derive(Debug, Clone, PartialEq)]
pub struct Resolved {
    pub threads: usize,
    pub available: usize,
    pub memory_bytes: Option<u64>,
    pub nice: i32,
    /// Set when the requested thread count was clamped.
    pub warning: Option<String>,
}

impl ResourcesCfg {
    pub fn validate(&self) -> Result<()> {
        if !self.memory_gb.is_finite() || self.memory_gb < 0.0 {
            bail!("resources.memory_gb must be 0 (no limit) or positive");
        }
        if !(0..=19).contains(&self.nice) {
            bail!("resources.nice must be between 0 and 19");
        }
        Ok(())
    }

    pub fn resolve(&self) -> Resolved {
        let available = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        let mut warning = None;
        let threads = if self.threads == 0 {
            available
        } else if self.threads > available {
            warning = Some(format!("resources.threads = {} exceeds the {available} available threads; using {available}", self.threads));
            available
        } else {
            self.threads
        };
        let memory_bytes = (self.memory_gb > 0.0).then_some((self.memory_gb * 1e9) as u64);
        Resolved { threads, available, memory_bytes, nice: self.nice, warning }
    }
}

fn yes() -> bool {
    true
}
fn one() -> f64 {
    1.0
}
fn auto() -> String {
    "auto".into()
}
fn default_roughness() -> f64 {
    60e-6
}
fn rounded() -> String {
    "rounded".into()
}
fn square() -> String {
    "square".into()
}

impl Config {
    pub fn load(path: &Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("cannot read config {}", path.display()))?;
        let raw: toml::Value = toml::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text)).with_context(|| format!("invalid config {}", path.display()))?;
        Self::from_value(raw, path.parent()).with_context(|| format!("invalid config {}", path.display()))
    }

    pub(crate) fn from_value(raw: toml::Value, dir: Option<&Path>) -> Result<Self> {
        if raw.get("environment").is_some() {
            bail!(
                "the [environment] section was removed: move wind_speed, wind_direction_deg, wind_ref_height and wind_exponent \
                 to [wind] as speed, direction_deg, ref_height and exponent"
            );
        }
        let mut cfg: Config = raw.try_into()?;
        cfg.base_dir = dir.map(Path::to_path_buf).unwrap_or_default();
        cfg.validate()?;
        Ok(cfg)
    }

    /// Range checks of every numeric key (finite, signs, bounds); errors name the key.
    fn validate_numbers(&self) -> Result<()> {
        fn finite(key: &str, v: f64) -> Result<()> {
            if !v.is_finite() {
                bail!("{key} must be a finite number, found {v}");
            }
            Ok(())
        }
        fn positive(key: &str, v: f64) -> Result<()> {
            if !(v.is_finite() && v > 0.0) {
                bail!("{key} must be a finite number > 0, found {v}");
            }
            Ok(())
        }
        fn non_negative(key: &str, v: f64) -> Result<()> {
            if !(v.is_finite() && v >= 0.0) {
                bail!("{key} must be a finite number >= 0, found {v}");
            }
            Ok(())
        }
        fn range(key: &str, v: f64, lo: f64, hi: f64) -> Result<()> {
            if !(v.is_finite() && (lo..=hi).contains(&v)) {
                bail!("{key} must be between {lo} and {hi}, found {v}");
            }
            Ok(())
        }
        let (r, m, l) = (&self.rocket, &self.motor, &self.launch);
        positive("rocket.stl_scale", r.stl_scale)?;
        positive("rocket.dry_mass", r.dry_mass)?;
        finite("rocket.cg_dry", r.cg_dry)?;
        if r.cg_dry < 0.0 {
            bail!("rocket.cg_dry must be >= 0 (measured aft of the nose tip), found {}", r.cg_dry);
        }
        positive("rocket.ixx_dry", r.ixx_dry)?;
        positive("rocket.iyy_dry", r.iyy_dry)?;
        if let Some(z) = r.izz_dry {
            positive("rocket.izz_dry", z)?;
        }
        for (k, v) in [("ixy_dry", r.ixy_dry), ("ixz_dry", r.ixz_dry), ("iyz_dry", r.iyz_dry), ("fin_cant_deg", r.fin_cant_deg)] {
            finite(&format!("rocket.{k}"), v)?;
        }
        finite("rocket.cg_lateral_dry[0]", r.cg_lateral_dry[0])?;
        finite("rocket.cg_lateral_dry[1]", r.cg_lateral_dry[1])?;
        non_negative("rocket.roughness", r.roughness)?;
        non_negative("rocket.extra_cd", r.extra_cd)?;
        if let Some(o) = &r.fin_override {
            for (k, v) in [("root_chord", o.root_chord), ("tip_chord", o.tip_chord), ("span", o.span), ("thickness", o.thickness)] {
                if let Some(v) = v {
                    non_negative(&format!("rocket.fin_override.{k}"), v)?;
                }
            }
            for (k, v) in [("sweep", o.sweep), ("x_le_root", o.x_le_root), ("cant_deg", o.cant_deg)] {
                if let Some(v) = v {
                    finite(&format!("rocket.fin_override.{k}"), v)?;
                }
            }
        }
        if let Some(a) = m.aft_x {
            non_negative("motor.aft_x", a)?;
        }
        non_negative("motor.nozzle_exit_diameter", m.nozzle_exit_diameter)?;
        if let Some(p) = m.propellant_mass {
            non_negative("motor.propellant_mass", p)?;
        }
        finite("motor.misalignment_deg", m.misalignment_deg)?;
        finite("motor.misalignment_azimuth_deg", m.misalignment_azimuth_deg)?;
        finite("motor.misalignment_offset[0]", m.misalignment_offset[0])?;
        finite("motor.misalignment_offset[1]", m.misalignment_offset[1])?;

        range("launch.latitude", l.latitude, -90.0, 90.0)?;
        range("launch.longitude", l.longitude, -180.0, 180.0)?;
        finite("launch.altitude", l.altitude)?;
        positive("launch.rail_length", l.rail_length)?;
        if !(l.elevation_deg.is_finite() && l.elevation_deg > 0.0 && l.elevation_deg <= 90.0) {
            bail!("launch.elevation_deg must be in (0, 90], found {}", l.elevation_deg);
        }
        finite("launch.azimuth_deg", l.azimuth_deg)?;
        finite("launch.roll_deg", l.roll_deg)?;
        non_negative("recovery.cd_s", self.recovery.cd_s)?;
        if self.recovery.enabled {
            positive("recovery.cd_s (recovery.enabled = true)", self.recovery.cd_s)?;
        }
        non_negative("recovery.delay", self.recovery.delay)?;
        self.recovery.validate()?;

        let a = &self.aero;
        non_negative("aero.mach_min", a.mach_min)?;
        finite("aero.mach_max", a.mach_max)?;
        if a.mach_max <= a.mach_min {
            bail!("aero.mach_max ({}) must be greater than aero.mach_min ({})", a.mach_max, a.mach_min);
        }
        positive("aero.mach_step", a.mach_step)?;
        positive("aero.alpha_max_deg", a.alpha_max_deg)?;
        positive("aero.alpha_step_deg", a.alpha_step_deg)?;
        positive("aero.general_mach_step", a.general_mach_step)?;
        if a.mach_max > MACH_LIMIT {
            bail!("aero.mach_max = {} is too large for a rocket flight (<= {MACH_LIMIT})", a.mach_max);
        }
        if a.alpha_max_deg > 180.0 {
            bail!("aero.alpha_max_deg = {} is outside (0, 180]", a.alpha_max_deg);
        }
        let n_mach = ((a.mach_max - a.mach_min) / a.mach_step).round().max(1.0) + 1.0;
        let n_alpha = (a.alpha_max_deg / a.alpha_step_deg).round().max(1.0) * 2.0 + 1.0;
        if n_mach * n_alpha > MAX_AERO_GRID_POINTS {
            bail!(
                "the aero table grid would have about {:.3e} points ({n_mach} Mach x {n_alpha} alpha; at most {MAX_AERO_GRID_POINTS:e}): increase aero.mach_step / aero.alpha_step_deg or reduce aero.mach_max / aero.alpha_max_deg",
                n_mach * n_alpha
            );
        }
        if a.n_slices < MIN_SLICES {
            bail!("aero.n_slices = {} is too small: the geometry extraction needs at least {MIN_SLICES} slices", a.n_slices);
        }
        if a.n_slices > MAX_SLICES {
            bail!("aero.n_slices = {} is too large (at most {MAX_SLICES})", a.n_slices);
        }
        non_negative("aero.fin_threshold", a.fin_threshold)?;

        let w = &self.wind;
        non_negative("wind.speed", w.speed)?;
        finite("wind.direction_deg", w.direction_deg)?;
        positive("wind.ref_height", w.ref_height)?;
        positive("wind.exponent", w.exponent)?;
        positive("wind.roughness_length", w.roughness_length)?;
        positive("wind.ground_blend_height", w.ground_blend_height)?;
        if w.model == sim::WindModel::Log && 2.0 * w.roughness_length >= w.ref_height {
            bail!("wind.roughness_length ({}) must be smaller than half of wind.ref_height ({}) for the log profile", w.roughness_length, w.ref_height);
        }

        let s = &self.sim;
        positive("sim.dt", s.dt)?;
        if s.dt > MAX_DT {
            bail!("sim.dt = {} s is too large for a rocket flight (<= {MAX_DT} s; 0.002 s is typical)", s.dt);
        }
        positive("sim.max_time", s.max_time)?;
        positive("sim.output_interval", s.output_interval)?;
        if s.output_interval > s.max_time {
            bail!("sim.output_interval = {} s exceeds sim.max_time = {} s", s.output_interval, s.max_time);
        }
        positive("sim.rtol", s.rtol)?;
        positive("sim.atol", s.atol)?;
        for (k, v) in [("rtol", s.rtol), ("atol", s.atol)] {
            if v > MAX_TOL {
                bail!("sim.{k} = {v} is too loose for a rocket flight (<= {MAX_TOL:e}; 1e-8 is typical)");
            }
        }

        let d = &self.dispersion;
        for (i, v) in d.wind_speeds.iter().enumerate() {
            non_negative(&format!("dispersion.wind_speeds[{i}]"), *v)?;
        }
        let mc = &d.monte_carlo;
        for (k, v) in [
            ("thrust_scale", mc.thrust_scale),
            ("burn_time_scale", mc.burn_time_scale),
            ("dry_mass", mc.dry_mass),
            ("cg", mc.cg),
            ("cn_scale", mc.cn_scale),
            ("ca_scale", mc.ca_scale),
            ("elevation_deg", mc.elevation_deg),
            ("azimuth_deg", mc.azimuth_deg),
            ("wind_speed", mc.wind_speed),
            ("wind_direction_deg", mc.wind_direction_deg),
            ("parachute_cd_s_scale", mc.parachute_cd_s_scale),
            ("thrust_misalignment_deg", mc.thrust_misalignment_deg),
            ("thrust_offset", mc.thrust_offset),
            ("fin_cant_deg", mc.fin_cant_deg),
            ("cg_lateral", mc.cg_lateral),
            ("cl_scale", mc.cl_scale),
            ("clp_scale", mc.clp_scale),
        ] {
            non_negative(&format!("dispersion.monte_carlo.{k} (1-sigma)"), v)?;
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        self.validate_numbers()?;
        if !matches!(self.rocket.fin_le.as_str(), "rounded" | "sharp") {
            bail!("rocket.fin_le must be \"rounded\" or \"sharp\"");
        }
        if !matches!(self.rocket.fin_te.as_str(), "square" | "tapered") {
            bail!("rocket.fin_te must be \"square\" or \"tapered\"");
        }
        if self.rocket.dry_mass <= 0.0 || self.rocket.iyy_dry <= 0.0 || self.rocket.ixx_dry <= 0.0 {
            bail!("rocket mass and inertias must be positive");
        }
        let r = &self.rocket;
        let izz = r.izz_dry.unwrap_or(r.iyy_dry);
        let tensor = geom::Mat3([[r.ixx_dry, r.ixy_dry, r.ixz_dry], [r.ixy_dry, r.iyy_dry, r.iyz_dry], [r.ixz_dry, r.iyz_dry, izz]]);
        let minors = [r.ixx_dry, r.ixx_dry * r.iyy_dry - r.ixy_dry * r.ixy_dry, tensor.det()];
        if !(izz > 0.0 && minors.iter().all(|m| *m > 0.0)) {
            bail!("the inertia tensor of the rocket (ixx_dry, iyy_dry, izz_dry and the products) must be positive definite");
        }
        let finite = [
            r.ixy_dry,
            r.ixz_dry,
            r.iyz_dry,
            r.cg_lateral_dry[0],
            r.cg_lateral_dry[1],
            r.fin_cant_deg,
            self.motor.misalignment_deg,
            self.motor.misalignment_azimuth_deg,
            self.motor.misalignment_offset[0],
            self.motor.misalignment_offset[1],
            self.launch.roll_deg,
        ];
        if finite.iter().any(|v| !v.is_finite()) {
            bail!("rocket inertia products, CG offset, fin cant, motor misalignment and launch.roll_deg must be finite numbers");
        }
        if self.aero.general_mach_step <= 0.0 {
            bail!("aero.general_mach_step must be positive");
        }
        if self.aero.mach_step <= 0.0 || self.aero.alpha_step_deg <= 0.0 || self.aero.mach_max <= self.aero.mach_min {
            bail!("invalid aero grid");
        }
        if self.dispersion.wind_speeds.is_empty() {
            bail!("dispersion.wind_speeds must list at least one speed");
        }
        if self.dispersion.directions == 0 {
            bail!("dispersion.directions must be at least 1");
        }
        let d = &self.dispersion;
        if d.directions > MAX_DIRECTIONS {
            bail!("dispersion.directions = {} is too large (at most {MAX_DIRECTIONS})", d.directions);
        }
        if d.wind_speeds.len() > MAX_WIND_SPEEDS {
            bail!("dispersion.wind_speeds lists {} speeds (at most {MAX_WIND_SPEEDS})", d.wind_speeds.len());
        }
        if d.wind_speeds.len() * d.directions > MAX_WIND_GRID_CASES {
            bail!("the wind grid would have {} cases (wind_speeds x directions; at most {MAX_WIND_GRID_CASES})", d.wind_speeds.len() * d.directions);
        }
        if d.monte_carlo.samples > MAX_MC_SAMPLES {
            bail!("dispersion.monte_carlo.samples = {} is too large (at most {MAX_MC_SAMPLES})", d.monte_carlo.samples);
        }
        if self.aero.method == AeroMethod::Table && self.aero.table.is_none() {
            bail!("aero.table is required when aero.method = \"table\"");
        }
        if self.aero.method == AeroMethod::Cfd {
            self.aero.cfd.validate()?;
        }
        self.atmosphere.build()?;
        if self.earth.model == sim::EarthModel::Flat && self.earth.gravity == sim::GravityModel::J2 {
            bail!("earth.gravity = \"j2\" requires earth.model = \"ecef\"");
        }
        if self.wind.model == sim::WindModel::Profile && self.wind.profile.is_none() {
            bail!("wind.profile (CSV path) is required when wind.model = \"profile\"");
        }
        if self.wind.ref_height <= 0.0 {
            bail!("wind.ref_height must be positive");
        }
        if self.wind.model == sim::WindModel::Log && !(self.wind.roughness_length > 0.0 && self.wind.roughness_length < self.wind.ref_height) {
            bail!("wind.roughness_length must be positive and smaller than wind.ref_height");
        }
        if matches!(self.wind.model, sim::WindModel::Power | sim::WindModel::Log)
            && !(self.wind.ground_blend_height > 0.0 && self.wind.ground_blend_height < self.wind.ref_height)
        {
            bail!("wind.ground_blend_height must be positive and smaller than wind.ref_height");
        }
        if self.sim.dt <= 0.0 || self.sim.output_interval <= 0.0 {
            bail!("sim.dt and sim.output_interval must be positive");
        }
        if self.sim.integrator.is_adaptive() && !(self.sim.rtol > 0.0 && self.sim.atol > 0.0) {
            bail!("sim.rtol and sim.atol must be positive for the adaptive integrators (rk45, dop853)");
        }
        self.resources.validate()?;
        self.nose_direction()?;
        Ok(())
    }

    /// Wind configuration with the profile path resolved against the config directory.
    pub fn wind_config(&self) -> sim::WindConfig {
        let mut w = self.wind.clone();
        w.profile = w.profile.as_deref().map(|p| self.resolve(p));
        w
    }

    pub fn resolve(&self, p: &Path) -> PathBuf {
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.base_dir.join(p)
        }
    }

    pub fn out_dir(&self) -> PathBuf {
        self.resolve(&self.output.dir)
    }

    pub fn nose_direction(&self) -> Result<Option<geom::Vec3>> {
        use geom::Vec3;
        Ok(match self.rocket.nose_direction.as_str() {
            "auto" => None,
            "+x" => Some(Vec3::new(1.0, 0.0, 0.0)),
            "-x" => Some(Vec3::new(-1.0, 0.0, 0.0)),
            "+y" => Some(Vec3::new(0.0, 1.0, 0.0)),
            "-y" => Some(Vec3::new(0.0, -1.0, 0.0)),
            "+z" => Some(Vec3::new(0.0, 0.0, 1.0)),
            "-z" => Some(Vec3::new(0.0, 0.0, -1.0)),
            other => bail!("rocket.nose_direction '{other}' is not one of auto, +x, -x, +y, -y, +z, -z"),
        })
    }
}

/// Largest accepted `sim.dt` [s].
const MAX_DT: f64 = 0.05;
/// Largest accepted `sim.rtol` and `sim.atol`.
const MAX_TOL: f64 = 1e-3;
/// Largest accepted `aero.mach_max`.
const MACH_LIMIT: f64 = 10.0;
/// Largest accepted number of aero table grid points.
const MAX_AERO_GRID_POINTS: f64 = 1e6;
const MAX_DIRECTIONS: usize = 360;
const MAX_WIND_SPEEDS: usize = 100;
const MAX_WIND_GRID_CASES: usize = 100_000;
const MAX_MC_SAMPLES: usize = 1_000_000;
const MAX_SLICES: usize = 10_000;
/// Fewest accepted `aero.n_slices` (the geometry extraction requires this many).
const MIN_SLICES: usize = 20;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<Config> {
        Config::from_value(toml::from_str(text)?, None)
    }

    #[test]
    fn parses_sample() {
        let text = include_str!("../../../examples/sample.toml");
        let c = parse(text).unwrap();
        assert_eq!(c.wind.speed, 4.0);
        assert_eq!(c.wind.model, sim::WindModel::Power);
        assert_eq!(c.aero.method, AeroMethod::Barrowman);
        assert_eq!(c.aero.panel.body_axial, 80);
        assert_eq!(c.dispersion.directions, 8);
        assert_eq!(c.sim.integrator, sim::IntegratorKind::Rk4);
    }

    #[test]
    fn old_environment_section_is_rejected() {
        let text = include_str!("../../../examples/sample.toml").replace("[wind]", "[environment]");
        let e = parse(&text).unwrap_err().to_string();
        assert!(e.contains("[environment]") && e.contains("[wind]"), "{e}");
    }

    #[test]
    fn mode_validation() {
        let sample = include_str!("../../../examples/sample.toml");
        let e = parse(&sample.replace("gravity = \"inverse_square\"", "gravity = \"j2\"")).unwrap_err().to_string();
        assert!(e.contains("j2") && e.contains("ecef"), "{e}");
        let ok = sample.replace("gravity = \"inverse_square\"", "gravity = \"j2\"").replace("model = \"flat\"", "model = \"ecef\"");
        assert!(parse(&ok).is_ok());
        let e = parse(&sample.replace("model = \"power\"", "model = \"profile\"")).unwrap_err().to_string();
        assert!(e.contains("wind.profile"), "{e}");
        let e = parse(&sample.replace("model = \"power\"", "model = \"log\"").replace("roughness_length = 0.03", "roughness_length = 5.0"))
            .unwrap_err()
            .to_string();
        assert!(e.contains("roughness_length"), "{e}");
        assert!(parse(&sample.replace("model = \"us1976\"", "model = \"constant\"")).is_ok());
        assert!(parse(&sample.replace("integrator = \"rk4\"", "integrator = \"rk45\"")).is_ok());
    }

    #[test]
    fn integrator_and_attitude_keys() {
        let sample = include_str!("../../../examples/sample.toml");
        let c = parse(sample).unwrap();
        assert_eq!((c.sim.integrator, c.sim.attitude), (sim::IntegratorKind::Rk4, sim::AttitudeKind::Normalize));
        let with = |i: &str, a: &str| sample.replace("integrator = \"rk4\"", &format!("integrator = \"{i}\"\nattitude = \"{a}\""));
        let c = parse(&with("dop853", "lie_group")).unwrap();
        assert_eq!((c.sim.integrator, c.sim.attitude), (sim::IntegratorKind::Dop853, sim::AttitudeKind::LieGroup));
        for i in ["rk4", "rk45", "dop853"] {
            for a in ["normalize", "lie_group"] {
                assert!(parse(&with(i, a)).is_ok(), "{i} {a}");
            }
        }
        assert!(parse(&with("rk4", "euler")).is_err());
        assert!(parse(&with("rk8", "normalize")).is_err());
        let e = parse(&with("dop853", "lie_group").replace("rtol = 1e-7", "rtol = 0.0")).unwrap_err().to_string();
        assert!(e.contains("rtol"), "{e}");
    }

    #[test]
    fn cfd_method_and_options() {
        let sample = include_str!("../../../examples/sample.toml");
        let text = sample.replace("method = \"barrowman\"", "method = \"cfd\"") + "\n[aero.cfd]\nmodel = \"rans\"\nmachs = [0.5, 2.0]\nalphas_deg = [0, 4]\n";
        let c = parse(&text).unwrap();
        assert_eq!(c.aero.method, AeroMethod::Cfd);
        assert_eq!(c.aero.cfd.model, cfd::FlowModel::Rans);
        let bad = text.replace("alphas_deg = [0, 4]", "alphas_deg = [2, 4]");
        assert!(parse(&bad).unwrap_err().to_string().contains("include 0"));
        assert!(parse(&(text + "bogus = 1\n")).is_err());
    }

    #[test]
    fn resources_section() {
        let sample = include_str!("../../../examples/sample.toml");
        let c = parse(sample).unwrap();
        assert_eq!(c.resources.threads, 0);
        // drop the sample's own [resources] section so the tests can append theirs
        let sample = sample
            .replace("[resources]\nthreads = 0 ", "[resources_x]\n#")
            .replace("memory_gb = 0 ", "#")
            .replace("nice = 0 ", "#")
            .replace("[resources_x]", "");
        let sample = sample.as_str();
        let r = c.resources.resolve();
        assert_eq!(r.threads, r.available);
        assert_eq!(r.memory_bytes, None);
        let c = parse(&format!("{sample}\n[resources]\nthreads = 1\nmemory_gb = 2.5\nnice = 10\n")).unwrap();
        let r = c.resources.resolve();
        assert_eq!((r.threads, r.memory_bytes, r.nice), (1, Some(2_500_000_000), 10));
        assert!(r.warning.is_none());
        let c = parse(&format!("{sample}\n[resources]\nthreads = 100000\n")).unwrap();
        let r = c.resources.resolve();
        assert_eq!(r.threads, r.available);
        assert!(r.warning.unwrap().contains("exceeds"));
        for bad in ["memory_gb = -1", "nice = 20", "nice = -1", "bogus = 1"] {
            assert!(parse(&format!("{sample}\n[resources]\n{bad}\n")).is_err(), "{bad}");
        }
    }

    #[test]
    fn six_component_keys() {
        let sample = include_str!("../../../examples/sample.toml");
        let c = parse(sample).unwrap();
        assert_eq!((c.rocket.cant_deg(), c.motor.misalignment_deg, c.launch.roll_deg), (0.0, 0.0, 0.0));
        assert_eq!(c.aero.table_kind, aero::table::TableKind::Axisymmetric);
        assert_eq!(c.aero.general_mach_step, 0.05);
        assert!(c.rocket.mass_properties().is_axisymmetric());
        let text = sample
            .replace(
                "iyy_dry = 1.5 ",
                "iyy_dry = 1.5\nizz_dry = 1.6\nixy_dry = 0.01\nixz_dry = -0.02\niyz_dry = 0.03\ncg_lateral_dry = [0.001, -0.002]\nfin_cant_deg = 0.5 #",
            )
            .replace(
                "nozzle_exit_diameter = 0.03",
                "nozzle_exit_diameter = 0.03\nmisalignment_deg = 0.2\nmisalignment_azimuth_deg = 90\nmisalignment_offset = [0.001, 0.0]",
            )
            .replace("azimuth_deg = 270.0 ", "roll_deg = 30.0\nazimuth_deg = 270.0 ")
            .replace("extrapolation = \"linear\" ", "table_kind = \"general\"\ngeneral_mach_step = 0.1\nextrapolation = \"linear\" ");
        let c = parse(&text).unwrap();
        assert_eq!((c.rocket.izz_dry, c.rocket.ixz_dry, c.rocket.cg_lateral_dry), (Some(1.6), -0.02, [0.001, -0.002]));
        assert_eq!(c.rocket.cant_deg(), 0.5);
        assert_eq!(c.motor.thrust_line(), sim::ThrustLine { misalignment_deg: 0.2, misalignment_azimuth_deg: 90.0, offset: [0.001, 0.0] });
        assert_eq!((c.launch.roll_deg, c.aero.table_kind, c.aero.general_mach_step), (30.0, aero::table::TableKind::General, 0.1));
        // A fin_override cant takes precedence.
        let c = parse(&(text + "\n[rocket.fin_override]\ncant_deg = 2.0\n")).unwrap();
        assert_eq!(c.rocket.cant_deg(), 2.0);
        // Products too large for a positive definite tensor.
        let bad = sample.replace("iyy_dry = 1.5 ", "iyy_dry = 1.5\nixy_dry = 1.6 #");
        assert!(parse(&bad).unwrap_err().to_string().contains("positive definite"));
    }

    #[test]
    fn table_method_requires_table() {
        let text = include_str!("../../../examples/sample.toml").replace("method = \"barrowman\"", "method = \"table\"");
        assert!(parse(&text).unwrap_err().to_string().contains("aero.table"));
    }

    fn sample_with(from: &str, to: &str) -> Result<Config> {
        let sample = include_str!("../../../examples/sample.toml");
        assert!(sample.contains(from), "sample has no `{from}`");
        parse(&sample.replacen(from, to, 1))
    }

    #[test]
    fn rejects_out_of_range_and_non_finite_numbers() {
        let sample = include_str!("../../../examples/sample.toml");
        let key_of = |line_start: &str| {
            sample
                .lines()
                .find(|l| l.trim_start().starts_with(line_start))
                .unwrap_or_else(|| panic!("no line {line_start}"))
                .split('#')
                .next()
                .unwrap()
                .trim_end()
                .to_string()
        };
        // (existing line prefix, replacement line, expected key in the message)
        let cases = [
            ("latitude", "latitude = 91.0", "launch.latitude"),
            ("longitude", "longitude = 181.0", "launch.longitude"),
            ("latitude", "latitude = nan", "launch.latitude"),
            ("elevation_deg", "elevation_deg = 0.0", "launch.elevation_deg"),
            ("elevation_deg", "elevation_deg = 95.0", "launch.elevation_deg"),
            ("rail_length", "rail_length = 0.0", "launch.rail_length"),
            ("azimuth_deg = 270", "azimuth_deg = inf", "launch.azimuth_deg"),
            ("dry_mass", "dry_mass = -1.0", "rocket.dry_mass"),
            ("dry_mass", "dry_mass = nan", "rocket.dry_mass"),
            ("cg_dry", "cg_dry = nan", "rocket.cg_dry"),
            ("cg_dry", "cg_dry = -0.1", "rocket.cg_dry"),
            ("cd_s", "cd_s = -1.0", "recovery.cd_s"),
            ("max_time", "max_time = 0.0", "sim.max_time"),
            ("dt", "dt = 0.0", "sim.dt"),
            ("output_interval", "output_interval = inf", "sim.output_interval"),
            ("rtol", "rtol = -1.0", "sim.rtol"),
            ("atol", "atol = 0.0", "sim.atol"),
            ("speed", "speed = -1.0", "wind.speed"),
            ("mach_min", "mach_min = -0.1", "aero.mach_min"),
            ("mach_max", "mach_max = 0.0", "aero.mach_max"),
            ("alpha_max_deg", "alpha_max_deg = 0.0", "aero.alpha_max_deg"),
            ("alpha_step_deg", "alpha_step_deg = 0.0", "aero.alpha_step_deg"),
            ("roughness =", "roughness = -1e-6", "rocket.roughness"),
            ("extra_cd", "extra_cd = -0.1", "rocket.extra_cd"),
            ("stl_scale", "stl_scale = 0.0", "rocket.stl_scale"),
        ];
        for (prefix, line, key) in cases {
            let old = key_of(prefix);
            let e = parse(&sample.replacen(&old, line, 1)).err().unwrap_or_else(|| panic!("`{line}` was accepted")).to_string();
            assert!(e.contains(key), "`{line}`: {e}");
        }
    }

    #[test]
    fn enabled_recovery_needs_a_positive_drag_area() {
        let sample = include_str!("../../../examples/sample.toml");
        let e = parse(&sample.replacen("cd_s = 3.5", "cd_s = 0.0", 1)).expect_err("cd_s = 0 accepted").to_string();
        assert!(e.contains("recovery.cd_s"), "{e}");
        let off = sample.replacen("cd_s = 3.5", "cd_s = 0.0", 1).replacen("enabled = true", "enabled = false", 1);
        parse(&off).unwrap();
    }

    #[test]
    fn dual_deploy_fields_are_validated_at_load() {
        let sample = include_str!("../../../examples/sample.toml");
        let with = |lines: &str| sample.replacen("delay = 1.0", &format!("delay = 1.0\n{lines}"), 1);
        let ok = parse(&with("main_cd_s = 8.0\nmain_altitude = 300.0")).unwrap();
        assert_eq!(ok.recovery.main(), Some((8.0, 300.0)));
        assert_eq!(parse(sample).unwrap().recovery.main(), None);
        for (lines, key) in [
            ("main_cd_s = 8.0", "recovery.main_cd_s and recovery.main_altitude"),
            ("main_altitude = 300.0", "recovery.main_cd_s and recovery.main_altitude"),
            ("main_cd_s = -1.0\nmain_altitude = 300.0", "recovery.main_cd_s must be positive"),
            ("main_cd_s = 8.0\nmain_altitude = 0.0", "recovery.main_altitude must be positive"),
            ("main_cd_s = 8.0\nmain_altitude = nan", "recovery.main_altitude must be positive"),
        ] {
            let e = parse(&with(lines)).err().unwrap_or_else(|| panic!("`{lines}` accepted")).to_string();
            assert!(e.contains(key), "`{lines}`: {e}");
        }
    }

    #[test]
    fn rejects_bad_motor_and_monte_carlo_values() {
        let sample = include_str!("../../../examples/sample.toml");
        // The six new sigmas are commented out in the sample, so they are added under the header.
        for (line, key) in [
            ("thrust_misalignment_deg = -1.0", "thrust_misalignment_deg"),
            ("thrust_offset = -0.001", "thrust_offset"),
            ("fin_cant_deg = -0.2", "fin_cant_deg"),
            ("cg_lateral = nan", "cg_lateral"),
            ("cl_scale = -0.1", "cl_scale"),
            ("clp_scale = -0.1", "clp_scale"),
        ] {
            let e = parse(&sample.replace("[dispersion.monte_carlo]", &format!("[dispersion.monte_carlo]\n{line}")))
                .err()
                .unwrap_or_else(|| panic!("{line} accepted"))
                .to_string();
            assert!(e.contains(key), "{line}: {e}");
        }
        let e = parse(&sample.replace("wind_speed = 1.0 ", "wind_speed = -1.0 ")).unwrap_err().to_string();
        assert!(e.contains("monte_carlo.wind_speed"), "{e}");
        assert!(sample_with("nozzle_exit_diameter = 0.03", "nozzle_exit_diameter = -0.03").unwrap_err().to_string().contains("motor.nozzle_exit_diameter"));
    }

    #[test]
    fn rejects_an_empty_wind_speed_list() {
        let e = sample_with("wind_speeds = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]", "wind_speeds = []").unwrap_err().to_string();
        assert!(e.contains("dispersion.wind_speeds") && e.contains("at least one"), "{e}");
    }

    #[test]
    fn default_max_time_is_one_hour() {
        assert_eq!(SimCfg::default().max_time, 3600.0);
    }

    #[test]
    fn integrator_and_aero_grid_bounds() {
        let sample = include_str!("../../../examples/sample.toml");
        let bad = |from: &str, to: &str| parse(&sample.replacen(from, to, 1)).unwrap_err().to_string();
        let e = bad("dt = 0.002", "dt = 5.0");
        assert!(e.contains("sim.dt = 5 s is too large for a rocket flight (<= 0.05 s; 0.002 s is typical)"), "{e}");
        let e = bad("rtol = 1e-7", "rtol = 1.0");
        assert!(e.contains("sim.rtol") && e.contains("too loose"), "{e}");
        let e = bad("atol = 1e-6", "atol = 1.0");
        assert!(e.contains("sim.atol"), "{e}");
        let e = bad("alpha_max_deg = 90.0", "alpha_max_deg = 1e9");
        assert!(e.contains("alpha_max_deg"), "{e}");
        let e = bad("mach_max = 3.0", "mach_max = 1000");
        assert!(e.contains("mach_max"), "{e}");
        let e = bad("mach_step = 0.02", "mach_step = 1e-6");
        assert!(e.contains("grid"), "{e}");
        let e = bad("n_slices = 600", "n_slices = 5");
        assert!(e.contains("n_slices") && e.contains("20"), "{e}");
    }

    #[test]
    fn dispersion_and_slice_counts_have_upper_bounds() {
        let sample = include_str!("../../../examples/sample.toml");
        let bad = |from: &str, to: &str| parse(&sample.replacen(from, to, 1)).unwrap_err().to_string();
        assert!(bad("directions = 8", "directions = 1000000000").contains("dispersion.directions"));
        assert!(bad("directions = 8", "directions = 361").contains("at most 360"));
        let many = format!("wind_speeds = [{}]", vec!["1.0"; 101].join(", "));
        assert!(bad("wind_speeds = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]", &many).contains("dispersion.wind_speeds"));
        assert!(bad("samples = 1000", "samples = 2000000").contains("monte_carlo.samples"));
        assert!(bad("n_slices = 600", "n_slices = 100000").contains("aero.n_slices"));
        let grid = format!("wind_speeds = [{}]\ndirections = 360", vec!["1.0"; 100].join(", "));
        let t = sample.replacen("wind_speeds = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]", &grid, 1).replacen("directions = 8", "", 1);
        parse(&t).unwrap();
    }

    #[test]
    fn log_wind_needs_room_above_the_blend() {
        let sample = include_str!("../../../examples/sample.toml");
        let t = sample.replacen("model = \"power\"", "model = \"log\"", 1).replacen("ref_height = 2.0", "ref_height = 0.5", 1).replacen(
            "roughness_length = 0.03",
            "roughness_length = 0.3\nground_blend_height = 0.1",
            1,
        );
        let e = parse(&t).unwrap_err().to_string();
        assert!(e.contains("roughness_length") && e.contains("ref_height"), "{e}");
    }
}
