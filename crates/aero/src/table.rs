// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Pre-computed aerodynamic coefficient tables with CSV/JSON persistence.
//!
//! Two kinds share one file format (version 4, recorded in the JSON sidecar):
//!
//! * **Axisymmetric**: (Mach, total angle of attack) grid with the normal force, axial force,
//!   centre of pressure and CG-independent damping sums of a body of revolution, the roll
//!   forcing per radian of fin cant and the roll-damping derivative, the lag (apparent-mass)
//!   sums `lag_l0`, `lag_l1`, the induced-roll coefficient `cl4`, the Magnus force slope `cnpa`
//!   and the Magnus moment slope `cmpa`. Bilinear interpolation.
//! * **General**: (Mach, alpha, beta) grid with the six body-axis coefficients
//!   CX, CY, CZ, Cl, Cm, Cn (moments about the nose tip, normalised by q S d), the roll terms,
//!   separate pitch and yaw damping sums, the lag sums and the Magnus slopes. Trilinear interpolation.
//!
//! Older column sets load with the missing columns zero: axisymmetric files with 8 value columns
//! (version 1, no version in the sidecar), 10 (version 2, with the roll terms) or 14 (version 3,
//! with the lag columns, `cl4` and `cnpa`), general files with 15 value columns (without the lag
//! and Magnus columns) or 18 (without `cmpa`).

use crate::model::{axial_alpha_factor, AeroModel};
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Counters of lookups outside the tabulated range (shared by all clones of a table).
#[derive(Debug, Default)]
struct RangeCounters {
    lookups: AtomicU64,
    mach_below: AtomicU64,
    mach_above: AtomicU64,
    angle_beyond: AtomicU64,
    /// Largest requested Mach and |alpha| [deg] as non-negative f64 bit patterns (monotone in the bits).
    max_mach: AtomicU64,
    max_abs_alpha_deg: AtomicU64,
}

/// Snapshot of the out-of-range lookup statistics of an [`AeroTable`].
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct OutOfRangeStats {
    /// Total number of lookups.
    pub lookups: u64,
    /// Lookups with Mach below the first grid value (clamped to it).
    pub mach_below: u64,
    /// Lookups with Mach above the last grid value (clamped to it).
    pub mach_above: u64,
    /// Lookups with alpha or beta outside the grid (linearly extrapolated or clamped).
    pub angle_beyond: u64,
    /// Largest Mach requested.
    pub max_mach: f64,
    /// Largest |alpha| requested [deg].
    pub max_abs_alpha_deg: f64,
}

