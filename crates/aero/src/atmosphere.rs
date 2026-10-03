// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Atmosphere models: U.S. Standard Atmosphere 1976 (0 - 86 km), optionally with a temperature
//! offset, and a constant-property atmosphere.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

pub const G0: f64 = 9.80665;
pub const R_AIR: f64 = 287.052_87;
pub const GAMMA: f64 = 1.4;
const R_EARTH: f64 = 6_356_766.0;

/// (geopotential base height [m], base temperature [K], lapse rate [K/m], base pressure [Pa])
const LAYERS: [(f64, f64, f64, f64); 7] = [
    (0.0, 288.15, -0.0065, 101_325.0),
    (11_000.0, 216.65, 0.0, 22_632.06),
    (20_000.0, 216.65, 0.001, 5_474.889),
    (32_000.0, 228.65, 0.0028, 868.0187),
    (47_000.0, 270.65, 0.0, 110.9063),
    (51_000.0, 270.65, -0.0028, 66.938_87),
    (71_000.0, 214.65, -0.002, 3.956_420),
];

#[derive(Debug, Clone, Copy)]
pub struct Atmosphere {
    pub temperature: f64,
    pub pressure: f64,
    pub density: f64,
    pub sound_speed: f64,
    /// Dynamic viscosity [Pa s].
    pub viscosity: f64,
}

impl Atmosphere {
    pub fn kinematic_viscosity(&self) -> f64 {
        self.viscosity / self.density
    }
}

/// Atmospheric state at geometric altitude `z` [m] above mean sea level.
pub fn us76(z: f64) -> Atmosphere {
    let h = geopotential(z);
    let layer = LAYERS.iter().rev().find(|l| h >= l.0).unwrap_or(&LAYERS[0]);
    let (hb, tb, lr, pb) = *layer;
    let t = tb + lr * (h - hb);
    let p = if lr == 0.0 {
        pb * (-G0 * (h - hb) / (R_AIR * tb)).exp()
    } else {
        pb * (tb / t).powf(G0 / (R_AIR * lr))
    };
    let density = p / (R_AIR * t);
    Atmosphere {
        temperature: t,
        pressure: p,
        density,
        sound_speed: (GAMMA * R_AIR * t).sqrt(),
        viscosity: 1.458e-6 * t.powf(1.5) / (t + 110.4),
    }
}

/// Layer index, geopotential height and local temperature offset handling shared by `us76` and
/// `us76_offset`.
fn geopotential(z: f64) -> f64 {
    (R_EARTH * z / (R_EARTH + z)).clamp(-5_000.0, 84_852.0)
}

fn state_from(t: f64, p: f64) -> Atmosphere {
    Atmosphere {
        temperature: t,
        pressure: p,
        density: p / (R_AIR * t),
        sound_speed: (GAMMA * R_AIR * t).sqrt(),
        viscosity: 1.458e-6 * t.powf(1.5) / (t + 110.4),
    }
}

/// Pressure at geopotential height `h` for the standard temperature profile shifted by `dt` K.
/// Integrates dp/dH = -g0 p / (R T(H)) from sea-level p0 = 101325 Pa layer by layer in closed form.
fn offset_pressure(h: f64, dt: f64) -> f64 {
    let mut p = LAYERS[0].3;
    for (i, &(hb, tb, lr, _)) in LAYERS.iter().enumerate() {
        let top = LAYERS.get(i + 1).map_or(f64::INFINITY, |l| l.0);
        let hh = h.min(top);
        if hh <= hb && i > 0 {
            break;
        }
        let tb = tb + dt;
        p *= if lr == 0.0 {
            (-G0 * (hh - hb) / (R_AIR * tb)).exp()
        } else {
            ((tb + lr * (hh - hb)) / tb).powf(-G0 / (R_AIR * lr))
        };
        if h <= top {
            break;
        }
    }
    p
}

/// Standard atmosphere with the temperature profile shifted by `dt` K at every altitude; the
/// pressure is recomputed so that the column stays hydrostatic. `dt = 0` is exactly `us76`.
pub fn us76_offset(z: f64, dt: f64) -> Atmosphere {
    if dt == 0.0 {
        return us76(z);
    }
    let h = geopotential(z);
    let layer = LAYERS.iter().rev().find(|l| h >= l.0).unwrap_or(&LAYERS[0]);
    let t = layer.1 + layer.2 * (h - layer.0) + dt;
    state_from(t, offset_pressure(h, dt))
}

/// Runtime atmosphere: state as a function of geometric altitude above mean sea level.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(tag = "model", rename_all = "lowercase")]
pub enum AtmosphereModel {
    /// US 1976 standard atmosphere with a temperature offset [K].
    Us1976 { temperature_offset: f64 },
    /// Altitude-independent density, speed of sound and dynamic viscosity.
    Constant { density: f64, sound_speed: f64, viscosity: f64 },
}

