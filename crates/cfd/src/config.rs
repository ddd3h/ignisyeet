// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! User-facing options of the CFD mode (`[aero.cfd]` in the configuration file).

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Governing equations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FlowModel {
    /// Inviscid compressible Euler equations; viscous drag terms come from the Barrowman build-up.
    #[default]
    Euler,
    /// Reynolds-averaged Navier-Stokes with the Spalart-Allmaras turbulence model.
    Rans,
}

/// Where the wall surface comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceKind {
    /// The geometry rebuilt from the extracted profile and fins as one fused solid (`"cad"`; the
    /// historical name `"panel_mesh"` is still accepted and is what is serialised).
    #[default]
    #[serde(alias = "cad")]
    PanelMesh,
    /// The raw STL, transformed to the CFD frame; it must be watertight.
    Stl,
    /// The CAD B-rep of `step_file` (STEP, IGES or BREP) with every detail, moved to the CFD frame like the STL and
    /// checked against it (see [`crate::step_mesh`]).
    Step,
}

/// `surface = "step"`: the length unit of the CAD file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StepUnit {
    /// The unit the file declares (STEP, IGES); a BREP file has none and is read as metres.
    #[default]
    Auto,
    M,
    Cm,
    Mm,
    /// Inch.
    In,
    /// Foot.
    Ft,
}

impl StepUnit {
    /// Metres per unit (`None` for `auto`).
    pub fn metres(self) -> Option<f64> {
        match self {
            StepUnit::Auto => None,
            StepUnit::M => Some(1.0),
            StepUnit::Cm => Some(0.01),
            StepUnit::Mm => Some(0.001),
            StepUnit::In => Some(0.0254),
            StepUnit::Ft => Some(0.3048),
        }
    }
}

/// File extensions accepted for `step_file` (case-insensitive).
pub const STEP_EXTENSIONS: [&str; 6] = ["step", "stp", "iges", "igs", "brep", "brp"];

/// Mirror symmetries of the shape that the case matrix may use to skip runs.
///
/// The symmetry planes are the body-axis planes y = 0 (pitch plane) and z = 0; the table of a
/// symmetric shape is completed by reflecting the solved points ([`crate::table`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShapeSymmetry {
    /// `cad`: from the fin layout (`axisymmetric` while the table stays axisymmetric); `stl`: the planes y = 0 and z = 0
    /// that the mirror check of the united surface verifies are used (both: `axisymmetric`, y = 0 only: `pitch_plane`, else `none`).
    Auto,
    /// Mirror symmetric about y = 0 and z = 0 (e.g. 4 fins at 0 or 45 deg roll): only `alpha >= 0`, `beta >= 0` are solved.
    Axisymmetric,
    /// Mirror symmetric about the pitch plane y = 0 only: `+-alpha`, `beta >= 0` are solved.
    PitchPlane,
    /// No symmetry: `+-alpha` and `+-beta` are solved.
    None,
}

/// `surface = "stl"`: how the wall surface of the union is prepared for the volume mesher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum StlRemesh {
    /// `classifySurfaces` (40 deg) + `createGeometry`, then gmsh's surface mesher at `wall_size` (the STL's thin
    /// facets on curved faces and huge facets on flat faces, e.g. fins, are replaced by an isotropic mesh; the
    /// fidelity report measures the deviation). An error when gmsh cannot parametrise the surface.
    #[default]
    Classify,
    /// The triangles of the union as they are. On the sample STL the fins are then under-resolved (their faces are
    /// a few huge triangles) and the normal force is about 30 % low.
    None,
}

/// Euler + STL surface: what to do with the flat base (the inviscid flow behind a flat base does not converge).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BaseTreatment {
    /// Append a closed cone behind the base (the input STL is modified; reported as a warning).
    Fairing,
    /// Keep the flat base; the case is likely not to converge (non-converged cases fail).
    Flat,
}

fn is_default<T: Default + PartialEq>(v: &T) -> bool {
    *v == T::default()
}

fn is_empty_path(p: &std::path::Path) -> bool {
    p.as_os_str().is_empty()
}

#[allow(clippy::ptr_arg)] // serde `skip_serializing_if` passes `&Vec<f64>`
fn zero_betas(v: &Vec<f64>) -> bool {
    v.len() == 1 && v[0] == 0.0
}

fn default_betas() -> Vec<f64> {
    vec![0.0]
}

fn default_roll_rate() -> f64 {
    0.01
}

fn is_default_roll_rate(v: &f64) -> bool {
    *v == 0.01
}

fn default_general_mach_step() -> f64 {
    0.1
}

fn is_default_general_mach_step(v: &f64) -> bool {
    *v == 0.1
}

/// Surface element size on the fins (`fin_size`): `"auto"` or a size [m].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum FinSize {
    /// Euler: `min(wall_size, fin thickness / 4)`; RANS: `wall_size` (see [`CfdOptions::fin_size_for`]).
    #[default]
    Auto,
    /// This size [m] on the fin faces.
    Size(f64),
}

impl Serialize for FinSize {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            FinSize::Auto => s.serialize_str("auto"),
            FinSize::Size(v) => s.serialize_f64(*v),
        }
    }
}

impl<'de> Deserialize<'de> for FinSize {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Raw {
            Num(f64),
            Text(String),
        }
        match Raw::deserialize(d)? {
            Raw::Num(v) => Ok(FinSize::Size(v)),
            Raw::Text(t) if t == "auto" => Ok(FinSize::Auto),
            Raw::Text(t) => Err(serde::de::Error::custom(format!("fin_size must be \"auto\" or a size in metres, got \"{t}\""))),
        }
    }
}

/// `fin_size = "auto"` with Euler: the fin faces get elements of the fin thickness divided by this.
pub const FIN_SIZE_THICKNESS_DIVISOR: f64 = 4.0;

/// Convective flux scheme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Scheme {
    #[default]
    Roe,
    Jst,
}

/// How the angle-of-attack matrix is solved and the zero-angle asymmetry of the steady solution is handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AlphaMode {
    /// Every solved `alpha > 0` is solved at `-alpha` too; `CN` and the nose moment are the odd part
    /// `(f(+a) - f(-a)) / 2`, `CA` the even part. `alpha = 0` is solved once and only reported
    /// (the measured asymmetry). Robust against any mirror-asymmetric numerical offset.
    #[default]
    Mirror,
    /// Only the configured angles; the `alpha = 0` values of `CN` and the nose moment are subtracted. General
    /// table: `CY`, `CZ`, `Cm`, `Cn` (and `Cl` of a mirror-symmetric shape) of the `(0, 0)` point are subtracted, as
    /// in `mirror` mode (see [`crate::asymmetry`]).
    Offset,
    /// Only the configured angles, results used as they are.
    Single,
}