impl OutOfRangeStats {
    /// True when any lookup left the tabulated range.
    pub fn any(&self) -> bool {
        self.mach_below + self.mach_above + self.angle_beyond > 0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Extrapolation {
    /// Continue the slope of the outermost grid cell in the angles (alpha, beta). Mach is never
    /// extrapolated (see [`AeroTable::lookup`]); the extrapolated coefficients are bounded to
    /// physically sane signs.
    #[default]
    Linear,
    /// Hold the value at the grid edge in every axis.
    Clamp,
}

/// Current table file format.
pub const FORMAT_VERSION: u32 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TableKind {
    /// Body of revolution: coefficients of the total angle of attack.
    #[default]
    Axisymmetric,
    /// Six body-axis coefficients of (alpha, beta).
    General,
}

fn format_v1() -> u32 {
    1
}
fn nose_tip() -> String {
    "nose_tip".into()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableMeta {
    /// File format version (1 when the sidecar has none).
    #[serde(default = "format_v1")]
    pub format_version: u32,
    #[serde(default)]
    pub kind: TableKind,
    /// Hash of the inputs (mesh + options) the table was built from.
    pub source_hash: String,
    pub ref_area: f64,
    pub ref_diameter: f64,
    pub length: f64,
    pub machs: Vec<f64>,
    /// Axisymmetric: total angle of attack 0..alpha_max. General: alpha = atan2(w, u), -180..180.
    pub alphas_deg: Vec<f64>,
    /// General tables only: sideslip beta = asin(v / V), -90..90.
    #[serde(default)]
    pub betas_deg: Vec<f64>,
    /// Fin cant [deg] already contained in the tabulated roll moment; the flight adds
    /// `cl_delta * (cant - fin_cant_ref)`.
    #[serde(default)]
    pub fin_cant_ref_deg: f64,
    /// Moment reference point of the tabulated moments.
    #[serde(default = "nose_tip")]
    pub moment_ref: String,
    pub extrapolation: Extrapolation,
    /// Roll angle of one fin [deg], from +y_b towards +z_b: the fin plane the induced roll `cl4`
    /// of an axisymmetric table is referred to (`Cl = cl4 alpha^2 sin(4 phi_w)`, `phi_w` the roll
    /// angle of the crossflow from this fin).
    #[serde(default)]
    pub fin_roll_deg: f64,
    /// Number of fins of the rocket the table describes (None: not recorded). The induced roll
    /// `cl4 alpha^2 sin(4 phi_w)` is the form of a cruciform (4-fin) set and is applied only when
    /// this is 4 or not recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fin_count: Option<usize>,
}

fn validate_axis(name: &str, axis: &[f64]) -> Result<()> {
    if axis.is_empty() {
        bail!("{name} axis is empty");
    }
    if let Some(i) = axis.iter().position(|x| !x.is_finite()) {
        bail!("{name}[{i}] is not finite: {}", axis[i]);
    }
    if let Some(i) = axis.windows(2).position(|w| w[1] <= w[0]) {
        bail!("{name} must be strictly increasing, but {name}[{}] = {} follows {name}[{i}] = {}", i + 1, axis[i + 1], axis[i]);
    }
    Ok(())
}

fn validate_meta(m: &TableMeta) -> Result<()> {
    for (name, v) in [("ref_area", m.ref_area), ("ref_diameter", m.ref_diameter), ("length", m.length)] {
        if !(v.is_finite() && v > 0.0) {
            bail!("{name} must be finite and positive, got {v}");
        }
    }
    if !m.fin_cant_ref_deg.is_finite() {
        bail!("fin_cant_ref_deg must be finite, got {}", m.fin_cant_ref_deg);
    }
    if !m.fin_roll_deg.is_finite() {
        bail!("fin_roll_deg must be finite, got {}", m.fin_roll_deg);
    }
    if m.moment_ref != "nose_tip" {
        bail!("moment_ref \"{}\" is not supported: tabulated moments must be about the \"nose_tip\" (the simulator transfers them to the CG)", m.moment_ref);
    }
    validate_axis("machs", &m.machs)?;
    validate_axis("alphas_deg", &m.alphas_deg)?;
    match m.kind {
        TableKind::Axisymmetric => {
            // The small-angle coefficients (zero-lift drag, normal-force slope, centre of pressure)
            // must be tabulated, not extrapolated from larger angles.
            if m.alphas_deg[0] != 0.0 {
                bail!(
                    "alphas_deg of an axisymmetric table must start at 0 deg (the total angle of attack), found {}: the small-angle coefficients would be extrapolated",
                    m.alphas_deg[0]
                );
            }
        }
        TableKind::General => {
            validate_axis("betas_deg", &m.betas_deg)?;
            for (name, axis) in [("alphas_deg", &m.alphas_deg), ("betas_deg", &m.betas_deg)] {
                let (lo, hi) = (axis[0], axis[axis.len() - 1]);
                if !(lo <= 0.0 && hi >= 0.0) {
                    bail!("{name} of a general table must include 0 deg between its first and last value, found {lo} to {hi}: the small-angle coefficients would be extrapolated");
                }
            }
        }
    }
    Ok(())
}

impl Default for TableMeta {
    fn default() -> Self {
        Self {
            format_version: FORMAT_VERSION,
            kind: TableKind::Axisymmetric,
            source_hash: String::new(),
            ref_area: 0.0,
            ref_diameter: 0.0,
            length: 0.0,
            machs: Vec::new(),
            alphas_deg: Vec::new(),
            betas_deg: Vec::new(),
            fin_cant_ref_deg: 0.0,
            moment_ref: nose_tip(),
            extrapolation: Extrapolation::Linear,
            fin_roll_deg: 0.0,
            fin_count: None,
        }
    }
}

/// Coefficients at one flight condition.
#[derive(Debug, Clone, Copy, Default)]
pub struct AeroCoeffs {
    pub cn: f64,
    pub ca_on: f64,
    pub ca_off: f64,
    /// Centre of pressure aft of the nose tip [m].
    pub xcp: f64,
    /// Normal-force slope at zero alpha [1/rad].
    pub cna: f64,
    /// Pitch-damping sums Σ CNa_i, Σ CNa_i x_i, Σ CNa_i x_i².
    pub damp: [f64; 3],
    /// Roll moment per radian of fin cant, d Cl / d delta (Cl normalised by q S d).
    pub cl_delta: f64,
    /// Roll-damping derivative d Cl / d p_hat with p_hat = p d / (2 V) (negative).
    pub clp: f64,
    /// Lag (alpha-dot) sums [m] and [m^2]: the lateral force `-(rho S / 2) lag_l0 du_perp/dt` of the
    /// air the body carries along (apparent mass), acting at `lag_l1 / lag_l0` aft of the nose tip.
    pub lag_l0: f64,
    pub lag_l1: f64,
    /// Induced roll: `Cl = cl4 alpha^2 sin(4 phi_w)` (alpha the total angle of attack [rad],
    /// `phi_w` the roll angle of the crossflow from a fin, see [`TableMeta::fin_roll_deg`]).
    pub cl4: f64,
    /// Magnus force slope [1/rad] (per radian of angle of attack; `p_hat = p d / (2 V)` is
    /// dimensionless): `C_N,Magnus = cnpa p_hat sin(alpha)`. With McCoy's normalisation by
    /// `p d / V` (Modern Exterior Ballistics, ch. 2), `cnpa = 2 C_Npa`. Positive along
    /// `u_b x e_x` (McCoy's `V x x`, `u_b` the velocity of the body relative to the air in body
    /// axes): for `p > 0` and a crossflow along +z_b the force points along +y_b, 90 deg from the
    /// static normal force. The flight model applies it at the centre of pressure of the static
    /// normal force; `cmpa` adds a Magnus moment about that point.
    pub cnpa: f64,
    /// Magnus moment slope [1/rad] about the centre of pressure of the static normal force:
    /// `dM = q S d cmpa p_hat sin(alpha) e_perp`, `e_perp` the unit lateral airspeed (0, u_y, u_z)/|.|
    /// of the body (McCoy's C_Mpa direction `v - (v.x) x`, `cmpa = 2 C_Mpa` for his `p d / V`). With
    /// the force at the static centre of pressure, McCoy's moment about the CG is
    /// `C_Mpa* = cnpa (x_cg - x_cp) / d + cmpa`. 0: the Magnus force alone.
    pub cmpa: f64,
}

/// Six body-axis coefficients and the rate-damping terms at one flight condition.
///
/// Body axes: x forward (nose), y and z from [`geom::Frame::body_basis`]. Forces are normalised by
/// q S, moments by q S d and taken about the nose tip.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Coeffs6 {
    pub cx_on: f64,
    pub cx_off: f64,
    pub cy: f64,
    pub cz: f64,
    pub cl: f64,
    pub cm: f64,
    pub cn: f64,
    /// d Cl / d delta per radian of fin cant (added for cant different from the table reference).
    pub cl_delta: f64,
    /// d Cl / d p_hat, p_hat = p d / (2 V).
    pub clp: f64,
    /// Pitch-damping sums about the nose (as `AeroCoeffs::damp`, x aft of the nose tip).
    pub sq: [f64; 3],
    /// Yaw-damping sums about the nose.
    pub sr: [f64; 3],
    /// Lag (apparent-mass) sums about the nose, as [`AeroCoeffs::lag_l0`] and `lag_l1`.
    pub lag_l0: f64,
    pub lag_l1: f64,
    /// Magnus force slope, as [`AeroCoeffs::cnpa`].
    pub cnpa: f64,
    /// Magnus moment slope about the effective centre of pressure, as [`AeroCoeffs::cmpa`].
    pub cmpa: f64,
}

impl Coeffs6 {
    fn to_arr(self) -> [f64; N_GEN] {
        let c = self;
        [
            c.cx_on, c.cx_off, c.cy, c.cz, c.cl, c.cm, c.cn, c.cl_delta, c.clp, c.sq[0], c.sq[1], c.sq[2], c.sr[0], c.sr[1], c.sr[2], c.lag_l0, c.lag_l1,
            c.cnpa, c.cmpa,
        ]
    }
    fn from_arr(a: [f64; N_GEN]) -> Self {
        Self {
            cx_on: a[0],
            cx_off: a[1],
            cy: a[2],
            cz: a[3],
            cl: a[4],
            cm: a[5],
            cn: a[6],
            cl_delta: a[7],
            clp: a[8],
            sq: [a[9], a[10], a[11]],
            sr: [a[12], a[13], a[14]],
            lag_l0: a[15],
            lag_l1: a[16],
            cnpa: a[17],
            cmpa: a[18],
        }
    }
}

const N_FIELDS: usize = 15;
/// Value columns of the older axisymmetric column sets (version 3 without `cmpa`, version 2 also
/// without the lag and Magnus columns, version 1 also without the roll columns).
const N_FIELDS_V3: usize = 14;
const N_FIELDS_V2: usize = 10;
const N_FIELDS_V1: usize = 8;
const N_GEN: usize = 19;
/// Value columns of the older general column sets (without `cmpa`; also without the lag and Magnus columns).
const N_GEN_V2: usize = 18;
const N_GEN_V1: usize = 15;
const HEADER: &str = "mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2,cl_delta,clp,lag_l0,lag_l1,cl4,cnpa,cmpa";
const HEADER_GEN: &str = "mach,alpha_deg,beta_deg,cx_on,cx_off,cy,cz,cl,cm,cn,cl_delta,clp,sq0,sq1,sq2,sr0,sr1,sr2,lag_l0,lag_l1,cnpa,cmpa";

#[derive(Debug, Clone)]
enum TableData {
    /// Row-major [mach][alpha]: cn, ca_on, ca_off, xcp, cna, s0, s1, s2, cl_delta, clp, lag_l0,
    /// lag_l1, cl4, cnpa.
    Axisymmetric(Vec<[f64; N_FIELDS]>),
    /// Row-major [mach][alpha][beta]: the fields of [`Coeffs6`].
    General(Vec<[f64; N_GEN]>),
}

#[derive(Debug, Clone)]
pub struct AeroTable {
    pub meta: TableMeta,
    data: TableData,
    counters: Arc<RangeCounters>,
    /// False for a table loaded from a file without the lag and Magnus columns.
    lag_columns: bool,
}

/// Unit vector of the vehicle velocity relative to the air in body axes for alpha and beta [rad]:
/// (cos b cos a, sin b, cos b sin a).
pub fn airspeed_direction(alpha: f64, beta: f64) -> [f64; 3] {
    [beta.cos() * alpha.cos(), beta.sin(), beta.cos() * alpha.sin()]
}

/// alpha = atan2(w, u) and beta = asin(v / V) of a body-axis airspeed (u, v, w) [rad].
pub fn aero_angles(u: f64, v: f64, w: f64) -> (f64, f64) {
    let speed = (u * u + v * v + w * w).sqrt();
    if speed <= 0.0 {
        return (0.0, 0.0);
    }
    (w.atan2(u), (v / speed).clamp(-1.0, 1.0).asin())
}

/// Six-component coefficients of an axisymmetric data point at angles (alpha, beta) [rad].
///
/// The total angle is the angle between the airspeed and the body axis; the normal force acts
/// against the crossflow at the centre of pressure, the axial force along the axis (reversed in
/// backward flight, as in the axisymmetric flight model), and the roll moment is the tabulated
/// roll forcing at the reference cant plus the induced roll [`induced_roll`] of the fin
/// at `fin_roll_rad` (both reversed in backward flight).
pub fn expand_axisymmetric(c: &AeroCoeffs, alpha: f64, beta: f64, ref_diameter: f64, cant_ref_rad: f64, fin_roll_rad: f64) -> Coeffs6 {
    let [u, v, w] = airspeed_direction(alpha, beta);
    let lat = v.hypot(w);
    let (ey, ez) = if lat > 1e-12 { (v / lat, w / lat) } else { (0.0, 0.0) };
    let sgn = if u >= 0.0 { 1.0 } else { -1.0 };
    let (cy, cz) = (-c.cn * ey, -c.cn * ez);
    let d = ref_diameter;
    let total = lat.atan2(u);
    let eff = total.min(std::f64::consts::PI - total);
    Coeffs6 {
        cx_on: -c.ca_on * sgn,
        cx_off: -c.ca_off * sgn,
        cy,
        cz,
        cl: (c.cl_delta * cant_ref_rad + induced_roll(c.cl4, eff, v, w, fin_roll_rad)) * sgn,
        cm: c.xcp * cz / d,
        cn: -c.xcp * cy / d,
        cl_delta: c.cl_delta,
        clp: c.clp,
        sq: c.damp,
        sr: c.damp,
        lag_l0: c.lag_l0,
        lag_l1: c.lag_l1,
        cnpa: c.cnpa,
        cmpa: c.cmpa,
    }
}

/// Induced roll coefficient `cl4 alpha^2 sin(4 phi_w)` of a cruciform fin set at the effective total
/// angle of attack `alpha` [rad] (nose-first flow; the caller reverses it in tail-first flow, as the
/// cant forcing). `phi_w = atan2(w, v) - fin_roll` is the roll angle of the crossflow (the
/// lateral airspeed `(v, w)` of the body in body axes) from the plane of a fin at roll angle
/// `fin_roll` [rad] from +y_b towards +z_b: zero with the crossflow in a fin plane (`+` and `x`
/// configurations are symmetric), largest at 22.5 deg.
pub fn induced_roll(cl4: f64, alpha: f64, v: f64, w: f64, fin_roll: f64) -> f64 {
    if cl4 == 0.0 || v.hypot(w) <= 1e-12 {
        return 0.0;
    }
    cl4 * alpha * alpha * (4.0 * (w.atan2(v) - fin_roll)).sin()
}

/// Non-uniform symmetric angle axis: `fine_step` up to +-`fine_limit`, then `coarse_step` up to +-`limit` [deg].
pub fn angle_axis(limit: f64, fine_limit: f64, fine_step: f64, coarse_step: f64) -> Vec<f64> {
    let fine_limit = fine_limit.min(limit);
    let mut pos = grid(0.0, fine_limit, fine_step);
    if limit > fine_limit + 1e-9 {
        let n = ((limit - fine_limit) / coarse_step).ceil().max(1.0) as usize;
        pos.extend((1..=n).map(|i| fine_limit + (limit - fine_limit) * i as f64 / n as f64));
    }
    let mut out: Vec<f64> = pos.iter().skip(1).rev().map(|v| -v).collect();
    out.extend(pos);
    out
}

fn grid(lo: f64, hi: f64, step: f64) -> Vec<f64> {
    let n = ((hi - lo) / step).round().max(1.0) as usize;
    (0..=n).map(|i| lo + (hi - lo) * i as f64 / n as f64).collect()
}

impl AeroTable {
    pub fn build(model: &AeroModel, source_hash: String, extrapolation: Extrapolation) -> Self {
        Self::build_with_progress(model, source_hash, extrapolation, &|_, _| {})
    }

    /// [`build`](Self::build) calling `progress(done_rows, total_rows)` after every Mach row.
    pub fn build_with_progress(model: &AeroModel, source_hash: String, extrapolation: Extrapolation, progress: &dyn Fn(usize, usize)) -> Self {
        let o = &model.opt;
        let machs = grid(o.mach_min.max(0.0), o.mach_max, o.mach_step);
        let alphas_deg = grid(0.0, o.alpha_max_deg, o.alpha_step_deg);
        let mut data = Vec::with_capacity(machs.len() * alphas_deg.len());
        for (row, &m) in machs.iter().enumerate() {
            let cd_on = model.cd0(m, true).total();
            let cd_off = model.cd0(m, false).total();
            let cna = model.cna(m);
            let s = model.damping_sums(m);
            let roll = model.roll_terms(m);
            let lag = model.lag_sums(m);
            for &a in &alphas_deg {
                let ar = a.to_radians();
                let (cn, xcp) = if a == 0.0 { (0.0, model.xcp0(m)) } else { model.normal(m, ar) };
                let k = axial_alpha_factor(ar);
                // The component build-up gives no induced roll or Magnus force (zero columns).
                data.push([cn, cd_on * k, cd_off * k, xcp, cna, s[0], s[1], s[2], roll.0, roll.1, lag.0, lag.1, 0.0, 0.0, 0.0]);
            }
            progress(row + 1, machs.len());
        }
        let g = &model.geom;
        let meta = TableMeta {
            source_hash,
            ref_area: g.ref_area,
            ref_diameter: 2.0 * g.ref_radius,
            length: g.length,
            machs,
            alphas_deg,
            fin_cant_ref_deg: g.fins.as_ref().map_or(0.0, |f| f.cant_deg),
            extrapolation,
            fin_count: Some(g.fins.as_ref().map_or(0, |f| f.count)),
            ..Default::default()
        };
        Self { meta, data: TableData::Axisymmetric(data), counters: Default::default(), lag_columns: true }
    }

    /// Axisymmetric table from an arbitrary function of (Mach, alpha [deg]); used for tests and custom data.
    pub fn from_fn(meta: TableMeta, f: impl Fn(f64, f64) -> AeroCoeffs) -> Self {
        let mut data = Vec::new();
        for &m in &meta.machs {
            for &a in &meta.alphas_deg {
                let c = f(m, a);
                data.push([
                    c.cn, c.ca_on, c.ca_off, c.xcp, c.cna, c.damp[0], c.damp[1], c.damp[2], c.cl_delta, c.clp, c.lag_l0, c.lag_l1, c.cl4, c.cnpa, c.cmpa,
                ]);
            }
        }
        let meta = TableMeta { kind: TableKind::Axisymmetric, format_version: FORMAT_VERSION, ..meta };
        Self { meta, data: TableData::Axisymmetric(data), counters: Default::default(), lag_columns: true }
    }

    /// General table from a function of (Mach, alpha [deg], beta [deg]) on `meta`'s grid.
    pub fn general_from_fn(meta: TableMeta, f: impl Fn(f64, f64, f64) -> Coeffs6) -> Self {
        let mut data = Vec::with_capacity(meta.machs.len() * meta.alphas_deg.len() * meta.betas_deg.len());
        for &m in &meta.machs {
            for &a in &meta.alphas_deg {
                for &b in &meta.betas_deg {
                    data.push(f(m, a, b).to_arr());
                }
            }
        }
        let meta = TableMeta { kind: TableKind::General, format_version: FORMAT_VERSION, ..meta };
        Self { meta, data: TableData::General(data), counters: Default::default(), lag_columns: true }
    }

    pub fn kind(&self) -> TableKind {
        self.meta.kind
    }

    /// General table with the same Mach grid on (alphas, betas) [deg], expanded from this
    /// axisymmetric table (a general table is returned unchanged).
    pub fn to_general(&self, alphas_deg: Vec<f64>, betas_deg: Vec<f64>) -> AeroTable {
        if self.meta.kind == TableKind::General {
            return self.clone();
        }
        let meta = TableMeta { alphas_deg, betas_deg, ..self.meta.clone() };
        let (d, cant, roll) = (self.meta.ref_diameter, self.meta.fin_cant_ref_deg.to_radians(), self.meta.fin_roll_deg.to_radians());
        let mut g = AeroTable::general_from_fn(meta, |m, a, b| {
            let (ar, br) = (a.to_radians(), b.to_radians());
            let [u, v, w] = airspeed_direction(ar, br);
            let total = v.hypot(w).atan2(u);
            let eff = total.min(std::f64::consts::PI - total);
            expand_axisymmetric(&self.cl4_for_fins(self.lookup(m, eff)), ar, br, d, cant, roll)
        });
        g.lag_columns = self.lag_columns;
        g
    }

    /// Writes the table as CSV plus a JSON sidecar next to it.
    ///
    /// Both files are written to temporary names in the same directory and renamed into place
    /// (CSV first, sidecar last), so an interrupted or concurrent run never leaves a half-written file.
    pub fn save(&self, csv_path: &Path) -> Result<()> {
        let json_path = csv_path.with_extension("json");
        let tmp = |p: &Path| {
            let mut n = p.file_name().map(|n| n.to_os_string()).unwrap_or_default();
            n.push(format!(".tmp{}", std::process::id()));
            p.with_file_name(n)
        };
        let (csv_tmp, json_tmp) = (tmp(csv_path), tmp(&json_path));
        let result = self.save_to(&csv_tmp, &json_tmp).and_then(|()| {
            std::fs::rename(&csv_tmp, csv_path).with_context(|| format!("cannot write {}", csv_path.display()))?;
            std::fs::rename(&json_tmp, &json_path).with_context(|| format!("cannot write {}", json_path.display()))
        });
        if result.is_err() {
            let _ = std::fs::remove_file(&csv_tmp);
            let _ = std::fs::remove_file(&json_tmp);
        }
        result
    }

    fn save_to(&self, csv_path: &Path, json_path: &Path) -> Result<()> {
        let mut f = std::io::BufWriter::new(std::fs::File::create(csv_path).with_context(|| format!("cannot write {}", csv_path.display()))?);
        let na = self.meta.alphas_deg.len();
        match &self.data {
            TableData::Axisymmetric(data) => {
                writeln!(f, "{HEADER}")?;
                for (i, row) in data.iter().enumerate() {
                    write!(f, "{},{}", self.meta.machs[i / na], self.meta.alphas_deg[i % na])?;
                    for v in row {
                        write!(f, ",{v:e}")?;
                    }
                    writeln!(f)?;
                }
            }
            TableData::General(data) => {
                writeln!(f, "{HEADER_GEN}")?;
                let nb = self.meta.betas_deg.len();
                for (i, row) in data.iter().enumerate() {
                    let (im, ia, ib) = (i / (na * nb), (i / nb) % na, i % nb);
                    write!(f, "{},{},{}", self.meta.machs[im], self.meta.alphas_deg[ia], self.meta.betas_deg[ib])?;
                    for v in row {
                        write!(f, ",{v:e}")?;
                    }
                    writeln!(f)?;
                }
            }
        }
        let meta = TableMeta { format_version: FORMAT_VERSION, ..self.meta.clone() };
        f.flush()?;
        std::fs::write(json_path, serde_json::to_string_pretty(&meta)?)?;
        Ok(())
    }

    /// Loads a table. Older column sets load with the missing columns zero: axisymmetric files with
    /// 8 value columns (version 1: no roll terms) or 10 (version 2: no lag and Magnus columns),
    /// general files with 15 (no lag and Magnus columns); see [`has_lag_columns`](Self::has_lag_columns).
    ///
    /// The sidecar is validated (finite positive reference lengths and area, finite strictly
    /// increasing non-empty axes, `moment_ref` = "nose_tip"; the angle grid of an axisymmetric table
    /// starts at 0 deg, both angle grids of a general table include 0 deg), every cell must be
    /// finite and every row's coordinate columns must equal the declared grid point in row-major
    /// order.
    pub fn load(csv_path: &Path) -> Result<Self> {
        let json_path = csv_path.with_extension("json");
        let json = std::fs::read_to_string(&json_path).with_context(|| format!("cannot read aero table metadata {}", json_path.display()))?;
        let json = json.strip_prefix('\u{feff}').unwrap_or(&json);
        let mut meta: TableMeta = serde_json::from_str(json).with_context(|| format!("bad aero table metadata {}", json_path.display()))?;
        validate_meta(&meta).with_context(|| format!("invalid aero table metadata {}", json_path.display()))?;
        let text = std::fs::read_to_string(csv_path).with_context(|| format!("cannot read aero table {}", csv_path.display()))?;
        let text = text.strip_prefix('\u{feff}').unwrap_or(&text);
        let header: Vec<&str> = text.lines().next().unwrap_or("").split(',').map(str::trim).collect();
        let nc = if meta.kind == TableKind::General { 3 } else { 2 };
        // Values are read by position, so the header must name the columns in the expected order
        // (older column sets end early: an axisymmetric table without `cmpa`, possibly also without
        // the lag and Magnus columns and the roll columns, a general table without `cmpa` and
        // possibly the lag and Magnus columns).
        let expected_header = if meta.kind == TableKind::General { HEADER_GEN } else { HEADER };
        let expected: Vec<&str> = expected_header.split(',').collect();
        let widths: &[usize] = if meta.kind == TableKind::General {
            &[N_GEN + 3, N_GEN_V2 + 3, N_GEN_V1 + 3]
        } else {
            &[N_FIELDS + 2, N_FIELDS_V3 + 2, N_FIELDS_V2 + 2, N_FIELDS_V1 + 2]
        };
        let ok = widths.contains(&header.len());
        if !ok || !header.iter().zip(&expected).all(|(h, e)| h.eq_ignore_ascii_case(e)) {
            bail!("aero table {}: unexpected CSV header `{}`; expected `{expected_header}`", csv_path.display(), header.join(","));
        }
        let rows = text.lines().enumerate().skip(1).filter(|(_, l)| !l.trim().is_empty()).map(|(ln, line)| {
            let v = line
                .split(',')
                .map(|s| s.trim().parse::<f64>())
                .collect::<Result<Vec<f64>, _>>()
                .with_context(|| format!("bad row {} in {}: {line}", ln + 1, csv_path.display()))?;
            Ok::<_, anyhow::Error>((ln + 1, v))
        });
        let (na, nb) = (meta.alphas_deg.len(), meta.betas_deg.len());
        let expected_coords = |i: usize| -> Vec<f64> {
            match meta.kind {
                TableKind::Axisymmetric => vec![meta.machs[i / na], meta.alphas_deg[i % na]],
                TableKind::General => vec![meta.machs[i / (na * nb)], meta.alphas_deg[(i / nb) % na], meta.betas_deg[i % nb]],
            }
        };
        let check_row = |idx: usize, ln: usize, v: &[f64]| -> Result<()> {
            // Every cell first, coordinates included (an infinite coordinate would pass the
            // tolerance comparison below).
            for (k, x) in v.iter().enumerate() {
                if !x.is_finite() {
                    bail!("row {ln} of {}: column {} ({}) is not finite: {x}", csv_path.display(), k + 1, header.get(k).unwrap_or(&"?"));
                }
            }
            let want = expected_coords(idx);
            let near = |a: f64, b: f64| (a - b).abs() <= 1e-9 + 1e-6 * a.abs().max(b.abs());
            if !want.iter().zip(&v[..nc]).all(|(&w, &f)| near(w, f)) {
                bail!("row {ln} of {}: coordinates {:?} do not match the grid point {:?} expected at this position (rows must follow the metadata axes in order: Mach outermost{})", csv_path.display(), &v[..nc], want, if nc == 3 { ", then alpha, then beta" } else { ", then alpha" });
            }
            Ok(())
        };
        let n_expected = match meta.kind {
            TableKind::Axisymmetric => meta.machs.len() * na,
            TableKind::General => meta.machs.len() * na * nb,
        };
        let data = match meta.kind {
            TableKind::Axisymmetric => {
                let mut data = Vec::new();
                for item in rows {
                    let (ln, v) = item?;
                    let mut r = [0.0; N_FIELDS];
                    if v.len() != header.len() {
                        bail!("bad row {ln} in {}: expected {} values (as the header), found {}", csv_path.display(), header.len(), v.len());
                    }
                    r[..v.len() - 2].copy_from_slice(&v[2..]);
                    if data.len() >= n_expected {
                        bail!("aero table {} has more rows than the {n_expected} grid points of its metadata", csv_path.display());
                    }
                    check_row(data.len(), ln, &v)?;
                    data.push(r);
                }
                TableData::Axisymmetric(data)
            }
            TableKind::General => {
                let mut data = Vec::new();
                for item in rows {
                    let (ln, v) = item?;
                    if v.len() != header.len() {
                        bail!("bad row {ln} in {}: expected {} values (as the header), found {}", csv_path.display(), header.len(), v.len());
                    }
                    if data.len() >= n_expected {
                        bail!("aero table {} has more rows than the {n_expected} grid points of its metadata", csv_path.display());
                    }
                    check_row(data.len(), ln, &v)?;
                    let mut r = [0.0; N_GEN];
                    r[..v.len() - 3].copy_from_slice(&v[3..]);
                    data.push(r);
                }
                TableData::General(data)
            }
        };
        let found = match &data {
            TableData::Axisymmetric(d) => d.len(),
            TableData::General(d) => d.len(),
        };
        if found != n_expected {
            bail!("aero table {} has {found} rows but its metadata declares {n_expected} grid points", csv_path.display());
        }
        meta.format_version = FORMAT_VERSION;
        // The lag columns are there from version 3 (axisymmetric) / the 18-column general set on.
        let lag_columns = header.len() >= widths[1];
        Ok(Self { meta, data, counters: Default::default(), lag_columns })
    }

    /// False for a table loaded from a file without the lag and Magnus columns (an older column
    /// set); its lag sums are zero unless [`set_lag_terms`](Self::set_lag_terms) fills them.
    pub fn has_lag_columns(&self) -> bool {
        self.lag_columns
    }

    /// Fills the lag (apparent-mass) sums of the table from `f(mach) -> (lag_l0, lag_l1)`, as
    /// [`set_roll_terms`](Self::set_roll_terms); used for tables whose source gives no lag terms.
    pub fn set_lag_terms(&mut self, f: impl Fn(f64) -> (f64, f64)) {
        let na = self.meta.alphas_deg.len();
        let (stride, k) = match &self.data {
            TableData::Axisymmetric(_) => (na, 10),
            TableData::General(_) => (na * self.meta.betas_deg.len().max(1), 15),
        };
        let machs = &self.meta.machs;
        let fill = |i: usize, row: &mut [f64]| {
            let (l0, l1) = f(machs[i / stride]);
            row[k] = l0;
            row[k + 1] = l1;
        };
        match &mut self.data {
            TableData::Axisymmetric(data) => data.iter_mut().enumerate().for_each(|(i, r)| fill(i, r)),
            TableData::General(data) => data.iter_mut().enumerate().for_each(|(i, r)| fill(i, r)),
        }
    }

    /// `c` with the induced roll `cl4` dropped unless the table is for a cruciform (4-fin) set or its
    /// fin count is not recorded: `sin(4 phi_w)` is the form of four fins.
    fn cl4_for_fins(&self, mut c: AeroCoeffs) -> AeroCoeffs {
        if self.meta.fin_count.unwrap_or(4) != 4 {
            c.cl4 = 0.0;
        }
        c
    }

    /// True when any induced-roll coefficient `cl4` of an axisymmetric table is nonzero.
    pub fn has_induced_roll(&self) -> bool {
        match &self.data {
            TableData::Axisymmetric(d) => d.iter().any(|r| r[12] != 0.0),
            TableData::General(_) => false,
        }
    }

    /// True when any Magnus slope (`cnpa` or `cmpa`) of the table is nonzero.
    pub fn has_magnus_terms(&self) -> bool {
        match &self.data {
            TableData::Axisymmetric(d) => d.iter().any(|r| r[13] != 0.0 || r[14] != 0.0),
            TableData::General(d) => d.iter().any(|r| r[17] != 0.0 || r[18] != 0.0),
        }
    }

    /// True when any lag sum of the table is nonzero.
    pub fn has_lag_terms(&self) -> bool {
        match &self.data {
            TableData::Axisymmetric(d) => d.iter().any(|r| r[10] != 0.0 || r[11] != 0.0),
            TableData::General(d) => d.iter().any(|r| r[15] != 0.0 || r[16] != 0.0),
        }
    }

    /// Fills the roll terms (cant forcing and roll damping) of an axisymmetric table from
    /// `f(mach) -> (cl_delta, clp)`; used for external tables without roll data.
    pub fn set_roll_terms(&mut self, f: impl Fn(f64) -> (f64, f64)) {
        let na = self.meta.alphas_deg.len();
        match &mut self.data {
            TableData::Axisymmetric(data) => {
                for (i, row) in data.iter_mut().enumerate() {
                    let (cd, cp) = f(self.meta.machs[i / na]);
                    row[8] = cd;
                    row[9] = cp;
                }
            }
            TableData::General(data) => {
                let nb = self.meta.betas_deg.len().max(1);
                for (i, row) in data.iter_mut().enumerate() {
                    let (cd, cp) = f(self.meta.machs[i / (na * nb)]);
                    row[7] = cd;
                    row[8] = cp;
                }
            }
        }
    }

    /// True when every roll term of the table is zero (e.g. a version-1 file).
    pub fn has_roll_terms(&self) -> bool {
        match &self.data {
            TableData::Axisymmetric(d) => d.iter().any(|r| r[8] != 0.0 || r[9] != 0.0),
            TableData::General(d) => d.iter().any(|r| r[7] != 0.0 || r[8] != 0.0),
        }
    }

    /// Snapshot of the out-of-range lookup statistics (all clones of a table share them).
    pub fn out_of_range_stats(&self) -> OutOfRangeStats {
        let c = &self.counters;
        OutOfRangeStats {
            lookups: c.lookups.load(Ordering::Relaxed),
            mach_below: c.mach_below.load(Ordering::Relaxed),
            mach_above: c.mach_above.load(Ordering::Relaxed),
            angle_beyond: c.angle_beyond.load(Ordering::Relaxed),
            max_mach: f64::from_bits(c.max_mach.load(Ordering::Relaxed)),
            max_abs_alpha_deg: f64::from_bits(c.max_abs_alpha_deg.load(Ordering::Relaxed)),
        }
    }

    /// Resets the out-of-range lookup statistics.
    pub fn reset_out_of_range_stats(&self) {
        let c = &self.counters;
        for a in [&c.lookups, &c.mach_below, &c.mach_above, &c.angle_beyond, &c.max_mach, &c.max_abs_alpha_deg] {
            a.store(0, Ordering::Relaxed);
        }
    }

    fn record(&self, m: f64, alpha_deg: f64, angle_out: bool) {
        let c = &self.counters;
        c.lookups.fetch_add(1, Ordering::Relaxed);
        let (lo, hi) = (self.meta.machs.first().copied().unwrap_or(0.0), self.meta.machs.last().copied().unwrap_or(0.0));
        if m < lo - 1e-9 {
            c.mach_below.fetch_add(1, Ordering::Relaxed);
        } else if m > hi + 1e-9 {
            c.mach_above.fetch_add(1, Ordering::Relaxed);
        }
        if angle_out {
            c.angle_beyond.fetch_add(1, Ordering::Relaxed);
        }
        if m.is_finite() && m > 0.0 {
            c.max_mach.fetch_max(m.to_bits(), Ordering::Relaxed);
        }
        let a = alpha_deg.abs();
        if a.is_finite() && a > 0.0 {
            c.max_abs_alpha_deg.fetch_max(a.to_bits(), Ordering::Relaxed);
        }
    }

    /// Coefficients at Mach `m` and total angle of attack `alpha` [rad] (≥ 0) of an axisymmetric table.
    ///
    /// The fields are interpolated bilinearly; the centre of pressure is not interpolated itself:
    /// the normal force CN and the nose moment CN x_cp are, and `xcp` = moment / CN, so the
    /// moment the flight applies (CN x_cp) is the interpolated moment, as in a general table
    /// expanded on the same grid. Where the interpolated CN is zero (zero angle of attack) `xcp`
    /// is the ratio of the interpolated zero-angle slopes, (CNa x_cp) / CNa; with CNa zero as well
    /// the tabulated `xcp` is interpolated.
    ///
    /// Extrapolation rules: Mach is always held at the grid edge (transonic/supersonic trends do not
    /// continue linearly beyond the table); alpha continues linearly with the
    /// [`Extrapolation::Linear`] option (a descending rocket needs large angles) and is held with
    /// `Clamp`. Linearly extrapolated values are bounded: they keep the sign of the edge value
    /// (CN, the nose moment, CA), CNa and the damping sums stay non-negative, `cl_delta` keeps its
    /// sign, the centre of pressure stays inside the body and the roll damping is never positive.
    /// Lookups outside the grid (above the last angle node or below the first) are counted, see
    /// [`out_of_range_stats`](Self::out_of_range_stats).
    ///
    /// Panics for a general table; use [`lookup6`](Self::lookup6).
    pub fn lookup(&self, m: f64, alpha: f64) -> AeroCoeffs {
        self.lookup_rec(m, alpha, true)
    }

    /// [`lookup`](Self::lookup); counts the lookup in the range statistics only when `rec` is set.
    fn lookup_rec(&self, m: f64, alpha: f64, rec: bool) -> AeroCoeffs {
        let TableData::Axisymmetric(data) = &self.data else { panic!("lookup() on a general aero table") };
        let clamp = self.meta.extrapolation == Extrapolation::Clamp;
        let (i, tm) = locate(&self.meta.machs, m, true);
        let alpha_deg = alpha.abs().to_degrees();
        let (j, ta) = locate(&self.meta.alphas_deg, alpha_deg, clamp);
        let na = self.meta.alphas_deg.len();
        let at = |ii: usize, jj: usize| &data[ii * na + jj];
        let (i1, j1) = ((i + 1).min(self.meta.machs.len() - 1), (j + 1).min(na - 1));
        // Bilinear interpolation of the table fields; the centre-of-pressure field (3) is replaced by
        // the nose moment CN x_cp, and an extra field holds the zero-angle moment slope CNa x_cp.
        let interp = |ta: f64| {
            let mut out = [0.0; N_FIELDS + 1];
            for (k, o) in out.iter_mut().enumerate() {
                let f = |r: &[f64; N_FIELDS]| match k {
                    3 => r[0] * r[3],
                    N_FIELDS => r[4] * r[3],
                    _ => r[k],
                };
                let (v00, v01, v10, v11) = (f(at(i, j)), f(at(i, j1)), f(at(i1, j)), f(at(i1, j1)));
                let a0 = v00 + (v01 - v00) * ta;
                let a1 = v10 + (v11 - v10) * ta;
                *o = a0 + (a1 - a0) * tm;
            }
            out
        };
        // Plain interpolation of the tabulated x_cp; only used when neither CN nor CNa gives a ratio.
        let xcp_plain = |ta: f64| {
            let (a0, a1) = (at(i, j)[3] + (at(i, j1)[3] - at(i, j)[3]) * ta, at(i1, j)[3] + (at(i1, j1)[3] - at(i1, j)[3]) * ta);
            a0 + (a1 - a0) * tm
        };
        let mut out = interp(ta);
        let extrapolated = !(0.0..=1.0).contains(&ta);
        let beyond = outside(&self.meta.alphas_deg, alpha_deg);
        if rec {
            self.record(m, alpha_deg, beyond);
        }
        if extrapolated {
            let edge = interp(ta.clamp(0.0, 1.0));
            // cn, the nose moment, ca_on, ca_off, cna, the damping sums, cl_delta, the lag sums,
            // cl4, cnpa, cmpa and the moment slope keep the sign of the edge value.
            for k in [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 11, 12, 13, 14, N_FIELDS] {
                out[k] = keep_sign(out[k], edge[k]);
            }
            for k in [4, 5, 6, 7] {
                out[k] = out[k].max(0.0);
            }
        }
        // x_cp = moment / CN; at zero normal force (zero angle) the ratio of the slopes CNa x_cp / CNa.
        let mut xcp = if out[0] != 0.0 {
            out[3] / out[0]
        } else if out[4] != 0.0 {
            out[N_FIELDS] / out[4]
        } else {
            xcp_plain(ta)
        };
        if extrapolated && self.meta.length > 0.0 {
            xcp = xcp.clamp(0.0, self.meta.length);
        }
        out[3] = xcp;
        AeroCoeffs {
            cn: out[0],
            ca_on: out[1].max(0.0),
            ca_off: out[2].max(0.0),
            xcp: out[3],
            cna: out[4],
            damp: [out[5], out[6], out[7]],
            cl_delta: out[8],
            clp: out[9].min(0.0),
            lag_l0: out[10],
            lag_l1: out[11],
            cl4: out[12],
            cnpa: out[13],
            cmpa: out[14],
        }
    }

    /// Six-component coefficients at Mach `m`, alpha = atan2(w, u) and beta = asin(v / V) [rad].
    ///
    /// General tables are interpolated trilinearly (Mach held at the grid edge; with
    /// [`Extrapolation::Linear`] the angles continue linearly, every coefficient except `clp`
    /// keeping the sign of its edge value, the damping sums non-negative and `clp` never
    /// positive); axisymmetric tables are expanded from the total angle (see
    /// [`expand_axisymmetric`]).
    pub fn lookup6(&self, m: f64, alpha: f64, beta: f64) -> Coeffs6 {
        self.lookup6_rec(m, alpha, beta, true)
    }

    /// [`lookup6`](Self::lookup6); counts the lookup in the range statistics only when `rec` is set.
    fn lookup6_rec(&self, m: f64, alpha: f64, beta: f64, rec: bool) -> Coeffs6 {
        let data = match &self.data {
            TableData::General(d) => d,
            TableData::Axisymmetric(_) => {
                let [u, v, w] = airspeed_direction(alpha, beta);
                let total = v.hypot(w).atan2(u);
                let eff = total.min(std::f64::consts::PI - total);
                return expand_axisymmetric(
                    &self.cl4_for_fins(self.lookup_rec(m, eff, rec)),
                    alpha,
                    beta,
                    self.meta.ref_diameter,
                    self.meta.fin_cant_ref_deg.to_radians(),
                    self.meta.fin_roll_deg.to_radians(),
                );
            }
        };
        let clamp = self.meta.extrapolation == Extrapolation::Clamp;
        let (na, nb) = (self.meta.alphas_deg.len(), self.meta.betas_deg.len());
        // Mach is always held at the grid edge, see `lookup`.
        let (i, tm) = locate(&self.meta.machs, m, true);
        let (j, ta) = locate(&self.meta.alphas_deg, alpha.to_degrees(), clamp);
        let (k, tb) = locate(&self.meta.betas_deg, beta.to_degrees(), clamp);
        let extrapolated = !(0.0..=1.0).contains(&ta) || !(0.0..=1.0).contains(&tb);
        if rec {
            // The largest angle is the total angle of attack acos(cos a cos b), not the pitch-plane angle alone.
            let [u, v, w] = airspeed_direction(alpha, beta);
            self.record(
                m,
                v.hypot(w).atan2(u).to_degrees(),
                outside(&self.meta.alphas_deg, alpha.to_degrees()) || outside(&self.meta.betas_deg, beta.to_degrees()),
            );
        }
        let (i1, j1, k1) = ((i + 1).min(self.meta.machs.len() - 1), (j + 1).min(na - 1), (k + 1).min(nb - 1));
        let at = |ii: usize, jj: usize, kk: usize| &data[(ii * na + jj) * nb + kk];
        let interp = |ta: f64, tb: f64| {
            let mut out = [0.0; N_GEN];
            for (f, o) in out.iter_mut().enumerate() {
                let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
                let c = |ii: usize| {
                    let b0 = lerp(at(ii, j, k)[f], at(ii, j, k1)[f], tb);
                    let b1 = lerp(at(ii, j1, k)[f], at(ii, j1, k1)[f], tb);
                    lerp(b0, b1, ta)
                };
                *o = lerp(c(i), c(i1), tm);
            }
            out
        };
        let mut out = interp(ta, tb);
        if extrapolated {
            // Axial, side and normal force, the moments, the roll forcing (the tabulated `cl` at the
            // reference cant and `cl_delta`), the damping sums, the lag sums and the Magnus slopes
            // keep the sign of the edge value.
            let edge = interp(ta.clamp(0.0, 1.0), tb.clamp(0.0, 1.0));
            for f in [0, 1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18] {
                out[f] = keep_sign(out[f], edge[f]);
            }
            for f in [9, 10, 11, 12, 13, 14] {
                out[f] = out[f].max(0.0);
            }
        }
        out[8] = out[8].min(0.0);
        Coeffs6::from_arr(out)
    }

    /// Static margins [cal] in pitch and yaw at Mach `m` for a CG `x_cg` aft of the nose tip [m];
    /// positive is stable.
    ///
    /// Sign convention (see [`expand_axisymmetric`]): a positive alpha gives `Cz = -CN < 0`, a positive
    /// beta `Cy = -CN < 0`, and the moments about the nose are `Cm = x_cp Cz / d`, `Cn = -x_cp Cy / d`.
    /// About the CG, `Cm_cg = Cm - (x_cg / d) Cz` and `Cn_cg = Cn + (x_cg / d) Cy`. Static stability is
    /// the sign of the restoring derivative
    ///
    /// * pitch (beta = 0): `R = -dCm_cg/dalpha`, normal-force slope `S = dCz/dalpha`;
    /// * yaw (alpha = 0): `R = +dCn_cg/dbeta`, normal-force slope `S = dCy/dbeta`;
    ///
    /// `R > 0` is stable. The margin is `R / |S|`. With the physical sign `S < 0` it equals
    /// `x_cp / d - x_cg / d` (pitch: `x_cp,pitch / d = (dCm/dalpha) / (dCz/dalpha)`; yaw:
    /// `x_cp,yaw / d = -(dCn/dbeta) / (dCy/dbeta)`). With `S > 0` (the normal force points along the
    /// crossflow) a destabilising moment `R < 0` still gives the negative margin `R / |S|` (unstable),
    /// but a restoring one cannot be expressed as a centre-of-pressure margin: that plane is NaN and
    /// [`StaticMargins::unknown`] names the reason, as for `S = 0`.
    ///
    /// The slopes are central differences at +-delta, delta = the smaller of 0.5 deg and the
    /// smallest nonzero grid angle of that axis (both give the slope of the interpolated table
    /// at zero when the grid holds zero, i.e. the secant over the first angle cell). An
    /// axisymmetric table uses the same rule on its alpha axis, in both planes, through the
    /// interpolant the flight applies ([`lookup`](Self::lookup): Mach-interpolated CN and nose
    /// moment CN x_cp). With `CN_d`, `x_cp,d` the values at the total angle delta,
    /// `S = -CN_d / delta` and `R = CN_d (x_cp,d - x_cg) / (d delta)`, so the margin is
    /// `(x_cp,d - x_cg) / d` for `CN_d > 0`, unknown for `CN_d = 0`, `R / |S|` (unstable) for
    /// `CN_d < 0` with a destabilising moment and unknown for `CN_d < 0` with a restoring one.
    /// With `CN(0) = 0`, CN and CN x_cp are linear in alpha over the first cell, so `x_cp,d` is the
    /// centre of pressure of the first nonzero angle row (Mach-interpolated): the point where the
    /// flight applies the normal force at every small angle. The tabulated zero-angle `xcp` and
    /// `cna` columns do not enter. A general table expanded from it gives the same value.
    /// Pitch-yaw coupling is not considered. The lookups are not counted in the range statistics.
    pub fn static_margins(&self, m: f64, x_cg: f64) -> StaticMargins {
        let d = self.meta.ref_diameter;
        let mut unknown = Vec::new();
        // `s`: normal-force slope (physical sign negative), `r`: restoring derivative; `wrong` names
        // the unphysical sign of the slope as the table states it.
        let mut plane = |s: f64, r: f64, what: &str, wrong: &str| {
            if s.abs() <= 1e-12 {
                unknown.push(format!("the {what} is zero at Mach {m:.2}"));
                f64::NAN
            } else if s < 0.0 || r < 0.0 {
                r / s.abs()
            } else {
                unknown.push(format!(
                    "the {what} is {wrong} at Mach {m:.2} (the normal force points along the crossflow, a restoring moment cannot be expressed as a margin; check the sign convention of the table)"
                ));
                f64::NAN
            }
        };
        let delta = |axis: &[f64]| axis.iter().map(|a| a.abs()).filter(|a| *a > 1e-9).fold(0.5f64, f64::min).to_radians();
        if self.meta.kind == TableKind::Axisymmetric {
            // Secant slopes of the interpolant the flight applies (the central differences of the
            // symmetric expansion): dCz/dalpha = -CN(delta) / delta and -dCm_cg/dalpha =
            // CN(delta) (x_cp(delta) - x_cg) / (d delta), x_cp(delta) the ratio of the interpolated
            // nose moment to CN. Not the zero-angle row: with CN(0) = 0 the interpolated moment over
            // the first cell carries the first nonzero row's centre of pressure.
            let da = delta(&self.meta.alphas_deg);
            let c = self.lookup_rec(m, da, false);
            let mg = plane(-c.cn / da, c.cn * (c.xcp - x_cg) / (d * da), "normal-force slope dCN/dalpha", "negative");
            return StaticMargins { pitch: mg, yaw: mg, unknown: (!unknown.is_empty()).then(|| unknown.join("; ")) };
        }
        let (da, db) = (delta(&self.meta.alphas_deg), delta(&self.meta.betas_deg));
        let (p, n) = (self.lookup6_rec(m, da, 0.0, false), self.lookup6_rec(m, -da, 0.0, false));
        let dcz = p.cz - n.cz;
        let pitch = plane(dcz, -((p.cm - n.cm) - x_cg / d * dcz), "pitch normal-force slope dCz/dalpha", "positive");
        let (p, n) = (self.lookup6_rec(m, 0.0, db, false), self.lookup6_rec(m, 0.0, -db, false));
        let dcy = p.cy - n.cy;
        let yaw = plane(dcy, (p.cn - n.cn) + x_cg / d * dcy, "yaw normal-force slope dCy/dbeta", "positive");
        StaticMargins { pitch, yaw, unknown: (!unknown.is_empty()).then(|| unknown.join("; ")) }
    }

    /// Restoring derivative `-dCm_cg/dalpha` [1/rad] (moments normalised by q S d) at Mach `m` for a CG
    /// `x_cg` aft of the nose tip: the smaller of the pitch and yaw values, from the same secant
    /// slopes as [`static_margins`](Self::static_margins) (positive restoring). It sets the pitch
    /// natural frequency `sqrt(R q S d / I)` of the rocket. The lookups are not counted in the range
    /// statistics.
    pub fn restoring_derivative(&self, m: f64, x_cg: f64) -> f64 {
        let d = self.meta.ref_diameter;
        let delta = |axis: &[f64]| axis.iter().map(|a| a.abs()).filter(|a| *a > 1e-9).fold(0.5f64, f64::min).to_radians();
        if self.meta.kind == TableKind::Axisymmetric {
            let da = delta(&self.meta.alphas_deg);
            let c = self.lookup_rec(m, da, false);
            return c.cn * (c.xcp - x_cg) / (d * da);
        }
        let (da, db) = (delta(&self.meta.alphas_deg), delta(&self.meta.betas_deg));
        let (p, n) = (self.lookup6_rec(m, da, 0.0, false), self.lookup6_rec(m, -da, 0.0, false));
        let pitch = -((p.cm - n.cm) - x_cg / d * (p.cz - n.cz)) / (2.0 * da);
        let (p, n) = (self.lookup6_rec(m, 0.0, db, false), self.lookup6_rec(m, 0.0, -db, false));
        let yaw = ((p.cn - n.cn) + x_cg / d * (p.cy - n.cy)) / (2.0 * db);
        pitch.min(yaw)
    }

    /// Largest |Cm| and |Cn| (moments about the nose tip, normalised by q S d) of a general table at
    /// zero angle of attack and sideslip over its Mach rows: an aerodynamic asymmetry (trim
    /// moment). 0 for an axisymmetric table, and when the grid holds no zero-angle node.
    pub fn max_zero_angle_moment(&self) -> f64 {
        let TableData::General(d) = &self.data else { return 0.0 };
        let (na, nb) = (self.meta.alphas_deg.len(), self.meta.betas_deg.len());
        let (Some(ia), Some(ib)) = (self.meta.alphas_deg.iter().position(|a| a.abs() < 1e-9), self.meta.betas_deg.iter().position(|b| b.abs() < 1e-9)) else {
            return 0.0;
        };
        (0..self.meta.machs.len()).map(|i| &d[(i * na + ia) * nb + ib]).map(|r| r[5].abs().max(r[6].abs())).fold(0.0, f64::max)
    }

    /// Sanity warnings for a table that was not built by this program: a negative axial force
    /// coefficient (`ca_on` or `ca_off`) at zero angle of attack is thrust-like drag. One message
    /// per column, naming the first offending Mach number.
    pub fn sanity_warnings(&self) -> Vec<String> {
        let (na, nb) = (self.meta.alphas_deg.len(), self.meta.betas_deg.len().max(1));
        let Some(ia) = self.meta.alphas_deg.iter().position(|a| a.abs() < 1e-9) else { return Vec::new() };
        let ib = self.meta.betas_deg.iter().position(|b| b.abs() < 1e-9).unwrap_or(0);
        let mut out = Vec::new();
        for (name, field) in [("ca_on", 0usize), ("ca_off", 1)] {
            let values: Vec<(f64, f64)> = self
                .meta
                .machs
                .iter()
                .enumerate()
                .map(|(i, &m)| {
                    let ca = match &self.data {
                        TableData::Axisymmetric(d) => d[i * na + ia][1 + field],
                        TableData::General(d) => -d[(i * na + ia) * nb + ib][field],
                    };
                    (m, ca)
                })
                .filter(|&(_, ca)| ca < 0.0)
                .collect();
            if let Some(&(m, ca)) = values.first() {
                let more = if values.len() > 1 { format!("; {} Mach values in all", values.len()) } else { String::new() };
                out.push(format!("aero table: {name} at Mach {m}, alpha 0 is {ca:.2} (negative axial force / thrust-like drag{more})"));
            }
        }
        out
    }

    pub fn rows(&self) -> usize {
        match &self.data {
            TableData::Axisymmetric(d) => d.len(),
            TableData::General(d) => d.len(),
        }
    }
}

/// Static margins of [`AeroTable::static_margins`] [cal]; positive is stable.
#[derive(Debug, Clone, PartialEq)]
pub struct StaticMargins {
    pub pitch: f64,
    pub yaw: f64,
    /// Why a plane has no margin (NaN): its normal-force slope is zero, or it has the unphysical sign
    /// together with a restoring moment. None when both planes have a margin.
    pub unknown: Option<String>,
}

impl StaticMargins {
    /// The smaller of the two margins; NaN when either is unknown.
    pub fn min(&self) -> f64 {
        if self.pitch.is_nan() || self.yaw.is_nan() {
            f64::NAN
        } else {
            self.pitch.min(self.yaw)
        }
    }
}

/// `v` limited to the sign of the reference value `edge` (zero when it would flip).
fn keep_sign(v: f64, edge: f64) -> f64 {
    if edge >= 0.0 {
        v.max(0.0)
    } else {
        v.min(0.0)
    }
}

/// Whether `x` lies outside the angle grid `g` (by more than 1e-9) on either side; with a single
/// node every other value is outside (it is held at the node).
fn outside(g: &[f64], x: f64) -> bool {
    g.first().is_some_and(|&lo| x < lo - 1e-9) || g.last().is_some_and(|&hi| x > hi + 1e-9)
}

/// Cell index and fractional position; the fraction leaves [0, 1] when extrapolating.
fn locate(g: &[f64], x: f64, clamp: bool) -> (usize, f64) {
    if g.len() < 2 {
        return (0, 0.0);
    }
    let i = match g.iter().position(|&v| v > x) {
        Some(0) => 0,
        Some(k) => k - 1,
        None => g.len() - 2,
    }
    .min(g.len() - 2);
    let t = (x - g[i]) / (g[i + 1] - g[i]);
    (i, if clamp { t.clamp(0.0, 1.0) } else { t })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::AeroOptions;
    use crate::test_tmp::TempDir;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn table(ext: Extrapolation) -> (AeroTable, AeroModel) {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let opt = AeroOptions { mach_max: 2.0, mach_step: 0.1, alpha_max_deg: 10.0, alpha_step_deg: 2.0, ..Default::default() };
        let model = AeroModel::new(g, opt);
        (AeroTable::build(&model, "h".into(), ext), model)
    }

    #[test]
    fn interpolates_on_and_between_nodes() {
        let (t, m) = table(Extrapolation::Linear);
        let c = t.lookup(0.5, 4f64.to_radians());
        assert!((c.cn - m.normal(0.5, 4f64.to_radians()).0).abs() < 1e-9);
        let mid = t.lookup(0.55, 5f64.to_radians());
        let lo = t.lookup(0.5, 4f64.to_radians()).cn;
        let hi = t.lookup(0.6, 6f64.to_radians()).cn;
        assert!(mid.cn > lo && mid.cn < hi);
    }

    #[test]
    fn extrapolates_linearly_or_clamps() {
        let (t, _) = table(Extrapolation::Linear);
        let a8 = t.lookup(0.3, 8f64.to_radians()).cn;
        let a10 = t.lookup(0.3, 10f64.to_radians()).cn;
        let a12 = t.lookup(0.3, 12f64.to_radians()).cn;
        assert!((a12 - (2.0 * a10 - a8)).abs() < 1e-9);
        let (tc, _) = table(Extrapolation::Clamp);
        assert!((tc.lookup(0.3, 12f64.to_radians()).cn - a10).abs() < 1e-9);
        assert!((tc.lookup(5.0, 0.0).ca_off - tc.lookup(2.0, 0.0).ca_off).abs() < 1e-12);
    }

    #[test]
    fn mach_is_clamped_even_with_linear_extrapolation() {
        let (t, _) = table(Extrapolation::Linear);
        let g = t.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        t.reset_out_of_range_stats();
        for a in [0.0, 3f64.to_radians(), 9f64.to_radians()] {
            let (hi, edge) = (t.lookup(6.0, a), t.lookup(2.0, a));
            assert_eq!((hi.cn, hi.ca_on, hi.ca_off, hi.xcp, hi.cna, hi.clp), (edge.cn, edge.ca_on, edge.ca_off, edge.xcp, edge.cna, edge.clp));
        }
        assert_eq!(g.lookup6(9.0, 0.05, 0.02), g.lookup6(2.0, 0.05, 0.02));
        let s = t.out_of_range_stats();
        assert_eq!(s.mach_above, 3);
        assert_eq!(s.lookups, 6);
        assert!(s.max_mach >= 6.0 && s.any());
        assert_eq!(g.out_of_range_stats().mach_above, 1);
    }

    #[test]
    fn extrapolated_alpha_is_bounded_and_counted() {
        let (t, _) = table(Extrapolation::Linear);
        t.reset_out_of_range_stats();
        for deg in [10.0, 15.0, 40.0, 90.0, 170.0] {
            let c = t.lookup(0.8, f64::to_radians(deg));
            assert!(c.clp <= 0.0 && c.cna >= 0.0 && c.cn >= 0.0 && c.ca_on >= 0.0);
            assert!(c.damp.iter().all(|d| *d >= 0.0));
            assert!(c.xcp >= 0.0 && c.xcp <= t.meta.length);
        }
        assert_eq!(t.out_of_range_stats().angle_beyond, 4);
        assert!((t.out_of_range_stats().max_abs_alpha_deg - 170.0).abs() < 1e-9);
    }

    #[test]
    fn clp_never_positive_and_cl_delta_keeps_sign() {
        let meta = TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.0, machs: vec![0.0, 1.0], alphas_deg: vec![0.0, 10.0], ..Default::default() };
        // Roll damping grows towards positive and cl_delta towards negative with alpha.
        let t = AeroTable::from_fn(meta, |_, a| AeroCoeffs {
            cn: a,
            xcp: 0.5,
            cna: 1.0,
            clp: -0.1 + a * 0.05,
            cl_delta: 0.5 - a * 0.05,
            damp: [1.0, 0.5, 0.3],
            ..Default::default()
        });
        for deg in [5.0, 10.0, 20.0, 60.0] {
            let c = t.lookup(0.5, f64::to_radians(deg));
            assert!(c.clp <= 0.0, "clp {} at {deg}", c.clp);
            assert!(c.cl_delta >= 0.0, "cl_delta {} at {deg}", c.cl_delta);
        }
    }

    #[test]
    fn save_load_roundtrip() {
        let (t, _) = table(Extrapolation::Linear);
        let td = TempDir::new("aero_test");
        let p = td.join("aero_test.csv");
        t.save(&p).unwrap();
        let u = AeroTable::load(&p).unwrap();
        let (a, b) = (t.lookup(1.23, 0.07), u.lookup(1.23, 0.07));
        assert!((a.cn - b.cn).abs() < 1e-7 && (a.xcp - b.xcp).abs() < 1e-7);
        assert_eq!(u.meta.source_hash, "h");
    }

    #[test]
    fn save_leaves_no_temporary_files_and_replaces_atomically() {
        let (t, _) = table(Extrapolation::Linear);
        let dir = TempDir::new("aero_atomic");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        std::fs::write(&p, "stale").unwrap();
        t.save(&p).unwrap();
        t.save(&p).unwrap();
        let mut names: Vec<_> = std::fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        names.sort();
        assert_eq!(names, ["t.csv", "t.json"]);
        AeroTable::load(&p).unwrap();
        // A failing save (missing directory) reports an error and creates nothing.
        assert!(t.save(&dir.join("no_such_dir").join("t.csv")).is_err());
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 2);
    }

