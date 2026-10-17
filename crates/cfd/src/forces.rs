// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2024-2026 西濱大将 (NISHIHAMA Daisuke)
//! Conversion of SU2 coefficients to IgnisYeet body-axis coefficients, hybrid viscous terms and
//! pitch-damping scaling.
//!
//! # Conventions
//!
//! The mesh is in the body frame (x aft from the nose tip, z up, see [`crate::su2cfg`]) and the
//! freestream is rotated, so SU2's `CFx`, `CFz`, `CMy` are body-axis quantities:
//!
//! * `CA = CFx` (positive aft), `CN = CFz` (positive for positive alpha).
//! * SU2 evaluates `M = r x F`; the y component is `z Fx - x Fz`. A normal force `Fz > 0`
//!   at `x = xcp` aft of the nose therefore gives `CMy = -xcp CN / L_ref`, i.e.
//!   `xcp = -CMy L_ref / CN`. The nose moment of the normal force is `mom = CN xcp = -CMy L_ref`.
//! * If only `CL`, `CD` are available they are rotated back: with the freestream direction
//!   `d = (cos a, 0, sin a)` and lift direction `(-sin a, 0, cos a)`:
//!   `Fx = CD cos a - CL sin a`, `Fz = CD sin a + CL cos a`.
//! * Half models are normalised with `REF_AREA / 2`, so no further factor is needed.

use crate::config::FlowModel;
use crate::history::{History, HistorySummary};
use aero::model::axial_alpha_factor;
use aero::AeroModel;
use anyhow::{bail, Result};

/// Force and moment coefficients as reported by SU2 (body axes, nose-tip moment origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Su2Coeffs {
    pub cfx: f64,
    pub cfz: f64,
    pub cmy: f64,
}

impl Su2Coeffs {
    /// From a history summary; `CFx` / `CFz` are rebuilt from `CD` / `CL` when absent.
    pub fn from_summary(s: &HistorySummary, alpha_deg: f64) -> Result<Self> {
        let a = alpha_deg.to_radians();
        let cfx = match (s.cfx, s.cd, s.cl) {
            (Some(x), _, _) => x,
            (None, Some(d), Some(l)) => d * a.cos() - l * a.sin(),
            _ => bail!("[IY-F103] history has neither CFx nor CD and CL"),
        };
        let cfz = match (s.cfz, s.cd, s.cl) {
            (Some(z), _, _) => z,
            (None, Some(d), Some(l)) => d * a.sin() + l * a.cos(),
            _ => bail!("[IY-F103] history has neither CFz nor CD and CL"),
        };
        let Some(cmy) = s.cmy else { bail!("[IY-F104] history has no CMy") };
        if ![cfx, cfz, cmy].iter().all(|v| v.is_finite()) {
            bail!("[IY-F105] non-finite coefficients in the history (diverged case)");
        }
        Ok(Self { cfx, cfz, cmy })
    }
}

/// Name of the CFD surface marker of the body wall (nose, body tube, boattail, fins).
pub const WALL_MARKER: &str = "wall";
/// Name of the CFD surface marker of the base cap.
pub const BASE_MARKER: &str = "base";

/// Per-marker coefficients of one history (`AERO_COEFF_SURF`): SU2 8.3 writes them as `CFx(wall)`,
/// `CFz(wall)`, `CMy(wall)`, `CFx(base)`, ... next to the totals. `None` where the column is absent.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MarkerCoeffs {
    pub wall_cfx: Option<f64>,
    pub wall_cfz: Option<f64>,
    pub wall_cmy: Option<f64>,
    pub base_cfx: Option<f64>,
    pub base_cfz: Option<f64>,
    pub base_cmy: Option<f64>,
}

impl MarkerCoeffs {
    /// Coefficients averaged over the last `window` rows (1 = last row). Accepts `CFx(wall)` and `CFx@wall`.
    pub fn from_history(h: &History, window: usize) -> Self {
        let n = h.rows.len();
        if n == 0 {
            return Self::default();
        }
        let w = window.clamp(1, n);
        let avg = |coef: &str, marker: &str| -> Option<f64> {
            let c = h.col(&[&format!("{coef}({marker})"), &format!("{coef}@{marker}")])?;
            let v: Vec<f64> = h.rows[n - w..].iter().map(|r| r[c]).filter(|x| x.is_finite()).collect();
            (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
        };
        Self {
            wall_cfx: avg("CFx", WALL_MARKER),
            wall_cfz: avg("CFz", WALL_MARKER),
            wall_cmy: avg("CMy", WALL_MARKER),
            base_cfx: avg("CFx", BASE_MARKER),
            base_cfz: avg("CFz", BASE_MARKER),
            base_cmy: avg("CMy", BASE_MARKER),
        }
    }

    /// Axial coefficient the table takes from this case, see [`AxialSplit`].
    ///
    /// * `base_from_cfd`: wall + base (the base column absent: the monitored total, which holds both).
    /// * otherwise: the wall alone, the base term then comes from the Barrowman model. Without the wall
    ///   column the monitored total is used and holds the base as well (`base_in_cfx`).
    pub fn axial(&self, total_cfx: f64, base_from_cfd: bool) -> AxialSplit {
        let (wall, base) = (self.wall_cfx, self.base_cfx);
        match (wall, base, base_from_cfd) {
            (Some(w), _, false) => AxialSplit { cfx: w, wall, base, base_in_cfx: false },
            (Some(w), Some(b), true) => AxialSplit { cfx: w + b, wall, base, base_in_cfx: true },
            _ => AxialSplit { cfx: total_cfx, wall, base, base_in_cfx: true },
        }
    }
}

/// The axial coefficient used for a case and what it contains.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxialSplit {
    /// `CFx` to use (CFD frame, positive aft).
    pub cfx: f64,
    pub wall: Option<f64>,
    pub base: Option<f64>,
    /// `cfx` holds the base-cap force. When this is true although the table asks for the Barrowman base
    /// (per-marker columns absent), the table must be built with `base_from_cfd = true`.
    pub base_in_cfx: bool,
}