/// Multigrid cycle (`MGCYCLE`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MultigridCycle {
    #[default]
    W,
    V,
}

/// RANS: where the base drag of the table comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum RansBase {
    /// The empirical base-drag term of the flight model, as for Euler; the pressure that RANS integrates on the
    /// base cap is not used.
    #[default]
    Model,
    /// The axial force that RANS integrates on the base cap (marker `base`) replaces the empirical base term.
    Cfd,
}

/// Default early-stop minimum iteration count (also the iteration at which the Euler slope limiter is frozen).
pub const DEFAULT_MIN_ITERATIONS: usize = 100;
/// Default length of the coefficient-stability window of the early stop.
pub const DEFAULT_STABLE_WINDOW: usize = 60;
/// Default agglomeration multigrid levels of Euler cases (RANS: 0).
pub const DEFAULT_EULER_MULTIGRID: usize = 0;
/// RANS defaults of `iterations`, `min_iterations`, `stable_window` and `wall_size`, used when the configuration
/// selects `model = "rans"` and does not set the value (programmatic options: [`CfdOptions::rans`]). In the sample
/// mesh study (8 / 4.8 / 3 mm, doc/data/cfd) the 4.8 mm level is within about 1 % (CN) and 1.3 % (CA) of the 3 mm
/// one; a half-model case needs about 7.5 GB at 5 mm and 18 GB at 3 mm (measured; the eq-cfd-mem-rans estimate is about
/// 9 GB and 22 GB).
pub const RANS_DEFAULT_ITERATIONS: usize = 4000;
pub const RANS_DEFAULT_MIN_ITERATIONS: usize = 400;
pub const RANS_DEFAULT_STABLE_WINDOW: usize = 200;
pub const RANS_DEFAULT_WALL_SIZE: f64 = 0.005;
/// RANS `yplus` ranges: resolved wall (no wall function) up to this value ...
pub const RANS_YPLUS_RESOLVED_MAX: f64 = 5.0;
/// ... or the log layer of the standard wall function.
pub const RANS_YPLUS_WALL_FUNCTION: std::ops::RangeInclusive<f64> = 30.0..=150.0;

/// Deserialisation placeholders of the options whose default depends on `model` (replaced right after parsing).
const UNSET_COUNT: usize = usize::MAX;
fn unset_count() -> usize {
    UNSET_COUNT
}
fn unset_size() -> f64 {
    f64::NAN
}

fn is_default_min_iterations(v: &usize) -> bool {
    *v == DEFAULT_MIN_ITERATIONS
}
fn is_default_stable_window(v: &usize) -> bool {
    *v == DEFAULT_STABLE_WINDOW
}