    #[test]
    fn load_rejects_a_reordered_or_renamed_header() {
        let (t, _) = table(Extrapolation::Linear);
        let dir = TempDir::new("aero_hdr");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        t.save(&p).unwrap();
        AeroTable::load(&p).unwrap();
        let text = std::fs::read_to_string(&p).unwrap();
        let (first, rest) = text.split_once('\n').unwrap();
        let swapped = first.replace("cn,ca_on,ca_off", "ca_on,cn,ca_off");
        assert_ne!(swapped, first);
        std::fs::write(&p, format!("{swapped}\n{rest}")).unwrap();
        let e = AeroTable::load(&p).unwrap_err().to_string();
        assert!(e.contains("unexpected CSV header") && e.contains("expected `mach,alpha_deg,cn,ca_on"), "{e}");
        // Case and surrounding blanks are tolerated.
        std::fs::write(&p, format!("{}\n{rest}", first.to_uppercase().replace(',', " , "))).unwrap();
        AeroTable::load(&p).unwrap();
    }

    #[test]
    fn loads_version_1_files_without_roll_terms() {
        let (t, _) = table(Extrapolation::Linear);
        let dir = TempDir::new("aero_v1");
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        // Write a version-1 file: 8 value columns and a sidecar without version or kind.
        let mut csv = String::from("mach,alpha_deg,cn,ca_on,ca_off,xcp,cna,damp_s0,damp_s1,damp_s2\n");
        let na = t.meta.alphas_deg.len();
        for i in 0..t.rows() {
            let (m, a) = (t.meta.machs[i / na], t.meta.alphas_deg[i % na]);
            let c = t.lookup(m, a.to_radians());
            csv += &format!("{m},{a},{},{},{},{},{},{},{},{}\n", c.cn, c.ca_on, c.ca_off, c.xcp, c.cna, c.damp[0], c.damp[1], c.damp[2]);
        }
        std::fs::write(&p, csv).unwrap();
        let mut j: serde_json::Value = serde_json::to_value(&t.meta).unwrap();
        for k in ["format_version", "kind", "betas_deg", "fin_cant_ref_deg", "moment_ref"] {
            j.as_object_mut().unwrap().remove(k);
        }
        std::fs::write(p.with_extension("json"), j.to_string()).unwrap();
        let u = AeroTable::load(&p).unwrap();
        assert_eq!(u.kind(), TableKind::Axisymmetric);
        assert!(!u.has_roll_terms());
        assert!((u.lookup(0.7, 0.05).cn - t.lookup(0.7, 0.05).cn).abs() < 1e-9);
    }

