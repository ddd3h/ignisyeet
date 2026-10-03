//! Component build-up aerodynamics (Barrowman with the supersonic and drag extensions
//! described in S. Niskanen, "Development of an Open Source model rocket simulation
//! software", 2009 — the basis of OpenRocket).
//!
//! Coordinates: x is measured aft from the nose tip [m]. Coefficients use the maximum
//! body cross-section as reference area.

use crate::atmosphere::AtmosphereModel;
use geom::math::hermite;
use geom::{FinSet, Geometry};
use serde::{Deserialize, Serialize};
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AeroOptions {
    pub mach_min: f64,
    pub mach_max: f64,
    pub mach_step: f64,
    pub alpha_max_deg: f64,
    pub alpha_step_deg: f64,
    /// Altitude [m MSL] used for the Reynolds number of the skin-friction model.
    pub reference_altitude: f64,
    /// Atmosphere used for the Reynolds number of the skin-friction model.
    pub atmosphere: AtmosphereModel,
    /// Equivalent sand roughness [m].
    pub roughness: f64,
    /// Rounded (true) or sharp (false) fin leading edges.
    pub fin_le_rounded: bool,
    /// Blunt (square) fin trailing edges that produce base drag.
    pub fin_te_square: bool,
    /// Constant drag increment for lugs, buttons, etc.
    pub extra_cd: f64,
    /// Nozzle exit area removed from the base during powered flight [m^2].
    pub nozzle_exit_area: f64,
}

impl Default for AeroOptions {
    fn default() -> Self {
        Self {
            mach_min: 0.0,
            mach_max: 3.0,
            mach_step: 0.05,
            alpha_max_deg: 30.0,
            alpha_step_deg: 1.0,
            reference_altitude: 0.0,
            atmosphere: AtmosphereModel::default(),
            roughness: 60e-6,
            fin_le_rounded: true,
            fin_te_square: true,
            extra_cd: 0.0,
            nozzle_exit_area: 0.0,
        }
    }
}

/// Barrowman normal-force derivative of one body segment and its centre of pressure.
#[derive(Debug, Clone, Copy)]
struct BodyPart {
    cna: f64,
    xcp: f64,
}

pub struct AeroModel {
    pub geom: Geometry,
    pub opt: AeroOptions,
    parts: Vec<BodyPart>,
    /// Sum of body CNa [1/rad], moment sum about the nose [m/rad], second moment [m^2/rad].
    body_s: [f64; 3],
}

const TRANSONIC_LO: f64 = 0.9;
const TRANSONIC_HI: f64 = 1.5;

fn beta(m: f64) -> f64 {
    (1.0 - m * m).abs().sqrt().max(1e-3)
}

impl AeroModel {
    pub fn new(geom: Geometry, opt: AeroOptions) -> Self {
        let aref = geom.ref_area;
        let mut pts = vec![(0.0, 0.0)];
        pts.extend(geom.profile.iter().map(|p| (p.x, p.r)));
        pts.push((geom.length, geom.base_radius));
        let mut parts = Vec::new();
        for w in pts.windows(2) {
            let ((x1, r1), (x2, r2)) = (w[0], w[1]);
            let l = x2 - x1;
            if l <= 0.0 {
                continue;
            }
            let (a1, a2) = (PI * r1 * r1, PI * r2 * r2);
            let da = a2 - a1;
            if da.abs() < 1e-14 {
                continue;
            }
            let vol = PI * l / 3.0 * (r1 * r1 + r1 * r2 + r2 * r2);
            let cna = 2.0 * da / aref;
            let xcp = x1 + (l * a2 - vol) / da;
            parts.push(BodyPart { cna, xcp });
        }
        let body_s = parts.iter().fold([0.0; 3], |s, p| [s[0] + p.cna, s[1] + p.cna * p.xcp, s[2] + p.cna * p.xcp * p.xcp]);
        Self { geom, opt, parts, body_s }
    }

    pub fn body_cna(&self) -> f64 {
        self.body_s[0]
    }

    pub fn body_xcp(&self) -> f64 {
        if self.body_s[0].abs() > 1e-12 { self.body_s[1] / self.body_s[0] } else { 0.0 }
    }

    pub fn body_part_count(&self) -> usize {
        self.parts.len()
    }

    fn fin_count_factor(n: usize) -> f64 {
        let k = match n {
            0..=4 => 1.0,
            5 => 0.948,
            6 => 0.913,
            7 => 0.854,
            8 => 0.81,
            _ => 0.75,
        };
        k * n as f64 / 2.0
    }

    fn fin_scale(&self, f: &FinSet) -> f64 {
        let kfb = 1.0 + f.body_radius / (f.span + f.body_radius);
        kfb * Self::fin_count_factor(f.count)
    }

