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
    /// Findings of the loader that do not stop the run (keys that have no effect).
    #[serde(skip)]
    pub load_warnings: Vec<String>,
    /// `--aero-table PATH`: an aero_table.csv built for this configuration in another output directory.
    #[serde(skip)]
    pub aero_table_source: Option<PathBuf>,
}

/// Changes of the configuration given on the command line.
#[derive(Debug, Clone, Default)]
pub struct Overrides {
    /// `--set KEY=VALUE`: dotted TOML key path and a TOML value (a bare word is read as a string).
    pub set: Vec<String>,
    /// `--out DIR`: replaces `output.dir`; a relative path is resolved from the current directory.
    pub out: Option<PathBuf>,
}

/// Applies one `--set KEY=VALUE` to the raw TOML of the configuration file.
///
/// `KEY` is a dotted path of table keys (`wind.profile_mean_speed`, `aero.panel.n_axial`); missing
/// tables are created. `VALUE` is parsed as a TOML value (`15.6`, `true`, `[5.0, 10.0]`,
/// `"monte_carlo"`); a value that is not valid TOML is taken as a string, so `wind.model=profile`
/// needs no quotes. The result is checked like the file itself (unknown keys, types, ranges).
pub fn apply_set(raw: &mut toml::Value, arg: &str) -> Result<()> {
    let (key, value) = arg.split_once('=').with_context(|| format!("[IY-C002] --set {arg}: expected KEY=VALUE"))?;
    let key = key.trim();
    let parts: Vec<&str> = key.split('.').map(str::trim).collect();
    if parts.iter().any(|p| p.is_empty()) {
        bail!("[IY-C001] --set {arg}: invalid key \"{key}\" (expected a dotted path such as wind.speed)");
    }
    let value = value.trim();
    if is_bare_integer(value) && value.parse::<i64>().is_err() {
        bail!("[IY-C003] --set {arg}: integer out of range (TOML integers are at most {})", i64::MAX);
    }
    let parsed = toml::from_str::<toml::Table>(&format!("v = {value}")).ok().and_then(|mut t| t.remove("v"));
    let value = parsed.unwrap_or_else(|| toml::Value::String(value.to_string()));
    let mut node = raw;
    let (last, tables) = parts.split_last().expect("non-empty key");
    for (i, p) in tables.iter().enumerate() {
        let t = node.as_table_mut().with_context(|| format!("[IY-C004] --set {arg}: {} is not a table", parts[..i].join(".")))?;
        node = t.entry(p.to_string()).or_insert_with(|| toml::Value::Table(toml::Table::new()));
    }
    let t = node.as_table_mut().with_context(|| format!("[IY-C004] --set {arg}: {} is not a table", tables.join(".")))?;
    t.insert(last.to_string(), value);
    Ok(())
}

/// True for an optionally signed run of decimal digits (a TOML integer without underscores).
fn is_bare_integer(v: &str) -> bool {
    let d = v.strip_prefix(['+', '-']).unwrap_or(v);
    !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit())
}

/// Checks the shape KEY=VALUE of one `--set` argument (a clap value parser: a malformed one is a usage error).
pub fn parse_set_arg(arg: &str) -> std::result::Result<String, String> {
    match arg.split_once('=') {
        None => Err(format!("[IY-C002] expected KEY=VALUE, got \"{arg}\"")),
        Some((k, _)) if k.split('.').any(|p| p.trim().is_empty()) => {
            Err(format!("[IY-C001] invalid key \"{}\" (expected a dotted path such as wind.speed)", k.trim()))
        }
        Some(_) => Ok(arg.to_string()),
    }
}

/// Applies the `--set` overrides in order (see [`apply_set`]); returns a warning for each key given
/// more than once, whose last value is the one used.
pub fn apply_sets(raw: &mut toml::Value, sets: &[String]) -> Result<Vec<String>> {
    let mut last: Vec<(String, String, usize)> = Vec::new();
    for arg in sets {
        apply_set(raw, arg)?;
        let (key, value) = arg.split_once('=').expect("checked by apply_set");
        let key = key.split('.').map(str::trim).collect::<Vec<_>>().join(".");
        match last.iter_mut().find(|(k, _, _)| *k == key) {
            Some(e) => {
                e.1 = value.trim().to_string();
                e.2 += 1;
            }
            None => last.push((key, value.trim().to_string(), 1)),
        }
    }
    Ok(last.into_iter().filter(|e| e.2 > 1).map(|(k, v, _)| format!("[IY-C500] --set {k} given more than once; the last value ({v}) is used")).collect())
}