    /// Writes `t` (axisymmetric) with only the first `n_values` value columns of its CSV and checks
    /// the loaded table: the kept columns equal, the dropped ones zero.
    fn load_truncated(t: &AeroTable, tag: &str, n_values: usize) -> AeroTable {
        let dir = TempDir::new(&format!("aero_cols_{tag}"));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        t.save(&p).unwrap();
        let nc = if t.kind() == TableKind::General { 3 } else { 2 };
        let text = std::fs::read_to_string(&p).unwrap();
        let cut: String = text.lines().map(|l| l.split(',').take(nc + n_values).collect::<Vec<_>>().join(",") + "\n").collect();
        std::fs::write(&p, cut).unwrap();
        AeroTable::load(&p).unwrap()
    }

    #[test]
    fn older_column_sets_load_with_zero_lag_and_magnus_columns() {
        let (mut t, model) = table(Extrapolation::Linear);
        // The Barrowman build fills the lag sums from the geometry; no induced roll, no Magnus.
        let (l0, l1) = model.lag_sums(0.5);
        let c = t.lookup(0.5, 0.03);
        assert!(l0 > 0.0 && (c.lag_l0 - l0).abs() < 1e-12 && (c.lag_l1 - l1).abs() < 1e-12);
        assert_eq!((c.cl4, c.cnpa, c.cmpa), (0.0, 0.0, 0.0));
        assert!(t.has_lag_terms() && t.has_lag_columns() && !t.has_magnus_terms());
        assert_eq!(t.meta.fin_count, Some(4));
        // Version 4 round trip with induced roll and Magnus columns.
        let meta = t.meta.clone();
        let full = AeroTable::from_fn(meta, |m, a| AeroCoeffs { cl4: 0.01 * (1.0 + a), cnpa: -0.5 - m, cmpa: 0.2 + m, ..t.lookup(m, a.to_radians()) });
        assert!(full.has_magnus_terms());
        let u = load_truncated(&full, "v4", N_FIELDS);
        let (a, b) = (full.lookup(0.73, 0.05), u.lookup(0.73, 0.05));
        assert!((a.cl4 - b.cl4).abs() < 1e-12 && (a.cnpa - b.cnpa).abs() < 1e-12 && (a.cmpa - b.cmpa).abs() < 1e-12 && (a.lag_l1 - b.lag_l1).abs() < 1e-12);
        assert!(u.has_lag_columns() && u.meta.fin_count == Some(4));
        // Version 3 (14 value columns): no Magnus moment column.
        let u = load_truncated(&full, "v3", N_FIELDS_V3);
        let c = u.lookup(0.73, 0.05);
        assert!(u.has_lag_columns() && c.cmpa == 0.0 && (c.cnpa - a.cnpa).abs() < 1e-12);
        // Version 2 (10 value columns) and version 1 (8): the missing columns are zero.
        for (n, roll) in [(N_FIELDS_V2, true), (N_FIELDS_V1, false)] {
            let u = load_truncated(&full, &format!("v{n}"), n);
            let c = u.lookup(0.73, 0.05);
            assert_eq!((c.lag_l0, c.lag_l1, c.cl4, c.cnpa), (0.0, 0.0, 0.0, 0.0), "{n} columns");
            assert!(!u.has_lag_columns() && !u.has_lag_terms());
            assert_eq!(u.has_roll_terms(), roll);
            assert!((c.cn - full.lookup(0.73, 0.05).cn).abs() < 1e-9);
        }
        // General tables: 15 value columns load without lag and Magnus; set_lag_terms fills them.
        let g = full.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        let c6 = g.lookup6(0.5, 0.04, -0.02);
        assert!((c6.lag_l0 - l0).abs() < 1e-12 && (c6.lag_l1 - l1).abs() < 1e-12 && c6.cnpa < 0.0);
        let mut v = load_truncated(&g, "gen15", N_GEN_V1);
        assert!(!v.has_lag_columns() && !v.has_lag_terms());
        assert_eq!(v.lookup6(0.5, 0.04, -0.02).cnpa, 0.0);
        v.set_lag_terms(|m| (1.0 + m, 2.0 * m));
        let c = v.lookup6(0.5, 0.04, -0.02);
        assert!((c.lag_l0 - 1.5).abs() < 1e-12 && (c.lag_l1 - 1.0).abs() < 1e-12);
        let w = load_truncated(&g, "gen18", N_GEN_V2);
        assert!(w.has_lag_columns() && (w.lookup6(0.5, 0.04, -0.02).cnpa - c6.cnpa).abs() < 1e-12 && w.lookup6(0.5, 0.04, -0.02).cmpa == 0.0);
        let w = load_truncated(&g, "gen19", N_GEN);
        assert!(w.has_lag_columns() && (w.lookup6(0.5, 0.04, -0.02).cmpa - c6.cmpa).abs() < 1e-12 && c6.cmpa > 0.0);
        // set_lag_terms on an axisymmetric table.
        t.set_lag_terms(|_| (0.25, 0.5));
        let c = t.lookup(1.1, 0.2);
        assert_eq!((c.lag_l0, c.lag_l1), (0.25, 0.5));
    }