    /// Fin-set normal force (including body interference) at Mach `m`, angle `a` [rad].
    pub fn fin_cn(&self, m: f64, a: f64) -> f64 {
        let Some(f) = &self.geom.fins else { return 0.0 };
        if f.span <= 0.0 || f.area() <= 0.0 {
            return 0.0;
        }
        if m <= TRANSONIC_LO {
            self.fin_cn_sub(f, m, a)
        } else if m >= TRANSONIC_HI {
            self.fin_cn_sup(f, m, a)
        } else {
            let h = 1e-3;
            let (f0, f1) = (self.fin_cn_sub(f, TRANSONIC_LO, a), self.fin_cn_sup(f, TRANSONIC_HI, a));
            let d0 = (f0 - self.fin_cn_sub(f, TRANSONIC_LO - h, a)) / h;
            let d1 = (self.fin_cn_sup(f, TRANSONIC_HI + h, a) - f1) / h;
            hermite(m, TRANSONIC_LO, TRANSONIC_HI, f0, f1, d0, d1)
        }
    }

    fn fin_cn_sub(&self, f: &FinSet, m: f64, a: f64) -> f64 {
        let s2 = f.span * f.span;
        let x = beta(m) * s2 / (f.area() * f.midchord_sweep().cos());
        let cna1 = 2.0 * PI * s2 / self.geom.ref_area / (1.0 + (1.0 + x * x).sqrt());
        self.fin_scale(f) * cna1 * a
    }

    fn fin_cn_sup(&self, f: &FinSet, m: f64, a: f64) -> f64 {
        let g = crate::atmosphere::GAMMA;
        let b = beta(m);
        let m2 = m * m;
        let k1 = 2.0 / b;
        let k3 = ((g + 1.0) * m2.powi(4) + (2.0 * g * g - 7.0 * g - 5.0) * m2.powi(3) + 10.0 * (g + 1.0) * m2 * m2 + 8.0)
            / (6.0 * b.powi(7));
        // Flat-plate Busemann (both surfaces) with a linear-theory tip-loss factor.
        let ar = 2.0 * f.span * f.span / f.area();
        let tip = (1.0 - 1.0 / (2.0 * b * ar)).clamp(0.5, 1.0);
        self.fin_scale(f) * f.area() / self.geom.ref_area * (2.0 * k1 * a + 2.0 * k3 * a * a * a) * tip
    }

    /// Fin centre of pressure (distance aft of the nose tip) at Mach `m`.
    pub fn fin_xcp(&self, m: f64) -> f64 {
        let Some(f) = &self.geom.fins else { return 0.0 };
        let ar = 2.0 * f.span * f.span / f.area().max(1e-12);
        let sup = |m: f64| {
            let ab = ar * beta(m);
            ((ab - 0.67) / (2.0 * ab - 1.0)).clamp(0.25, 0.5)
        };
        let frac = if m <= 0.5 {
            0.25
        } else if m >= 2.0 {
            sup(m)
        } else {
            let h = 1e-3;
            hermite(m, 0.5, 2.0, 0.25, sup(2.0), 0.0, (sup(2.0 + h) - sup(2.0)) / h)
        };
        f.x_le_root + f.mac_le_offset() + frac * f.mac()
    }

    /// Total normal force coefficient and centre of pressure at Mach `m`, angle `a` [rad].
    pub fn normal(&self, m: f64, a: f64) -> (f64, f64) {
        let a = a.max(1e-6);
        let (sa, ca) = a.sin_cos();
        let cn_body = self.body_s[0] * sa * ca;
        let cn_lift = 1.1 * self.geom.planform_area / self.geom.ref_area * sa * sa;
        let cn_fin = self.fin_cn(m, a);
        let xf = self.fin_xcp(m);
        let cn = cn_body + cn_lift + cn_fin;
        let mom = self.body_s[1] * sa * ca + cn_lift * self.geom.planform_centroid + cn_fin * xf;
        (cn, if cn.abs() > 1e-12 { mom / cn } else { 0.0 })
    }

    /// Normal-force slope at zero angle of attack [1/rad].
    pub fn cna(&self, m: f64) -> f64 {
        let h = 1e-4;
        self.normal(m, h).0 / h
    }

    /// Sums Σ CNa_i, Σ CNa_i x_i, Σ CNa_i x_i² used for pitch damping.
    pub fn damping_sums(&self, m: f64) -> [f64; 3] {
        let h = 1e-4;
        let cnf = self.fin_cn(m, h) / h;
        let xf = self.fin_xcp(m);
        [self.body_s[0] + cnf, self.body_s[1] + cnf * xf, self.body_s[2] + cnf * xf * xf]
    }

