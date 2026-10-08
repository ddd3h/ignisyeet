// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Zero-angle asymmetry: the lateral force of a solved `alpha = beta = 0` case, as an accuracy indicator.
//!
//! # Where it comes from
//!
//! A body that is symmetric about `y = 0` and `z = 0` has no side or normal force at `alpha = beta = 0`. On a mesh
//! that is not mirror symmetric the steady solution nevertheless carries one. For the sample rocket (Euler,
//! M 0.8, half model, unstructured mesh) it is a fin lift: the fins carry about 70 % of it and the body next to
//! the fins the rest, it is the same steady solution from a cold start and from the converged `+4` and `-4` deg
//! solutions (not a second solution), it does not change when the residual is driven 5 orders further or with the
//! first-order scheme, it shrinks above M 1 and is absent without fins, with a pointed or a cut tail fairing.
//! It is set by the surface elements on the thin fins: with 6 mm elements on the 4 mm fins it is
//! `CN(0) = -0.139` (12 % of `CN(4 deg)`), and its size and sign vary at random when the wall size is changed
//! from 10 to 5 mm (`-0.054`, `+0.106`, `-0.139`, `+0.202`), because every wall size is still larger than the fin
//! thickness. Elements of 0.75 mm on the fins alone (the body still 6 mm) bring it down to `-0.007`
//! (0.6 %) and change the odd part `CN(4 deg)` by 3.5 %. Euler therefore meshes the fin faces at a quarter of the fin
//! thickness by default (`fin_size = "auto"`, [`crate::CfdOptions::fin_size_for`]): at 2 mm on the sample (fins 1 mm)
//! `CN(0)` at M 0.8 drops from 4.2 % to 0.15 % of `CN(4 deg)` (doc/data/cfd/fin_resolution.csv).
//!
//! RANS (half model with the prism layer, flat base) has a smaller one of another origin: at M 0.8 `CN(0) =
//! 0.011` (8 mm) and `0.034` (4.8 mm), 1 to 3 % of `CN(4 deg)`, and `0.006` to `0.013` without fins. The force is steady (the
//! residual stalls, the coefficient does not oscillate) and sits on the last base diameter of the body ahead of the
//! flat base and on the fins next to it: an asymmetric steady base wake. At M 2 it is below `0.001`. RANS keeps the fin
//! faces at `wall_size` (`fin_size = "auto"`): the source is not the fin surface, and prism columns on fins meshed at a
//! quarter of their thickness would far exceed the memory of a case.
//!
//! # What the table does with it
//!
//! * A mirrored mesh (quarter or half mirrored, shapes that pass the symmetry check) has none.
//! * `alpha_mode = "mirror"` and `"offset"` remove it: the axisymmetric table uses the odd part of `+-alpha` or
//!   subtracts `CN(0)` ([`crate::table::apply_alpha_mode`]); the general table subtracts `CY`, `CZ`, `Cm` and `Cn`
//!   of the `(0, 0)` point from every point ([`crate::general`]; `Cl` too for a mirror-symmetric shape, while for a
//!   shape without mirror symmetry it is the roll forcing of canted fins and stays). For a shape without mirror symmetry this also removes
//!   the zero-angle force of its asymmetric detail (a launch lug, rail buttons), which is small next to the
//!   spurious fin lift.
//! * `alpha_mode = "single"` keeps what the shape symmetry does not zero (all of it without mirror symmetry, `CZ`
//!   of a pitch-plane shape, `CN` of the axisymmetric table): a steady trim force that the shape does not have.
//!
//! `check` reports it per Mach number relative to the normal force at [`REF_ALPHA_DEG`] and warns above
//! [`WARN_RATIO`].

use crate::config::{AlphaMode, ShapeSymmetry};
use crate::general::{Slopes, SolvedPoint6};
use crate::table::Asymmetry;
use serde::{Deserialize, Serialize};

/// Reference angle of the normal force the zero-angle force is compared with \[deg\].
pub const REF_ALPHA_DEG: f64 = 4.0;
/// Warning threshold of `|F(0)| / CN(REF_ALPHA_DEG)`.
pub const WARN_RATIO: f64 = 0.05;

/// Zero-angle force of one Mach number.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ZeroAngle {
    pub mach: f64,
    /// Side force `CY` at `alpha = beta = 0` (general table; `None` for the axisymmetric table).
    pub cy: Option<f64>,
    /// Normal force `CZ` at `alpha = beta = 0` (axisymmetric table: `-CN`).
    pub cz: f64,
    /// Normal force at [`REF_ALPHA_DEG`] from the zero-angle slope, `CNa * 4 deg`.
    pub cn_ref: f64,
    /// `|(CY, CZ)| / cn_ref`.
    pub ratio: f64,
    /// Part of `ratio` that stays in the table (the rest is removed by symmetry or the alpha mode).
    pub ratio_kept: f64,
}