impl Su2Coeffs {
    /// RANS: the axial coefficient replaced by [`MarkerCoeffs::axial`]; normal force and moment are the totals.
    pub fn with_markers(self, m: &MarkerCoeffs, base_from_cfd: bool) -> (Self, AxialSplit) {
        let a = m.axial(self.cfx, base_from_cfd);
        (Self { cfx: a.cfx, ..self }, a)
    }
}

impl Su2Coeffs6 {
    /// See [`Su2Coeffs::with_markers`].
    pub fn with_markers(self, m: &MarkerCoeffs, base_from_cfd: bool) -> (Self, AxialSplit) {
        let a = m.axial(self.cfx, base_from_cfd);
        (Self { cfx: a.cfx, ..self }, a)
    }
}

/// Body-axis coefficients of one solved case (before hybrid terms).
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BodyCoeffs {
    pub cn: f64,
    /// Axial force coefficient from CFD (positive aft).
    pub ca: f64,
    /// Nose-tip moment of the normal force, `CN xcp` \[m\].
    pub mom: f64,
}

impl BodyCoeffs {
    pub fn from_su2(c: &Su2Coeffs, ref_length: f64) -> Self {
        Self { cn: c.cfz, ca: c.cfx, mom: -c.cmy * ref_length }
    }

    /// Centre of pressure aft of the nose \[m\]; `None` when `|CN|` is negligible.
    pub fn xcp(&self) -> Option<f64> {
        (self.cn.abs() > 1e-9).then(|| self.mom / self.cn)
    }
}

/// All six force and moment coefficients of SU2 (CFD frame: x aft, nose-tip moment origin).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Su2Coeffs6 {
    pub cfx: f64,
    pub cfy: f64,
    pub cfz: f64,
    pub cmx: f64,
    pub cmy: f64,
    pub cmz: f64,
}

impl Su2Coeffs6 {
    /// From a history summary; fails when one of the six columns is missing or not finite.
    pub fn from_summary(s: &HistorySummary) -> Result<Self> {
        let get = |v: Option<f64>, n: &str| v.ok_or_else(|| anyhow::anyhow!("[IY-F104] history has no {n}"));
        let c = Self {
            cfx: get(s.cfx, "CFx")?,
            cfy: get(s.cfy, "CFy")?,
            cfz: get(s.cfz, "CFz")?,
            cmx: get(s.cmx, "CMx")?,
            cmy: get(s.cmy, "CMy")?,
            cmz: get(s.cmz, "CMz")?,
        };
        if ![c.cfx, c.cfy, c.cfz, c.cmx, c.cmy, c.cmz].iter().all(|v| v.is_finite()) {
            bail!("[IY-F105] non-finite coefficients in the history (diverged case)");
        }
        Ok(c)
    }
}

/// Body-axis coefficients of one solved case (x forward, forces / (q S), moments about the nose tip / (q S d)).
///
/// `CX = -CFx, CY = CFy, CZ = -CFz, Cl = -CMx, Cm = CMy, Cn = -CMz`: the CFD frame is the body frame
/// rotated by 180 deg about y (`diag(-1, 1, -1)`), and moments are axial vectors.
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
pub struct BodyCoeffs6 {
    pub cx: f64,
    pub cy: f64,
    pub cz: f64,
    pub cl: f64,
    pub cm: f64,
    pub cn: f64,
}

impl BodyCoeffs6 {
    pub fn from_su2(c: &Su2Coeffs6) -> Self {
        Self { cx: -c.cfx, cy: c.cfy, cz: -c.cfz, cl: -c.cmx, cm: c.cmy, cn: -c.cmz }
    }

    /// Half model (SU2 `SYMMETRY` plane at y = 0): the plane carries the pressure of the half body, so `CFy`, `CMx` and `CMz`
    /// (here `cy`, `cl`, `cn`) are plane artefacts, not physical. The body is symmetric about y = 0, so they are 0.
    pub fn half_model(self) -> Self {
        Self { cy: 0.0, cl: 0.0, cn: 0.0, ..self }
    }

    /// Image under the reflection z -> -z (flow `(alpha, beta) -> (-alpha, beta)`): `CZ`, `Cl`, `Cm` change sign.
    pub fn mirror_z(&self) -> Self {
        Self { cx: self.cx, cy: self.cy, cz: -self.cz, cl: -self.cl, cm: -self.cm, cn: self.cn }
    }

