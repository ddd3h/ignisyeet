// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
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
            alpha_max_deg: 90.0,
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

/// Upper angle of attack [rad] of the attached-flow fin theory (Barrowman linear, Busemann cubic).
///
/// Linear fin theory assumes small angles; OpenRocket's fin model (FinSetCalc, after Niskanen 2009) caps the
/// fin angle of attack at a 20° stall angle. 15° keeps a margin below that and
/// keeps the Busemann cubic term a correction (it adds at most about 20 % to the linear term at 15°, at
/// M ≈ 1.2 through the transonic blend) instead of letting it dominate, as it does at large angles where
/// the expansion is no longer valid (the shock detaches once alpha exceeds the maximum wedge deflection,
/// 12° at M = 1.5, 23° at M = 2).
pub const FIN_ALPHA_LINEAR: f64 = 15.0 * PI / 180.0;

fn beta(m: f64) -> f64 {
    (1.0 - m * m).abs().sqrt().max(1e-3)
}

/// Drag coefficient of a finite flat plate normal to the flow, on its planform area.
///
/// Subsonic: 1.2 for plates of aspect ratio 1–5 (Hoerner, Fluid-Dynamic Drag, 1965, ch. 3: 1.17–1.2;
/// the two-dimensional value 2.0 applies only to very long plates). Supersonic: 1.7, within the
/// range 1.6–1.8 of plates and disks normal to supersonic flow (Hoerner 1965); the windward face carries
/// at most the pitot pressure behind the normal shock (pressure coefficient 1.66 at M = 2, 1.76 at M = 3)
/// and the base adds a suction that falls with Mach. C1 Hermite rise between M = 0.8 and 1.2.
pub fn plate_cd(m: f64) -> f64 {
    const SUB: f64 = 1.2;
    const SUP: f64 = 1.7;
    if m <= 0.8 {
        SUB
    } else if m >= 1.2 {
        SUP
    } else {
        hermite(m, 0.8, 1.2, SUB, SUP, 0.0, 0.0)
    }
}

/// Supersonic fin centre of pressure as a fraction of the mean aerodynamic chord, from `ab = A beta`.
///
/// For `A beta >= 1.2` the expression of Fleeman (Tactical Missile Design, 2001), used by Niskanen
/// (2009) for M >= 2: `(A beta - 0.67) / (2 A beta - 1)`, which rises from 0.38 at 1.2 to 0.5 as
/// `A beta -> inf`. It has a pole at `A beta = 0.5` and falls to the quarter chord at `A beta = 0.84`,
/// so it is not used for small `A beta`. There the low-aspect-ratio (slender-wing) limit applies: the
/// loading of a low-aspect-ratio wing does not depend on Mach number (R. T. Jones, "Properties of
/// low-aspect-ratio pointed wings at speeds below and above the speed of sound", NACA Report 835,
/// 1946), so the centre of pressure keeps its subsonic position, the quarter chord, for
/// `A beta <= 0.84`. Between 0.84 and 1.2 a monotone C1 cubic Hermite in `A beta` joins the two
/// (zero slope at 0.84, the slope of the expression at 1.2). The result is continuous in Mach and in
/// the fin geometry and lies in [0.25, 0.5].
pub fn supersonic_fin_cp_fraction(ab: f64) -> f64 {
    const LO: f64 = 0.84;
    const HI: f64 = 1.2;
    let fleeman = |x: f64| (x - 0.67) / (2.0 * x - 1.0);
    if ab <= LO {
        0.25
    } else if ab >= HI {
        fleeman(ab).min(0.5)
    } else {
        let d_hi = 0.34 / (2.0 * HI - 1.0).powi(2);
        hermite(ab, LO, HI, 0.25, fleeman(HI), 0.0, d_hi)
    }
}

/// Bounds of the shape parameter `u = k t_end` of [`crossflow_blend`]. Within them the transition is the
/// pure exponential saturation; beyond them (the attached-flow value close to the crossflow value, or
/// an attached-flow slope near zero) `u` is held and the remaining slope goes into a bump that vanishes
/// at both ends. The sample rocket stays inside (u = 3.7 .. 7.4 over Mach 0 .. 3).
const BLEND_U_MIN: f64 = -20.0;
const BLEND_U_MAX: f64 = 20.0;