/// `Serialize` / `Deserialize` go through the derived (`remote = "Self"`) functions; deserialisation then fills the
/// model-dependent defaults (see [`RANS_DEFAULT_ITERATIONS`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, default, remote = "Self")]
pub struct CfdOptions {
    /// Treatment of the angle-of-attack asymmetry, see [`AlphaMode`].
    pub alpha_mode: AlphaMode,
    /// Roll of the fin set about the body axis [deg] (0: fins at +-y and +-z, so two fins lie in the
    /// y = 0 symmetry plane; 45: no fin in the symmetry plane). Only meaningful for symmetric layouts.
    pub fin_roll_deg: f64,
    /// Mesh a quarter of the domain and mirror it about the x-y plane, so the half-model mesh is exactly
    /// symmetric in z. An unstructured mesh of the half model is not, which gives a spurious normal force
    /// at alpha = 0 (see `alpha_mode`). With the exact z-symmetry `-alpha` is the mirror image of `+alpha`,
    /// so `alpha_mode = "mirror"` needs no extra `-alpha` runs. Used only when the fin layout is symmetric
    /// about the x-y plane too (see [`z_symmetric_layout`]); otherwise ignored.
    pub z_mirror_mesh: bool,
    pub model: FlowModel,
    /// Solved Mach numbers (strictly increasing, positive).
    pub machs: Vec<f64>,
    /// Solved angles of attack [deg] (strictly increasing, >= 0, must contain 0). In `mirror` mode each
    /// angle above 0 is also solved at its negative.
    pub alphas_deg: Vec<f64>,
    pub surface: SurfaceKind,
    /// `surface = "cad"`: allow the half model about the pitch plane (y = 0, keeping y >= 0). The half
    /// model is used only while the case matrix permits it (shape symmetric about y = 0, `betas_deg = [0]`,
    /// no roll cases); otherwise the full domain is used (for a symmetric shape the half mesh is
    /// mirrored). `false` forces the full domain. `surface = "stl"`: the same rule, with the symmetry verified on the
    /// united surface (see `shape_symmetry`).
    pub symmetry: bool,
    /// Solved sideslip angles [deg] (increasing, >= 0, starting with 0). For shapes with a y = 0 mirror
    /// symmetry only `beta >= 0` is solved (the negative side follows by reflection); `shape_symmetry = "none"`
    /// solves `+-beta`. Anything but `[0]` makes the table a general (six-component) table.
    #[serde(default = "default_betas", skip_serializing_if = "zero_betas")]
    pub betas_deg: Vec<f64>,
    /// Solve the roll-damping pair (rotating frame at `+-roll_rate_hat`, alpha = beta = 0, full domain) per Mach
    /// number and take `Clp` from CFD instead of the Barrowman estimate.
    #[serde(skip_serializing_if = "is_default")]
    pub roll_damping: bool,
    /// Non-dimensional roll rate `p d / (2 V)` of the roll-damping runs.
    #[serde(default = "default_roll_rate", skip_serializing_if = "is_default_roll_rate")]
    pub roll_rate_hat: f64,
    /// Mirror symmetries used to skip cases (`None` = `auto`, see [`ShapeSymmetry`]).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shape_symmetry: Option<ShapeSymmetry>,
    /// `surface = "stl"` with Euler: required, see [`BaseTreatment`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub base_treatment: Option<BaseTreatment>,
    /// `surface = "step"`: the CAD file (STEP, IGES or BREP). A relative path is relative to the configuration file
    /// (the command line resolves it before the CFD mode sees it).
    #[serde(skip_serializing_if = "is_empty_path")]
    pub step_file: PathBuf,
    /// `surface = "step"`: length unit of `step_file`, see [`StepUnit`].
    #[serde(skip_serializing_if = "is_default")]
    pub step_unit: StepUnit,
    /// `surface = "stl"`: preparation of the wall surface, see [`StlRemesh`].
    #[serde(skip_serializing_if = "is_default")]
    pub stl_remesh: StlRemesh,
    /// `surface = "stl"`: the run fails when the CFD wall is farther than this [m] from the input STL or its
    /// union (see `fidelity`); `None` = `wall_size`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_geometry_error: Option<f64>,
    /// Mach step of a general (six-component) table; 0 = the Mach step of `[aero]`. The general grid has
    /// 61 x 43 angle nodes per Mach number, so a coarser Mach step keeps the file small.
    #[serde(default = "default_general_mach_step", skip_serializing_if = "is_default_general_mach_step")]
    pub general_mach_step: f64,
    /// Euler only: the flat base is followed by a conical tail fairing of this length in base radii
    /// (0 = flat base). The inviscid flow behind a flat base is ill-posed (asymmetric steady wake);
    /// the fairing, marker `base`, is not monitored, so the base drag comes from the Barrowman term.
    /// RANS always keeps the flat base (the wake is part of the solution).
    pub tail_fairing: f64,
    /// Farfield radius in body lengths.
    pub farfield: f64,
    /// Surface element size [m]. 2 mm gives about 1.4 M half-model cells for the 1.5 m sample rocket with the Euler
    /// default `fin_size` (1 mm on the 4 mm fins; 0.9 M with the fins at 2 mm too) and a normal force within 0.1 % of
    /// the 1.8 mm mesh (mesh study with the fins at 1 mm, doc/data/cfd/mesh_convergence.csv).
    /// RANS default: 5 mm.
    #[serde(default = "unset_size")]
    pub wall_size: f64,
    /// Surface element size on the fin faces [m], see [`FinSize`] and [`CfdOptions::fin_size_for`]. Euler on fins
    /// meshed at `wall_size` (thicker than the fin) carries a spurious steady fin lift at subsonic Mach numbers
    /// (`CN(0)` about 12 % of `CN(4 deg)` at 6 mm on the 4 mm fins of the sample); elements of a quarter of the fin
    /// thickness remove it (see [`crate::asymmetry`]).
    #[serde(skip_serializing_if = "is_default")]
    pub fin_size: FinSize,
    /// RANS: target y+ of the first layer. At most 5: the boundary layer is resolved down to the wall (no wall
    /// function); 30..150: the standard wall function bridges the inner layer.
    pub yplus: f64,
    /// Maximum iterations per case (RANS default: 4000, counted in the second-order stage).
    #[serde(default = "unset_count")]
    pub iterations: usize,
    /// Agglomeration multigrid levels (`MGLEVEL`, 0..=4; 0 = single grid). `None` = automatic: the measured
    /// default for Euler, 0 for RANS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub multigrid: Option<usize>,
    /// Multigrid cycle (`MGCYCLE`).
    #[serde(skip_serializing_if = "is_default")]
    pub multigrid_cycle: MultigridCycle,
    /// Early stop: no case is stopped before this iteration, and the Euler slope limiter is frozen at it
    /// (`LIMITER_ITER`), so that the residual can drop below the limiter-chatter level (RANS: never frozen).
    /// RANS default: 400.
    #[serde(default = "unset_count", skip_serializing_if = "is_default_min_iterations")]
    pub min_iterations: usize,
    /// Early stop: the force coefficients must be flat over this many last iterations.
    /// RANS default: 200.
    #[serde(default = "unset_count", skip_serializing_if = "is_default_stable_window")]
    pub stable_window: usize,
    pub cfl: f64,
    /// Euler: target level of the RMS density residual (e.g. 1e-6 -> `CONV_RESIDUAL_MINVAL = -6`). RANS cases are
    /// judged by the drop below the residual peak of their cold start instead (see `runner`).
    pub convergence: f64,
    /// RANS: source of the base drag, see [`RansBase`].
    #[serde(skip_serializing_if = "is_default")]
    pub rans_base: RansBase,
    /// Convective scheme.
    pub scheme: Scheme,
    /// Installation prefix of the CFD tools (`<prefix>/bin/SU2_CFD`, `mpirun`, `python` with gmsh).
    /// Empty: use the `IGNISYEET_CFD_PREFIX` environment variable, else look in `PATH`.
    /// A leading `~` is expanded.
    pub prefix: String,
    /// SU2 executable name (or path).
    pub su2: String,
    /// MPI launcher (only needed when `ranks_per_case > 1`).
    pub mpi: String,
    /// MPI ranks per case; 0 = automatic (4, or fewer if the thread budget is smaller).
    pub ranks_per_case: usize,
    /// Concurrent cases; 0 = automatic (as many as fit the thread and memory budget).
    pub parallel_cases: usize,
    /// Wall-clock limit per case [minutes]; 0 disables it. A case hitting the limit is stopped and
    /// accepted only if it is practically converged (see `runner`).
    pub timeout_minutes: f64,
    /// Working directory below `output.dir`; one sub-directory per case.
    pub workdir: PathBuf,
}

impl Default for CfdOptions {
    fn default() -> Self {
        Self {
            alpha_mode: AlphaMode::Mirror,
            fin_roll_deg: 0.0,
            z_mirror_mesh: true,
            model: FlowModel::Euler,
            machs: vec![0.3, 0.6, 0.8, 0.95, 1.1, 1.3, 1.6, 2.0, 2.5, 3.0],
            alphas_deg: vec![0.0, 2.0, 4.0, 8.0, 12.0, 16.0],
            surface: SurfaceKind::PanelMesh,
            symmetry: true,
            betas_deg: default_betas(),
            roll_damping: false,
            roll_rate_hat: default_roll_rate(),
            shape_symmetry: None,
            base_treatment: None,
            step_file: PathBuf::new(),
            step_unit: StepUnit::Auto,
            stl_remesh: StlRemesh::Classify,
            max_geometry_error: None,
            general_mach_step: default_general_mach_step(),
            tail_fairing: 6.0,
            farfield: 20.0,
            wall_size: 0.002,
            fin_size: FinSize::Auto,
            yplus: 1.0,
            iterations: 3000,
            multigrid: None,
            multigrid_cycle: MultigridCycle::W,
            min_iterations: DEFAULT_MIN_ITERATIONS,
            stable_window: DEFAULT_STABLE_WINDOW,
            cfl: 5.0,
            convergence: 1e-6,
            rans_base: RansBase::Model,
            scheme: Scheme::Roe,
            prefix: String::new(),
            su2: "SU2_CFD".into(),
            mpi: "mpirun".into(),
            ranks_per_case: 4,
            parallel_cases: 4,
            timeout_minutes: 0.0,
            workdir: "cfd".into(),
        }
    }
}