    #[test]
    fn induced_roll_applies_to_four_fins_only() {
        let meta = TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.0, machs: vec![0.0, 1.0], alphas_deg: vec![0.0, 30.0], ..Default::default() };
        let mk = |n: Option<usize>| {
            let mut t = AeroTable::from_fn(meta.clone(), |_, a| AeroCoeffs { cn: a.to_radians(), cna: 1.0, xcp: 0.5, cl4: 0.3, ..Default::default() });
            t.meta.fin_count = n;
            t
        };
        let (al, be) = (0.1, 0.05);
        assert!(mk(Some(4)).lookup6(0.5, al, be).cl != 0.0);
        assert!(mk(None).lookup6(0.5, al, be).cl != 0.0);
        assert_eq!(mk(Some(3)).lookup6(0.5, al, be).cl, 0.0);
        let g = mk(Some(3)).to_general(vec![-10.0, 0.0, 10.0], vec![-10.0, 0.0, 10.0]);
        assert_eq!(g.lookup6(0.5, al, be).cl, 0.0);
    }

    #[test]
    fn induced_roll_follows_the_crossflow_roll_angle_from_the_fin_plane() {
        // Crossflow in a fin plane: zero; 22.5 deg from it: the full cl4 alpha^2.
        let a = 0.2;
        assert!(induced_roll(0.3, a, 1.0, 0.0, 0.0).abs() < 1e-15);
        let p = 22.5f64.to_radians();
        assert!((induced_roll(0.3, a, p.cos(), p.sin(), 0.0) - 0.3 * a * a).abs() < 1e-12);
        // Fins at 45 deg (x configuration): the same crossflow is now 22.5 deg on the other side.
        assert!((induced_roll(0.3, a, p.cos(), p.sin(), 45f64.to_radians()) + 0.3 * a * a).abs() < 1e-12);
        // The expansion carries it (reversed in tail-first flow, as the cant forcing).
        let c = AeroCoeffs { cl4: 0.3, ..Default::default() };
        let (al, be) = (0.1f64, 0.05f64);
        let [_, v, w] = airspeed_direction(al, be);
        let total = v.hypot(w).atan2(airspeed_direction(al, be)[0]);
        let e = expand_axisymmetric(&c, al, be, 0.1, 0.0, 0.0);
        assert!((e.cl - induced_roll(0.3, total, v, w, 0.0)).abs() < 1e-15 && e.cl != 0.0);
        let back = expand_axisymmetric(&c, std::f64::consts::PI - al, be, 0.1, 0.0, 0.0);
        let [_, v2, w2] = airspeed_direction(std::f64::consts::PI - al, be);
        let total2 = v2.hypot(w2).atan2(airspeed_direction(std::f64::consts::PI - al, be)[0]);
        let eff2 = total2.min(std::f64::consts::PI - total2);
        assert!((back.cl + induced_roll(0.3, eff2, v2, w2, 0.0)).abs() < 1e-15);
    }

    #[test]
    fn general_table_reproduces_axisymmetric_expansion() {
        let (t, _) = table(Extrapolation::Linear);
        let g = t.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        assert_eq!(g.kind(), TableKind::General);
        // On grid nodes the trilinear lookup equals the expansion.
        let (m, a, b) = (0.5, 4f64.to_radians(), 0.0);
        let c6 = g.lookup6(m, a, b);
        let c = t.lookup(m, a);
        assert!((c6.cz + c.cn).abs() < 1e-9, "CZ = -CN for pure pitch");
        assert!(c6.cy.abs() < 1e-12);
        assert!((c6.cm - c.xcp * c6.cz / t.meta.ref_diameter).abs() < 1e-9);
        assert!((c6.cx_off + c.ca_off).abs() < 1e-9);
        // Sideslip: CY odd in beta, Cn = -xcp CY / d; alpha: CZ odd.
        let p = g.lookup6(0.5, 0.0, 4f64.to_radians());
        let q = g.lookup6(0.5, 0.0, -4f64.to_radians());
        assert!((p.cy + q.cy).abs() < 1e-9 && p.cy < 0.0);
        assert!((p.cn + c.xcp * p.cy / t.meta.ref_diameter).abs() < 1e-9);
        let r = g.lookup6(0.5, -4f64.to_radians(), 0.0);
        assert!((r.cz + c6.cz).abs() < 1e-9);
        // Backward flight (alpha = 180 deg): the axial force reverses.
        let back = g.lookup6(0.5, std::f64::consts::PI, 0.0);
        assert!((back.cx_off - c.ca_off.max(0.0) * 0.0 - t.lookup(0.5, 0.0).ca_off).abs() < 1e-9);
        // Save / load round trip of a general table.
        let p_td = TempDir::new("aero_gen");
        let p = p_td.join("aero_gen.csv");
        g.save(&p).unwrap();
        let u = AeroTable::load(&p).unwrap();
        let (x, y) = (u.lookup6(0.73, 0.1, -0.05), g.lookup6(0.73, 0.1, -0.05));
        for (a, b) in x.to_arr().iter().zip(y.to_arr()) {
            assert!((a - b).abs() <= 1e-8 * b.abs().max(1.0));
        }
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(p.with_extension("json"));
    }

    #[test]
    fn static_margins_of_an_axisymmetric_expansion_equal_the_axisymmetric_margin() {
        // Centre of pressure independent of alpha: the expansion gives the same margin exactly.
        let meta = TableMeta {
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.5,
            machs: vec![0.0, 0.5, 1.0],
            alphas_deg: vec![0.0, 2.0, 4.0, 10.0],
            ..Default::default()
        };
        let t = AeroTable::from_fn(meta, |m, a| AeroCoeffs {
            cn: 0.2 * a * (1.0 + m),
            cna: 0.2f64.to_degrees() * (1.0 + m),
            xcp: 1.1 - 0.1 * m,
            ca_on: 0.5,
            ca_off: 0.6,
            ..Default::default()
        });
        let g = t.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        t.reset_out_of_range_stats();
        g.reset_out_of_range_stats();
        // On and between the Mach nodes (and beyond the grid, where Mach is held): both interpolate
        // the normal force and the nose moment, not their ratio x_cp.
        for (m, x_cg) in [(0.0, 0.8), (0.5, 0.9), (1.0, 1.2), (1.4, 0.7), (0.25, 0.9), (0.8, 1.0)] {
            let StaticMargins { pitch: ap, yaw: ay, .. } = t.static_margins(m, x_cg);
            let StaticMargins { pitch: gp, yaw: gy, .. } = g.static_margins(m, x_cg);
            let before = (t.lookup(m, 0.0).xcp - x_cg) / 0.1;
            assert!((ap - before).abs() < 1e-12 && (ay - before).abs() < 1e-12, "{ap} {ay} {before}");
            assert!((gp - before).abs() < 1e-9 && (gy - before).abs() < 1e-9, "{gp} {gy} {before}");
        }
        // The margins are not counted in the range statistics (only the explicit lookups are).
        assert_eq!(g.out_of_range_stats().lookups, 0);
        assert_eq!(t.out_of_range_stats().lookups, 6);
        // The sample rocket, whose centre of pressure varies with alpha: the slopes over the first
        // 2 deg cell of the expanded grid give the centre of pressure at 2 deg.
        let (s, _) = table(Extrapolation::Linear);
        let sg = s.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        let before = (s.lookup(0.5, 2f64.to_radians()).xcp - 0.9) / s.meta.ref_diameter;
        let StaticMargins { pitch: p, yaw: y, .. } = sg.static_margins(0.5, 0.9);
        assert!((p - y).abs() < 1e-9 && (p - before).abs() < 1e-9, "{p} {y} {before}");
    }

    #[test]
    fn static_margins_separate_pitch_and_yaw() {
        // Pitch centre of pressure 1.2 m, yaw 0.6 m aft of the nose; CG at 0.9 m, d = 0.1 m.
        let meta = TableMeta {
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.5,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![-10.0, -1.0, 0.0, 1.0, 10.0],
            betas_deg: vec![-10.0, 0.0, 10.0],
            ..Default::default()
        };
        let g = AeroTable::general_from_fn(meta, |_, a, b| {
            let (cz, cy) = (-0.2 * a, -0.15 * b);
            Coeffs6 { cx_on: -0.5, cx_off: -0.6, cz, cy, cm: 1.2 * cz / 0.1, cn: -0.6 * cy / 0.1, ..Default::default() }
        });
        let StaticMargins { pitch: p, yaw: y, .. } = g.static_margins(0.5, 0.9);
        assert!((p - 3.0).abs() < 1e-9 && (y + 3.0).abs() < 1e-9, "{p} {y}");
    }

    #[test]
    fn rows_above_90_deg_are_not_read_by_the_flight_lookups() {
        // A table built up to 180 deg holds reverse-flow rows (fin CP mirrored towards the trailing edge),
        // but the six-component lookup folds the total angle to min(alpha, 180 deg - alpha): tail-first
        // flow uses the nose-first coefficients.
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let opt = AeroOptions { mach_max: 1.0, mach_step: 0.5, alpha_max_deg: 180.0, alpha_step_deg: 5.0, ..Default::default() };
        let model = AeroModel::new(g, opt);
        let t = AeroTable::build(&model, String::new(), Extrapolation::Clamp);
        let d = t.meta.ref_diameter;
        for deg in [10.0f64, 30.0, 60.0] {
            let (a, r) = (deg.to_radians(), (180.0 - deg).to_radians());
            assert_ne!(t.lookup(0.5, r).xcp, t.lookup(0.5, a).xcp, "the reverse-flow rows differ");
            let back = t.lookup6(0.5, r, 0.0);
            assert!((back.cz - t.lookup6(0.5, a, 0.0).cz).abs() < 1e-12);
            assert!((back.cm * d / back.cz - t.lookup(0.5, a).xcp).abs() < 1e-12, "{deg} deg");
        }
    }

    #[test]
    fn general_extrapolation_keeps_the_sign_of_the_roll_forcing() {
        // Roll forcing falling with alpha: positive on the grid, it would cross zero beyond it.
        let meta = TableMeta {
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.5,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![0.0, 10.0, 20.0],
            betas_deg: vec![-10.0, 0.0, 10.0],
            extrapolation: Extrapolation::Linear,
            ..Default::default()
        };
        let g = AeroTable::general_from_fn(meta, |_, a, _| Coeffs6 {
            cx_on: -0.5,
            cx_off: -0.6,
            cl: 0.02 - 0.0009 * a,
            cl_delta: 0.3 - 0.01 * a,
            ..Default::default()
        });
        let edge = g.lookup6(0.5, 20f64.to_radians(), 0.0);
        assert!((edge.cl - 0.002).abs() < 1e-12);
        let c = g.lookup6(0.5, 40f64.to_radians(), 0.0);
        assert_eq!((c.cl, c.cl_delta), (0.0, 0.0), "{c:?}");
        // Inside the grid the values are unchanged.
        assert!((g.lookup6(0.5, 15f64.to_radians(), 0.0).cl - (0.02 - 0.0009 * 15.0)).abs() < 1e-12);
    }

    #[test]
    fn general_extrapolation_keeps_the_sign_of_the_transverse_forces_and_moments() {
        // Transverse forces and moments shrinking towards the grid edge: linear extrapolation beyond
        // it would flip their sign.
        let meta = TableMeta {
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.5,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![0.0, 10.0, 20.0],
            betas_deg: vec![0.0, 10.0, 20.0],
            extrapolation: Extrapolation::Linear,
            ..Default::default()
        };
        let g = AeroTable::general_from_fn(meta, |_, a, b| Coeffs6 {
            cx_on: -0.5,
            cx_off: -0.6,
            cy: -(2.0 - 0.09 * b),
            cz: -(2.0 - 0.09 * a),
            cm: 1.0 - 0.045 * a,
            cn: 1.0 - 0.045 * b,
            ..Default::default()
        });
        let c = g.lookup6(0.5, 40f64.to_radians(), 40f64.to_radians());
        assert_eq!((c.cy, c.cz, c.cm, c.cn), (0.0, 0.0, 0.0, 0.0), "{c:?}");
        // Short of the sign change the linear extrapolation is kept.
        let c = g.lookup6(0.5, 22f64.to_radians(), 22f64.to_radians());
        assert!((c.cz + 2.0 - 0.09 * 22.0).abs() < 1e-12 && (c.cm - (1.0 - 0.045 * 22.0)).abs() < 1e-12, "{c:?}");
    }

    /// General table with linear Cz = kz alpha, Cm = km alpha, Cy = ky beta, Cn = kn beta (d = 0.1 m).
    fn linear_table(kz: f64, km: f64, ky: f64, kn: f64) -> AeroTable {
        let meta = TableMeta {
            ref_area: 0.01,
            ref_diameter: 0.1,
            length: 1.5,
            machs: vec![0.0, 1.0],
            alphas_deg: vec![-10.0, -1.0, 0.0, 1.0, 10.0],
            betas_deg: vec![-10.0, -1.0, 0.0, 1.0, 10.0],
            ..Default::default()
        };
        AeroTable::general_from_fn(meta, move |_, a, b| {
            let (a, b) = (a.to_radians(), b.to_radians());
            Coeffs6 { cx_on: -0.5, cx_off: -0.6, cz: kz * a, cm: km * a, cy: ky * b, cn: kn * b, ..Default::default() }
        })
    }

    #[test]
    fn static_margins_follow_the_signed_cg_moment_derivative() {
        // Cz = alpha (normal-force slope with the unphysical sign), Cm = 12 alpha, x_cg / d = 9:
        // dCm_cg/dalpha = 12 - 9 = +3 is destabilising, so the pitch plane is unstable.
        let t = linear_table(1.0, 12.0, -1.0, 12.0);
        let m = t.static_margins(0.5, 0.9);
        assert!(m.pitch < 0.0 && (m.pitch + 3.0).abs() < 1e-9, "{m:?}");
        // Yaw with the same reversed force: Cy = beta, Cn = -12 beta: dCn_cg/dbeta = -12 + 9 = -3 < 0 destabilising.
        let t = linear_table(-1.0, -12.0, 1.0, -12.0);
        let m = t.static_margins(0.5, 0.9);
        assert!((m.pitch - 3.0).abs() < 1e-9 && (m.yaw + 3.0).abs() < 1e-9, "{m:?}");
        assert!(m.unknown.is_none());
        // Reversed force with a restoring moment (Cz = alpha, Cm = 6 alpha: dCm_cg/dalpha = -3): no margin in
        // calibers can be given; unknown, with the reason.
        let t = linear_table(1.0, 6.0, -1.0, 12.0);
        let m = t.static_margins(0.5, 0.9);
        assert!(m.pitch.is_nan() && (m.yaw - 3.0).abs() < 1e-9, "{m:?}");
        let why = m.unknown.unwrap();
        assert!(why.contains("pitch normal-force slope dCz/dalpha is positive at Mach 0.50"), "{why}");
        // No normal-force slope in yaw.
        let t = linear_table(-1.0, -12.0, 0.0, 0.0);
        let m = t.static_margins(0.5, 0.9);
        assert!((m.pitch - 3.0).abs() < 1e-9 && m.yaw.is_nan(), "{m:?}");
        assert!(m.unknown.unwrap().contains("yaw normal-force slope dCy/dbeta is zero"));
        // Physical signs: the margin is x_cp / d - x_cg / d in both planes, negative when x_cp is ahead of the CG.
        let t = linear_table(-2.0, -2.0 * 6.0, -2.0, 2.0 * 12.0);
        let m = t.static_margins(0.5, 0.9);
        assert!((m.pitch + 3.0).abs() < 1e-9 && (m.yaw - 3.0).abs() < 1e-9, "{m:?}");
    }

    #[test]
    fn aero_angles_match_the_direction_vector() {
        for (a, b) in [(0.3, -0.2), (2.9, 0.4), (-2.5, -1.2)] {
            let [u, v, w] = airspeed_direction(a, b);
            let (a2, b2) = aero_angles(3.0 * u, 3.0 * v, 3.0 * w);
            assert!((a - a2).abs() < 1e-12 && (b - b2).abs() < 1e-12);
        }
        let ax = angle_axis(180.0, 30.0, 2.0, 10.0);
        assert_eq!(ax.first(), Some(&-180.0));
        assert_eq!(ax.last(), Some(&180.0));
        assert!(ax.windows(2).all(|w| w[1] > w[0]));
        assert!(ax.contains(&0.0) && ax.contains(&30.0) && ax.contains(&40.0));
    }

    fn loadable_files(tag: &str) -> (TempDir, std::path::PathBuf, AeroTable) {
        let (t, _) = table(Extrapolation::Linear);
        let dir = TempDir::new(&format!("aero_{tag}"));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.csv");
        t.save(&p).unwrap();
        (dir, p, t)
    }

    fn load_err(p: &Path) -> String {
        format!("{:#}", AeroTable::load(p).unwrap_err())
    }

    #[test]
    fn load_rejects_reordered_or_mismatched_rows() {
        let (_dir, p, _) = loadable_files("order");
        let text = std::fs::read_to_string(&p).unwrap();
        let mut lines: Vec<&str> = text.lines().collect();
        let header = lines.remove(0);
        lines.reverse();
        std::fs::write(&p, format!("{header}\n{}\n", lines.join("\n"))).unwrap();
        let e = load_err(&p);
        assert!(e.contains("row 2") && e.contains("do not match the grid point"), "{e}");
        // a shifted coordinate
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let mut cells: Vec<String> = lines[2].split(',').map(String::from).collect();
        cells[0] = "0.123".into();
        lines[2] = cells.join(",");
        std::fs::write(&p, lines.join("\n") + "\n").unwrap();
        assert!(load_err(&p).contains("row 3"));
        // too few rows
        let short: String = text.lines().take(3).map(|l| format!("{l}\n")).collect();
        std::fs::write(&p, short).unwrap();
        assert!(load_err(&p).contains("rows but its metadata declares"));
    }

    #[test]
    fn load_rejects_non_finite_cells() {
        let (_dir, p, _) = loadable_files("nan");
        let text = std::fs::read_to_string(&p).unwrap();
        let mut lines: Vec<String> = text.lines().map(String::from).collect();
        let mut cells: Vec<String> = lines[3].split(',').map(String::from).collect();
        cells[4] = "NaN".into();
        lines[3] = cells.join(",");
        std::fs::write(&p, lines.join("\n") + "\n").unwrap();
        let e = load_err(&p);
        assert!(e.contains("row 4") && e.contains("column 5") && e.contains("not finite"), "{e}");
    }

    #[test]
    fn load_validates_metadata_and_names_missing_files() {
        let (_dir, p, _) = loadable_files("meta");
        let jp = p.with_extension("json");
        let good: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&jp).unwrap()).unwrap();
        let cases: [(&str, serde_json::Value, &str); 6] = [
            ("ref_area", serde_json::json!(0.0), "ref_area"),
            ("ref_diameter", serde_json::json!(-1.0), "ref_diameter"),
            ("machs", serde_json::json!([]), "machs axis is empty"),
            ("alphas_deg", serde_json::json!([0.0, 2.0, 2.0]), "strictly increasing"),
            ("machs", serde_json::json!([0.5, 0.2]), "strictly increasing"),
            ("moment_ref", serde_json::json!("cg"), "moment_ref"),
        ];
        for (k, v, want) in cases {
            let mut j = good.clone();
            j[k] = v;
            std::fs::write(&jp, j.to_string()).unwrap();
            let e = load_err(&p);
            assert!(e.contains(want), "{k}: {e}");
            assert!(e.contains("t.json"), "{e}");
        }
        std::fs::remove_file(&jp).unwrap();
        assert!(load_err(&p).contains("t.json"));
        std::fs::remove_file(&p).unwrap();
        std::fs::write(&jp, good.to_string()).unwrap();
        assert!(load_err(&p).contains("t.csv"));
    }

    #[test]
    fn closed_finless_body_has_a_finite_zero_angle_centre_of_pressure() {
        // A finless body of revolution closing to a point at the tail: the body normal-force slope
        // sums to zero, the zero-angle centre of pressure s1 / s0 is undefined.
        let mut g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        g.fins = None;
        g.base_radius = 0.0;
        let model = AeroModel::new(g, AeroOptions::default());
        assert!(model.damping_sums(0.5)[0].abs() < 1e-12);
        let t = AeroTable::build(&model, String::new(), Extrapolation::Linear);
        let TableData::Axisymmetric(d) = &t.data else { unreachable!() };
        assert!(d.iter().all(|r| r.iter().all(|x| x.is_finite())), "non-finite cell");
        assert_eq!(t.lookup(0.5, 0.0).xcp, model.body_xcp());
    }

    #[test]
    fn load_rejects_infinite_coordinates() {
        let (_dir, p, _) = loadable_files("infcoord");
        let text = std::fs::read_to_string(&p).unwrap();
        for col in [0, 1] {
            let mut lines: Vec<String> = text.lines().map(String::from).collect();
            let mut cells: Vec<String> = lines[3].split(',').map(String::from).collect();
            cells[col] = "inf".into();
            lines[3] = cells.join(",");
            std::fs::write(&p, lines.join("\n") + "\n").unwrap();
            let e = load_err(&p);
            assert!(e.contains("row 4") && e.contains(&format!("column {}", col + 1)) && e.contains("not finite"), "{e}");
        }
    }

    #[test]
    fn load_requires_angle_grids_through_zero() {
        let (_dir, p, t) = loadable_files("zero");
        let jp = p.with_extension("json");
        let mut j: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&jp).unwrap()).unwrap();
        let mut shifted = t.meta.alphas_deg.clone();
        shifted[0] = 0.25 * shifted[1];
        j["alphas_deg"] = serde_json::json!(shifted);
        std::fs::write(&jp, j.to_string()).unwrap();
        let e = load_err(&p);
        assert!(e.contains("alphas_deg of an axisymmetric table must start at 0 deg") && e.contains("t.json"), "{e}");
        // General tables: both axes must hold 0 between their ends.
        let g = t.to_general(vec![-10.0, 0.0, 10.0], vec![-10.0, 0.0, 10.0]);
        for (key, axis) in [("alphas_deg", vec![2.0, 10.0, 20.0]), ("betas_deg", vec![-20.0, -10.0, -2.0])] {
            let mut meta = g.meta.clone();
            if key == "alphas_deg" {
                meta.alphas_deg = axis;
            } else {
                meta.betas_deg = axis;
            }
            let e = format!("{:#}", validate_meta(&meta).unwrap_err());
            assert!(e.contains(&format!("{key} of a general table must include 0 deg")), "{e}");
        }
        assert!(validate_meta(&g.meta).is_ok());
    }

    #[test]
    fn lookups_below_the_first_angle_node_are_out_of_range() {
        // An axisymmetric table (built without the load validation) whose grid starts at 5 deg.
        let meta = TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.0, machs: vec![0.0, 1.0], alphas_deg: vec![5.0, 90.0], ..Default::default() };
        let t = AeroTable::from_fn(meta, |_, a| AeroCoeffs { cn: a, xcp: 0.5, ..Default::default() });
        t.reset_out_of_range_stats();
        t.lookup(0.5, 1f64.to_radians());
        t.lookup(0.5, 10f64.to_radians());
        assert_eq!(t.out_of_range_stats().angle_beyond, 1);
        // A general table with a single sideslip node: any other sideslip is held, i.e. out of range.
        let g = t.to_general(vec![0.0, 10.0], vec![0.0]);
        g.reset_out_of_range_stats();
        g.lookup6(0.5, 5f64.to_radians(), 0.0);
        assert_eq!(g.out_of_range_stats().angle_beyond, 0);
        g.lookup6(0.5, 5f64.to_radians(), 3f64.to_radians());
        assert_eq!(g.out_of_range_stats().angle_beyond, 1);
    }

    #[test]
    fn general_table_records_the_total_angle_of_attack() {
        let (t, _) = table(Extrapolation::Linear);
        let g = t.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        g.reset_out_of_range_stats();
        // Pure sideslip of 40 deg: the pitch-plane angle is 0, the total angle 40 deg.
        g.lookup6(0.5, 0.0, 40f64.to_radians());
        assert!((g.out_of_range_stats().max_abs_alpha_deg - 40.0).abs() < 1e-9);
        let (a, b) = (30f64.to_radians(), 30f64.to_radians());
        g.reset_out_of_range_stats();
        g.lookup6(0.5, a, b);
        let total = (a.cos() * b.cos()).acos().to_degrees();
        assert!((g.out_of_range_stats().max_abs_alpha_deg - total).abs() < 1e-9);
    }

    #[test]
    fn save_load_is_bit_identical() {
        let (_dir, p, t) = loadable_files("bits");
        let u = AeroTable::load(&p).unwrap();
        let (TableData::Axisymmetric(a), TableData::Axisymmetric(b)) = (&t.data, &u.data) else { panic!() };
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().flatten().zip(b.iter().flatten()) {
            assert_eq!(x.to_bits(), y.to_bits());
        }
        let g = t.to_general(angle_axis(180.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        g.save(&p).unwrap();
        let w = AeroTable::load(&p).unwrap();
        let (TableData::General(a), TableData::General(b)) = (&g.data, &w.data) else { panic!() };
        assert!(a.iter().flatten().zip(b.iter().flatten()).all(|(x, y)| x.to_bits() == y.to_bits()));
    }

    #[test]
    fn load_strips_a_utf8_bom_and_warns_about_negative_axial_force() {
        let (_dir, p, t) = loadable_files("bom");
        assert!(t.sanity_warnings().is_empty());
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<String> = text
            .lines()
            .map(|l| {
                let mut c: Vec<String> = l.split(',').map(String::from).collect();
                if c[0].parse::<f64>().is_ok_and(|m| (m - 0.3).abs() < 1e-9) && c[1].parse::<f64>().is_ok_and(|a| a == 0.0) {
                    c[4] = "-1".into();
                }
                c.join(",")
            })
            .collect();
        std::fs::write(&p, format!("\u{feff}{}\n", lines.join("\n"))).unwrap();
        let jp = p.with_extension("json");
        let json = std::fs::read_to_string(&jp).unwrap();
        std::fs::write(&jp, format!("\u{feff}{json}")).unwrap();
        let back = AeroTable::load(&p).unwrap();
        let w = back.sanity_warnings();
        assert_eq!(w, vec!["aero table: ca_off at Mach 0.3, alpha 0 is -1.00 (negative axial force / thrust-like drag)".to_string()]);
    }

    fn two_cell_table() -> AeroTable {
        // Zero-angle rows: CNa 1 and 3 /rad with x_cp 1 and 3 m; 10 deg: (CN, x_cp) = (1, 1) and (2, 1.5);
        // 20 deg: (3, 3) and (4, 2).
        let meta = TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 4.0, machs: vec![0.0, 1.0], alphas_deg: vec![0.0, 10.0, 20.0], ..Default::default() };
        AeroTable::from_fn(meta, |m, a| {
            let (cn, xcp, cna) = match (m == 0.0, a as i32) {
                (true, 0) => (0.0, 1.0, 1.0),
                (false, 0) => (0.0, 3.0, 3.0),
                (true, 10) => (1.0, 1.0, 1.0),
                (false, 10) => (2.0, 1.5, 3.0),
                (true, _) => (3.0, 3.0, 1.0),
                (false, _) => (4.0, 2.0, 3.0),
            };
            AeroCoeffs { cn, xcp, cna, ca_on: 0.5, ca_off: 0.6, ..Default::default() }
        })
    }

    #[test]
    fn axisymmetric_lookup_interpolates_the_nose_moment() {
        let t = two_cell_table();
        // Between (CN, x_cp) = (1, 1) and (3, 3): CN 2 and nose moment (1 + 9) / 2 = 5, so x_cp = 2.5.
        let c = t.lookup(0.0, 15f64.to_radians());
        assert!((c.cn - 2.0).abs() < 1e-12 && (c.cn * c.xcp - 5.0).abs() < 1e-12, "{c:?}");
        // Bilinear in Mach as well: CN = (1 + 2 + 3 + 4) / 4, moment = (1 + 3 + 9 + 8) / 4.
        let c = t.lookup(0.5, 15f64.to_radians());
        assert!((c.cn - 2.5).abs() < 1e-12 && (c.cn * c.xcp - 21.0 / 4.0).abs() < 1e-12, "{c:?}");
        // At zero angle the centre of pressure is the ratio of the interpolated slopes:
        // (1 * 1 + 3 * 3) / (1 + 3) = 2.5 at Mach 0.5.
        let c = t.lookup(0.5, 0.0);
        assert!((c.xcp - 2.5).abs() < 1e-12 && (c.cna - 2.0).abs() < 1e-12, "{c:?}");
        // The general expansion on the same grid gives the same moment off the nodes.
        let g = t.to_general(vec![-20.0, -10.0, 0.0, 10.0, 20.0], vec![-20.0, -10.0, 0.0, 10.0, 20.0]);
        for (m, a) in [(0.3, 4.0), (0.5, 15.0), (0.8, 13.0)] {
            let (c6, a6) = (g.lookup6(m, f64::to_radians(a), 0.0), t.lookup6(m, f64::to_radians(a), 0.0));
            assert!((c6.cz - a6.cz).abs() < 1e-12 && (c6.cm - a6.cm).abs() < 1e-12, "M {m} a {a}: {c6:?} vs {a6:?}");
        }
    }

    #[test]
    fn axisymmetric_static_margins_follow_the_sign_of_the_normal_force_slope() {
        let meta = TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.5, machs: vec![0.0, 1.0], alphas_deg: vec![0.0, 10.0], ..Default::default() };
        let mk = |k: f64| {
            AeroTable::from_fn(meta.clone(), move |_, a| AeroCoeffs { cn: k * a.to_radians(), cna: k, xcp: 1.2, ca_on: 0.5, ca_off: 0.6, ..Default::default() })
        };
        // Physical slope: (x_cp - x_cg) / d.
        let m = mk(2.0).static_margins(0.5, 0.9);
        assert!((m.pitch - 3.0).abs() < 1e-9 && (m.yaw - 3.0).abs() < 1e-9 && m.unknown.is_none(), "{m:?}");
        // Zero slope: unknown.
        let m = mk(0.0).static_margins(0.5, 0.9);
        assert!(m.pitch.is_nan() && m.yaw.is_nan(), "{m:?}");
        assert!(m.unknown.unwrap().contains("normal-force slope dCN/dalpha is zero at Mach 0.50"));
        // Negative slope with x_cp aft of the CG: the moment is destabilising, -3 cal.
        let m = mk(-2.0).static_margins(0.5, 0.9);
        assert!((m.pitch + 3.0).abs() < 1e-9 && (m.yaw + 3.0).abs() < 1e-9 && m.unknown.is_none(), "{m:?}");
        // Negative slope with x_cp ahead of the CG: restoring, but no margin can be given.
        let m = mk(-2.0).static_margins(0.5, 1.4);
        assert!(m.pitch.is_nan() && m.yaw.is_nan(), "{m:?}");
        assert!(m.unknown.unwrap().contains("normal-force slope dCN/dalpha is negative at Mach 0.50"));
    }

    #[test]
    fn axisymmetric_static_margins_use_the_centre_of_pressure_the_lookup_applies() {
        // Tabulated x_cp 1.2 m at alpha 0 but 0.8 m from the first nonzero row on; CG 1.0 m, d = 0.1 m.
        // CN(0) = 0, so the interpolated CN and CN x_cp put the applied centre of pressure of every
        // small angle at the first row's 0.8 m: -2 cal, not the +2 cal of the zero-angle row.
        let meta = TableMeta { ref_area: 0.01, ref_diameter: 0.1, length: 1.5, machs: vec![0.0, 1.0], alphas_deg: vec![0.0, 2.0, 10.0], ..Default::default() };
        let t = AeroTable::from_fn(meta, |m, a| AeroCoeffs {
            cn: 2.0 * (1.0 + m) * a.to_radians(),
            cna: 2.0 * (1.0 + m),
            xcp: if a == 0.0 { 1.2 } else { 0.8 + 0.1 * m },
            ca_on: 0.5,
            ca_off: 0.6,
            ..Default::default()
        });
        let m = t.static_margins(0.0, 1.0);
        assert!((m.pitch + 2.0).abs() < 1e-9 && (m.yaw + 2.0).abs() < 1e-9 && m.unknown.is_none(), "{m:?}");
        // Between the Mach nodes the margin is that of the Mach-interpolated force and moment, the
        // same as the centre of pressure the lookup applies at a small angle.
        for mach in [0.0, 0.3, 0.5, 1.0, 1.4] {
            let applied = (t.lookup(mach, 0.01f64.to_radians()).xcp - 1.0) / 0.1;
            let m = t.static_margins(mach, 1.0);
            assert!((m.pitch - applied).abs() < 1e-9 && (m.yaw - applied).abs() < 1e-9, "Mach {mach}: {m:?} vs {applied}");
        }
        // The general expansion of the table gives the same margins.
        let g = t.to_general(angle_axis(90.0, 30.0, 2.0, 10.0), angle_axis(90.0, 30.0, 2.0, 10.0));
        for mach in [0.0, 0.5, 1.0] {
            let (a, b) = (t.static_margins(mach, 1.0), g.static_margins(mach, 1.0));
            assert!((a.pitch - b.pitch).abs() < 1e-9 && (a.yaw - b.yaw).abs() < 1e-9, "Mach {mach}: {a:?} vs {b:?}");
        }
    }
}