/// The error of the first `--set` override that alone makes the configuration `base` fail to read
/// (a value of the wrong type, an unknown key), with the override named; None when `base` itself
/// fails or no single override does.
fn blame_set(base: &toml::Value, sets: &[String]) -> Option<anyhow::Error> {
    base.clone().try_into::<Config>().ok()?;
    sets.iter().find_map(|arg| {
        let mut raw = base.clone();
        apply_set(&mut raw, arg).ok()?;
        raw.try_into::<Config>().err().map(|e| anyhow::anyhow!("[IY-C005] --set {}: {e}", arg.trim()))
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputCfg {
    #[serde(default = "out_dir")]
    pub dir: PathBuf,
    /// Also write the Google Earth file ignisyeet.kml (flight and dispersion results).
    #[serde(default = "yes")]
    pub kml: bool,
}

fn out_dir() -> PathBuf {
    "out".into()
}

impl Default for OutputCfg {
    fn default() -> Self {
        Self { dir: out_dir(), kml: true }
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
    /// Largest angle of attack of the table [deg]. Default 90: a ballistic descent turns broadside, and
    /// lookups beyond the table make the results unusable.
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
            alpha_max_deg: 90.0,
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
    /// Fly with the quasi-steady aerodynamics only: no apparent-mass (lag and convective) terms, no
    /// Magnus force or moment, no rotation-induced (CN_q) force (sensitivity checks; default false).
    pub quasi_steady: bool,
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
            quasi_steady: false,
        }
    }
}

/// Compute resources IgnisYeet may use. They never change results, so they are not part of the aero hash.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct ResourcesCfg {
    /// Maximum number of CPU threads for everything; 0 = all available.
    pub threads: usize,
    /// Memory budget in GB for heavy steps (CFD cases, the panel dense matrix); 0 = the memory available to the
    /// process (available memory, cgroup limit, address-space limit; see `panel::memory`).
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
            bail!("[IY-C006] resources.memory_gb must be 0 (use the available memory) or positive");
        }
        if !(0..=19).contains(&self.nice) {
            bail!("[IY-C007] resources.nice must be between 0 and 19");
        }
        Ok(())
    }

    pub fn resolve(&self) -> Resolved {
        let available = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
        let mut warning = None;
        let threads = if self.threads == 0 {
            available
        } else if self.threads > available {
            warning = Some(format!("[IY-C501] resources.threads = {} exceeds the {available} available threads; using {available}", self.threads));
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
    /// Loads the configuration file with the command-line [`Overrides`] applied before validation.
    pub fn load_with(path: &Path, ov: &Overrides) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("[IY-C060] cannot read config {}", path.display()))?;
        let mut raw: toml::Value =
            toml::from_str(text.strip_prefix('\u{feff}').unwrap_or(&text)).with_context(|| format!("[IY-C061] invalid config {}", path.display()))?;
        let base = raw.clone();
        let repeated = apply_sets(&mut raw, &ov.set)?;
        let what = if ov.set.is_empty() { String::new() } else { " with the --set overrides".to_string() };
        let mut cfg = Self::from_value(raw, path.parent()).map_err(|e| blame_set(&base, &ov.set).unwrap_or(e)).map_err(|e| {
            // The headline carries its own code only when the cause has none (an unknown key, a wrong type).
            let code = if format!("{e:#}").contains("[IY-") { "" } else { "[IY-C062] " };
            e.context(format!("{code}invalid config {}{what}", path.display()))
        })?;
        cfg.load_warnings.splice(0..0, repeated);
        if let Some(out) = &ov.out {
            // Relative to the current directory, not to the config file.
            cfg.output.dir =
                if out.is_absolute() { out.clone() } else { std::env::current_dir().context("[IY-C063] cannot read the current directory")?.join(out) };
        }
        Ok(cfg)
    }

    pub(crate) fn from_value(raw: toml::Value, dir: Option<&Path>) -> Result<Self> {
        if raw.get("environment").is_some() {
            bail!(
                "[IY-C008] the [environment] section was removed: move wind_speed, wind_direction_deg, wind_ref_height and wind_exponent \
                 to [wind] as speed, direction_deg, ref_height and exponent"
            );
        }
        let explicit_wind = |k: &str| raw.get("wind").and_then(|w| w.get(k)).is_some();
        let (explicit_speed, explicit_direction) = (explicit_wind("speed"), explicit_wind("direction_deg"));
        let mut cfg: Config = raw.try_into()?;
        cfg.base_dir = dir.map(Path::to_path_buf).unwrap_or_default();
        cfg.validate()?;
        cfg.check_profile_targets(explicit_speed, explicit_direction)?;
        Ok(cfg)
    }

    /// Range checks of every numeric key (finite, signs, bounds); errors name the key.
    fn validate_numbers(&self) -> Result<()> {
        fn finite(key: &str, v: f64) -> Result<()> {
            if !v.is_finite() {
                bail!("[IY-C009] {key} must be a finite number, found {v}");
            }
            Ok(())
        }
        fn positive(key: &str, v: f64) -> Result<()> {
            if !(v.is_finite() && v > 0.0) {
                bail!("[IY-C010] {key} must be a finite number > 0, found {v}");
            }
            Ok(())
        }
        fn non_negative(key: &str, v: f64) -> Result<()> {
            if !(v.is_finite() && v >= 0.0) {
                bail!("[IY-C011] {key} must be a finite number >= 0, found {v}");
            }
            Ok(())
        }
        fn range(key: &str, v: f64, lo: f64, hi: f64) -> Result<()> {
            if !(v.is_finite() && (lo..=hi).contains(&v)) {
                bail!("[IY-C012] {key} must be between {lo} and {hi}, found {v}");
            }
            Ok(())
        }
        let (r, m, l) = (&self.rocket, &self.motor, &self.launch);
        positive("rocket.stl_scale", r.stl_scale)?;
        positive("rocket.dry_mass", r.dry_mass)?;
        finite("rocket.cg_dry", r.cg_dry)?;
        if r.cg_dry < 0.0 {
            bail!("[IY-C013] rocket.cg_dry must be >= 0 (measured aft of the nose tip), found {}", r.cg_dry);
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
            // The panel mesh and the CAD surface of the CFD mesh build the fins as biconvex sections of this
            // thickness and leave fins without thickness out; Barrowman needs it only for the fin drag.
            let meshed =
                self.aero.method == AeroMethod::Panel || (self.aero.method == AeroMethod::Cfd && self.aero.cfd.surface == cfd::config::SurfaceKind::PanelMesh);
            if let (Some(t), true) = (o.thickness, meshed) {
                if t <= 0.0 {
                    bail!(
                        "[IY-C014] rocket.fin_override.thickness must be > 0 for aero.method = \"{}\", found {t}: the fins are meshed as biconvex sections of this thickness and fins without thickness would be left out of the mesh",
                        if self.aero.method == AeroMethod::Panel { "panel" } else { "cfd" }
                    );
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
            bail!("[IY-C015] launch.elevation_deg must be in (0, 90], found {}", l.elevation_deg);
        }
        finite("launch.azimuth_deg", l.azimuth_deg)?;
        finite("launch.roll_deg", l.roll_deg)?;
        non_negative("recovery.cd_s", self.recovery.cd_s)?;
        if self.recovery.enabled {
            positive("recovery.cd_s (recovery.enabled = true)", self.recovery.cd_s)?;
        }
        non_negative("recovery.delay", self.recovery.delay)?;
        self.recovery.validate()?;
        if self.sim.descent == Some(sim::Descent::Parachute) && !self.recovery.enabled {
            bail!("[IY-C016] sim.descent = \"parachute\" needs recovery.enabled = true");
        }

        let a = &self.aero;
        non_negative("aero.mach_min", a.mach_min)?;
        finite("aero.mach_max", a.mach_max)?;
        if a.mach_max <= a.mach_min {
            bail!("[IY-C017] aero.mach_max ({}) must be greater than aero.mach_min ({})", a.mach_max, a.mach_min);
        }
        positive("aero.mach_step", a.mach_step)?;
        positive("aero.alpha_max_deg", a.alpha_max_deg)?;
        positive("aero.alpha_step_deg", a.alpha_step_deg)?;
        positive("aero.general_mach_step", a.general_mach_step)?;
        if a.mach_max > MACH_LIMIT {
            bail!("[IY-C018] aero.mach_max = {} is too large for a rocket flight (<= {MACH_LIMIT})", a.mach_max);
        }
        if a.alpha_max_deg > 180.0 {
            bail!("[IY-C019] aero.alpha_max_deg = {} is outside (0, 180]", a.alpha_max_deg);
        }
        // The Mach step the table is really built with (see `aero_options`).
        let general = a.table_kind == aero::table::TableKind::General;
        let step = if general && a.method == AeroMethod::Barrowman { a.general_mach_step } else { a.mach_step };
        let step_key = if general && a.method == AeroMethod::Barrowman { "aero.general_mach_step" } else { "aero.mach_step" };
        let n_mach = ((a.mach_max - a.mach_min) / step).round().max(1.0) + 1.0;
        // One-sided total-angle grid 0..alpha_max (as `AeroTable::build`).
        let n_alpha = (a.alpha_max_deg / a.alpha_step_deg).round().max(1.0) + 1.0;
        let mut points = n_mach * n_alpha;
        let mut what = format!("{n_mach} Mach x {n_alpha} alpha");
        // Every method but an external table is expanded to the general grid when the table kind is general.
        if general && a.method != AeroMethod::Table {
            let (alphas, betas) = crate::general_axes();
            points = points.max(n_mach * (alphas.len() * betas.len()) as f64);
            what = format!(
                "{n_mach} Mach x {n_alpha} alpha (axisymmetric table), then {n_mach} Mach x {} alpha x {} beta (general table)",
                alphas.len(),
                betas.len()
            );
        }
        if points > MAX_AERO_GRID_POINTS {
            bail!(
                "[IY-C020] the aero table grid would have about {points:.3e} points ({what}; at most {MAX_AERO_GRID_POINTS:e}): increase {step_key} / aero.alpha_step_deg or reduce aero.mach_max / aero.alpha_max_deg"
            );
        }
        if a.n_slices < MIN_SLICES {
            bail!("[IY-C021] aero.n_slices = {} is too small: the geometry extraction needs at least {MIN_SLICES} slices", a.n_slices);
        }
        if a.n_slices > MAX_SLICES {
            bail!("[IY-C022] aero.n_slices = {} is too large (at most {MAX_SLICES})", a.n_slices);
        }
        non_negative("aero.fin_threshold", a.fin_threshold)?;
        // [aero.panel] is checked whatever the method, so a bad value fails before the panel run that reads it.
        a.panel.validate()?;
        let hi_max = a.mach_max.max(PANEL_TRANSONIC_HI_MIN_CAP);
        if a.panel.transonic[1] > hi_max {
            bail!(
                "[IY-C023] aero.panel.transonic[1] = {} is above the table's aero.mach_max ({}): the transonic blend would run beyond the table; use at most {hi_max}",
                a.panel.transonic[1],
                a.mach_max
            );
        }

        let w = &self.wind;
        non_negative("wind.speed", w.speed)?;
        range("wind.direction_deg", w.direction_deg, 0.0, 360.0)?;
        positive("wind.ref_height", w.ref_height)?;
        // Each profile parameter is checked only for the model that uses it (as in `Wind::from_config`).
        if w.model == sim::WindModel::Power {
            positive("wind.exponent", w.exponent)?;
        }
        if w.model == sim::WindModel::Log {
            positive("wind.roughness_length", w.roughness_length)?;
        }
        if matches!(w.model, sim::WindModel::Power | sim::WindModel::Log) {
            positive("wind.ground_blend_height", w.ground_blend_height)?;
        }
        if w.model == sim::WindModel::Log && 2.0 * w.roughness_length >= w.ref_height {
            bail!(
                "[IY-C024] wind.roughness_length ({}) must be smaller than half of wind.ref_height ({}) for the log profile",
                w.roughness_length,
                w.ref_height
            );
        }

        let s = &self.sim;
        positive("sim.dt", s.dt)?;
        if s.dt > MAX_DT {
            bail!("[IY-C025] sim.dt = {} s is too large for a rocket flight (<= {MAX_DT} s; 0.002 s is typical)", s.dt);
        }
        if s.dt < MIN_DT {
            bail!("[IY-C026] sim.dt = {} s is too small (>= {MIN_DT:e} s): the run would take days; 0.002 s is typical", s.dt);
        }
        positive("sim.max_time", s.max_time)?;
        if !s.integrator.is_adaptive() && s.max_time / s.dt > MAX_STEPS {
            bail!(
                "[IY-C027] sim.max_time / sim.dt = {:.3e} steps is too many (at most {MAX_STEPS:e}): increase sim.dt or reduce sim.max_time",
                s.max_time / s.dt
            );
        }
        positive("sim.output_interval", s.output_interval)?;
        if s.output_interval < MIN_OUTPUT_INTERVAL {
            bail!(
                "[IY-C028] sim.output_interval = {} s is too small (>= {MIN_OUTPUT_INTERVAL} s): it would write an enormous trajectory and limit the adaptive step",
                s.output_interval
            );
        }
        if s.max_time / s.output_interval > MAX_OUTPUT_ROWS {
            bail!(
                "[IY-C029] sim.max_time / sim.output_interval = {:.3e} output rows is too many (at most {MAX_OUTPUT_ROWS:e}): increase sim.output_interval",
                s.max_time / s.output_interval
            );
        }
        if s.output_interval > s.max_time {
            bail!("[IY-C030] sim.output_interval = {} s exceeds sim.max_time = {} s", s.output_interval, s.max_time);
        }
        positive("sim.rtol", s.rtol)?;
        positive("sim.atol", s.atol)?;
        for (k, v) in [("rtol", s.rtol), ("atol", s.atol)] {
            if v > MAX_TOL {
                bail!("[IY-C031] sim.{k} = {v} is too loose for a rocket flight (<= {MAX_TOL:e}; 1e-8 is typical)");
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
            ("lag_scale", mc.lag_scale),
            ("magnus_scale", mc.magnus_scale),
            ("cmq_scale", mc.cmq_scale),
        ] {
            non_negative(&format!("dispersion.monte_carlo.{k} (1-sigma)"), v)?;
        }
        if let Some(k) = mc.truncate_sigma {
            if !(k.is_finite() && k > 0.0) {
                bail!("[IY-C032] dispersion.monte_carlo.truncate_sigma must be a finite number > 0 (the bound of the clipped draws in sigma, e.g. 3.0), found {k}");
            }
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        self.validate_numbers()?;
        if !matches!(self.rocket.fin_le.as_str(), "rounded" | "sharp") {
            bail!("[IY-C033] rocket.fin_le must be \"rounded\" or \"sharp\"");
        }
        if !matches!(self.rocket.fin_te.as_str(), "square" | "tapered") {
            bail!("[IY-C034] rocket.fin_te must be \"square\" or \"tapered\"");
        }
        if self.rocket.dry_mass <= 0.0 || self.rocket.iyy_dry <= 0.0 || self.rocket.ixx_dry <= 0.0 {
            bail!("[IY-C035] rocket mass and inertias must be positive");
        }
        let r = &self.rocket;
        let izz = r.izz_dry.unwrap_or(r.iyy_dry);
        let tensor = geom::Mat3([[r.ixx_dry, r.ixy_dry, r.ixz_dry], [r.ixy_dry, r.iyy_dry, r.iyz_dry], [r.ixz_dry, r.iyz_dry, izz]]);
        let minors = [r.ixx_dry, r.ixx_dry * r.iyy_dry - r.ixy_dry * r.ixy_dry, tensor.det()];
        if !(izz > 0.0 && minors.iter().all(|m| *m > 0.0)) {
            bail!("[IY-C036] the inertia tensor of the rocket (ixx_dry, iyy_dry, izz_dry and the products) must be positive definite");
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
            bail!("[IY-C037] rocket inertia products, CG offset, fin cant, motor misalignment and launch.roll_deg must be finite numbers");
        }
        if self.aero.general_mach_step <= 0.0 {
            bail!("[IY-C038] aero.general_mach_step must be positive");
        }
        if self.aero.mach_step <= 0.0 || self.aero.alpha_step_deg <= 0.0 || self.aero.mach_max <= self.aero.mach_min {
            bail!("[IY-C039] invalid aero grid");
        }
        if self.dispersion.wind_speeds.is_empty() {
            bail!("[IY-C040] dispersion.wind_speeds must list at least one speed");
        }
        if self.dispersion.directions == 0 {
            bail!("[IY-C041] dispersion.directions must be at least 1");
        }
        let d = &self.dispersion;
        if d.directions > MAX_DIRECTIONS {
            bail!("[IY-C042] dispersion.directions = {} is too large (at most {MAX_DIRECTIONS})", d.directions);
        }
        if d.wind_speeds.len() > MAX_WIND_SPEEDS {
            bail!("[IY-C043] dispersion.wind_speeds lists {} speeds (at most {MAX_WIND_SPEEDS})", d.wind_speeds.len());
        }
        if d.wind_speeds.len() * d.directions > MAX_WIND_GRID_CASES {
            bail!("[IY-C044] the wind grid would have {} cases (wind_speeds x directions; at most {MAX_WIND_GRID_CASES})", d.wind_speeds.len() * d.directions);
        }
        if d.monte_carlo.samples == 0 {
            bail!("[IY-C045] dispersion.monte_carlo.samples must be at least 1");
        }
        if d.monte_carlo.samples > MAX_MC_SAMPLES {
            bail!("[IY-C046] dispersion.monte_carlo.samples = {} is too large (at most {MAX_MC_SAMPLES})", d.monte_carlo.samples);
        }
        if self.aero.method == AeroMethod::Table && self.aero.table.is_none() {
            bail!("[IY-C047] aero.table is required when aero.method = \"table\"");
        }
        if self.aero.method == AeroMethod::Cfd {
            self.aero.cfd.validate()?;
            if self.aero.cfd.surface == cfd::config::SurfaceKind::Step {
                let f = self.resolve(&self.aero.cfd.step_file);
                if !f.is_file() {
                    bail!(
                        "[IY-C048] aero.cfd.step_file = \"{}\": no such file ({}; a relative path is relative to the configuration file)",
                        self.aero.cfd.step_file.display(),
                        f.display()
                    );
                }
            }
        }
        self.atmosphere.build()?;
        if self.earth.model == sim::EarthModel::Flat && self.earth.gravity == sim::GravityModel::J2 {
            bail!("[IY-C049] earth.gravity = \"j2\" requires earth.model = \"ecef\"");
        }
        if self.wind.model == sim::WindModel::Profile && self.wind.profile.is_none() {
            bail!("[IY-C050] wind.profile (CSV path) is required when wind.model = \"profile\"");
        }
        if self.wind.ref_height <= 0.0 {
            bail!("[IY-C051] wind.ref_height must be positive");
        }
        if self.wind.model == sim::WindModel::Log && !(self.wind.roughness_length > 0.0 && self.wind.roughness_length < self.wind.ref_height) {
            bail!("[IY-C052] wind.roughness_length must be positive and smaller than wind.ref_height");
        }
        if matches!(self.wind.model, sim::WindModel::Power | sim::WindModel::Log)
            && !(self.wind.ground_blend_height > 0.0 && self.wind.ground_blend_height < self.wind.ref_height)
        {
            bail!("[IY-C053] wind.ground_blend_height must be positive and smaller than wind.ref_height");
        }
        if self.sim.dt <= 0.0 || self.sim.output_interval <= 0.0 {
            bail!("[IY-C054] sim.dt and sim.output_interval must be positive");
        }
        if self.sim.integrator.is_adaptive() && !(self.sim.rtol > 0.0 && self.sim.atol > 0.0) {
            bail!("[IY-C055] sim.rtol and sim.atol must be positive for the adaptive integrators (rk45, dop853)");
        }
        self.resources.validate()?;
        self.nose_direction()?;
        Ok(())
    }

    /// `wind.profile_mean_speed` / `profile_mean_direction_deg` (profile model only) and warnings
    /// for `wind.speed` / `direction_deg` set in the file although the profile model ignores them.
    fn check_profile_targets(&mut self, explicit_speed: bool, explicit_direction: bool) -> Result<()> {
        let w = &self.wind;
        let profile = w.model == sim::WindModel::Profile;
        for (key, v) in [("wind.profile_mean_speed", w.profile_mean_speed), ("wind.profile_mean_direction_deg", w.profile_mean_direction_deg)] {
            if v.is_some() && !profile {
                bail!(
                    "[IY-C056] {key} only applies to wind.model = \"profile\" (the model is \"{}\"); set wind.speed / wind.direction_deg instead",
                    model_name(w.model)
                );
            }
        }
        if let Some(v) = w.profile_mean_speed {
            if !(v.is_finite() && v >= 0.0) {
                bail!("[IY-C057] wind.profile_mean_speed must be a finite number >= 0, found {v}");
            }
        }
        if let Some(v) = w.profile_mean_direction_deg {
            if !(v.is_finite() && (0.0..=360.0).contains(&v)) {
                bail!("[IY-C058] wind.profile_mean_direction_deg must be between 0 and 360, found {v}");
            }
        }
        if profile && explicit_speed {
            self.load_warnings.push("[IY-C502] wind.speed is ignored for wind.model = \"profile\"; use wind.profile_mean_speed to scale the profile".into());
        }
        if profile && explicit_direction {
            self.load_warnings
                .push("[IY-C503] wind.direction_deg is ignored for wind.model = \"profile\"; use wind.profile_mean_direction_deg to rotate the profile".into());
        }
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

    /// `[aero.cfd]` with `step_file` resolved against the configuration directory (what the CFD mode is given).
    pub fn cfd_options(&self) -> cfd::CfdOptions {
        let mut o = self.aero.cfd.clone();
        if !o.step_file.as_os_str().is_empty() {
            o.step_file = self.resolve(&o.step_file);
        }
        o
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
            other => bail!("[IY-C059] rocket.nose_direction '{other}' is not one of auto, +x, -x, +y, -y, +z, -z"),
        })
    }
}

fn model_name(m: sim::WindModel) -> String {
    serde_json::to_value(m).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
}

/// Largest accepted `sim.dt` [s].
const MAX_DT: f64 = 0.05;
/// Largest accepted `sim.rtol` and `sim.atol`.
const MAX_TOL: f64 = 1e-3;
/// Largest accepted `aero.mach_max`.
const MACH_LIMIT: f64 = 10.0;
/// Upper edge of the panel transonic blend that is always accepted (the default), also for tables
/// that end below it; otherwise the edge must lie within the table (`aero.mach_max`).
const PANEL_TRANSONIC_HI_MIN_CAP: f64 = 1.2;
/// Largest accepted number of aero table grid points.
const MAX_AERO_GRID_POINTS: f64 = 1e6;
/// Smallest sim.output_interval [s] and largest number of output rows (max_time / output_interval).
const MIN_OUTPUT_INTERVAL: f64 = 1e-4;
/// Smallest sim.dt [s] and largest number of steps (max_time / dt).
const MIN_DT: f64 = 1e-5;
const MAX_STEPS: f64 = 1e8;
const MAX_OUTPUT_ROWS: f64 = 1e7;
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
    use crate::test_tmp::TempDir;

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
    fn cfd_wall_size_nan_is_rejected_not_read_as_unset() {
        let sample = include_str!("../../../examples/sample.toml");
        let with = |v: &str| parse(&format!("{sample}\n[aero.cfd]\nwall_size = {v}\n"));
        for bad in ["nan", "inf", "-inf"] {
            let e = format!("{:#}", with(bad).err().unwrap_or_else(|| panic!("wall_size = {bad} was accepted")));
            assert!(e.contains("wall_size") && e.contains("finite"), "{e}");
        }
        assert_eq!(with("0.003").unwrap().aero.cfd.wall_size, 0.003);
    }

    #[test]
    fn monte_carlo_truncation_key() {
        let sample = include_str!("../../../examples/sample.toml");
        assert_eq!(parse(sample).unwrap().dispersion.monte_carlo.truncate_sigma, None);
        let with = |v: &str| sample.replace("[dispersion.monte_carlo]\n", &format!("[dispersion.monte_carlo]\ntruncate_sigma = {v}\n"));
        assert_eq!(parse(&with("3.0")).unwrap().dispersion.monte_carlo.truncate_sigma, Some(3.0));
        for bad in ["0.0", "-1.0", "nan", "inf"] {
            let e = parse(&with(bad)).unwrap_err().to_string();
            assert!(e.contains("dispersion.monte_carlo.truncate_sigma"), "{bad}: {e}");
        }
    }

    #[test]
    fn panel_options_are_validated() {
        let sample = include_str!("../../../examples/sample.toml");
        let cases = [
            ("subsonic_machs = [0.0, 0.3, 0.5, 0.6, 0.7, 0.8]", "subsonic_machs = [nan, 0.3]", "aero.panel.subsonic_machs"),
            ("tail_radii = 6.0", "tail_radii = -1", "aero.panel.tail_radii"),
            ("tail_radii = 6.0", "tail_radii = nan", "aero.panel.tail_radii"),
            ("wake_length = 20.0", "wake_length = 0", "aero.panel.wake_length"),
            ("wake_length = 20.0", "wake_length = -5", "aero.panel.wake_length"),
            ("wake_length = 20.0", "wake_length = nan", "aero.panel.wake_length"),
            ("transonic = [0.8, 1.2]", "transonic = [nan, 1.2]", "aero.panel.transonic"),
            ("transonic = [0.8, 1.2]", "transonic = [0.3, 5]", "aero.panel.transonic[1]"),
            ("body_circ = 48", "body_circ = 4", "aero.panel.body_circ"),
            ("fin_chord = 16", "fin_chord = 0", "aero.panel.fin_chord"),
        ];
        for (from, to, key) in cases {
            assert!(sample.contains(from), "{from}");
            let e = parse(&sample.replace(from, to)).unwrap_err().to_string();
            assert!(e.contains(key), "{to}: {e}");
        }
        // the default blend edge stays valid for a table that ends below it
        assert!(parse(&sample.replace("mach_max = 3.0", "mach_max = 1.0")).is_ok());
        assert!(parse(&sample.replace("transonic = [0.8, 1.2]", "transonic = [0.6, 2.5]")).is_ok());
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
    fn wind_direction_is_range_checked() {
        let sample = include_str!("../../../examples/sample.toml");
        let at = |d: &str| sample.replace("direction_deg = 0.0 ", &format!("direction_deg = {d} #"));
        assert!(parse(&at("400.0")).unwrap_err().to_string().contains("wind.direction_deg must be between 0 and 360, found 400"));
        assert!(parse(&at("-10.0")).unwrap_err().to_string().contains("wind.direction_deg"));
        assert!(parse(&at("360.0")).is_ok());
    }

    #[test]
    fn effective_mach_step_and_output_interval_are_capped() {
        let sample = include_str!("../../../examples/sample.toml");
        let general = sample.replace("extrapolation = \"linear\" ", "table_kind = \"general\"\ngeneral_mach_step = 1e-6\nextrapolation = \"linear\" ");
        let e = parse(&general).unwrap_err().to_string();
        assert!(e.contains("aero table grid") && e.contains("aero.general_mach_step"), "{e}");
        assert!(parse(&general.replace("general_mach_step = 1e-6", "general_mach_step = 0.1")).is_ok());
        // an axisymmetric table does not use the general step
        assert!(parse(sample).is_ok());
        let e = sample_with("output_interval = 0.05", "output_interval = 1e-9").unwrap_err().to_string();
        assert!(e.contains("sim.output_interval") && e.contains("too small"), "{e}");
        let e = sample_with("output_interval = 0.05", "output_interval = 0.0002").unwrap_err().to_string();
        assert!(e.contains("output rows"), "{e}");
    }

    #[test]
    fn dt_has_a_lower_bound_and_a_step_count_limit() {
        let e = sample_with("dt = 0.002", "dt = 1e-9").unwrap_err().to_string();
        assert!(e.contains("sim.dt = 0.000000001 s is too small (>= 1e-5 s)"), "{e}");
        assert!(sample_with("dt = 0.002", "dt = 1e-4").is_ok());
        let e = sample_with("dt = 0.002", "dt = 1e-5").unwrap_err().to_string();
        assert!(e.contains("sim.max_time / sim.dt = 3.600e8 steps is too many (at most 1e8)"), "{e}");
        // the adaptive integrators do not step with dt
        let rk45 = include_str!("../../../examples/sample.toml").replace("integrator = \"rk4\"", "integrator = \"rk45\"").replace("dt = 0.002", "dt = 1e-5");
        assert!(parse(&rk45).is_ok());
    }

    #[test]
    fn set_shape_is_checked_by_the_parser_and_huge_integers_are_named() {
        assert_eq!(parse_set_arg("wind.speed=3").unwrap(), "wind.speed=3");
        assert!(parse_set_arg("wind.speed").unwrap_err().contains("[IY-C002] expected KEY=VALUE"));
        assert!(parse_set_arg("wind..speed=3").unwrap_err().contains("[IY-C001] invalid key"));
        let e = sample_set(&["dispersion.monte_carlo.seed=18446744073709551615"]).unwrap_err().to_string();
        assert_eq!(e, "[IY-C003] --set dispersion.monte_carlo.seed=18446744073709551615: integer out of range (TOML integers are at most 9223372036854775807)");
        assert!(sample_set(&["dispersion.monte_carlo.seed=9223372036854775807"]).is_ok());
        assert!(sample_set(&["dispersion.monte_carlo.seed=-9223372036854775809"]).unwrap_err().to_string().contains("integer out of range"));
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
    fn parachute_descent_needs_enabled_recovery() {
        let sample = include_str!("../../../examples/sample.toml");
        let off = sample.replacen("enabled = true", "enabled = false", 1);
        let chute = off.replacen("[sim]", "[sim]\ndescent = \"parachute\"", 1);
        assert_eq!(parse(&chute).unwrap_err().to_string(), "[IY-C016] sim.descent = \"parachute\" needs recovery.enabled = true");
        parse(&off.replacen("[sim]", "[sim]\ndescent = \"ballistic\"", 1)).unwrap();
        parse(&sample.replacen("[sim]", "[sim]\ndescent = \"parachute\"", 1)).unwrap();
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
        // These sigmas are not set in the sample, so they are added under the header.
        for (line, key) in [
            ("thrust_misalignment_deg = -1.0", "thrust_misalignment_deg"),
            ("thrust_offset = -0.001", "thrust_offset"),
            ("fin_cant_deg = -0.2", "fin_cant_deg"),
            ("cg_lateral = nan", "cg_lateral"),
            ("cl_scale = -0.1", "cl_scale"),
            ("clp_scale = -0.1", "clp_scale"),
            ("lag_scale = -0.1", "lag_scale"),
            ("magnus_scale = inf", "magnus_scale"),
            ("cmq_scale = nan", "cmq_scale"),
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
    fn default_aero_table_reaches_90_degrees() {
        // A ballistic descent reaches alpha ~ 90 deg; a shorter table is extrapolated there.
        assert_eq!(AeroCfg::default().alpha_max_deg, 90.0);
        assert_eq!(aero::AeroOptions::default().alpha_max_deg, 90.0);
        let sample = include_str!("../../../examples/sample.toml");
        let c = parse(&sample.replace("alpha_max_deg = 90.0 ", "# alpha_max_deg removed ")).unwrap();
        assert_eq!(c.aero.alpha_max_deg, 90.0);
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
    fn aero_grid_cap_counts_the_one_sided_alpha_grid() {
        // 3001 Mach x 301 alpha (0..90 deg in 0.3 deg) = 903301 points: within the cap of 1e6.
        let sample = include_str!("../../../examples/sample.toml");
        let t = sample.replacen("mach_step = 0.02", "mach_step = 0.001", 1).replacen("alpha_step_deg = 1.0", "alpha_step_deg = 0.3", 1);
        parse(&t).unwrap();
        let e = parse(&t.replacen("alpha_step_deg = 0.3", "alpha_step_deg = 0.2", 1)).unwrap_err().to_string();
        assert!(e.contains("(3001 Mach x 451 alpha;"), "{e}");
    }

    #[test]
    fn output_without_dir_and_zero_monte_carlo_samples() {
        let sample = include_str!("../../../examples/sample.toml");
        let c = parse(&sample.replacen("dir = \"out\"", "", 1)).unwrap();
        assert_eq!(c.output.dir, PathBuf::from("out"));
        assert!(c.output.kml);
        let e = parse(&sample.replacen("samples = 1000", "samples = 0", 1)).unwrap_err().to_string();
        assert!(e.contains("dispersion.monte_carlo.samples must be at least 1"), "{e}");
    }

    #[test]
    fn panel_and_cad_cfd_need_fins_with_thickness() {
        let sample = include_str!("../../../examples/sample.toml");
        let fins = "[rocket.fin_override]\nthickness = 0.0\n\n[launch]";
        let t = sample.replacen("[launch]", fins, 1);
        // Barrowman uses the thickness only for the fin drag: zero is allowed.
        parse(&t).unwrap();
        for method in ["panel", "cfd"] {
            let e = parse(&t.replacen("method = \"barrowman\"", &format!("method = \"{method}\""), 1)).unwrap_err().to_string();
            assert!(e.contains("rocket.fin_override.thickness must be > 0"), "{e}");
        }
        let ok =
            sample.replacen("[launch]", "[rocket.fin_override]\nthickness = 0.004\n\n[launch]", 1).replacen("method = \"barrowman\"", "method = \"panel\"", 1);
        parse(&ok).unwrap();
    }

    #[test]
    fn general_table_expansion_counts_for_every_method_that_expands() {
        let sample = include_str!("../../../examples/sample.toml");
        let t =
            sample.replacen("method = \"barrowman\"", "method = \"panel\"", 1).replacen("mach_step = 0.02", "mach_step = 0.001\ntable_kind = \"general\"", 1);
        let e = parse(&t).unwrap_err().to_string();
        assert!(e.contains("the aero table grid would have about") && e.contains("beta (general table)"), "{e}");
        // the axisymmetric table of the same fine step is within the cap
        parse(&t.replacen("table_kind = \"general\"", "table_kind = \"axisymmetric\"", 1)).unwrap();
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

    fn sample_set(sets: &[&str]) -> Result<Config> {
        let mut raw: toml::Value = toml::from_str(include_str!("../../../examples/sample.toml"))?;
        for a in sets {
            apply_set(&mut raw, a)?;
        }
        Config::from_value(raw, None)
    }

    #[test]
    fn set_overrides_parse_toml_values_and_bare_strings() {
        let c = sample_set(&[
            "wind.speed=7.5",
            "dispersion.mode=monte_carlo",
            "dispersion.wind_speeds=[2.0, 4.0]",
            "dispersion.monte_carlo.samples=3",
            "output.kml=false",
        ])
        .unwrap();
        assert_eq!(c.wind.speed, 7.5);
        assert_eq!(c.dispersion.mode, sim::DispersionMode::MonteCarlo);
        assert_eq!(c.dispersion.wind_speeds, vec![2.0, 4.0]);
        assert_eq!(c.dispersion.monte_carlo.samples, 3);
        assert!(!c.output.kml);
        // a quoted TOML string, whitespace around the parts, a new table
        let c = sample_set(&[" dispersion.mode = \"wind_grid\" ", "rocket.nose_direction=-z", "rocket.fin_override.count=4"]).unwrap();
        assert_eq!(c.dispersion.mode, sim::DispersionMode::WindGrid);
        assert_eq!(c.rocket.nose_direction, "-z");
        assert_eq!(c.rocket.fin_override.as_ref().and_then(|f| f.count), Some(4));
        // integers are accepted for floats as in the file
        assert_eq!(sample_set(&["wind.speed=6"]).unwrap().wind.speed, 6.0);
    }

    #[test]
    fn set_overrides_are_checked_like_the_file() {
        let e = sample_set(&["wind.sped=3"]).unwrap_err().to_string();
        assert!(e.contains("unknown field") && e.contains("sped"), "{e}");
        let e = format!("{:#}", sample_set(&["wind.speed=-1"]).unwrap_err());
        assert!(e.contains("wind.speed"), "{e}");
        let e = format!("{:#}", sample_set(&["wind.speed=fast"]).unwrap_err());
        assert!(e.contains("invalid type") || e.contains("expected"), "{e}");
        assert!(sample_set(&["wind.speed"]).unwrap_err().to_string().contains("expected KEY=VALUE"));
        assert!(sample_set(&["wind..speed=1"]).unwrap_err().to_string().contains("invalid key"));
        let e = sample_set(&["wind.model.x=1"]).unwrap_err().to_string();
        assert!(e.contains("wind.model is not a table"), "{e}");
    }

    #[test]
    fn rans_sample_uses_the_rans_defaults() {
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let c = Config::load_with(&examples.join("sample_rans.toml"), &Overrides::default()).unwrap();
        let o = c.cfd_options();
        assert_eq!(o.model, cfd::FlowModel::Rans);
        assert_eq!((o.wall_size, o.iterations, o.min_iterations, o.stable_window), (0.005, 4000, 400, 200));
        assert!(o.warnings_for(64).is_empty(), "{:?}", o.warnings_for(64));
        // The Euler sample sets wall_size and iterations explicitly: switched to RANS it keeps them and warns.
        let text = std::fs::read_to_string(examples.join("sample_cfd.toml")).unwrap().replace("model = \"euler\"", "model = \"rans\"");
        let dir = TempDir::new("cfg_rans");
        std::fs::create_dir_all(&dir).unwrap();
        for f in ["sample_rocket.stl", "sample_motor.eng"] {
            std::fs::copy(examples.join(f), dir.join(f)).unwrap();
        }
        std::fs::write(dir.join("c.toml"), text).unwrap();
        let o = Config::load_with(&dir.join("c.toml"), &Overrides::default()).unwrap().cfd_options();
        assert_eq!((o.model, o.wall_size, o.iterations), (cfd::FlowModel::Rans, 0.002, 3000));
        assert!(o.warnings_for(64).iter().any(|w| w.contains("wall_size = 0.002 m")), "{:?}", o.warnings_for(64));
    }

    #[test]
    fn step_sample_resolves_the_cad_file_next_to_the_config() {
        let examples = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples");
        let c = Config::load_with(&examples.join("sample_step.toml"), &Overrides::default()).unwrap();
        assert_eq!(c.aero.cfd.surface, cfd::SurfaceKind::Step);
        // The configuration keeps the relative path; the CFD mode gets it resolved against the configuration directory.
        assert_eq!(c.aero.cfd.step_file, std::path::PathBuf::from("sample_rocket.step"));
        assert_eq!(c.cfd_options().step_file, examples.join("sample_rocket.step"));
        assert!(c.cfd_options().step_file.is_file());
        // A missing file is an error at load time that names the key and the resolved path.
        let dir = TempDir::new("cfg_step");
        std::fs::create_dir_all(&dir).unwrap();
        let text = std::fs::read_to_string(examples.join("sample_step.toml")).unwrap();
        std::fs::write(dir.join("c.toml"), &text).unwrap();
        let e = format!("{:#}", Config::load_with(&dir.join("c.toml"), &Overrides::default()).unwrap_err());
        assert!(
            e.contains("aero.cfd.step_file = \"sample_rocket.step\": no such file") && e.contains(&dir.join("sample_rocket.step").display().to_string()),
            "{e}"
        );
        // Not a CAD format.
        std::fs::write(dir.join("c.toml"), text.replace("step_file = \"sample_rocket.step\"", "step_file = \"rocket.stl\"")).unwrap();
        let e = format!("{:#}", Config::load_with(&dir.join("c.toml"), &Overrides::default()).unwrap_err());
        assert!(e.contains("not a CAD file OpenCASCADE reads"), "{e}");
    }

    #[test]
    fn load_with_applies_set_and_out() {
        let dir = TempDir::new("cfg_ov");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("c.toml");
        std::fs::write(&path, include_str!("../../../examples/sample.toml")).unwrap();
        let ov = Overrides { set: vec!["wind.speed=9".into()], out: Some("run_a".into()) };
        let c = Config::load_with(&path, &ov).unwrap();
        assert_eq!(c.wind.speed, 9.0);
        // --out is relative to the current directory, not to the config file
        assert_eq!(c.out_dir(), std::env::current_dir().unwrap().join("run_a"));
        let abs = Overrides { out: Some(dir.join("abs")), ..Default::default() };
        assert_eq!(Config::load_with(&path, &abs).unwrap().out_dir(), dir.join("abs"));
        let bad = Overrides { set: vec!["wind.nope=1".into()], out: None };
        let e = format!("{:#}", Config::load_with(&path, &bad).unwrap_err());
        assert!(e.contains("with the --set overrides") && e.contains("[IY-C005] --set wind.nope=1: unknown field `nope`") && !e.contains("IY-C062"), "{e}");
        // a value of the wrong type names the override
        let bad = Overrides { set: vec!["wind.speed=5".into(), "rocket.dry_mass=abc".into()], out: None };
        let e = format!("{:#}", Config::load_with(&path, &bad).unwrap_err());
        assert!(e.contains("with the --set overrides: [IY-C005] --set rocket.dry_mass=abc: invalid type: string \"abc\", expected f64"), "{e}");
        // a range error keeps its own message (it names the key)
        let bad = Overrides { set: vec!["wind.speed=-1".into()], out: None };
        let e = format!("{:#}", Config::load_with(&path, &bad).unwrap_err());
        assert!(e.contains("[IY-C011] wind.speed") && !e.contains("--set wind.speed=-1:") && !e.contains("IY-C062"), "{e}");
        // a repeated key: the last value is used, with a warning
        let twice = Overrides { set: vec!["wind.speed=3".into(), "wind.direction_deg=90".into(), " wind . speed = 8 ".into()], out: None };
        let c = Config::load_with(&path, &twice).unwrap();
        assert_eq!(c.wind.speed, 8.0);
        assert_eq!(c.load_warnings, vec!["[IY-C500] --set wind.speed given more than once; the last value (8) is used".to_string()]);
    }

    #[test]
    fn profile_targets_and_ignored_wind_keys() {
        let profile = ["wind.model=profile", "wind.profile=wind_profile_example.csv"];
        // the sample sets speed and direction_deg explicitly: both are reported as ignored
        let c = sample_set(&profile).unwrap();
        assert_eq!(
            c.load_warnings,
            vec![
                "[IY-C502] wind.speed is ignored for wind.model = \"profile\"; use wind.profile_mean_speed to scale the profile".to_string(),
                "[IY-C503] wind.direction_deg is ignored for wind.model = \"profile\"; use wind.profile_mean_direction_deg to rotate the profile".to_string(),
            ]
        );
        // defaults (keys absent) are not reported
        let text = include_str!("../../../examples/sample.toml").replacen("speed = 4.0 ", "# speed", 1).replacen("direction_deg = 0.0 ", "# direction_deg", 1);
        let mut raw: toml::Value = toml::from_str(&text).unwrap();
        for a in profile {
            apply_set(&mut raw, a).unwrap();
        }
        assert!(Config::from_value(raw, None).unwrap().load_warnings.is_empty());
        // the power model reports nothing
        assert!(sample_set(&[]).unwrap().load_warnings.is_empty());
        let c = sample_set(&[profile[0], profile[1], "wind.profile_mean_speed=15.6", "wind.profile_mean_direction_deg=270"]).unwrap();
        assert_eq!((c.wind.profile_mean_speed, c.wind.profile_mean_direction_deg), (Some(15.6), Some(270.0)));
        let e = sample_set(&["wind.profile_mean_speed=10"]).unwrap_err().to_string();
        assert!(e.contains("wind.profile_mean_speed only applies to wind.model = \"profile\" (the model is \"power\")"), "{e}");
        let e = sample_set(&[profile[0], profile[1], "wind.profile_mean_speed=-1"]).unwrap_err().to_string();
        assert!(e.contains("wind.profile_mean_speed must be"), "{e}");
        let e = sample_set(&[profile[0], profile[1], "wind.profile_mean_direction_deg=361"]).unwrap_err().to_string();
        assert!(e.contains("wind.profile_mean_direction_deg must be between 0 and 360"), "{e}");
    }

    #[test]
    fn wind_profile_parameters_are_checked_only_for_the_models_that_use_them() {
        let sample = include_str!("../../../examples/sample.toml");
        let with = |model: &str, key: &str, value: &str| {
            let t = sample.replacen("model = \"power\"", &format!("model = \"{model}\""), 1);
            let t = match key {
                "exponent" => t.replacen("exponent = 6.0", &format!("exponent = {value}"), 1),
                "roughness_length" => t.replacen("roughness_length = 0.03", &format!("roughness_length = {value}"), 1),
                _ => t.replacen("roughness_length = 0.03", &format!("roughness_length = 0.03\nground_blend_height = {value}"), 1),
            };
            parse(&t).map(|_| ()).map_err(|e| e.to_string())
        };
        // Unused by the model: any value is accepted.
        for (model, key) in [
            ("constant", "exponent"),
            ("constant", "roughness_length"),
            ("constant", "ground_blend_height"),
            ("log", "exponent"),
            ("power", "roughness_length"),
        ] {
            assert!(with(model, key, "0.0").is_ok(), "{model} {key}: {:?}", with(model, key, "0.0"));
        }
        // Used by the model: must be positive.
        for (model, key) in [("power", "exponent"), ("log", "roughness_length"), ("power", "ground_blend_height"), ("log", "ground_blend_height")] {
            let e = with(model, key, "0.0").unwrap_err();
            assert!(e.contains(&format!("wind.{key}")), "{model} {key}: {e}");
        }
    }

    #[test]
    fn config_errors_carry_one_code_each() {
        let dir = std::env::temp_dir().join(format!("iy_cfg_codes_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let ov = Overrides { set: vec![], out: None };
        let count = |e: &anyhow::Error| format!("{e:#}").matches("[IY-").count();
        let missing = Config::load_with(&dir.join("none.toml"), &ov).unwrap_err();
        assert!(format!("{missing:#}").starts_with("[IY-C060] cannot read config"), "{missing:#}");
        std::fs::write(dir.join("syntax.toml"), "a = = 1").unwrap();
        let e = Config::load_with(&dir.join("syntax.toml"), &ov).unwrap_err();
        assert!(format!("{e:#}").starts_with("[IY-C061] invalid config") && count(&e) == 1, "{e:#}");
        // an unknown key: the headline carries the code
        let sample = include_str!("../../../examples/sample.toml");
        std::fs::write(dir.join("unknown.toml"), sample.replacen("[wind]", "[wind]\nnope = 1", 1)).unwrap();
        let e = Config::load_with(&dir.join("unknown.toml"), &ov).unwrap_err();
        assert!(format!("{e:#}").starts_with("[IY-C062] invalid config") && count(&e) == 1, "{e:#}");
        // a range error: the cause carries the code, the headline none
        std::fs::write(dir.join("range.toml"), sample.replacen("[sim]", "[sim]\ndescent = \"parachute\"", 1).replacen("enabled = true", "enabled = false", 1))
            .unwrap();
        let e = Config::load_with(&dir.join("range.toml"), &ov).unwrap_err();
        assert!(format!("{e:#}").contains("[IY-C016]") && !format!("{e:#}").contains("IY-C062") && count(&e) == 1, "{e:#}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