    /// Zero-lift axial (drag) coefficient at Mach `m`.
    pub fn cd0(&self, m: f64, power_on: bool) -> DragBreakdown {
        let g = &self.geom;
        let aref = g.ref_area;
        let atm = self.opt.atmosphere.at(self.opt.reference_altitude);
        // Floor the Reynolds-number Mach so the M = 0 table node stays representative of slow flight.
        let v = m.max(0.05) * atm.sound_speed;
        let re = v * g.length / atm.kinematic_viscosity();
        let cf = skin_friction(re, m, self.opt.roughness / g.length);
        let fineness = g.length / (2.0 * g.ref_radius);
        let mut wet = (1.0 + 1.0 / (2.0 * fineness)) * g.wetted_area_body;
        if let Some(f) = &g.fins {
            let tc = if f.mac() > 0.0 { f.thickness / f.mac() } else { 0.0 };
            wet += (1.0 + 2.0 * tc) * 2.0 * f.count as f64 * f.area();
        }
        let friction = cf * wet / aref;

        let nose = self.nose_pressure_cd(m) * PI * g.nose.base_radius.powi(2) / aref;

        let mut base_area = PI * g.base_radius * g.base_radius;
        if power_on {
            base_area = (base_area - self.opt.nozzle_exit_area).max(0.0);
        }
        let base = base_cd(m) * base_area / aref;

        let mut fins = 0.0;
        if let Some(f) = &g.fins {
            let frontal = f.count as f64 * f.span * f.thickness / aref;
            if self.opt.fin_le_rounded {
                fins += le_cd(m) * f.le_sweep().cos().powi(2) * frontal;
            }
            if self.opt.fin_te_square {
                fins += base_cd(m) * frontal;
            }
        }

        let mut boattail = 0.0;
        if let Some(b) = &g.boattail {
            let gamma = b.length / (2.0 * (b.r_fore - b.r_aft)).max(1e-9);
            let k = if gamma < 1.0 { 1.0 } else if gamma > 3.0 { 0.0 } else { (3.0 - gamma) / 2.0 };
            boattail = k * base_cd(m) * PI * (b.r_fore.powi(2) - b.r_aft.powi(2)) / aref;
        }

        DragBreakdown { friction, nose, base, fins, boattail, extra: self.opt.extra_cd, reynolds: re }
    }