/// C1 transition from the attached-flow normal force to the crossflow value, continuous in its parameters.
///
/// `t` is the angle beyond the validity angle, `t_end = T` the span up to 90°, `cn1` and `d0` the value
/// and slope at the validity angle, `cn90` the crossflow value at 90°, `delta = cn90 - cn1`. With
/// `tau = T sin(pi t / (2 T))`, `s = t / T` and the bump `b(t) = t (1 - s)^2` (b = 0 at both ends,
/// b'(0) = 1, b'(T) = 0) it returns
///
/// `cn1 + delta (1 - e^{-u tau / T}) / (1 - e^{-u}) + (d0 - d_e) b(t)`,  `d_e = delta (pi / 2) g(u) / T`,
///
/// with `g(u) = u / (1 - e^{-u})` the slope factor of the exponential term at `t = 0`. For `delta > 0`
/// `u` solves `g(u) = d0 T / (delta pi / 2)`, so that the exponential alone carries the slope `d0` and
/// the bump vanishes; the curve is then monotone (any `u`, positive or negative). `u` is held in
/// [`BLEND_U_MIN`, `BLEND_U_MAX`], and the slope the exponential cannot carry goes into the bump. For
/// `delta <= 0` (the attached-flow value at or above the crossflow value: large-aspect-ratio fins, which
/// stall) the exponential term is replaced by `delta (3 s^2 - 2 s^3)`, which makes the whole expression
/// the cubic Hermite with zero end slope: a bounded overshoot that relaxes to `cn90`.
///
/// Value and slope match at `t = 0`, the value is `cn90` and the slope zero at `t = T`. Both branches
/// tend to `cn1 + d0 b(t)` as `delta -> 0` (the exponential term and `d_e` are bounded multiples of
/// `delta` because `u` is bounded), so the result is continuous in `cn1`, `cn90` and `d0`.
fn crossflow_blend(t: f64, t_end: f64, cn1: f64, d0: f64, cn90: f64) -> f64 {
    let t = t.clamp(0.0, t_end);
    let s = t / t_end;
    let bump = t * (1.0 - s) * (1.0 - s);
    let delta = cn90 - cn1;
    if delta <= 0.0 {
        return cn1 + delta * s * s * (3.0 - 2.0 * s) + d0 * bump;
    }
    // d/dt at t = 0 of the exponential term: delta (pi/2) g(u) / T.
    let r = d0 * t_end / (delta * PI / 2.0);
    let g = |u: f64| if u.abs() < 1e-12 { 1.0 } else { u / -(-u).exp_m1() };
    // g increases from 0 (u -> -inf) through 1 (u = 0) to ~u (u -> +inf).
    let u = if r <= g(BLEND_U_MIN) {
        BLEND_U_MIN
    } else if r >= g(BLEND_U_MAX) {
        BLEND_U_MAX
    } else {
        let (mut lo, mut hi) = (BLEND_U_MIN, BLEND_U_MAX);
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if g(mid) < r {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    };
    let d_e = delta * (PI / 2.0) * g(u) / t_end;
    let tau = t_end * (PI * t / (2.0 * t_end)).sin();
    let frac = if u.abs() < 1e-12 { tau / t_end } else { (-u * tau / t_end).exp_m1() / (-u).exp_m1() };
    cn1 + delta * frac + (d0 - d_e) * bump
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
        if self.body_s[0].abs() > 1e-12 {
            self.body_s[1] / self.body_s[0]
        } else {
            0.0
        }
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
    ///
    /// Up to [`FIN_ALPHA_LINEAR`] this is the attached-flow theory of [`fin_cn_attached`](Self::fin_cn_attached)
    /// (Barrowman linear subsonic, Busemann linear + cubic supersonic). Beyond it the fins are treated as
    /// flat plates in crossflow: the normal force rises monotonically, C1 at the validity angle, to the
    /// flat-plate value at 90°,
    ///
    /// `CN(90°) = k_N C_D,plate(M) A_fin / A_ref`,
    ///
    /// where `k_N` is the Barrowman fin-count factor (N/2 fins effective, the projection of N fins evenly
    /// spaced in roll) and `C_D,plate` the drag coefficient of a flat plate normal to the flow
    /// ([`plate_cd`]). The body-interference factor K_fb is a small-angle potential-flow upwash and is not
    /// applied to the crossflow value. Between the two the curve is the exponential saturation
    ///
    /// `CN = CN_1 + (CN_90 - CN_1) (1 - e^{-k tau}) / (1 - e^{-k T})`,  `tau = T sin(pi t / (2 T))`,
    ///
    /// with `t = alpha - alpha_1`, `T = 90° - alpha_1`; `k` is chosen so that the slope at `alpha_1` equals the
    /// attached-flow slope there, and `tau` makes the slope zero at 90° (flat plate symmetric about 90°).
    /// When `CN_1` approaches or exceeds `CN_90` the curve turns continuously into a cubic Hermite with
    /// a bounded overshoot (see [`crossflow_blend`]), so the force stays continuous in Mach and geometry.
    /// The resulting shape — a knee above the validity angle levelling off to the crossflow value — follows
    /// the measured normal force of low-aspect-ratio flat plates, which per unit plate area reaches about
    /// 1.0 at 30° and 1.1–1.2 between 45° and 90° (Hoerner, Fluid-Dynamic Drag, 1965, ch. 3; Fluid-Dynamic
    /// Lift, 1985). For the sample fins (4 fins, aspect ratio of an opposing pair 1.3) this model gives 1.05
    /// per unit area at 30° and M = 0.3. Angles above 90° (reverse flow) are mirrored about 90°; the
    /// function is odd in `a`.
    pub fn fin_cn(&self, m: f64, a: f64) -> f64 {
        let sign = a.signum();
        let a = a.abs().min(PI);
        let a = if a > PI / 2.0 { PI - a } else { a };
        if a <= FIN_ALPHA_LINEAR {
            return sign * self.fin_cn_attached(m, a);
        }
        let Some(f) = &self.geom.fins else { return 0.0 };
        if f.span <= 0.0 || f.area() <= 0.0 {
            return 0.0;
        }
        let a1 = FIN_ALPHA_LINEAR;
        let cn1 = self.fin_cn_attached(m, a1);
        let h = 1e-5;
        let d0 = (cn1 - self.fin_cn_attached(m, a1 - h)) / h;
        let cn90 = Self::fin_count_factor(f.count) * plate_cd(m) * f.area() / self.geom.ref_area;
        sign * crossflow_blend(a - a1, PI / 2.0 - a1, cn1, d0, cn90)
    }

    /// Attached-flow fin-set normal force (Barrowman / Busemann, valid at small `a`).
    fn fin_cn_attached(&self, m: f64, a: f64) -> f64 {
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
        let k3 = ((g + 1.0) * m2.powi(4) + (2.0 * g * g - 7.0 * g - 5.0) * m2.powi(3) + 10.0 * (g + 1.0) * m2 * m2 + 8.0) / (6.0 * b.powi(7));
        // Flat-plate Busemann (both surfaces) with a linear-theory tip-loss factor. The expansion in alpha
        // holds only for small angles (attached shock); `fin_cn` uses it up to FIN_ALPHA_LINEAR only.
        let ar = 2.0 * f.span * f.span / f.area();
        let tip = (1.0 - 1.0 / (2.0 * b * ar)).clamp(0.5, 1.0);
        self.fin_scale(f) * f.area() / self.geom.ref_area * (2.0 * k1 * a + 2.0 * k3 * a * a * a) * tip
    }

    /// Fin centre of pressure (distance aft of the nose tip) at Mach `m`.
    ///
    /// Subsonic (M <= 0.5): the quarter chord of the mean aerodynamic chord (Barrowman). Supersonic
    /// (M >= 2): the fraction [`supersonic_fin_cp_fraction`] of the MAC from `A beta`, `A` the aspect
    /// ratio of an opposing fin pair `2 s^2 / A_fin`. In between a C1 Hermite in Mach (Niskanen 2009).
    pub fn fin_xcp(&self, m: f64) -> f64 {
        let Some(f) = &self.geom.fins else { return 0.0 };
        let ar = 2.0 * f.span * f.span / f.area().max(1e-12);
        let sup = |m: f64| supersonic_fin_cp_fraction(ar * beta(m));
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

    /// Fin centre of pressure at Mach `m` and angle `a` [rad].
    ///
    /// Equal to [`fin_xcp`](Self::fin_xcp) up to [`FIN_ALPHA_LINEAR`]; beyond it the chordwise position
    /// moves with a C1 smoothstep to the planform centroid at 90°, where the crossflow pressure of a flat
    /// plate normal to the flow is symmetric about it. For a trapezoidal fin the centroid lies at half the
    /// mean aerodynamic chord. Above 90° (reverse flow, trailing edge leading) the fraction is mirrored to
    /// `1 - frac(180° - a)`, reaching three quarters of the mean aerodynamic chord at 180° in subsonic flow.
    pub fn fin_xcp_at(&self, m: f64, a: f64) -> f64 {
        let x0 = self.fin_xcp(m);
        let Some(f) = &self.geom.fins else { return x0 };
        let base = f.x_le_root + f.mac_le_offset();
        let mac = f.mac();
        if mac <= 0.0 {
            return x0;
        }
        let a = a.abs().min(PI);
        let (a, rev) = if a > PI / 2.0 { (PI - a, true) } else { (a, false) };
        let frac0 = (x0 - base) / mac;
        let frac = if a <= FIN_ALPHA_LINEAR { frac0 } else { hermite(a, FIN_ALPHA_LINEAR, PI / 2.0, frac0, 0.5, 0.0, 0.0) };
        base + if rev { 1.0 - frac } else { frac } * mac
    }

    /// Total normal force coefficient and centre of pressure at Mach `m`, angle `a` [rad].
    pub fn normal(&self, m: f64, a: f64) -> (f64, f64) {
        let a = a.max(1e-6);
        let (sa, ca) = a.sin_cos();
        let cn_body = self.body_s[0] * sa * ca;
        let cn_lift = 1.1 * self.geom.planform_area / self.geom.ref_area * sa * sa;
        let cn_fin = self.fin_cn(m, a);
        let xf = self.fin_xcp_at(m, a);
        let cn = cn_body + cn_lift + cn_fin;
        let mom = self.body_s[1] * sa * ca + cn_lift * self.geom.planform_centroid + cn_fin * xf;
        (cn, if cn.abs() > 1e-12 { mom / cn } else { 0.0 })
    }

    /// Normal-force slope at zero angle of attack [1/rad].
    pub fn cna(&self, m: f64) -> f64 {
        let h = 1e-4;
        self.normal(m, h).0 / h
    }

    /// Roll terms of the fin set at Mach `m`: (d Cl / d delta per radian of cant, d Cl / d p_hat).
    ///
    /// Cant forcing (Niskanen 2009): each fin's lift slope acts at the spanwise position of its
    /// mean aerodynamic chord, `Cl_delta = N (y_MAC + r) CNa1 / d`. Roll damping by strip theory:
    /// a strip at radius xi sees the extra angle `p xi / V`, giving
    /// `Clp = -(2 a N / (A d^2)) * integral(xi^2 c(xi) dxi)`. The strip lift slope `a` is the
    /// three-dimensional slope of one fin per unit fin area, `CNa1 A / A_fin`, so that the finite
    /// span (tip loss) is included; the two-dimensional slope would overestimate the damping of
    /// low-aspect-ratio fins several times. A and d are the reference area and diameter.
    pub fn roll_terms(&self, m: f64) -> (f64, f64) {
        let Some(f) = &self.geom.fins else { return (0.0, 0.0) };
        if f.span <= 0.0 || f.area() <= 0.0 || f.count == 0 {
            return (0.0, 0.0);
        }
        let d = 2.0 * self.geom.ref_radius;
        let n = f.count as f64;
        let (cr, ct, s, r) = (f.root_chord, f.tip_chord, f.span, f.body_radius);
        let y_mac = if cr + ct > 0.0 { s / 3.0 * (cr + 2.0 * ct) / (cr + ct) } else { 0.5 * s };
        // integral of xi^2 c(xi) over the span, c linear from cr at xi = r to ct at xi = r + s.
        let k = (ct - cr) / s;
        let prim = |xi: f64| (cr - k * r) * xi.powi(3) / 3.0 + k * xi.powi(4) / 4.0;
        let i2 = prim(r + s) - prim(r);
        // Single-fin normal-force slope (reference area A), blended like `fin_cn`.
        let a = 1e-4;
        let cna1 = self.fin_cn(m, a) / (a * self.fin_scale(f));
        let cl_delta = n * (y_mac + r) * cna1 / d;
        let slope = cna1 * self.geom.ref_area / f.area();
        let clp = -2.0 * slope * n * i2 / (self.geom.ref_area * d * d);
        (cl_delta, clp)
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
            let k = if gamma < 1.0 {
                1.0
            } else if gamma > 3.0 {
                0.0
            } else {
                (3.0 - gamma) / 2.0
            };
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
        let shape = if n.volume_ratio <= 1.0 / 3.0 { 1.0 } else { (1.0 - 0.4 * (n.volume_ratio - 1.0 / 3.0) / (0.55 - 1.0 / 3.0)).max(0.6) };
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
    if m < 1.0 {
        0.12 + 0.13 * m * m
    } else {
        0.25 / m
    }
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

    #[test]
    fn fin_normal_force_is_bounded_at_large_alpha() {
        let (m, _) = model();
        let f = m.geom.fins.clone().unwrap();
        for i in 0..=60 {
            let mach = 0.05 * i as f64;
            let cn90 = m.fin_cn(mach, PI / 2.0);
            // Flat-plate crossflow: k_N C_D,plate A_fin / A_ref, C_D,plate 1.2 subsonic .. 1.7 supersonic.
            let expect = AeroModel::fin_count_factor(f.count) * plate_cd(mach) * f.area() / m.geom.ref_area;
            assert!((cn90 - expect).abs() < 1e-9 * expect, "M={mach}: fin CN(90) {cn90} vs {expect}");
            assert!(cn90 > 2.0 * 1.2 * f.area() / m.geom.ref_area - 1e-9 && cn90 < 2.0 * 1.7 * f.area() / m.geom.ref_area + 1e-9);
            // Monotone non-decreasing from 0 to 90 deg, never above the 90 deg value.
            let mut prev = 0.0;
            for k in 1..=900 {
                let a = (0.1 * k as f64).to_radians();
                let cn = m.fin_cn(mach, a);
                assert!(cn >= prev - 1e-12, "M={mach}: fin CN decreases at {} deg: {prev} -> {cn}", 0.1 * k as f64);
                assert!(cn <= cn90 + 1e-12, "M={mach}: fin CN {cn} above its 90 deg value {cn90}");
                prev = cn;
            }
            // Total normal force stays within the crossflow estimate of the whole rocket.
            let (cn_tot, _) = m.normal(mach, PI / 2.0);
            assert!(cn_tot < 30.0, "M={mach}: total CN(90) {cn_tot}");
        }
        // Mirrored above 90 deg and odd in alpha.
        let a = 70f64.to_radians();
        assert!((m.fin_cn(1.2, PI - a) - m.fin_cn(1.2, a)).abs() < 1e-12);
        assert!((m.fin_cn(1.2, -a) + m.fin_cn(1.2, a)).abs() < 1e-12);
    }

    #[test]
    fn small_alpha_fin_normal_force_is_unchanged() {
        let (m, _) = model();
        // CNa of the sample rocket before the large-alpha fin model (body + Barrowman/Busemann fins).
        for (mach, cna) in [(0.3, 11.692), (0.8, 12.239), (1.2, 13.966), (1.5, 14.091), (2.0, 11.193), (3.0, 8.230)] {
            assert!((m.cna(mach) - cna).abs() < 6e-4, "M={mach}: CNa {} vs {cna}", m.cna(mach));
        }
        let f = m.geom.fins.clone().unwrap();
        for i in 0..=60 {
            let mach = 0.05 * i as f64;
            for k in 0..=150 {
                let a = (0.1 * k as f64).to_radians();
                assert_eq!(m.fin_cn(mach, a), m.fin_cn_attached(mach, a), "M={mach}, alpha {} deg", 0.1 * k as f64);
                assert_eq!(m.fin_xcp_at(mach, a), m.fin_xcp(mach));
            }
            // Slope at zero: the Barrowman / Busemann linear term.
            let lin = if mach <= TRANSONIC_LO {
                m.fin_cn_sub(&f, mach, 1.0)
            } else if mach >= TRANSONIC_HI {
                m.fin_cn_sup(&f, mach, 1e-6) / 1e-6
            } else {
                continue;
            };
            assert!((m.fin_cn(mach, 1e-6) / 1e-6 - lin).abs() < 1e-6 * lin, "M={mach}");
        }
    }

    #[test]
    fn fin_normal_force_is_c1_in_alpha_and_continuous_in_mach() {
        let (m, _) = model();
        let a1 = FIN_ALPHA_LINEAR;
        let h = 1e-6;
        for i in 0..=60 {
            let mach = 0.05 * i as f64;
            // Value and slope match at the validity angle.
            let (l, r) = (m.fin_cn(mach, a1 - h), m.fin_cn(mach, a1 + h));
            let c = m.fin_cn(mach, a1);
            let (dl, dr) = ((c - l) / h, (r - c) / h);
            assert!((r - l).abs() < 1e-4 * c, "M={mach}: jump at the validity angle");
            assert!((dl - dr).abs() < 1e-3 * dl, "M={mach}: slope {dl} -> {dr} at the validity angle");
            // Zero slope at 90 deg (C1 with the mirrored branch).
            let d90 = (m.fin_cn(mach, PI / 2.0) - m.fin_cn(mach, PI / 2.0 - h)) / h;
            assert!(d90.abs() < 1e-3, "M={mach}: slope at 90 deg {d90}");
            let (xl, xr) = (m.fin_xcp_at(mach, a1 - h), m.fin_xcp_at(mach, a1 + h));
            assert!((xl - xr).abs() < 1e-6);
        }
        for deg in [20.0f64, 30.0, 45.0, 60.0, 90.0] {
            let a = deg.to_radians();
            let mut prev = m.fin_cn(0.0, a);
            let mut prev_x = m.fin_xcp_at(0.0, a);
            let mut mach = 0.0;
            while mach < 3.0 {
                mach += 0.002;
                let cn = m.fin_cn(mach, a);
                let x = m.fin_xcp_at(mach, a);
                assert!((cn - prev).abs() < 0.01 * prev, "alpha {deg} deg: fin CN jump at M={mach}: {prev} -> {cn}");
                assert!((x - prev_x).abs() < 1e-3, "alpha {deg} deg: fin xcp jump at M={mach}");
                prev = cn;
                prev_x = x;
            }
        }
    }

    #[test]
    fn fin_cp_moves_to_the_planform_centroid_at_90_deg() {
        let (m, _) = model();
        let f = m.geom.fins.clone().unwrap();
        let (cr, ct, sw) = (f.root_chord, f.tip_chord, f.sweep);
        // Area centroid of the trapezoid: integral of c(eta) (x_le(eta) + c(eta)/2) over its area.
        let xc = f.x_le_root + (sw * (cr + 2.0 * ct) + cr * cr + cr * ct + ct * ct) / (3.0 * (cr + ct));
        for mach in [0.3, 1.2, 2.0, 3.0] {
            assert!((m.fin_xcp_at(mach, PI / 2.0) - xc).abs() < 1e-12);
            let x180 = m.fin_xcp_at(mach, PI);
            let (base, mac) = (f.x_le_root + f.mac_le_offset(), f.mac());
            let frac0 = (m.fin_xcp(mach) - base) / mac;
            assert!((x180 - (base + (1.0 - frac0) * mac)).abs() < 1e-12, "reverse flow at M={mach}");
        }
    }

    #[test]
    fn crossflow_blend_is_monotone_and_matches_its_ends() {
        let t_end = 75f64.to_radians();
        for &(cn1, d0, cn90) in &[(2.5, 9.7, 4.6), (3.7, 25.0, 6.5), (1.0, 0.5, 6.0), (1.0, 1e-3, 6.0), (1.0, 10.0, 1.5)] {
            assert!((crossflow_blend(0.0, t_end, cn1, d0, cn90) - cn1).abs() < 1e-12);
            assert!((crossflow_blend(t_end, t_end, cn1, d0, cn90) - cn90).abs() < 1e-9);
            let h = 1e-7;
            let s0 = (crossflow_blend(h, t_end, cn1, d0, cn90) - cn1) / h;
            assert!((s0 - d0).abs() < 1e-3 * d0.max(1.0), "slope {s0} vs {d0}");
            let mut prev = cn1;
            for k in 1..=1000 {
                let v = crossflow_blend(t_end * k as f64 / 1000.0, t_end, cn1, d0, cn90);
                assert!(v >= prev - 1e-12 && v <= cn90 + 1e-12, "({cn1}, {d0}, {cn90}) at step {k}: {v}");
                prev = v;
            }
        }
        // Attached value above the crossflow value, or just below it with a steep attached slope
        // (beyond the exponential's range): bounded relaxation to cn90, slope matched at the start.
        for &(cn1, d0, cn90) in &[(6.0, 10.0, 4.0), (1.0, 200.0, 1.01), (1.0, 200.0, 1.0)] {
            let v: Vec<f64> = (0..=100).map(|k| crossflow_blend(t_end * k as f64 / 100.0, t_end, cn1, d0, cn90)).collect();
            assert!((v[100] - cn90).abs() < 1e-12);
            // Bounded by the Hermite bump: cn1 + |delta| + d0 * max b = cn1 + |delta| + d0 * 4 T / 27.
            assert!(v.iter().all(|x| x.is_finite() && *x <= cn1.max(cn90) + d0 * 4.0 * t_end / 27.0 + 1e-9), "({cn1}, {d0}, {cn90})");
            let h = 1e-7;
            let s0 = (crossflow_blend(h, t_end, cn1, d0, cn90) - cn1) / h;
            assert!((s0 - d0).abs() < 1e-4 * d0, "slope {s0} vs {d0}");
        }
    }

    #[test]
    fn sample_rocket_stays_in_the_exponential_regime_of_the_crossflow_blend() {
        // The sample's fin force beyond the validity angle is the pure exponential saturation (no bump).
        let (m, _) = model();
        let f = m.geom.fins.clone().unwrap();
        let g = |u: f64| u / -(-u).exp_m1();
        let t_end = PI / 2.0 - FIN_ALPHA_LINEAR;
        for i in 0..=150 {
            let mach = 0.02 * i as f64;
            let cn1 = m.fin_cn_attached(mach, FIN_ALPHA_LINEAR);
            let d0 = (cn1 - m.fin_cn_attached(mach, FIN_ALPHA_LINEAR - 1e-5)) / 1e-5;
            let cn90 = AeroModel::fin_count_factor(f.count) * plate_cd(mach) * f.area() / m.geom.ref_area;
            let r = d0 * t_end / ((cn90 - cn1) * PI / 2.0);
            assert!(cn90 > cn1 && r > g(BLEND_U_MIN) && r < g(BLEND_U_MAX), "M={mach}: r = {r}");
        }
    }

    #[test]
    fn roll_terms_signs_and_scaling() {
        let g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let m4 = AeroModel::new(g.clone(), AeroOptions::default());
        let (cd, cp) = m4.roll_terms(0.8);
        assert!(cd > 0.0, "positive cant gives a positive roll moment");
        assert!(cp < 0.0, "roll damping opposes the roll rate");
        let mut g3 = g.clone();
        g3.fins.as_mut().unwrap().count = 3;
        let m3 = AeroModel::new(g3, AeroOptions::default());
        assert!((m3.roll_terms(0.8).1 / cp - 0.75).abs() < 1e-12, "damping is proportional to the fin count");
        // Strip theory with the fin's own lift slope: Clp = -2 (CNa1 A / A_fin) N I2 / (A d^2).
        let f = g.fins.as_ref().unwrap();
        let (cr, ct, s, r) = (f.root_chord, f.tip_chord, f.span, f.body_radius);
        let n = 20000;
        let i2: f64 = (0..n)
            .map(|i| {
                let xi = r + (i as f64 + 0.5) * s / n as f64;
                xi * xi * (cr + (ct - cr) * (xi - r) / s) * s / n as f64
            })
            .sum();
        let d = 2.0 * g.ref_radius;
        let cna1 = m4.fin_cn(0.8, 1e-4) / (1e-4 * m4.fin_scale(f));
        let expect = -2.0 * cna1 / f.area() * 4.0 * i2 / (d * d);
        assert!((cp / expect - 1.0).abs() < 1e-6);
        // Continuous through the transonic blend.
        let (a, b) = (m4.roll_terms(0.899).1, m4.roll_terms(0.901).1);
        assert!((a - b).abs() < 1e-2 * a.abs());
    }

    /// Sample body with rectangular fins of span `span` and chord `chord` on a body of radius 0.05 m.
    fn rect_fins(span: f64, chord: f64) -> AeroModel {
        let mut g = extract(&SampleRocket::default().mesh(), &ExtractOptions::default()).unwrap();
        let f = g.fins.as_mut().unwrap();
        (f.span, f.root_chord, f.tip_chord, f.sweep, f.body_radius) = (span, chord, chord, 0.0, 0.05);
        AeroModel::new(g, AeroOptions::default())
    }

    #[test]
    fn large_alpha_fin_force_is_continuous_where_the_attached_value_meets_the_crossflow_value() {
        // Rectangular fins (span 0.15 m, chord 0.10 m, body radius 0.05 m): the attached-flow value
        // at the validity angle crosses the crossflow value near Mach 0.59.
        let m = rect_fins(0.15, 0.10);
        let (a1, h) = (FIN_ALPHA_LINEAR, 1e-6);
        let f = m.geom.fins.clone().unwrap();
        let cn90 = |mach: f64| AeroModel::fin_count_factor(f.count) * plate_cd(mach) * f.area() / m.geom.ref_area;
        let gap = |mach: f64| cn90(mach) - m.fin_cn_attached(mach, a1);
        let (mut lo, mut hi) = (0.3, 0.9);
        assert!(gap(lo) * gap(hi) < 0.0, "no crossover in [{lo}, {hi}]: {} {}", gap(lo), gap(hi));
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if gap(lo) * gap(mid) <= 0.0 {
                hi = mid;
            } else {
                lo = mid;
            }
        }
        let mc = 0.5 * (lo + hi);
        assert!((mc - 0.590458).abs() < 1e-3, "crossover at Mach {mc}");
        for deg in [20.0f64, 30.0, 40.0, 60.0, 80.0] {
            let a = deg.to_radians();
            let (l, r) = (m.fin_cn(mc - h, a), m.fin_cn(mc + h, a));
            assert!((r - l).abs() < 1e-3 * l.abs(), "alpha {deg} deg: fin CN jumps {l} -> {r} at Mach {mc}");
        }
        // Mach sweep through the crossover: no jump at any angle.
        for deg in [16.0f64, 25.0, 40.0, 55.0, 75.0, 89.0] {
            let a = deg.to_radians();
            let mut mach = 0.3;
            let mut prev = m.fin_cn(mach, a);
            while mach < 0.9 {
                mach += 1e-4;
                let cn = m.fin_cn(mach, a);
                assert!((cn - prev).abs() < 2e-3 * prev.abs(), "alpha {deg} deg: fin CN jump at Mach {mach}: {prev} -> {cn}");
                prev = cn;
            }
        }
    }

    #[test]
    fn crossflow_blend_is_continuous_in_its_parameters() {
        let t_end = 75f64.to_radians();
        // Through cn90 = cn1 (the attached value meeting the crossflow value) and through d0 = 0.
        for &(cn1, d0) in &[(2.0, 5.0), (2.0, 0.3), (1.0, 40.0)] {
            for k in 0..=50 {
                let t = t_end * k as f64 / 50.0;
                let (l, r) = (crossflow_blend(t, t_end, cn1, d0, cn1 - 1e-9), crossflow_blend(t, t_end, cn1, d0, cn1 + 1e-9));
                assert!((l - r).abs() < 1e-6, "({cn1}, {d0}) t = {t}: {l} vs {r}");
            }
        }
        for k in 0..=50 {
            let t = t_end * k as f64 / 50.0;
            let (l, r) = (crossflow_blend(t, t_end, 1.0, -1e-9, 3.0), crossflow_blend(t, t_end, 1.0, 1e-9, 3.0));
            assert!((l - r).abs() < 1e-6, "d0 through 0, t = {t}: {l} vs {r}");
        }
    }

    #[test]
    fn supersonic_fin_cp_is_continuous_for_small_aspect_ratio() {
        // Aspect ratio of an opposing pair 2 s^2 / A = 0.25: AR beta passes 0.5 at Mach sqrt(5).
        let m = rect_fins(0.0125, 0.10);
        let f = m.geom.fins.clone().unwrap();
        assert!((2.0 * f.span * f.span / f.area() - 0.25).abs() < 1e-12);
        let (base, mac) = (f.x_le_root + f.mac_le_offset(), f.mac());
        let frac = |mach: f64| (m.fin_xcp(mach) - base) / mac;
        let mut mach = 0.0;
        let mut prev = frac(mach);
        while mach < 6.0 {
            mach += 1e-3;
            let x = frac(mach);
            assert!((0.25 - 1e-12..=0.5 + 1e-12).contains(&x), "Mach {mach}: CP fraction {x}");
            assert!((x - prev).abs() < 2e-3, "fin CP jumps at Mach {mach}: {prev} -> {x} of the MAC");
            prev = x;
        }
        // The fraction itself: monotone, in [0.25, 0.5], C1 at the joins, Fleeman's value beyond 1.2.
        let mut prev = supersonic_fin_cp_fraction(0.0);
        for k in 1..=10000 {
            let x = supersonic_fin_cp_fraction(k as f64 * 1e-3);
            assert!(x >= prev - 1e-15 && (0.25..=0.5).contains(&x), "A beta {}: {x}", k as f64 * 1e-3);
            prev = x;
        }
        for x0 in [0.84, 1.2] {
            let h = 1e-7;
            let (l, c, r) = (supersonic_fin_cp_fraction(x0 - h), supersonic_fin_cp_fraction(x0), supersonic_fin_cp_fraction(x0 + h));
            assert!(((c - l) / h - (r - c) / h).abs() < 1e-4, "slope jump at A beta = {x0}");
        }
        assert!((supersonic_fin_cp_fraction(2.25) - (2.25 - 0.67) / 3.5).abs() < 1e-15);
        // Other aspect ratios: continuous as well.
        for (span, chord) in [(0.03, 0.10), (0.05, 0.10), (0.15, 0.10)] {
            let m = rect_fins(span, chord);
            let mut prev = m.fin_xcp(0.0);
            let mut mach = 0.0;
            while mach < 6.0 {
                mach += 1e-3;
                let x = m.fin_xcp(mach);
                assert!((x - prev).abs() < 2e-3 * chord, "span {span}: fin CP jumps at Mach {mach}");
                prev = x;
            }
        }
    }
}