    /// Image under the reflection y -> -y (flow `(alpha, beta) -> (alpha, -beta)`): `CY`, `Cl`, `Cn` change sign.
    pub fn mirror_y(&self) -> Self {
        Self { cx: self.cx, cy: -self.cy, cz: self.cz, cl: -self.cl, cm: self.cm, cn: -self.cn }
    }

    pub fn as_array(&self) -> [f64; 6] {
        [self.cx, self.cy, self.cz, self.cl, self.cm, self.cn]
    }

    pub fn from_array(a: [f64; 6]) -> Self {
        Self { cx: a[0], cy: a[1], cz: a[2], cl: a[3], cm: a[4], cn: a[5] }
    }
}

/// Axial force coefficients (power-on, power-off) at `mach` / `alpha_rad` from the CFD axial
/// coefficient `ca_cfd` plus the terms CFD does not supply.
///
/// * Euler: friction + base + fin-edge (rounded leading edge, square trailing edge) + boattail +
///   extra drag of the Barrowman build-up. The nose pressure term is **excluded** (CFD gives the
///   pressure / wave drag). The CFD base cap is not monitored (see `su2cfg`), so the base term is
///   fully taken from the model; power-on uses the base area reduced by the nozzle exit area.
/// * RANS: friction, pressure, fin and boattail drag come from CFD (`ca_cfd` is the wall marker). The base term
///   is the CFD base marker when `base_from_cfd` (it is then part of `ca_cfd`; for power-on the
///   base-pressure contribution over the nozzle exit area is removed, `base_cd(M) (A_base - A_base,on) / S`),
///   otherwise the Barrowman base drag with its nozzle-area power-on correction. `extra_cd` is added, and the
///   skin-friction increment of the configured surface roughness ([`roughness_increment`]): the RANS wall is smooth, and
///   without `fin_edges_in_cfd` the fin-edge drag of the configured edges ([`fin_edge_increment`]).
///   No Euler (inviscid) term is ever added to a RANS value.
///
/// `fin_edges_in_cfd`: the integrated CFD wall already holds the fin leading / trailing faces (STL surface), so
/// their pressure drag is in `ca_cfd` and the Barrowman fin-edge term is not added again. For the rebuilt
/// (CAD) geometry the term is kept.
///
/// The additions are multiplied by the model's `axial_alpha_factor`, like the Barrowman path.
#[allow(clippy::too_many_arguments)]
pub fn hybrid_axial(flow: FlowModel, fin_edges_in_cfd: bool, base_from_cfd: bool, model: &AeroModel, mach: f64, alpha_rad: f64, ca_cfd: f64) -> (f64, f64) {
    let k = axial_alpha_factor(alpha_rad);
    let (on, off) = (model.cd0(mach, true), model.cd0(mach, false));
    let rough = roughness_increment(model, mach);
    let (add_on, add_off) = match flow {
        FlowModel::Euler => {
            let fins = |b: &aero::model::DragBreakdown| if fin_edges_in_cfd { 0.0 } else { b.fins };
            let te = fin_te_base_increment(model, mach, fin_edges_in_cfd);
            let t = |b: &aero::model::DragBreakdown| b.friction + b.base + fins(b) + te + b.boattail + b.extra;
            (t(&on), t(&off))
        }
        FlowModel::Rans if base_from_cfd => {
            let rough = rough + fin_edge_increment(model, mach, fin_edges_in_cfd);
            let removed = (off.base - on.base).max(0.0);
            // Base drag over the nozzle exit area, `base_cd(M) A_nozzle / S`, is the model's off-on base difference.
            (off.extra + rough - removed, off.extra + rough)
        }
        FlowModel::Rans => {
            let rough = rough + fin_edge_increment(model, mach, fin_edges_in_cfd) + fin_te_base_increment(model, mach, fin_edges_in_cfd);
            (on.base + on.extra + rough, off.base + off.extra + rough)
        }
    };
    ((ca_cfd + add_on * k).max(0.0), (ca_cfd + add_off * k).max(0.0))
}

/// Skin-friction increment of the configured equivalent sand roughness over a smooth wall at `mach`: the Barrowman
/// rough-minus-smooth friction drag (same wetted area, Reynolds number and compressibility corrections). The RANS
/// wall is smooth, so this is a model correction on top of the smooth-wall CFD friction. 0 for zero roughness.
pub fn roughness_increment(model: &AeroModel, mach: f64) -> f64 {
    let rel = model.opt.roughness / model.geom.length;
    if rel <= 0.0 {
        return 0.0;
    }
    let b = model.cd0(mach, false);
    let (rough, smooth) = (aero::model::skin_friction(b.reynolds, mach, rel), aero::model::skin_friction(b.reynolds, mach, 0.0));
    if rough > 0.0 {
        b.friction * (1.0 - smooth / rough)
    } else {
        0.0
    }
}

/// Fraction of the configured fin-edge drag that the rebuilt (CAD) fins already carry in a RANS wall.
///
/// The rebuilt biconvex fin is cut at `BL_EDGE_BLUNT` of the chord from each edge, so its edges are faces of the
/// section thickness at that chord position, `4 e (1 - e)` of the maximum thickness (7.8 % for 2 %). The Barrowman
/// fin-edge terms (rounded leading edge, square trailing edge) scale with that frontal thickness, so the same fraction
/// of them is already in the CFD wall and is subtracted; this is an estimate (the pressure drag of a 0.3 mm face is
/// modelled with the coefficient of the configured edge), small against the term itself.
pub fn rebuilt_lens_edge_fraction() -> f64 {
    let e = crate::mesh::BL_EDGE_BLUNT;
    4.0 * e * (1.0 - e)
}

