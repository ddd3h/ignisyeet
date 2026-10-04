// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
// The coefficients below are transcribed verbatim from `dop853.f` and therefore
// carry more decimal digits than an `f64` can represent.  The trailing digits
// are intentional provenance, so silence `excessive_precision` for this module.
#![allow(clippy::excessive_precision)]
//! DOP853: the explicit Runge-Kutta method of Dormand & Prince, order 8 with
//! embedded 5th- and 3rd-order error estimates.
//!
//! Coefficients are transcribed from `dop853.f` by E. Hairer and G. Wanner
//! (version of October 11, 2009; minor modification of August 4, 2025), as
//! published in E. Hairer, S. P. Norsett and G. Wanner, *Solving Ordinary
//! Differential Equations I*, 2nd ed., Springer (1993), Section II.6.
//!
//! The 12-stage propagating method is implemented.  As in the original, FSAL
//! is not used by the 12-stage step: the 13th function evaluation at the
//! accepted end point, `f(t + h, y_new)`, is returned as [`Dop853Step::k_new`]
//! so that it can serve as the first stage of the next step.  The dense-output
//! coefficients (`c14..c16`, `a14*..a16*`, `d4*..d7*`) are not needed by this
//! API and are intentionally omitted.

use crate::tableau::RkTableau;

// ---------------------------------------------------------------------------
// Nodes c (c1 = 0 and c12 = 1 are implicit).
// ---------------------------------------------------------------------------
const C2: f64 = 0.526001519587677318785587544488e-1; // c2
const C3: f64 = 0.789002279381515978178381316732e-1; // c3
const C4: f64 = 0.118350341907227396726757197510; // c4
const C5: f64 = 0.281649658092772603273242802490; // c5
const C6: f64 = 0.333333333333333333333333333333; // c6
const C7: f64 = 0.25; // c7
const C8: f64 = 0.307692307692307692307692307692; // c8
const C9: f64 = 0.651282051282051282051282051282; // c9
const C10: f64 = 0.6; // c10
const C11: f64 = 0.857142857142857142857142857142; // c11

// ---------------------------------------------------------------------------
// Strictly lower-triangular coefficients a_{i,j}.
// ---------------------------------------------------------------------------
const A21: f64 = 5.26001519587677318785587544488e-2; // a21
const A31: f64 = 1.97250569845378994544595329183e-2; // a31
const A32: f64 = 5.91751709536136983633785987549e-2; // a32
const A41: f64 = 2.95875854768068491816892993775e-2; // a41
const A43: f64 = 8.87627564304205475450678981324e-2; // a43
const A51: f64 = 2.41365134159266685502369798665e-1; // a51
const A53: f64 = -8.84549479328286085344864962717e-1; // a53
const A54: f64 = 9.24834003261792003115737966543e-1; // a54
const A61: f64 = 3.7037037037037037037037037037e-2; // a61
const A64: f64 = 1.70828608729473871279604482173e-1; // a64
const A65: f64 = 1.25467687566822425016691814123e-1; // a65
const A71: f64 = 3.7109375e-2; // a71
const A74: f64 = 1.70252211019544039314978060272e-1; // a74
const A75: f64 = 6.02165389804559606850219397283e-2; // a75
const A76: f64 = -1.7578125e-2; // a76
const A81: f64 = 3.70920001185047927108779319836e-2; // a81
const A84: f64 = 1.70383925712239993810214054705e-1; // a84
const A85: f64 = 1.07262030446373284651809199168e-1; // a85
const A86: f64 = -1.53194377486244017527936158236e-2; // a86
const A87: f64 = 8.27378916381402288758473766002e-3; // a87
const A91: f64 = 6.24110958716075717114429577812e-1; // a91
const A94: f64 = -3.36089262944694129406857109825; // a94
const A95: f64 = -8.68219346841726006818189891453e-1; // a95
const A96: f64 = 2.75920996994467083049415600797e1; // a96
const A97: f64 = 2.01540675504778934086186788979e1; // a97
const A98: f64 = -4.34898841810699588477366255144e1; // a98
const A101: f64 = 4.77662536438264365890433908527e-1; // a101
const A104: f64 = -2.48811461997166764192642586468; // a104
const A105: f64 = -5.90290826836842996371446475743e-1; // a105
const A106: f64 = 2.12300514481811942347288949897e1; // a106
const A107: f64 = 1.52792336328824235832596922938e1; // a107
const A108: f64 = -3.32882109689848629194453265587e1; // a108
const A109: f64 = -2.03312017085086261358222928593e-2; // a109
const A111: f64 = -9.3714243008598732571704021658e-1; // a111
const A114: f64 = 5.18637242884406370830023853209; // a114
const A115: f64 = 1.09143734899672957818500254654; // a115
const A116: f64 = -8.14978701074692612513997267357; // a116
const A117: f64 = -1.85200656599969598641566180701e1; // a117
const A118: f64 = 2.27394870993505042818970056734e1; // a118
const A119: f64 = 2.49360555267965238987089396762; // a119
const A1110: f64 = -3.0467644718982195003823669022; // a1110
const A121: f64 = 2.27331014751653820792359768449; // a121
const A124: f64 = -1.05344954667372501984066689879e1; // a124
const A125: f64 = -2.00087205822486249909675718444; // a125
const A126: f64 = -1.79589318631187989172765950534e1; // a126
const A127: f64 = 2.79488845294199600508499808837e1; // a127
const A128: f64 = -2.85899827713502369474065508674; // a128
const A129: f64 = -8.87285693353062954433549289258; // a129
const A1210: f64 = 1.23605671757943030647266201528e1; // a1210
const A1211: f64 = 6.43392746015763530355970484046e-1; // a1211

