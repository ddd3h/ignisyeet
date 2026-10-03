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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AeroMethod {
    #[default]
    Barrowman,
    Panel,
    Table,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct AeroCfg {
    pub method: AeroMethod,
    /// External coefficient table (CSV with JSON sidecar); required for method "table".
    pub table: Option<PathBuf>,
    pub panel: panel::PanelOptions,
    pub mach_min: f64,
    pub mach_max: f64,
    pub mach_step: f64,
    pub alpha_max_deg: f64,
    pub alpha_step_deg: f64,
    pub n_slices: usize,
    pub fin_threshold: f64,
    pub extrapolation: aero::Extrapolation,
}

impl Default for AeroCfg {
    fn default() -> Self {
        Self {
            method: AeroMethod::Barrowman,
            table: None,
            panel: panel::PanelOptions::default(),
            mach_min: 0.0,
            mach_max: 3.0,
            mach_step: 0.02,
            alpha_max_deg: 30.0,
            alpha_step_deg: 1.0,
            n_slices: 600,
            fin_threshold: 0.05,
            extrapolation: aero::Extrapolation::Linear,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct SimCfg {
    pub integrator: sim::IntegratorKind,
    pub dt: f64,
    /// Tolerances of the adaptive integrator.
    pub rtol: f64,
    pub atol: f64,
    pub max_time: f64,
    pub output_interval: f64,
    /// Descent mode for `sim`: "parachute" or "ballistic". Defaults to parachute when recovery is enabled.
    pub descent: Option<sim::Descent>,
}

impl Default for SimCfg {
    fn default() -> Self {
        Self { integrator: sim::IntegratorKind::Rk4, dt: 0.002, rtol: 1e-7, atol: 1e-6, max_time: 1200.0, output_interval: 0.05, descent: None }
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
        let raw: toml::Value = toml::from_str(&text).with_context(|| format!("invalid config {}", path.display()))?;
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

    fn validate(&self) -> Result<()> {
        if !matches!(self.rocket.fin_le.as_str(), "rounded" | "sharp") {
            bail!("rocket.fin_le must be \"rounded\" or \"sharp\"");
        }
        if !matches!(self.rocket.fin_te.as_str(), "square" | "tapered") {
            bail!("rocket.fin_te must be \"square\" or \"tapered\"");
        }
        if self.rocket.dry_mass <= 0.0 || self.rocket.iyy_dry <= 0.0 || self.rocket.ixx_dry <= 0.0 {
            bail!("rocket mass and inertias must be positive");
        }
        if self.aero.mach_step <= 0.0 || self.aero.alpha_step_deg <= 0.0 || self.aero.mach_max <= self.aero.mach_min {
            bail!("invalid aero grid");
        }
        if self.dispersion.directions == 0 {
            bail!("dispersion.directions must be at least 1");
        }
        if self.aero.method == AeroMethod::Table && self.aero.table.is_none() {
            bail!("aero.table is required when aero.method = \"table\"");
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
        if self.sim.dt <= 0.0 || self.sim.output_interval <= 0.0 {
            bail!("sim.dt and sim.output_interval must be positive");
        }
        if self.sim.integrator == sim::IntegratorKind::Rk45 && !(self.sim.rtol > 0.0 && self.sim.atol > 0.0) {
            bail!("sim.rtol and sim.atol must be positive for integrator = \"rk45\"");
        }
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
        if p.is_absolute() { p.to_path_buf() } else { self.base_dir.join(p) }
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
        let e = parse(&sample.replace("model = \"power\"", "model = \"log\"").replace("roughness_length = 0.03", "roughness_length = 5.0")).unwrap_err().to_string();
        assert!(e.contains("roughness_length"), "{e}");
        assert!(parse(&sample.replace("model = \"us1976\"", "model = \"constant\"")).is_ok());
        assert!(parse(&sample.replace("integrator = \"rk4\"", "integrator = \"rk45\"")).is_ok());
    }

    #[test]
    fn table_method_requires_table() {
        let text = include_str!("../../../examples/sample.toml").replace("method = \"barrowman\"", "method = \"table\"");
        assert!(parse(&text).unwrap_err().to_string().contains("aero.table"));
    }
}