/// Which zero-angle components stay in the table: `(cy, cz)`. The `mirror` and `offset` modes remove them (by the
/// odd part of `+-alpha` or by subtracting the `alpha = beta = 0` value); `single` keeps what symmetry does not zero.
pub fn kept_components(general: bool, shape: ShapeSymmetry, mode: AlphaMode) -> (bool, bool) {
    if mode != AlphaMode::Single {
        return (false, false);
    }
    match (general, shape) {
        (false, _) | (true, ShapeSymmetry::PitchPlane) => (false, true),
        (true, ShapeSymmetry::Axisymmetric) => (false, false),
        _ => (true, true),
    }
}

fn entry(mach: f64, cy: Option<f64>, cz: f64, slopes: &[Slopes], kept: (bool, bool)) -> Option<ZeroAngle> {
    let s = slopes.iter().find(|s| s.mach == mach)?;
    let cn_ref = s.cna_pitch * REF_ALPHA_DEG.to_radians();
    if cn_ref.is_nan() || cn_ref.abs() <= 1e-9 {
        return None;
    }
    let y = cy.unwrap_or(0.0);
    let ratio = y.hypot(cz) / cn_ref.abs();
    let ky = if kept.0 { y } else { 0.0 };
    let kz = if kept.1 { cz } else { 0.0 };
    Some(ZeroAngle { mach, cy, cz, cn_ref, ratio, ratio_kept: ky.hypot(kz) / cn_ref.abs() })
}

/// Zero-angle entries of a general table from the solved `(0, 0)` points (raw, before symmetry completion).
pub fn from_general(points: &[SolvedPoint6], slopes: &[Slopes], shape: ShapeSymmetry, mode: AlphaMode) -> Vec<ZeroAngle> {
    let kept = kept_components(true, shape, mode);
    points
        .iter()
        .filter(|p| p.alpha_deg == 0.0 && p.beta_deg == 0.0)
        .filter_map(|p| p.coeffs.and_then(|c| entry(p.mach, Some(c.cy), c.cz, slopes, kept)))
        .collect()
}

/// Zero-angle entries of the axisymmetric table from the measured `alpha = 0` asymmetry.
pub fn from_axisymmetric(asym: &[Asymmetry], slopes: &[Slopes], mode: AlphaMode) -> Vec<ZeroAngle> {
    let kept = kept_components(false, ShapeSymmetry::Axisymmetric, mode);
    asym.iter().filter_map(|a| entry(a.mach, None, -a.cn0, slopes, kept)).collect()
}