// ---------------------------------------------------------------------------
// Propagating weights b (8th-order solution).
// ---------------------------------------------------------------------------
const B1: f64 = 5.42937341165687622380535766363e-2; // b1
const B6: f64 = 4.45031289275240888144113950566; // b6
const B7: f64 = 1.89151789931450038304281599044; // b7
const B8: f64 = -5.8012039600105847814672114227; // b8
const B9: f64 = 3.1116436695781989440891606237e-1; // b9
const B10: f64 = -1.52160949662516078556178806805e-1; // b10
const B11: f64 = 2.01365400804030348374776537501e-1; // b11
const B12: f64 = 4.47106157277725905176885569043e-2; // b12

/// Full 12-entry weight vector of the propagating solution (zeros where
/// `dop853.f` has no term).  Positions follow the Fortran stage numbering:
/// `B[0] = b1`, `B[5] = b6`, ..., `B[10] = b11`, `B[11] = b12`.
const B: [f64; 12] = [
    B1, 0.0, 0.0, 0.0, 0.0, B6, B7, B8, B9, B10, B11, B12,
];

// ---------------------------------------------------------------------------
// Error coefficients: the embedded 3rd-order estimate (`bhh`) and the
// 5th-order estimate (`er`).
// ---------------------------------------------------------------------------
const BHH1: f64 = 0.244094488188976377952755905512; // bhh1
const BHH2: f64 = 0.733846688281611857341361741547; // bhh2
const BHH3: f64 = 0.220588235294117647058823529412e-1; // bhh3
const ER1: f64 = 0.1312004499419488073250102996e-1; // er1
const ER6: f64 = -0.1225156446376204440720569753e1; // er6
const ER7: f64 = -0.4957589496572501915214079952; // er7
const ER8: f64 = 0.1664377182454986536961530415e1; // er8
const ER9: f64 = -0.3503288487499736816886487290; // er9
const ER10: f64 = 0.3341791187130174790297318841; // er10
const ER11: f64 = 0.8192320648511571246570742613e-1; // er11
const ER12: f64 = -0.2235530786388629525884427845e-1; // er12

/// `BHH[0] = bhh1`, `BHH[8] = bhh2` (stage 9), `BHH[11] = bhh3` (stage 12).
const BHH: [f64; 12] = [
    BHH1, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, BHH2, 0.0, 0.0, BHH3,
];