impl Default for AtmosphereModel {
    fn default() -> Self {
        Self::Us1976 { temperature_offset: 0.0 }
    }
}

impl AtmosphereModel {
    pub fn us1976(temperature_offset: f64) -> Result<Self> {
        if !temperature_offset.is_finite() || temperature_offset <= -200.0 {
            bail!("atmosphere.temperature_offset must be finite and greater than -200 K");
        }
        Ok(Self::Us1976 { temperature_offset })
    }

    pub fn constant(density: f64, sound_speed: f64, viscosity: f64) -> Result<Self> {
        if !(density > 0.0 && sound_speed > 0.0 && viscosity > 0.0) {
            bail!("constant atmosphere needs positive density, sound_speed and viscosity");
        }
        Ok(Self::Constant { density, sound_speed, viscosity })
    }

    /// Atmospheric state at altitude `z` [m MSL]. For the constant model, temperature is
    /// a^2 / (gamma R) and pressure rho R T.
    pub fn at(&self, z: f64) -> Atmosphere {
        match *self {
            Self::Us1976 { temperature_offset } => us76_offset(z, temperature_offset),
            Self::Constant { density, sound_speed, viscosity } => {
                let t = sound_speed * sound_speed / (GAMMA * R_AIR);
                Atmosphere { temperature: t, pressure: density * R_AIR * t, density, sound_speed, viscosity }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_values() {
        let s = us76(0.0);
        assert!((s.density - 1.225).abs() < 1e-3);
        assert!((s.sound_speed - 340.29).abs() < 0.1);
        let t = us76(11_000.0);
        assert!((t.temperature - 216.77).abs() < 0.2);
        let u = us76(20_000.0);
        assert!((u.density - 0.088_9).abs() < 1e-3);
    }

    #[test]
    fn zero_offset_equals_us76() {
        for z in [-500.0, 0.0, 20.0, 5_000.0, 11_000.0, 15_000.0, 25_000.0, 40_000.0, 60_000.0, 80_000.0] {
            let a = AtmosphereModel::us1976(0.0).unwrap().at(z);
            let b = us76(z);
            assert_eq!((a.temperature, a.pressure, a.density, a.sound_speed, a.viscosity), (b.temperature, b.pressure, b.density, b.sound_speed, b.viscosity));
            // the generic recursion itself reproduces the tabulated base pressures
            let h = geopotential(z);
            assert!((offset_pressure(h, 0.0) / b.pressure - 1.0).abs() < 2e-5, "z {z}");
        }
    }

    #[test]
    fn offset_is_hydrostatic() {
        let dt = 15.0;
        for z in [0.0, 3_000.0, 11_000.0, 18_000.0, 30_000.0, 50_000.0] {
            let a = us76_offset(z, dt);
            assert!((a.temperature - (us76(z).temperature + dt)).abs() < 1e-9);
            // integrate dp/dz = -rho g(z) from the sea level with the trapezoid rule on a fine grid
            let n = 20_000;
            let dz = z / n as f64;
            let mut p = 101_325.0;
            let g = |zz: f64| G0 * (R_EARTH / (R_EARTH + zz)).powi(2);
            let f = |zz: f64, p: f64| -p / (R_AIR * us76_offset(zz, dt).temperature) * g(zz);
            for i in 0..n {
                let z0 = i as f64 * dz;
                let k1 = f(z0, p);
                let k2 = f(z0 + 0.5 * dz, p + 0.5 * dz * k1);
                let k3 = f(z0 + 0.5 * dz, p + 0.5 * dz * k2);
                let k4 = f(z0 + dz, p + dz * k3);
                p += dz * (k1 + 2.0 * k2 + 2.0 * k3 + k4) / 6.0;
            }
            assert!((a.pressure / p - 1.0).abs() < 1e-3, "z {z}: {} vs {}", a.pressure, p);
        }
        // warmer air is thinner at the same altitude
        assert!(us76_offset(5_000.0, 15.0).density < us76(5_000.0).density);
    }

    #[test]
    fn constant_atmosphere_returns_inputs() {
        let m = AtmosphereModel::constant(1.1, 345.0, 1.8e-5).unwrap();
        for z in [0.0, 5_000.0, 40_000.0] {
            let a = m.at(z);
            assert_eq!((a.density, a.sound_speed, a.viscosity), (1.1, 345.0, 1.8e-5));
            assert!((a.pressure - a.density * R_AIR * a.temperature).abs() < 1e-9);
            assert!(((GAMMA * R_AIR * a.temperature).sqrt() - 345.0).abs() < 1e-9);
        }
        assert!(AtmosphereModel::constant(0.0, 340.0, 1e-5).is_err());
    }
}