/// Warnings for the entries above [`WARN_RATIO`] (one line per Mach number). `fin` is `(wall_size, fin
/// thickness)` \[m\] when the rocket has fins; `rans` names the RANS source (the base wake) instead.
pub fn warnings(entries: &[ZeroAngle], fin: Option<(f64, f64)>, rans: bool) -> Vec<String> {
    let hint = match fin {
        _ if rans => "; a symmetric body has none: it is the asymmetric steady base wake of the mesh (force on the body \
                      ahead of the base and on the fins next to it), which grows with mesh refinement"
            .to_string(),
        Some((w, t)) if t > 0.0 => format!(
            "; a symmetric body has none: it is a spurious fin lift of under-resolved fins (wall_size {:.1} mm, fin thickness {:.1} mm; \
             surface elements of about a quarter of the fin thickness remove it)",
            w * 1e3,
            t * 1e3
        ),
        _ => "; a symmetric body has none (numerical asymmetry of the mesh)".into(),
    };
    entries
        .iter()
        .filter(|e| e.ratio > WARN_RATIO)
        .map(|e| {
            let force = match e.cy {
                Some(cy) => format!("CY = {:+.4}, CZ = {:+.4}", cy, e.cz),
                None => format!("CN = {:+.4}", -e.cz),
            };
            let fate = if e.ratio_kept > WARN_RATIO {
                format!(
                    "{:.0} % of it stays in the table as a steady force (alpha_mode = \"single\"; \"mirror\" or \"offset\" remove it)",
                    100.0 * e.ratio_kept / e.ratio
                )
            } else {
                "the table removes it by symmetry / alpha_mode, but CN at other angles has an error of the same origin".to_string()
            };
            format!(
                "[IY-F509] zero-angle asymmetry at M={}: {} at alpha = beta = 0 ({:.1} % of CN at {} deg = {:.3}){}; {}",
                e.mach,
                force,
                100.0 * e.ratio,
                REF_ALPHA_DEG,
                e.cn_ref,
                hint,
                fate
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::forces::BodyCoeffs6;

    fn slopes(mach: f64, cna: f64) -> Slopes {
        Slopes { mach, cna_pitch: cna, xcp_pitch: 1.0, cna_yaw: None, xcp_yaw: None }
    }

    #[test]
    fn kept_components_follow_symmetry_and_mode() {
        use AlphaMode::*;
        use ShapeSymmetry::*;
        assert_eq!(kept_components(false, Axisymmetric, Mirror), (false, false));
        assert_eq!(kept_components(false, Axisymmetric, Offset), (false, false));
        assert_eq!(kept_components(false, Axisymmetric, Single), (false, true));
        assert_eq!(kept_components(true, Axisymmetric, Single), (false, false));
        assert_eq!(kept_components(true, PitchPlane, Single), (false, true));
        assert_eq!(kept_components(true, PitchPlane, Mirror), (false, false));
        assert_eq!(kept_components(true, None, Single), (true, true));
        assert_eq!(kept_components(true, None, Mirror), (false, false));
        assert_eq!(kept_components(true, None, Offset), (false, false));
    }

    /// The measured case: full-domain mesh of a shape without mirror symmetry, M 0.8.
    #[test]
    fn general_entry_reports_and_warns_the_kept_side_force() {
        let c = BodyCoeffs6 { cx: 0.3, cy: 0.116, cz: 0.02, cl: 0.0, cm: 0.0, cn: -1.4 };
        let pts = [
            SolvedPoint6 { mach: 0.8, alpha_deg: 0.0, beta_deg: 0.0, coeffs: Some(c) },
            SolvedPoint6 { mach: 0.8, alpha_deg: 4.0, beta_deg: 0.0, coeffs: Some(c) },
            SolvedPoint6 { mach: 2.0, alpha_deg: 0.0, beta_deg: 0.0, coeffs: Some(BodyCoeffs6 { cy: 0.001, cz: 0.0, ..c }) },
        ];
        // CNa = 17 /rad -> CN(4 deg) = 1.187.
        let s = [slopes(0.8, 17.0), slopes(2.0, 12.0)];
        let e = from_general(&pts, &s, ShapeSymmetry::None, AlphaMode::Single);
        assert_eq!(e.len(), 2);
        let r = 0.116f64.hypot(0.02) / (17.0 * 4f64.to_radians());
        assert!((e[0].ratio - r).abs() < 1e-12 && (e[0].ratio_kept - r).abs() < 1e-12, "{e:?}");
        let w = warnings(&e, Some((0.005, 0.004)), false);
        assert_eq!(w.len(), 1, "{w:?}");
        assert!(
            w[0].starts_with("[IY-F509] zero-angle asymmetry at M=0.8: CY = +0.1160, CZ = +0.0200 at alpha = beta = 0 (9.9 % of CN at 4 deg = 1.187)"),
            "{}",
            w[0]
        );
        assert!(w[0].contains("wall_size 5.0 mm, fin thickness 4.0 mm") && w[0].contains("100 % of it stays in the table"), "{}", w[0]);
        // The mirror and offset modes remove it: still reported, no longer kept.
        for mode in [AlphaMode::Mirror, AlphaMode::Offset] {
            let o = from_general(&pts, &s, ShapeSymmetry::None, mode);
            assert_eq!(o[0].ratio_kept, 0.0);
            assert!(warnings(&o, None, false)[0].contains("the table removes it"));
        }
        // Pitch-plane shape, single mode: CY is zeroed at beta = 0, CZ stays.
        let p = from_general(&pts, &s, ShapeSymmetry::PitchPlane, AlphaMode::Single);
        assert!((p[0].ratio_kept - 0.02 / (17.0 * 4f64.to_radians())).abs() < 1e-12);
    }

    #[test]
    fn axisymmetric_entry_uses_the_measured_cn0() {
        let a = [Asymmetry { mach: 0.8, cn0: -0.139, mom0: -0.2 }, Asymmetry { mach: 3.0, cn0: 0.1, mom0: 0.0 }];
        // M 3 has no slope: skipped.
        let e = from_axisymmetric(&a, &[slopes(0.8, 17.0)], AlphaMode::Mirror);
        assert_eq!(e.len(), 1);
        assert_eq!((e[0].cy, e[0].cz, e[0].ratio_kept), (None, 0.139, 0.0));
        let w = warnings(&e, None, false);
        assert!(w[0].contains("CN = -0.1390") && w[0].contains("11.7 %"), "{}", w[0]);
        assert!(warnings(&e, Some((0.008, 0.004)), true)[0].contains("asymmetric steady base wake"));
        assert_eq!(from_axisymmetric(&a, &[slopes(0.8, 17.0)], AlphaMode::Single)[0].ratio_kept, e[0].ratio);
        // Below the threshold: no warning.
        let small = [Asymmetry { mach: 0.8, cn0: 0.01, mom0: 0.0 }];
        assert!(warnings(&from_axisymmetric(&small, &[slopes(0.8, 17.0)], AlphaMode::Mirror), None, false).is_empty());
    }
}