/// `ER[0] = er1`, `ER[5] = er6`, ..., `ER[10] = er11`, `ER[11] = er12`.
const ER: [f64; 12] = [
    ER1, 0.0, 0.0, 0.0, 0.0, ER6, ER7, ER8, ER9, ER10, ER11, ER12,
];

/// The 12-stage propagating tableau of DOP853 (`c`, lower-triangular `a`, `b`;
/// order 8).  Row `i` of `a` has `i` entries, with explicit zeros wherever
/// `dop853.f` omits a term.
pub fn tableau() -> RkTableau {
    RkTableau {
        c: vec![0.0, C2, C3, C4, C5, C6, C7, C8, C9, C10, C11, 1.0],
        a: vec![
            vec![],
            vec![A21],
            vec![A31, A32],
            vec![A41, 0.0, A43],
            vec![A51, 0.0, A53, A54],
            vec![A61, 0.0, 0.0, A64, A65],
            vec![A71, 0.0, 0.0, A74, A75, A76],
            vec![A81, 0.0, 0.0, A84, A85, A86, A87],
            vec![A91, 0.0, 0.0, A94, A95, A96, A97, A98],
            vec![A101, 0.0, 0.0, A104, A105, A106, A107, A108, A109],
            vec![A111, 0.0, 0.0, A114, A115, A116, A117, A118, A119, A1110],
            vec![
                A121, 0.0, 0.0, A124, A125, A126, A127, A128, A129, A1210, A1211,
            ],
        ],
        b: B.to_vec(),
        order: 8,
    }
}

/// Result of one DOP853 step on a state of length `n`.
pub struct Dop853Step {
    /// Solution at `t + h` (8th order).
    pub y: Vec<f64>,
    /// Scaled error norm as in `dop853.f` (combination of the 5th- and
    /// 3rd-order estimates, `err * sqrt(1/(err + 0.01*err2))` form); the step
    /// is accepted when `err <= 1`.
    pub err: f64,
    /// `f(t + h, y)` — derivative at the end point, usable as the first stage
    /// of the next step.
    pub k_new: Vec<f64>,
}

/// One step.  `f(t, y, dydt)` writes the derivative into `dydt`; `k1` is
/// `f(t, y)`.  Error scaling: `sk_i = atol + rtol * max(|y_i|, |y_new_i|)`
/// (scalar tolerances).
pub fn step(
    f: &mut impl FnMut(f64, &[f64], &mut [f64]),
    t: f64,
    y: &[f64],
    k1: &[f64],
    h: f64,
    rtol: f64,
    atol: f64,
) -> Dop853Step {
    let n = y.len();
    let tab = tableau();

    // Stage derivatives k[0..12] (k[0] = supplied f(t, y)); stage s+1 is k[s].
    let mut k: Vec<Vec<f64>> = vec![vec![0.0; n]; 12];
    k[0].copy_from_slice(k1);

    let mut y1 = vec![0.0; n];
    for s in 1..12 {
        let row = &tab.a[s];
        for i in 0..n {
            let mut acc = 0.0;
            for (a_sj, k_j) in row.iter().zip(k.iter()) {
                acc += a_sj * k_j[i];
            }
            y1[i] = y[i] + h * acc;
        }
        f(t + tab.c[s] * h, &y1, &mut k[s]);
    }

    // 8th-order propagating combination (`K4` in `dop853.f`) and new solution
    // (`K5 = y + h*K4`).
    let mut dy = vec![0.0; n];
    let mut y_new = vec![0.0; n];
    for i in 0..n {
        let mut acc = 0.0;
        for (b_j, k_j) in B.iter().zip(k.iter()) {
            acc += b_j * k_j[i];
        }
        dy[i] = acc;
        y_new[i] = y[i] + h * acc;
    }

    // Error estimation, exactly as in `dop853.f`.
    let mut err_sum = 0.0;
    let mut err2_sum = 0.0;
    for i in 0..n {
        let sk = atol + rtol * y[i].abs().max(y_new[i].abs());
        let mut e2 = dy[i];
        let mut e = 0.0;
        for j in 0..12 {
            e2 -= BHH[j] * k[j][i];
            e += ER[j] * k[j][i];
        }
        err2_sum += (e2 / sk) * (e2 / sk);
        err_sum += (e / sk) * (e / sk);
    }
    let mut deno = err_sum + 0.01 * err2_sum;
    if deno <= 0.0 {
        deno = 1.0;
    }
    let err = h.abs() * err_sum * (1.0 / (n as f64 * deno)).sqrt();

    // The 13th function evaluation at the accepted end point.
    let mut k_new = vec![0.0; n];
    f(t + h, &y_new, &mut k_new);

    Dop853Step {
        y: y_new,
        err,
        k_new,
    }
}