/// Whether `count` fins (one at +z before the roll of `roll_deg`) form a layout that is mirror symmetric
/// about the x-y plane (z -> -z), so that a quarter model can represent it. No fins is symmetric.
pub fn z_symmetric_layout(count: usize, roll_deg: f64) -> bool {
    let ang = |k: usize| (90.0 + roll_deg + 360.0 * k as f64 / count.max(1) as f64).rem_euclid(360.0);
    count == 0
        || (0..count).all(|k| {
            let m = (-ang(k)).rem_euclid(360.0);
            (0..count).any(|j| {
                let d = (ang(j) - m).rem_euclid(360.0);
                d < 1e-6 || d > 360.0 - 1e-6
            })
        })
}

/// Whether `count` fins (one at +z before the roll of `roll_deg`) form a layout that is mirror symmetric
/// about the pitch plane y = 0 (fin angle `a` maps to `180 - a`).
pub fn y_symmetric_layout(count: usize, roll_deg: f64) -> bool {
    let ang = |k: usize| (90.0 + roll_deg + 360.0 * k as f64 / count.max(1) as f64).rem_euclid(360.0);
    count == 0
        || (0..count).all(|k| {
            let m = (180.0 - ang(k)).rem_euclid(360.0);
            (0..count).any(|j| {
                let d = (ang(j) - m).rem_euclid(360.0);
                d < 1e-6 || d > 360.0 - 1e-6
            })
        })
}

fn strictly_increasing(v: &[f64]) -> bool {
    v.windows(2).all(|w| w[0] < w[1])
}

impl<'de> Deserialize<'de> for CfdOptions {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let mut o = CfdOptions::deserialize(d)?;
        o.fill_model_defaults();
        Ok(o)
    }
}

impl Serialize for CfdOptions {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        CfdOptions::serialize(self, s)
    }
}

impl CfdOptions {
    /// The defaults with `model = "rans"` and the RANS defaults of the iteration counts and the wall size.
    pub fn rans() -> Self {
        Self {
            model: FlowModel::Rans,
            iterations: RANS_DEFAULT_ITERATIONS,
            min_iterations: RANS_DEFAULT_MIN_ITERATIONS,
            stable_window: RANS_DEFAULT_STABLE_WINDOW,
            wall_size: RANS_DEFAULT_WALL_SIZE,
            ..Default::default()
        }
    }

    /// Replaces the placeholders of the values the configuration did not set by the defaults of `model`.
    fn fill_model_defaults(&mut self) {
        let d = if self.model == FlowModel::Rans { Self::rans() } else { Self::default() };
        if self.iterations == UNSET_COUNT {
            self.iterations = d.iterations;
        }
        if self.min_iterations == UNSET_COUNT {
            self.min_iterations = d.min_iterations;
        }
        if self.stable_window == UNSET_COUNT {
            self.stable_window = d.stable_window;
        }
        if self.wall_size.is_nan() {
            self.wall_size = d.wall_size;
        }
    }

    /// Surface element size on the fin faces for fins of `thickness` [m] (`None`: no fins): an explicit `fin_size`;
    /// `auto` is `min(wall_size, thickness / 4)` for Euler and `wall_size` for RANS (the prism layer on fins meshed at a
    /// quarter of their thickness is too large a mesh, and the zero-angle force of RANS is a base-wake effect). Never
    /// larger than `wall_size`; equal to it means no fin refinement.
    pub fn fin_size_for(&self, thickness: Option<f64>) -> f64 {
        match self.fin_size {
            FinSize::Size(v) => v.min(self.wall_size),
            FinSize::Auto => match (self.model, thickness) {
                (FlowModel::Euler, Some(t)) if t.is_finite() && t > 0.0 => (t / FIN_SIZE_THICKNESS_DIVISOR).min(self.wall_size),
                _ => self.wall_size,
            },
        }
    }

    /// RANS with `rans_base = "cfd"`: the base drag is the force integrated on the base cap.
    pub fn base_from_cfd(&self) -> bool {
        self.model == FlowModel::Rans && self.rans_base == RansBase::Cfd
    }

    /// RANS with the standard wall function (`yplus` in the log layer); otherwise the wall is resolved.
    pub fn wall_function(&self) -> bool {
        self.model == FlowModel::Rans && self.yplus > RANS_YPLUS_RESOLVED_MAX
    }