/// Fin-edge drag the RANS table adds on the rebuilt (CAD) surface: the Barrowman terms of the configured fin edges
/// (`fin_le_rounded`, `fin_te_square`) minus what the blunted lens already carries ([`rebuilt_lens_edge_fraction`]).
/// 0 when the CFD wall holds the real edges (`fin_edges_in_cfd`: STL or STEP surface).
pub fn fin_edge_increment(model: &AeroModel, mach: f64, fin_edges_in_cfd: bool) -> f64 {
    if fin_edges_in_cfd {
        return 0.0;
    }
    model.cd0(mach, false).fins * (1.0 - rebuilt_lens_edge_fraction())
}

/// The fin trailing edges lie flush with the base plane: both the root and the tip trailing edge are within
/// `1e-4 length` (the mesh `base_tol`) of `x = length`, so the unswept trailing-edge faces are planar aft-facing faces
/// at the base and the STL / STEP base rule puts them into the `base` marker.
pub fn fin_te_flush(model: &AeroModel) -> bool {
    let g = &model.geom;
    let Some(f) = &g.fins else { return false };
    let tol = 1e-4 * g.length;
    let (root, tip) = (f.x_le_root + f.root_chord, f.x_le_root + f.sweep + f.tip_chord);
    f.count > 0 && (root - g.length).abs() <= tol && (tip - g.length).abs() <= tol
}

/// Trailing-edge base drag of fins flush with the base (the Barrowman square trailing edge, `base_cd(M) n b t / S`).
/// On an STL / STEP surface (`fin_edges_in_cfd`) the leading edges are in the CFD wall, but flush trailing-edge faces
/// are in the `base` marker, whose force is replaced by the model (Euler, RANS `rans_base = "model"`), so this term
/// stands in for them; it is added together with the body base term. 0 for inset trailing edges (their faces are in
/// the wall marker), for `fin_te_square = false`, and on the rebuilt surface (the full fin-edge term is added there).
pub fn fin_te_base_increment(model: &AeroModel, mach: f64, fin_edges_in_cfd: bool) -> f64 {
    if !fin_edges_in_cfd || !model.opt.fin_te_square || !fin_te_flush(model) {
        return 0.0;
    }
    let (g, f) = (&model.geom, model.geom.fins.as_ref().expect("flush implies fins"));
    aero::model::base_cd(mach) * f.count as f64 * f.span * f.thickness / g.ref_area
}

/// Axial breakdown of one RANS case for `cfd_report.json` (all coefficients positive aft).
#[derive(Debug, Clone, Copy, PartialEq, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct AxialReportEntry {
    pub mach: f64,
    pub alpha_deg: f64,
    /// `CFx` of the wall marker (pressure + friction + fin edges) and of the base marker; `None` without the columns.
    pub cfd_wall_ca: Option<f64>,
    pub cfd_base_ca: Option<f64>,
    /// Barrowman skin friction and base drag at this Mach number (the friction is not added to a RANS table; shown for comparison).
    pub barrowman_friction: f64,
    pub barrowman_base: f64,
    /// Roughness friction increment added to the smooth-wall RANS value ([`roughness_increment`]).
    pub roughness_increment: f64,
    /// Fin-edge increment added on the rebuilt surface ([`fin_edge_increment`]); 0 for STL / STEP.
    pub fin_edge_increment: f64,
    /// Trailing-edge base drag of fins flush with the base, added on STL / STEP with the model base ([`fin_te_base_increment`]).
    pub fin_te_base_increment: f64,
    /// The base term of the table comes from CFD (`base_from_cfd`, or the per-marker columns were absent).
    pub base_from_cfd: bool,
}

impl AxialReportEntry {
    pub fn new(model: &AeroModel, mach: f64, alpha_deg: f64, markers: &MarkerCoeffs, base_from_cfd: bool, fin_edges_in_cfd: bool) -> Self {
        let b = model.cd0(mach, false);
        let in_cfx = markers.axial(0.0, base_from_cfd).base_in_cfx;
        Self {
            mach,
            alpha_deg,
            cfd_wall_ca: markers.wall_cfx,
            cfd_base_ca: markers.base_cfx,
            barrowman_friction: b.friction,
            barrowman_base: b.base,
            roughness_increment: roughness_increment(model, mach),
            fin_edge_increment: fin_edge_increment(model, mach, fin_edges_in_cfd),
            fin_te_base_increment: if in_cfx { 0.0 } else { fin_te_base_increment(model, mach, fin_edges_in_cfd) },
            base_from_cfd: in_cfx,
        }
    }
}