/// Step-size factor from `dop853.f`.  With `beta = 0`:
///
/// ```text
/// fac11 = err^expo1,  expo1 = 1/8
/// fac   = clamp(fac11 / 0.9, 1/6 .. 1/0.333)
/// ```
///
/// Returns the multiplier for `h` (`new_h = h * step_factor(err)`), i.e.
/// `1/fac`.  The result lies in `[0.333, 6]` and is monotonically decreasing
/// in `err`.  Step-rejection memory (the `facold / beta` stabilization) is not
/// included.
pub fn step_factor(err: f64) -> f64 {
    // `dop853.f` defaults: SAFE = 0.9, FAC1 = 0.333, FAC2 = 6, BETA = 0.
    // FACC1 = 1/FAC1, FACC2 = 1/FAC2 and EXPO1 = 1/8 - BETA*0.2 = 1/8.
    const EXPO1: f64 = 0.125;
    const SAFE: f64 = 0.9;
    const FAC1: f64 = 0.333;
    const FAC2: f64 = 6.0;
    const FACC1: f64 = 1.0 / FAC1;
    const FACC2: f64 = 1.0 / FAC2;

    let fac11 = err.powf(EXPO1);
    let fac = (fac11 / SAFE).clamp(FACC2, FACC1);
    1.0 / fac
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tableau_is_consistent() {
        let tab = tableau();
        tab.check().unwrap();
        assert_eq!(tab.stages(), 12);
        assert_eq!(tab.order, 8);
        assert_eq!(tab.c.len(), 12);
        assert_eq!(tab.a.len(), 12);
        for (i, row) in tab.a.iter().enumerate() {
            assert_eq!(row.len(), i);
        }
    }

    #[test]
    fn quadrature_order_conditions() {
        let tab = tableau();
        // sum_i b_i c_i^k = 1/(k+1) for k = 0..7.
        for k in 0..8 {
            let mut s = 0.0;
            for (bi, ci) in tab.b.iter().zip(tab.c.iter()) {
                s += bi * ci.powi(k);
            }
            let expected = 1.0 / (k as f64 + 1.0);
            assert!(
                (s - expected).abs() < 1e-12,
                "k = {k}: sum b c^k = {s}, expected {expected}"
            );
        }
    }

    /// Integrate `y' = y*cos(t)`, `y(0) = 1` up to `t = 2` with a fixed number
    /// of equally spaced steps and return the absolute error at `t = 2`.
    fn fixed_step_error(n_steps: usize) -> f64 {
        let mut f = |t: f64, y: &[f64], d: &mut [f64]| {
            d[0] = y[0] * t.cos();
        };
        let t_end = 2.0;
        let h = t_end / n_steps as f64;
        let mut t = 0.0;
        let mut y = vec![1.0];
        let mut k1 = vec![0.0];
        f(t, &y, &mut k1);
        for _ in 0..n_steps {
            let out = step(&mut f, t, &y, &k1, h, 1e-14, 1e-14);
            t += h;
            y = out.y;
            k1 = out.k_new;
        }
        (y[0] - t_end.sin().exp()).abs()
    }

    #[test]
    fn fixed_step_convergence_order() {
        let e8 = fixed_step_error(8);
        let e16 = fixed_step_error(16);
        let order = (e8 / e16).log2();
        assert!(
            order > 7.5 && order < 8.5,
            "observed order {order} (e8 = {e8:e}, e16 = {e16:e})"
        );
    }

    /// Adaptive integration with local-error control.  Returns
    /// `(t, y, n_accepted)`.
    fn integrate_adaptive(
        f: &mut impl FnMut(f64, &[f64], &mut [f64]),
        t0: f64,
        t_end: f64,
        y0: &[f64],
        h0: f64,
        rtol: f64,
        atol: f64,
    ) -> (f64, Vec<f64>, usize) {
        let n = y0.len();
        let mut t = t0;
        let mut y = y0.to_vec();
        let mut k1 = vec![0.0; n];
        f(t, &y, &mut k1);
        let mut h = h0;
        let mut accepted = 0usize;
        let mut attempts = 0usize;
        while t < t_end {
            let hs = h.min(t_end - t);
            let out = step(f, t, &y, &k1, hs, rtol, atol);
            attempts += 1;
            if out.err <= 1.0 {
                t += hs;
                y = out.y;
                k1 = out.k_new;
                accepted += 1;
            }
            h = hs * step_factor(out.err);
            assert!(attempts < 1_000_000, "adaptive integrator did not terminate");
        }
        (t, y, accepted)
    }

    #[test]
    fn adaptive_harmonic_oscillator() {
        let rtol = 1e-10;
        let atol = 1e-10;
        let mut f = |_t: f64, y: &[f64], d: &mut [f64]| {
            d[0] = y[1];
            d[1] = -y[0];
        };
        let t_end = 20.0;
        let (_, y, steps) = integrate_adaptive(
            &mut f,
            0.0,
            t_end,
            &[1.0, 0.0],
            0.1,
            rtol,
            atol,
        );
        let exact = [t_end.cos(), -t_end.sin()];
        let err = ((y[0] - exact[0]).powi(2) + (y[1] - exact[1]).powi(2)).sqrt();
        assert!(err < 1e-8, "final error {err}");
        assert!(steps < 400, "took {steps} accepted steps");
    }

    #[test]
    fn arenstorf_orbit() {
        let mu = 0.012277471;
        let mu2 = 1.0 - mu;
        let t_end = 17.0652165601579625588917206249;
        let y0 = [0.994, 0.0, 0.0, -2.00158510637908252240537862224];
        let mut f = move |_t: f64, y: &[f64], d: &mut [f64]| {
            let r1 = ((y[0] + mu).powi(2) + y[1].powi(2)).powf(1.5);
            let r2 = ((y[0] - mu2).powi(2) + y[1].powi(2)).powf(1.5);
            d[0] = y[2];
            d[1] = y[3];
            d[2] = y[0] + 2.0 * y[3] - mu2 * (y[0] + mu) / r1 - mu * (y[0] - mu2) / r2;
            d[3] = y[1] - 2.0 * y[2] - mu2 * y[1] / r1 - mu * y[1] / r2;
        };
        let (_, y, _) = integrate_adaptive(&mut f, 0.0, t_end, &y0, 0.01, 1e-10, 1e-10);
        let max_err = (0..4)
            .map(|i| (y[i] - y0[i]).abs())
            .fold(0.0f64, f64::max);
        assert!(max_err < 1e-5, "return error {max_err:e}");
    }

    #[test]
    fn step_factor_is_monotone_and_bounded() {
        // Limits of `dop853.f`: h_new/h in [FAC1, FAC2] = [0.333, 6].
        let lo = step_factor(1e30);
        let hi = step_factor(1e-30);
        assert!((lo - 0.333).abs() < 1e-12, "lower limit {lo}");
        assert!((hi - 6.0).abs() < 1e-12, "upper limit {hi}");

        let errs = [1e-6, 0.01, 0.1, 0.5, 1.0, 2.0, 10.0, 100.0, 1e6];
        for w in errs.windows(2) {
            let a = step_factor(w[0]);
            let b = step_factor(w[1]);
            assert!(a >= b, "not monotone: f({}) = {a} < f({}) = {b}", w[0], w[1]);
        }
        for &e in &errs {
            let fac = step_factor(e);
            assert!((0.333 - 1e-12..=6.0 + 1e-12).contains(&fac), "f({e}) = {fac}");
        }
        // A perfect step (err = 1) yields the safety factor 0.9.
        assert!((step_factor(1.0) - 0.9).abs() < 1e-12);
    }
}