    fn nose_pressure_cd(&self, m: f64) -> f64 {
        let n = &self.geom.nose;
        if n.length <= 0.0 || n.base_radius <= 0.0 {
            return 0.0;
        }
        let sj = n.joint_half_angle_deg.to_radians().sin();
        let sub = 0.8 * sj * sj;
        let phi = (n.base_radius / n.length).atan();
        let shape = if n.volume_ratio <= 1.0 / 3.0 {
            1.0
        } else {
            (1.0 - 0.4 * (n.volume_ratio - 1.0 / 3.0) / (0.55 - 1.0 / 3.0)).max(0.6)
        };
        let sup = |m: f64| shape * (2.1 * phi.sin().powi(2) + 0.5 * phi.sin() / (m * m - 1.0).sqrt());
        if m <= 0.8 {
            sub
        } else if m >= 1.3 {
            sup(m)
        } else {
            let h = 1e-3;
            hermite(m, 0.8, 1.3, sub, sup(1.3), 0.0, (sup(1.3 + h) - sup(1.3)) / h)
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct DragBreakdown {
    pub friction: f64,
    pub nose: f64,
    pub base: f64,
    pub fins: f64,
    pub boattail: f64,
    pub extra: f64,
    pub reynolds: f64,
}

impl DragBreakdown {
    pub fn total(&self) -> f64 {
        self.friction + self.nose + self.base + self.fins + self.boattail + self.extra
    }
}

/// Skin-friction coefficient with roughness limit and compressibility corrections.
pub fn skin_friction(re: f64, m: f64, rel_rough: f64) -> f64 {
    if re < 1e4 {
        return 1.48e-2;
    }
    let turb = 1.0 / (1.50 * re.ln() - 5.6).powi(2);
    let turb = if m < 1.0 { turb * (1.0 - 0.1 * m * m) } else { turb / (1.0 + 0.15 * m * m).powf(0.58) };
    if rel_rough <= 0.0 {
        return turb;
    }
    let rough = 0.032 * rel_rough.powf(0.2);
    let rough = if m < 1.0 { rough * (1.0 - 0.1 * m * m) } else { rough / (1.0 + 0.18 * m * m) };
    turb.max(rough)
}

/// Base drag coefficient (relative to base area).
pub fn base_cd(m: f64) -> f64 {
    if m < 1.0 { 0.12 + 0.13 * m * m } else { 0.25 / m }
}

/// Rounded leading-edge pressure drag (relative to frontal area).
fn le_cd(m: f64) -> f64 {
    if m < 0.9 {
        (1.0 - m * m).powf(-0.417) - 1.0
    } else if m < 1.0 {
        1.0 - 1.785 * (m - 0.9)
    } else {
        1.214 - 0.502 / (m * m) + 0.1095 / m.powi(4)
    }
}

/// Multiplier applied to the zero-lift axial coefficient as a function of angle of attack.
pub fn axial_alpha_factor(a: f64) -> f64 {
    let a = a.abs().min(PI / 2.0);
    let a17 = 17f64.to_radians();
    if a <= a17 {
        hermite(a, 0.0, a17, 1.0, 1.3, 0.0, 0.0)
    } else {
        hermite(a, a17, PI / 2.0, 1.3, 0.0, 0.0, 0.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use geom::sample::SampleRocket;
    use geom::{extract, ExtractOptions};

    fn model() -> (AeroModel, SampleRocket) {
        let rk = SampleRocket::default();
        let g = extract(&rk.mesh(), &ExtractOptions::default()).unwrap();
        (AeroModel::new(g, AeroOptions::default()), rk)
    }

    #[test]
    fn body_cna_is_two() {
        // Nose-plus-cylinder with flat base: slender-body CNa = 2.
        let (m, _) = model();
        assert!((m.body_cna() - 2.0).abs() < 0.02, "body cna {}", m.body_cna());
        // Tangent ogive CP ≈ 0.466 L_nose.
        assert!((m.body_xcp() - 0.466 * 0.30).abs() < 0.01, "body xcp {}", m.body_xcp());
    }

    #[test]
    fn matches_hand_barrowman() {
        let (m, rk) = model();
        // Hand calculation of the classic Barrowman equations for the nominal geometry.
        let (a, b, s, sw, d) = (rk.root_chord, rk.tip_chord, rk.span, rk.sweep, 2.0 * rk.radius);
        let lf = (s * s + (sw + b / 2.0 - a / 2.0).powi(2)).sqrt();
        let kfb = 1.0 + (d / 2.0) / (s + d / 2.0);
        let cna_f = kfb * 4.0 * 4.0 * (s / d).powi(2) / (1.0 + (1.0 + (2.0 * lf / (a + b)).powi(2)).sqrt());
        let x_f = (rk.length - a) + sw * (a + 2.0 * b) / (3.0 * (a + b)) + (a + b - a * b / (a + b)) / 6.0;
        let cna_n = 2.0;
        let x_n = 0.466 * rk.nose_length;
        let xcp = (cna_n * x_n + cna_f * x_f) / (cna_n + cna_f);
        let mm = 0.01;
        let cna = m.cna(mm);
        let (_, x) = m.normal(mm, 1e-4);
        assert!((cna - (cna_n + cna_f)).abs() / (cna_n + cna_f) < 0.03, "cna {cna} vs {}", cna_n + cna_f);
        assert!((x - xcp).abs() < 0.015, "xcp {x} vs {xcp}");
    }

    #[test]
    fn continuous_through_transonic() {
        let (m, _) = model();
        let a = 2f64.to_radians();
        let mut prev = m.normal(0.5, a).0;
        let mut prev_cd = m.cd0(0.5, false).total();
        let mut mach = 0.5;
        while mach < 2.5 {
            mach += 0.005;
            let cn = m.normal(mach, a).0;
            let cd = m.cd0(mach, false).total();
            assert!((cn - prev).abs() < 0.02 * prev.abs().max(0.1), "CN jump at M={mach}: {prev} -> {cn}");
            assert!((cd - prev_cd).abs() < 0.03 * prev_cd.max(0.1), "CD jump at M={mach}: {prev_cd} -> {cd}");
            prev = cn;
            prev_cd = cd;
        }
    }

    #[test]
    fn drag_is_plausible() {
        let (m, _) = model();
        let cd_sub = m.cd0(0.3, false).total();
        let cd_trans = m.cd0(1.1, false).total();
        assert!(cd_sub > 0.25 && cd_sub < 0.7, "subsonic CD {cd_sub}");
        assert!(cd_trans > cd_sub, "transonic rise {cd_trans}");
    }

    #[test]
    fn alpha_factor_shape() {
        assert!((axial_alpha_factor(0.0) - 1.0).abs() < 1e-12);
        assert!((axial_alpha_factor(17f64.to_radians()) - 1.3).abs() < 1e-12);
        assert!(axial_alpha_factor(PI / 2.0).abs() < 1e-12);
    }
}