/// Pitch-damping sums `[S0, S1, S2]` (sum CNa_i, sum CNa_i x_i, sum CNa_i x_i^2) rescaled to CFD.
///
/// CFD steady runs give no damping. The model's sums `model_s` (Barrowman, or the panel method
/// when available) keep their *spread* about the centroid, `var = S2/S0 - (S1/S0)^2`, while the
/// total slope and the centroid are replaced by the CFD values: `S0 = cna`,
/// `S1 = cna xcp0`, `S2 = cna (xcp0^2 + var)`.
pub fn scale_damping(model_s: [f64; 3], cna: f64, xcp0: f64) -> [f64; 3] {
    let var = if model_s[0].abs() > 1e-12 { (model_s[2] / model_s[0] - (model_s[1] / model_s[0]).powi(2)).max(0.0) } else { 0.0 };
    [cna, cna * xcp0, cna * (xcp0 * xcp0 + var)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use aero::AeroOptions;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn model(nozzle: f64) -> AeroModel {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        AeroModel::new(g, AeroOptions { nozzle_exit_area: nozzle, extra_cd: 0.02, ..Default::default() })
    }

    /// SU2's moment about the origin: M = r x F, y component (Fx z - Fz x), normalised by q S L.
    fn su2_cmy(r: [f64; 3], f: [f64; 3], qsl: f64) -> f64 {
        (f[0] * r[2] - f[2] * r[0]) / qsl
    }

    #[test]
    fn xcp_from_synthetic_force() {
        // Normal force 12 N (+z) and axial force 3 N (+x) acting at x = 0.8 m on the axis.
        let (q, s, l) = (500.0, 0.0078, 0.1);
        let qsl = q * s * l;
        let cmy = su2_cmy([0.8, 0.0, 0.0], [3.0, 0.0, 12.0], qsl);
        assert!(cmy < 0.0);
        let c = BodyCoeffs::from_su2(&Su2Coeffs { cfx: 3.0 / (q * s), cfz: 12.0 / (q * s), cmy }, l);
        assert!((c.cn - 12.0 / (q * s)).abs() < 1e-12 && (c.ca - 3.0 / (q * s)).abs() < 1e-12);
        assert!((c.xcp().unwrap() - 0.8).abs() < 1e-12);
        // Force applied off-axis at z = 0.05: the axial force adds a moment, xcp changes accordingly.
        let cmy2 = su2_cmy([0.8, 0.0, 0.05], [3.0, 0.0, 12.0], qsl);
        let c2 = BodyCoeffs::from_su2(&Su2Coeffs { cfx: 3.0 / (q * s), cfz: 12.0 / (q * s), cmy: cmy2 }, l);
        assert!((c2.xcp().unwrap() - (0.8 - 3.0 * 0.05 / 12.0)).abs() < 1e-12);
    }

    #[test]
    fn six_component_mapping_and_mirrors() {
        let s = HistorySummary { cfx: Some(0.3), cfy: Some(0.2), cfz: Some(1.1), cmx: Some(0.05), cmy: Some(-14.0), cmz: Some(2.0), ..Default::default() };
        let c = BodyCoeffs6::from_su2(&Su2Coeffs6::from_summary(&s).unwrap());
        assert_eq!(c, BodyCoeffs6 { cx: -0.3, cy: 0.2, cz: -1.1, cl: -0.05, cm: -14.0, cn: -2.0 });
        assert_eq!(c.mirror_z().mirror_z(), c);
        assert_eq!(c.mirror_y().mirror_y(), c);
        assert_eq!(BodyCoeffs6::from_array(c.as_array()), c);
        let m = BodyCoeffs6::from_su2(&Su2Coeffs6 { cfx: 0.3, cfy: 0.0, cfz: 1.0, cmx: 0.0, cmy: -1.0, cmz: 0.0 });
        // z-mirror of a pure pitch-plane state: CZ and Cm flip, the axial force stays.
        assert_eq!((m.mirror_z().cx, m.mirror_z().cz, m.mirror_z().cm), (m.cx, -m.cz, -m.cm));
        assert!(Su2Coeffs6::from_summary(&HistorySummary { cfx: Some(0.3), ..Default::default() }).is_err());
    }

    #[test]
    fn half_model_side_quantities_are_zeroed() {
        // Synthetic half-model history: the y = 0 plane gives CFy 1.56, CMx 0.3, CMz 16.9.
        let h = History::parse("\"Inner_Iter\",\"CFx\",\"CFy\",\"CFz\",\"CMx\",\"CMy\",\"CMz\"\n0,0.3,1.56,0.5,0.3,-2.0,16.9\n").unwrap();
        let s = crate::history::summarize(&h, 1).unwrap();
        let raw = BodyCoeffs6::from_su2(&Su2Coeffs6::from_summary(&s).unwrap());
        assert!((raw.cy - 1.56).abs() < 1e-12 && (raw.cn + 16.9).abs() < 1e-12);
        let c = raw.half_model();
        assert_eq!((c.cy, c.cl, c.cn), (0.0, 0.0, 0.0));
        assert_eq!((c.cx, c.cz, c.cm), (raw.cx, raw.cz, raw.cm));
    }

    #[test]
    fn cl_cd_rotation_roundtrip() {
        let a = 7f64.to_radians();
        let (fx, fz) = (0.31, 0.52);
        let (cd, cl) = (fx * a.cos() + fz * a.sin(), -fx * a.sin() + fz * a.cos());
        let s = HistorySummary { cd: Some(cd), cl: Some(cl), cmy: Some(-0.1), ..Default::default() };
        let c = Su2Coeffs::from_summary(&s, 7.0).unwrap();
        assert!((c.cfx - fx).abs() < 1e-12 && (c.cfz - fz).abs() < 1e-12);
        assert!(Su2Coeffs::from_summary(&HistorySummary::default(), 0.0).is_err());
    }

    #[test]
    fn hybrid_terms() {
        let m = model(0.0);
        let (on, off) = hybrid_axial(FlowModel::Euler, false, false, &m, 0.5, 0.0, 0.1);
        let b = m.cd0(0.5, false);
        assert!((off - (0.1 + b.friction + b.base + b.fins + b.boattail + b.extra)).abs() < 1e-12);
        assert!((on - off).abs() < 1e-12, "no nozzle: power-on equals power-off");
        let m = model(0.5 * std::f64::consts::PI * 0.05 * 0.05);
        let (on, off) = hybrid_axial(FlowModel::Euler, false, false, &m, 0.5, 0.0, 0.1);
        assert!(on < off);
        // STL surface: the fin-edge drag is in the CFD wall and is not added again.
        let (_, off_stl) = hybrid_axial(FlowModel::Euler, true, false, &m, 0.5, 0.0, 0.1);
        let b = m.cd0(0.5, false);
        assert!(b.fins > 0.0 && (off - off_stl - b.fins).abs() < 1e-12);
        // RANS with the CFD base: only extra_cd (and the nozzle correction) are added.
        let (ron, roff) = hybrid_axial(FlowModel::Rans, true, true, &m, 0.5, 0.0, 0.4);
        let r = roughness_increment(&m, 0.5);
        assert!((roff - (0.42 + r)).abs() < 1e-12);
        assert!(ron < roff && ron > 0.0);
    }

    #[test]
    fn rans_base_model_versus_cfd() {
        let nozzle = 0.5 * std::f64::consts::PI * 0.05 * 0.05;
        let m = model(nozzle);
        let (on, off) = (m.cd0(0.5, true), m.cd0(0.5, false));
        assert!(off.base > on.base && off.base > 0.0);
        // Barrowman base: wall + base model + extra, the nozzle area reduces the power-on base.
        let (ron, roff) = hybrid_axial(FlowModel::Rans, true, false, &m, 0.5, 0.0, 0.4);
        let r = roughness_increment(&m, 0.5);
        assert!((roff - (0.4 + off.base + off.extra + r)).abs() < 1e-12);
        assert!((ron - (0.4 + on.base + on.extra + r)).abs() < 1e-12 && ron < roff);
        // CFD base: the base is in the CFD value, only extra (and the nozzle correction) is added.
        let (con, coff) = hybrid_axial(FlowModel::Rans, true, true, &m, 0.5, 0.0, 0.4 + off.base);
        assert!((coff - (0.4 + off.base + off.extra + r)).abs() < 1e-12, "same total when the CFD base equals the model base");
        assert!((con - (coff - (off.base - on.base))).abs() < 1e-12);
        // No friction, fin-edge or boattail term of the Euler build-up reaches a RANS value, for either base source.
        let euler = hybrid_axial(FlowModel::Euler, false, false, &m, 0.5, 0.0, 0.4).1;
        for flag in [false, true] {
            let r = hybrid_axial(FlowModel::Rans, true, flag, &m, 0.5, 0.0, 0.4).1;
            assert!(euler - r >= off.friction + off.fins + off.boattail - 1e-12 - r - if flag { off.base } else { 0.0 });
        }
        // The off-axis factor scales the additions only.
        let a = 8f64.to_radians();
        let k = axial_alpha_factor(a);
        let r = hybrid_axial(FlowModel::Rans, true, false, &m, 0.5, a, 0.4).1;
        assert!((r - (0.4 + (off.base + off.extra + roughness_increment(&m, 0.5)) * k)).abs() < 1e-12);
    }

    fn hist(cols: &[&str], row: &[f64]) -> History {
        History { columns: cols.iter().map(|s| s.to_string()).collect(), rows: vec![row.to_vec(), row.to_vec()] }
    }

    #[test]
    fn marker_columns_parsed_and_absent_ones_fall_back() {
        let h = History::parse(
            "\"Inner_Iter\",\"CFx\",\"CFz\",\"CMy\",\"CFx(wall)\",\"CFz(wall)\",\"CMy(wall)\",\"CFx(base)\",\"CFz(base)\",\"CMy(base)\"\n0,0.5,1.0,-2.0,0.4,1.0,-2.0,0.1,0.0,0.0\n1,0.5,1.0,-2.0,0.4,1.0,-2.0,0.1,0.0,0.0\n",
        )
        .unwrap();
        let m = MarkerCoeffs::from_history(&h, 1);
        assert_eq!((m.wall_cfx, m.base_cfx, m.wall_cmy, m.base_cfz), (Some(0.4), Some(0.1), Some(-2.0), Some(0.0)));
        // '@' spelling of newer / older SU2 versions.
        let m2 = MarkerCoeffs::from_history(&hist(&["CFx@wall", "CFx@base"], &[0.3, 0.2]), 2);
        assert_eq!((m2.wall_cfx, m2.base_cfx, m2.wall_cfz), (Some(0.3), Some(0.2), None));
        // Absent columns: totals are used and flagged as containing the base.
        let none = MarkerCoeffs::from_history(&hist(&["CFx", "CFz"], &[0.5, 1.0]), 1);
        assert_eq!(none, MarkerCoeffs::default());
        let a = none.axial(0.5, false);
        assert!((a.cfx - 0.5).abs() < 1e-15 && a.base_in_cfx && a.wall.is_none());
        // Wall present: Barrowman base -> wall alone; CFD base -> wall + base; base column missing -> total.
        let a = m.axial(0.5, false);
        assert!((a.cfx - 0.4).abs() < 1e-15 && !a.base_in_cfx);
        let a = m.axial(0.5, true);
        assert!((a.cfx - 0.5).abs() < 1e-15 && a.base_in_cfx);
        let wall_only = MarkerCoeffs { wall_cfx: Some(0.4), ..Default::default() };
        assert!((wall_only.axial(0.5, true).cfx - 0.5).abs() < 1e-15 && wall_only.axial(0.5, true).base_in_cfx);
        // The normal force and moment stay the totals.
        let (c, _) = Su2Coeffs { cfx: 0.5, cfz: 1.0, cmy: -2.0 }.with_markers(&m, false);
        assert_eq!((c.cfx, c.cfz, c.cmy), (0.4, 1.0, -2.0));
        let (c6, split) = Su2Coeffs6 { cfx: 0.5, cfy: 0.0, cfz: 1.0, cmx: 0.0, cmy: -2.0, cmz: 0.0 }.with_markers(&m, false);
        assert_eq!((c6.cfx, split.base), (0.4, Some(0.1)));
        // Report entry: base source flag follows the columns.
        let mdl = model(0.0);
        assert!(!AxialReportEntry::new(&mdl, 0.5, 0.0, &m, false, true).base_from_cfd);
        assert!(AxialReportEntry::new(&mdl, 0.5, 0.0, &none, false, true).base_from_cfd);
        let e = AxialReportEntry::new(&mdl, 0.5, 0.0, &m, true, true);
        assert!(e.base_from_cfd && e.barrowman_friction > 0.0 && e.cfd_base_ca == Some(0.1));
        let back: AxialReportEntry = serde_json::from_str("{\"mach\":1.0}").unwrap();
        assert_eq!(back.mach, 1.0);
    }

    #[test]
    fn roughness_increment_is_barrowman_rough_minus_smooth() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let mk = |r: f64| AeroModel::new(g.clone(), AeroOptions { roughness: r, extra_cd: 0.0, ..Default::default() });
        let (smooth, rough) = (mk(0.0), mk(60e-6));
        assert_eq!(roughness_increment(&smooth, 0.8), 0.0);
        for m in [0.3, 0.8, 1.5, 2.5] {
            let want = rough.cd0(m, false).friction - smooth.cd0(m, false).friction;
            assert!(want > 0.0);
            assert!((roughness_increment(&rough, m) - want).abs() < 1e-12, "M {m}");
            // RANS value: wall + base model + increment; zero roughness adds nothing.
            let b = rough.cd0(m, false);
            let r = hybrid_axial(FlowModel::Rans, true, false, &rough, m, 0.0, 0.2).1;
            assert!((r - (0.2 + b.base + want)).abs() < 1e-12);
            let r0 = hybrid_axial(FlowModel::Rans, true, true, &smooth, m, 0.0, 0.2).1;
            assert!((r0 - 0.2).abs() < 1e-12);
            // The Euler value is unchanged by this correction.
            let e = hybrid_axial(FlowModel::Euler, false, false, &rough, m, 0.0, 0.2).1;
            assert!((e - (0.2 + b.friction + b.base + b.fins + b.boattail)).abs() < 1e-12);
        }
        let e = AxialReportEntry::new(&rough, 0.8, 0.0, &MarkerCoeffs::default(), false, false);
        assert!(e.roughness_increment > 0.0 && e.fin_edge_increment > 0.0);
        assert_eq!(AxialReportEntry::new(&rough, 0.8, 0.0, &MarkerCoeffs::default(), false, true).fin_edge_increment, 0.0);
    }

    #[test]
    fn rans_fin_edge_increment_on_the_rebuilt_surface() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let m = AeroModel::new(g, AeroOptions { roughness: 0.0, extra_cd: 0.0, ..Default::default() });
        let f = rebuilt_lens_edge_fraction();
        assert!((f - 0.0784).abs() < 1e-12);
        for mach in [0.5, 1.2, 2.0] {
            let b = m.cd0(mach, false);
            assert!(b.fins > 0.0);
            let inc = fin_edge_increment(&m, mach, false);
            assert!((inc - b.fins * (1.0 - f)).abs() < 1e-15 && inc > 0.0 && inc < b.fins);
            assert_eq!(fin_edge_increment(&m, mach, true), 0.0, "STL / STEP: the edges are in the wall");
            let cad = hybrid_axial(FlowModel::Rans, false, true, &m, mach, 0.0, 0.3).1;
            let real = hybrid_axial(FlowModel::Rans, true, true, &m, mach, 0.0, 0.3).1;
            assert!((real - 0.3).abs() < 1e-12 && (cad - real - inc).abs() < 1e-12);
            // The alpha factor scales it like the other additions.
            let a = 8f64.to_radians();
            let ca = hybrid_axial(FlowModel::Rans, false, true, &m, mach, a, 0.3).1;
            assert!((ca - 0.3 - inc * axial_alpha_factor(a)).abs() < 1e-12);
        }
        // Switching the configured edges off removes the term.
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let plain = AeroModel::new(g, AeroOptions { fin_le_rounded: false, fin_te_square: false, ..Default::default() });
        assert_eq!(fin_edge_increment(&plain, 1.2, false), 0.0);
    }

    /// Sample rocket with the fin trailing edges `inset` metres ahead of the base (0 = flush, unswept trailing edge).
    fn finned(inset: f64, te_square: bool) -> AeroModel {
        let mut g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let l = g.length;
        let f = g.fins.as_mut().unwrap();
        f.x_le_root = l - inset - f.root_chord;
        f.sweep = f.root_chord - f.tip_chord;
        AeroModel::new(g, AeroOptions { fin_te_square: te_square, extra_cd: 0.0, ..Default::default() })
    }

    #[test]
    fn flush_fin_trailing_edges_add_the_base_drag_on_step_and_stl() {
        let flush = finned(0.0, true);
        let inset = finned(0.05, true);
        assert!(fin_te_flush(&flush) && !fin_te_flush(&inset));
        assert!(!fin_te_flush(&finned(2e-4 * flush.geom.length, true)), "outside the mesh base_tol");
        for mach in [0.5, 1.2, 2.4] {
            let f = flush.geom.fins.as_ref().unwrap();
            let want = aero::model::base_cd(mach) * f.count as f64 * f.span * f.thickness / flush.geom.ref_area;
            assert!(want > 0.0);
            let inc = fin_te_base_increment(&flush, mach, true);
            assert!((inc - want).abs() < 1e-15);
            // Only where the CFD wall holds the edges; the rebuilt surface keeps the full Barrowman fin-edge term.
            assert_eq!(fin_te_base_increment(&flush, mach, false), 0.0);
            assert_eq!(fin_te_base_increment(&inset, mach, true), 0.0);
            assert_eq!(fin_te_base_increment(&finned(0.0, false), mach, true), 0.0);
            // Euler: the term is on top of the CFD value, the model base and friction (STEP / STL).
            let b = flush.cd0(mach, false);
            let e = hybrid_axial(FlowModel::Euler, true, false, &flush, mach, 0.0, 0.2).1;
            assert!((e - (0.2 + b.friction + b.base + inc + b.boattail + b.extra)).abs() < 1e-12);
            let e_inset = hybrid_axial(FlowModel::Euler, true, false, &inset, mach, 0.0, 0.2).1;
            let bi = inset.cd0(mach, false);
            assert!((e_inset - (0.2 + bi.friction + bi.base + bi.boattail + bi.extra)).abs() < 1e-12);
            // CAD (rebuilt) surface: unchanged, the whole fin term.
            let cad = hybrid_axial(FlowModel::Euler, false, false, &flush, mach, 0.0, 0.2).1;
            assert!((cad - (0.2 + b.friction + b.base + b.fins + b.boattail + b.extra)).abs() < 1e-12);
            // RANS rans_base = "model": added; rans_base = "cfd": the CFD base marker holds the faces, nothing is added.
            let r_model = hybrid_axial(FlowModel::Rans, true, false, &flush, mach, 0.0, 0.3).1;
            assert!((r_model - (0.3 + b.base + b.extra + inc + roughness_increment(&flush, mach))).abs() < 1e-12);
            let r_cfd = hybrid_axial(FlowModel::Rans, true, true, &flush, mach, 0.0, 0.3).1;
            assert!((r_cfd - (0.3 + roughness_increment(&flush, mach))).abs() < 1e-12);
            // The alpha factor scales it like the other additions.
            let a = 8f64.to_radians();
            let ea = hybrid_axial(FlowModel::Euler, true, false, &flush, mach, a, 0.2).1;
            let k = axial_alpha_factor(a);
            assert!((ea - (0.2 + k * (b.friction + b.base + inc + b.boattail + b.extra))).abs() < 1e-12);
        }
        // Report entry: the increment is shown when the model supplies the base, 0 when the base is in the CFD value.
        let none = MarkerCoeffs::default();
        let both = MarkerCoeffs { wall_cfx: Some(0.2), base_cfx: Some(0.1), ..Default::default() };
        assert!(AxialReportEntry::new(&flush, 2.4, 0.0, &both, false, true).fin_te_base_increment > 0.0);
        assert_eq!(AxialReportEntry::new(&flush, 2.4, 0.0, &both, true, true).fin_te_base_increment, 0.0);
        assert_eq!(AxialReportEntry::new(&flush, 2.4, 0.0, &none, false, true).fin_te_base_increment, 0.0);
        assert_eq!(AxialReportEntry::new(&flush, 2.4, 0.0, &both, false, false).fin_te_base_increment, 0.0);
    }

    #[test]
    fn damping_scaling_matches_cfd_slope_and_centroid() {
        let s = scale_damping([3.0, 3.6, 4.9], 4.0, 1.1);
        assert!((s[0] - 4.0).abs() < 1e-12 && (s[1] / s[0] - 1.1).abs() < 1e-12);
        let var = 4.9 / 3.0 - 1.2f64 * 1.2;
        assert!((s[2] / s[0] - (1.21 + var)).abs() < 1e-12);
    }
}