    pub fn validate(&self) -> Result<()> {
        if self.machs.is_empty() || self.machs.iter().any(|&m| !(m.is_finite() && m > 0.0)) {
            bail!("aero.cfd.machs must be a non-empty list of positive Mach numbers");
        }
        if !strictly_increasing(&self.machs) {
            bail!("aero.cfd.machs must be sorted in increasing order without duplicates");
        }
        if self.alphas_deg.is_empty() || self.alphas_deg.iter().any(|&a| !(a.is_finite() && a >= 0.0)) {
            bail!("aero.cfd.alphas_deg must be a non-empty list of angles >= 0");
        }
        if !strictly_increasing(&self.alphas_deg) {
            bail!("aero.cfd.alphas_deg must be sorted in increasing order without duplicates");
        }
        if self.alphas_deg[0] != 0.0 {
            bail!("aero.cfd.alphas_deg must include 0 (the zero-lift axial force and the small-angle slopes)");
        }
        if self.alphas_deg.len() < 2 {
            bail!("aero.cfd.alphas_deg needs at least one angle above 0");
        }
        for (name, v) in [("farfield", self.farfield), ("wall_size", self.wall_size), ("yplus", self.yplus), ("cfl", self.cfl)] {
            if !(v.is_finite() && v > 0.0) {
                bail!("aero.cfd.{name} must be positive");
            }
        }
        if let FinSize::Size(v) = self.fin_size {
            if !(v.is_finite() && v > 0.0) {
                bail!("aero.cfd.fin_size must be \"auto\" or a positive size in metres");
            }
            if v > self.wall_size {
                bail!("aero.cfd.fin_size = {v} is larger than wall_size = {}: the fin faces are never meshed coarser than the wall", self.wall_size);
            }
            if v < self.wall_size / 50.0 {
                bail!("aero.cfd.fin_size = {v} is below wall_size / 50 = {}: the mesh would be impractically large", self.wall_size / 50.0);
            }
        }
        if self.betas_deg.is_empty() || self.betas_deg.iter().any(|&b| !(b.is_finite() && (0.0..=90.0).contains(&b))) {
            bail!("aero.cfd.betas_deg must be a non-empty list of sideslip angles in [0, 90] deg");
        }
        if !strictly_increasing(&self.betas_deg) {
            bail!("aero.cfd.betas_deg must be sorted in increasing order without duplicates");
        }
        if self.betas_deg[0] != 0.0 {
            bail!("aero.cfd.betas_deg must start with 0");
        }
        if !(self.roll_rate_hat.is_finite() && self.roll_rate_hat > 0.0 && self.roll_rate_hat <= 0.05) {
            bail!("aero.cfd.roll_rate_hat must be in (0, 0.05] (p d / 2V of the roll-damping runs)");
        }
        if !(self.general_mach_step.is_finite() && self.general_mach_step >= 0.0) {
            bail!("aero.cfd.general_mach_step must be >= 0 (0 = the [aero] mach_step)");
        }
        if self.max_geometry_error.is_some_and(|e| !(e.is_finite() && e > 0.0)) {
            bail!("aero.cfd.max_geometry_error must be positive");
        }
        if self.surface == SurfaceKind::Stl && self.model == FlowModel::Euler && self.base_treatment.is_none() {
            bail!(
                "aero.cfd.surface = \"stl\" with model = \"euler\" needs base_treatment: the inviscid flow behind a flat base does not converge \
                 (the residual stalls and the normal force flips sign).\n  \
                 base_treatment = \"fairing\": a closed cone (tail_fairing x base radius long) is united with the body behind the base; \
                 this MODIFIES the input STL and the base drag comes from the empirical base term.\n  \
                 base_treatment = \"flat\": the shape is kept, the base is not integrated, but the solution is likely not to converge \
                 and cases that do not converge fail."
            );
        }
        if self.surface == SurfaceKind::Step {
            if self.step_file.as_os_str().is_empty() {
                bail!("aero.cfd.surface = \"step\" needs step_file = \"<path>\" (the CAD model as STEP, IGES or BREP, relative to the configuration file)");
            }
            let ext = self.step_file.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).unwrap_or_default();
            if !STEP_EXTENSIONS.contains(&ext.as_str()) {
                bail!(
                    "aero.cfd.step_file = \"{}\": not a CAD file OpenCASCADE reads; use a STEP (.step, .stp), IGES (.iges, .igs) or BREP (.brep, .brp) file",
                    self.step_file.display()
                );
            }
            if self.model == FlowModel::Euler && self.base_treatment.is_none() {
                bail!(
                    "aero.cfd.surface = \"step\" with model = \"euler\" needs base_treatment: the inviscid flow behind a flat base does not converge \
                     (the residual stalls and the normal force flips sign).\n  \
                     base_treatment = \"fairing\": an exact cone (tail_fairing x base radius long) is fused with the CAD solid behind its flat base; \
                     this MODIFIES the CAD shape and the base drag comes from the empirical base term.\n  \
                     base_treatment = \"flat\": the shape is kept, the base is not integrated, but the solution is likely not to converge \
                     and cases that do not converge fail."
                );
            }
        } else if !self.step_file.as_os_str().is_empty() || self.step_unit != StepUnit::Auto {
            bail!("aero.cfd.step_file and aero.cfd.step_unit are only used with surface = \"step\"");
        }
        if self.base_treatment == Some(BaseTreatment::Fairing)
            && matches!(self.surface, SurfaceKind::Stl | SurfaceKind::Step)
            && self.model == FlowModel::Euler
            && self.tail_fairing <= 0.0
        {
            bail!("aero.cfd.base_treatment = \"fairing\" needs tail_fairing > 0 (cone length in base radii)");
        }
        if self.surface == SurfaceKind::Stl && self.model == FlowModel::Rans {
            bail!(
                "aero.cfd.surface = \"stl\" with model = \"rans\": RANS needs the boundary-layer mesh, which is built only for the CAD surface \
                 (surface = \"cad\") and the CAD B-rep (surface = \"step\")"
            );
        }
        if self.model == FlowModel::Rans && !(self.yplus <= RANS_YPLUS_RESOLVED_MAX || RANS_YPLUS_WALL_FUNCTION.contains(&self.yplus)) {
            bail!(
                "aero.cfd.yplus = {} lies in the buffer layer, where neither a resolved wall nor a wall function is valid: \
                 use yplus <= 5 (resolved wall, recommended 1) or 30..150 (standard wall function)",
                self.yplus
            );
        }
        if self.rans_base == RansBase::Cfd && self.model != FlowModel::Rans {
            bail!("aero.cfd.rans_base = \"cfd\" needs model = \"rans\" (the inviscid base pressure is not used)");
        }
        if self.roll_damping && self.model == FlowModel::Rans {
            bail!("aero.cfd.roll_damping is only implemented for model = \"euler\"");
        }
        if !(self.fin_roll_deg.is_finite() && self.fin_roll_deg.abs() <= 360.0) {
            bail!("aero.cfd.fin_roll_deg must be a finite angle");
        }
        if !(self.tail_fairing.is_finite() && self.tail_fairing >= 0.0) {
            bail!("aero.cfd.tail_fairing must be >= 0");
        }
        if self.farfield < 5.0 {
            bail!("aero.cfd.farfield must be at least 5 body lengths");
        }
        if !(self.convergence > 0.0 && self.convergence < 1.0) {
            bail!("aero.cfd.convergence must be in (0, 1), e.g. 1e-6");
        }
        if self.iterations == 0 {
            bail!("aero.cfd.iterations must be at least 1");
        }
        if self.multigrid.is_some_and(|m| m > 4) {
            bail!("aero.cfd.multigrid must be in 0..=4 (agglomeration multigrid levels)");
        }
        if self.stable_window < 10 {
            bail!("aero.cfd.stable_window must be at least 10 iterations");
        }
        if self.min_iterations < self.stable_window {
            bail!("aero.cfd.min_iterations ({}) must not be smaller than stable_window ({})", self.min_iterations, self.stable_window);
        }
        if !(self.timeout_minutes.is_finite() && self.timeout_minutes >= 0.0) {
            bail!("aero.cfd.timeout_minutes must be >= 0");
        }
        if self.su2.trim().is_empty() {
            bail!("aero.cfd.su2 must name the SU2_CFD executable");
        }
        Ok(())
    }

    /// Non-fatal remarks, e.g. oversubscribed cores.
    pub fn warnings(&self) -> Vec<String> {
        self.warnings_for(std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1))
    }

    pub fn warnings_for(&self, available: usize) -> Vec<String> {
        let mut w = Vec::new();
        let need = self.ranks_per_case * self.parallel_cases;
        if self.ranks_per_case > 0 && self.parallel_cases > 0 && need > available {
            w.push(format!("aero.cfd: ranks_per_case x parallel_cases = {need} exceeds the {available} available cores; cases will be slowed down"));
        }
        w
    }

    /// Whether the table built from this configuration is a general (six-component) table: an STL or STEP surface,
    /// an explicit `none` / `pitch_plane` shape symmetry, or a non-zero sideslip.
    pub fn general_table(&self) -> bool {
        self.surface_is_user_shape()
            || matches!(self.shape_symmetry, Some(ShapeSymmetry::None | ShapeSymmetry::PitchPlane))
            || self.betas_deg.iter().any(|&b| b != 0.0)
    }

    /// The shape symmetry that determines the case matrix. `auto` is `axisymmetric` for the rebuilt geometry (use
    /// [`crate::resolve_options`] to take the fin layout into account when sideslip is solved). For an STL or STEP
    /// surface it is `none` until the shape has been checked: [`crate::stl_mesh::ensure_mesh_stl`] and
    /// [`crate::step_mesh::ensure_mesh_step`] return the options with the verified symmetry, and the case matrix must be
    /// made from those.
    pub fn shape(&self) -> ShapeSymmetry {
        match self.shape_symmetry {
            Some(s) if s != ShapeSymmetry::Auto => s,
            _ if self.surface_is_user_shape() => ShapeSymmetry::None,
            _ => ShapeSymmetry::Axisymmetric,
        }
    }

    /// The wall is the user's own geometry (`stl` or `step`), not the rebuilt body of revolution: its symmetry is
    /// checked on the shape, the table is a general (six-component) table, and fin cant and fin edges are part of it.
    pub fn surface_is_user_shape(&self) -> bool {
        matches!(self.surface, SurfaceKind::Stl | SurfaceKind::Step)
    }

    /// Multigrid levels in effect: the explicit value, else [`DEFAULT_EULER_MULTIGRID`] for Euler and 0 for RANS.
    pub fn multigrid_levels(&self) -> usize {
        self.multigrid.unwrap_or(if self.model == FlowModel::Euler { DEFAULT_EULER_MULTIGRID } else { 0 })
    }

    /// `CONV_RESIDUAL_MINVAL` (log10 of the RMS density residual target).
    pub fn residual_minval(&self) -> f64 {
        self.convergence.log10()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_valid_and_match_plan() {
        let o = CfdOptions::default();
        o.validate().unwrap();
        assert_eq!(o.machs.len(), 10);
        assert_eq!(o.alphas_deg, vec![0.0, 2.0, 4.0, 8.0, 12.0, 16.0]);
        assert_eq!((o.ranks_per_case, o.parallel_cases, o.iterations), (4, 4, 3000));
        assert!((o.residual_minval() + 6.0).abs() < 1e-12);
    }

    #[test]
    fn layout_symmetry() {
        assert!(z_symmetric_layout(4, 0.0) && z_symmetric_layout(4, 45.0) && z_symmetric_layout(2, 0.0) && z_symmetric_layout(0, 0.0));
        assert!(z_symmetric_layout(6, 0.0));
        assert!(!z_symmetric_layout(3, 0.0) && !z_symmetric_layout(4, 10.0) && !z_symmetric_layout(5, 0.0));
    }

    #[test]
    fn rejects_bad_grids() {
        let bad = |f: fn(&mut CfdOptions)| {
            let mut o = CfdOptions::default();
            f(&mut o);
            o.validate().unwrap_err().to_string()
        };
        assert!(bad(|o| o.machs = vec![0.0, 1.0]).contains("positive"));
        assert!(bad(|o| o.machs = vec![0.5, 0.5]).contains("sorted"));
        assert!(bad(|o| o.machs = vec![2.0, 1.0]).contains("sorted"));
        assert!(bad(|o| o.alphas_deg = vec![2.0, 4.0]).contains("include 0"));
        assert!(bad(|o| o.alphas_deg = vec![0.0, -1.0]).contains(">= 0"));
        assert!(bad(|o| o.alphas_deg = vec![0.0]).contains("at least one"));
        assert!(bad(|o| o.wall_size = 0.0).contains("wall_size"));
        assert!(bad(|o| o.convergence = 2.0).contains("convergence"));
    }

    #[test]
    fn new_options_defaults_aliases_and_validation() {
        let o = CfdOptions::default();
        assert_eq!((o.betas_deg.clone(), o.roll_damping, o.roll_rate_hat, o.shape_symmetry, o.base_treatment), (vec![0.0], false, 0.01, None, None));
        assert!(!o.general_table());
        // `cad` is an alias, `panel_mesh` is what is written.
        let c: CfdOptions = serde_json::from_str(r#"{"surface":"cad"}"#).unwrap();
        assert_eq!(c.surface, SurfaceKind::PanelMesh);
        assert!(serde_json::to_string(&c).unwrap().contains("\"panel_mesh\""));
        // Defaults are not serialised (hash of existing configurations).
        let j = serde_json::to_string(&o).unwrap();
        assert!(!j.contains("betas_deg") && !j.contains("roll_damping") && !j.contains("shape_symmetry") && !j.contains("base_treatment"));
        // STL + Euler needs an explicit base treatment (the message explains both options).
        let stl = CfdOptions { surface: SurfaceKind::Stl, ..Default::default() };
        let e = stl.validate().unwrap_err().to_string();
        assert!(e.contains("base_treatment") && e.contains("fairing") && e.contains("flat") && e.contains("MODIFIES"), "{e}");
        CfdOptions { base_treatment: Some(BaseTreatment::Flat), ..stl.clone() }.validate().unwrap();
        CfdOptions { base_treatment: Some(BaseTreatment::Fairing), ..stl.clone() }.validate().unwrap();
        assert!(CfdOptions { base_treatment: Some(BaseTreatment::Fairing), tail_fairing: 0.0, ..stl.clone() }
            .validate()
            .unwrap_err()
            .to_string()
            .contains("tail_fairing"));
        assert!(CfdOptions { base_treatment: Some(BaseTreatment::Flat), max_geometry_error: Some(0.0), ..stl.clone() }
            .validate()
            .unwrap_err()
            .to_string()
            .contains("max_geometry_error"));
        let p: CfdOptions =
            serde_json::from_str(r#"{"shape_symmetry":"pitch_plane","base_treatment":"fairing","betas_deg":[0,4],"roll_damping":true,"symmetry":false}"#)
                .unwrap();
        assert_eq!((p.shape_symmetry, p.base_treatment), (Some(ShapeSymmetry::PitchPlane), Some(BaseTreatment::Fairing)));
        p.validate().unwrap();
        assert!(p.general_table());
        let bad = |f: fn(&mut CfdOptions)| {
            let mut o = CfdOptions::default();
            f(&mut o);
            o.validate().unwrap_err().to_string()
        };
        assert!(bad(|o| o.betas_deg = vec![]).contains("betas_deg"));
        assert!(bad(|o| o.betas_deg = vec![0.0, -2.0]).contains("betas_deg"));
        assert!(bad(|o| o.betas_deg = vec![2.0, 4.0]).contains("start with 0"));
        assert!(bad(|o| o.betas_deg = vec![0.0, 4.0, 4.0]).contains("sorted"));
        assert!(bad(|o| o.roll_rate_hat = 0.0).contains("roll_rate_hat"));
        assert!(bad(|o| o.roll_rate_hat = 0.5).contains("roll_rate_hat"));
        assert!(bad(|o| {
            o.roll_damping = true;
            o.model = FlowModel::Rans;
        })
        .contains("roll_damping"));
        assert!(CfdOptions { surface: SurfaceKind::Stl, ..Default::default() }.general_table());
        assert_eq!(CfdOptions { surface: SurfaceKind::Stl, ..Default::default() }.shape(), ShapeSymmetry::None);
        assert_eq!(CfdOptions::default().shape(), ShapeSymmetry::Axisymmetric);
        assert!(y_symmetric_layout(4, 0.0) && y_symmetric_layout(4, 45.0) && y_symmetric_layout(3, 0.0) && y_symmetric_layout(0, 0.0));
        assert!(!y_symmetric_layout(4, 10.0) && !y_symmetric_layout(5, 10.0) && !y_symmetric_layout(3, 20.0));
    }

    #[test]
    fn solver_acceleration_options_are_validated_and_not_serialised_at_their_defaults() {
        let o = CfdOptions::default();
        assert_eq!(
            (o.multigrid, o.multigrid_cycle, o.min_iterations, o.stable_window),
            (None, MultigridCycle::W, DEFAULT_MIN_ITERATIONS, DEFAULT_STABLE_WINDOW)
        );
        let j = serde_json::to_string(&o).unwrap();
        assert!(!j.contains("multigrid") && !j.contains("min_iterations") && !j.contains("stable_window"), "{j}");
        let p: CfdOptions = serde_json::from_str(r#"{"multigrid":3,"multigrid_cycle":"v","min_iterations":120,"stable_window":50}"#).unwrap();
        p.validate().unwrap();
        assert_eq!((p.multigrid_levels(), p.multigrid_cycle, p.min_iterations, p.stable_window), (3, MultigridCycle::V, 120, 50));
        let j = serde_json::to_string(&p).unwrap();
        assert!(j.contains("\"multigrid\":3") && j.contains("\"multigrid_cycle\":\"v\"") && j.contains("min_iterations") && j.contains("stable_window"), "{j}");
        // RANS keeps the single grid unless asked.
        let r = CfdOptions { model: FlowModel::Rans, ..Default::default() };
        assert_eq!(r.multigrid_levels(), 0);
        assert_eq!(CfdOptions { model: FlowModel::Rans, multigrid: Some(2), ..Default::default() }.multigrid_levels(), 2);
        let bad = |f: fn(&mut CfdOptions)| {
            let mut o = CfdOptions::default();
            f(&mut o);
            o.validate().unwrap_err().to_string()
        };
        assert!(bad(|o| o.multigrid = Some(5)).contains("multigrid"));
        assert!(bad(|o| o.stable_window = 5).contains("stable_window"));
        assert!(bad(|o| {
            o.min_iterations = 60;
            o.stable_window = 80;
        })
        .contains("min_iterations"));
        assert!(serde_json::from_str::<CfdOptions>(r#"{"multigrid_cycle":"f"}"#).is_err());
    }

    #[test]
    fn oversubscription_is_a_warning() {
        let o = CfdOptions::default();
        assert!(o.warnings_for(20).is_empty());
        assert_eq!(o.warnings_for(8).len(), 1);
        o.validate().unwrap();
    }

    #[test]
    fn parses_and_rejects_unknown_fields() {
        let o: CfdOptions = serde_json::from_str(r#"{"model":"rans","surface":"stl","machs":[0.5,2.0]}"#).unwrap();
        assert_eq!(o.model, FlowModel::Rans);
        assert_eq!(o.surface, SurfaceKind::Stl);
        assert!(serde_json::from_str::<CfdOptions>(r#"{"bogus":1}"#).is_err());
    }
    #[test]
    fn rans_defaults_apply_only_to_values_left_unset() {
        // Euler: the familiar defaults, and nothing new is serialised.
        let e: CfdOptions = serde_json::from_str("{}").unwrap();
        assert_eq!((e.iterations, e.min_iterations, e.stable_window, e.wall_size), (3000, 100, 60, 0.002));
        assert_eq!(serde_json::to_string(&e).unwrap(), serde_json::to_string(&CfdOptions::default()).unwrap());
        assert!(!serde_json::to_string(&e).unwrap().contains("rans_base"));
        // RANS without explicit values: the RANS defaults.
        let r: CfdOptions = serde_json::from_str(r#"{"model":"rans"}"#).unwrap();
        assert_eq!((r.iterations, r.min_iterations, r.stable_window, r.wall_size), (4000, 400, 200, 0.005));
        r.validate().unwrap();
        assert_eq!(serde_json::to_string(&r).unwrap(), serde_json::to_string(&CfdOptions::rans()).unwrap());
        // Explicit values are kept, also when they equal the Euler defaults.
        let x: CfdOptions = serde_json::from_str(r#"{"model":"rans","iterations":3000,"min_iterations":100,"stable_window":60,"wall_size":0.002}"#).unwrap();
        assert_eq!((x.iterations, x.min_iterations, x.stable_window, x.wall_size), (3000, 100, 60, 0.002));
        // A serialised RANS configuration reads back unchanged.
        let back: CfdOptions = serde_json::from_str(&serde_json::to_string(&r).unwrap()).unwrap();
        assert_eq!((back.iterations, back.min_iterations, back.stable_window, back.wall_size), (4000, 400, 200, 0.005));
        // `deny_unknown_fields` still applies.
        assert!(serde_json::from_str::<CfdOptions>(r#"{"model":"rans","bogus":1}"#).is_err());
    }

    #[test]
    fn step_surface_options_are_validated() {
        // Parsing: `surface = "step"`, the file and the unit; defaults are not serialised.
        let o: CfdOptions = serde_json::from_str(r#"{"surface":"step","step_file":"cad/rocket.STEP","step_unit":"mm","base_treatment":"fairing"}"#).unwrap();
        assert_eq!((o.surface, o.step_unit), (SurfaceKind::Step, StepUnit::Mm));
        o.validate().unwrap();
        assert!(o.general_table() && o.surface_is_user_shape());
        assert_eq!(o.shape(), ShapeSymmetry::None);
        let j = serde_json::to_string(&CfdOptions::default()).unwrap();
        assert!(!j.contains("step_file") && !j.contains("step_unit"), "{j}");
        assert!(serde_json::from_str::<CfdOptions>(r#"{"step_unit":"furlong"}"#).is_err());
        let step = CfdOptions { surface: SurfaceKind::Step, base_treatment: Some(BaseTreatment::Flat), ..Default::default() };
        // The file is required and must be a CAD format OpenCASCADE reads.
        let e = step.validate().unwrap_err().to_string();
        assert!(e.contains("needs step_file"), "{e}");
        for f in ["r.step", "r.stp", "r.STP", "r.iges", "r.igs", "r.brep", "r.brp"] {
            CfdOptions { step_file: f.into(), ..step.clone() }.validate().unwrap();
        }
        for f in ["r.stl", "r", "r.obj"] {
            let e = CfdOptions { step_file: f.into(), ..step.clone() }.validate().unwrap_err().to_string();
            assert!(e.contains("STEP (.step, .stp), IGES (.iges, .igs) or BREP (.brep, .brp)"), "{e}");
        }
        // Euler needs a base treatment as for the STL; RANS does not.
        let e = CfdOptions { base_treatment: None, step_file: "r.step".into(), ..step.clone() }.validate().unwrap_err().to_string();
        assert!(e.contains("surface = \"step\" with model = \"euler\" needs base_treatment") && e.contains("MODIFIES the CAD shape"), "{e}");
        CfdOptions { surface: SurfaceKind::Step, step_file: "r.step".into(), ..CfdOptions::rans() }.validate().unwrap();
        let e = CfdOptions { step_file: "r.step".into(), base_treatment: Some(BaseTreatment::Fairing), tail_fairing: 0.0, ..step.clone() }
            .validate()
            .unwrap_err()
            .to_string();
        assert!(e.contains("tail_fairing"), "{e}");
        // The STEP options belong to the STEP surface.
        let e = CfdOptions { step_file: "r.step".into(), ..Default::default() }.validate().unwrap_err().to_string();
        assert!(e.contains("only used with surface = \"step\""), "{e}");
        assert!(CfdOptions { step_unit: StepUnit::In, ..Default::default() }.validate().is_err());
        assert_eq!((StepUnit::Auto.metres(), StepUnit::In.metres(), StepUnit::Mm.metres()), (None, Some(0.0254), Some(0.001)));
    }

    #[test]
    fn fin_size_parses_validates_and_resolves_per_model() {
        // Default `auto`, not serialised (existing configuration hashes stay).
        let o = CfdOptions::default();
        assert_eq!(o.fin_size, FinSize::Auto);
        assert!(!serde_json::to_string(&o).unwrap().contains("fin_size"));
        let a: CfdOptions = serde_json::from_str(r#"{"fin_size":"auto"}"#).unwrap();
        assert_eq!(a.fin_size, FinSize::Auto);
        let n: CfdOptions = serde_json::from_str(r#"{"fin_size":0.0005}"#).unwrap();
        assert_eq!(n.fin_size, FinSize::Size(0.0005));
        n.validate().unwrap();
        assert!(serde_json::to_string(&n).unwrap().contains("\"fin_size\":0.0005"));
        let back: CfdOptions = serde_json::from_str(&serde_json::to_string(&n).unwrap()).unwrap();
        assert_eq!(back.fin_size, FinSize::Size(0.0005));
        let e = serde_json::from_str::<CfdOptions>(r#"{"fin_size":"fine"}"#).unwrap_err().to_string();
        assert!(e.contains("fin_size must be \"auto\" or a size in metres"), "{e}");
        // auto: Euler a quarter of the fin thickness (at most wall_size), RANS wall_size; no fins: wall_size.
        assert_eq!(o.fin_size_for(Some(0.004)), 0.001);
        assert_eq!(CfdOptions { wall_size: 0.0008, ..Default::default() }.fin_size_for(Some(0.004)), 0.0008);
        assert_eq!(o.fin_size_for(None), o.wall_size);
        assert_eq!(o.fin_size_for(Some(0.0)), o.wall_size);
        let r = CfdOptions::rans();
        assert_eq!(r.fin_size_for(Some(0.004)), r.wall_size);
        // An explicit size applies to both models.
        assert_eq!(CfdOptions { fin_size: FinSize::Size(0.0015), ..CfdOptions::rans() }.fin_size_for(Some(0.004)), 0.0015);
        assert_eq!(CfdOptions { fin_size: FinSize::Size(0.0015), ..Default::default() }.fin_size_for(None), 0.0015);
        // Validation.
        let bad = |v: f64| CfdOptions { fin_size: FinSize::Size(v), ..Default::default() }.validate().unwrap_err().to_string();
        assert!(bad(0.0).contains("positive") && bad(f64::NAN).contains("positive"));
        assert!(bad(0.003).contains("larger than wall_size"), "{}", bad(0.003));
        assert!(bad(1e-5).contains("below wall_size / 50"), "{}", bad(1e-5));
        CfdOptions { fin_size: FinSize::Size(0.002), ..Default::default() }.validate().unwrap();
    }

    #[test]
    fn rans_surface_yplus_and_base_validation() {
        let e = CfdOptions { surface: SurfaceKind::Stl, ..CfdOptions::rans() }.validate().unwrap_err().to_string();
        assert!(e.contains("RANS needs the boundary-layer mesh, which is built only for the CAD surface"), "{e}");
        for y in [0.5, 1.0, 5.0, 30.0, 100.0, 150.0] {
            CfdOptions { yplus: y, ..CfdOptions::rans() }.validate().unwrap();
        }
        for y in [5.5, 10.0, 29.0, 151.0, 300.0] {
            let e = CfdOptions { yplus: y, ..CfdOptions::rans() }.validate().unwrap_err().to_string();
            assert!(e.contains("yplus") && e.contains("buffer layer"), "{e}");
        }
        // Euler ignores yplus (it is only checked for being positive).
        CfdOptions { yplus: 10.0, ..Default::default() }.validate().unwrap();
        assert!(!CfdOptions::rans().wall_function() && CfdOptions { yplus: 50.0, ..CfdOptions::rans() }.wall_function());
        assert!(!CfdOptions { yplus: 50.0, ..Default::default() }.wall_function());
        // rans_base
        let r: CfdOptions = serde_json::from_str(r#"{"model":"rans","rans_base":"cfd"}"#).unwrap();
        assert!(r.base_from_cfd());
        assert!(serde_json::to_string(&r).unwrap().contains("\"rans_base\":\"cfd\""));
        assert!(!CfdOptions::rans().base_from_cfd() && !CfdOptions::default().base_from_cfd());
        let e = CfdOptions { rans_base: RansBase::Cfd, ..Default::default() }.validate().unwrap_err().to_string();
        assert!(e.contains("rans_base"), "{e}");
        assert!(serde_json::from_str::<CfdOptions>(r#"{"rans_base":"wake"}"#).is_err());
    }
}
