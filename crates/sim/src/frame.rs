// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
//! Integration frames. The flight dynamics in `flight.rs` are written once; everything that
//! depends on the Earth model lives here:
//!
//! * `flat`: inertial local ENU frame at the launch site (x east, y north, z up), uniform "up".
//! * `ecef`: Earth-fixed rotating frame (WGS84). Positions/velocities are relative to the Earth in
//!   ECEF axes; gravity is central (or J2), and the Coriolis and centrifugal accelerations act.
//!   Attitude is the body-to-ECEF quaternion and `w` the body angular velocity relative to ECEF;
//!   the Euler equations use the inertial angular velocity w + R^T Omega (see `flight.rs`).
//!
//! Gravity in `ecef`: `constant` (g0) and `inverse_square` (g0 (R_e / (R_e + h))^2, h the
//! geodetic altitude) are *effective* gravities (they already include the centrifugal term), taken
//! along the local geodetic down so they agree with the flat frame. `Frame::gravity` returns the
//! pure gravitation g_eff + Omega x (Omega x p), so that adding the apparent acceleration
//! -Omega x (Omega x p) in the equations of motion gives net g_eff. `j2` is the full point-mass +
//! J2 gravitation vector (with its small horizontal components) and needs no correction.
//!
//! Output quantities are always expressed in the launch-site ENU frame.

use crate::env::{j2_gravity, Earth, EarthModel, GravityModel, R_EARTH};
use crate::flight::Launch;
use crate::geo::{ecef_to_lla, enu_basis, lla_to_ecef};
use aero::atmosphere::G0;
use geom::Vec3;

/// Local geometry at a position: height above the launch altitude and the local ENU axes
/// expressed in the integration frame.
#[derive(Debug, Clone, Copy)]
pub struct Local {
    pub height: f64,
    pub east: Vec3,
    pub north: Vec3,
    pub up: Vec3,
}

impl Local {
    /// Local ENU vector -> integration frame.
    pub fn to_frame(&self, v: Vec3) -> Vec3 {
        self.east * v.x + self.north * v.y + self.up * v.z
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frame {
    kind: EarthModel,
    gravity: GravityModel,
    launch_alt: f64,
    /// Earth rotation vector in the frame (zero for flat) [rad/s].
    omega: Vec3,
    /// Launch site in the frame and its ENU axes in the frame.
    origin: Vec3,
    basis: [Vec3; 3],
    /// Launch site geodetic coordinates (lat deg, lon deg, alt m).
    site: (f64, f64, f64),
}

impl Frame {
    pub fn new(earth: &Earth, launch: &Launch) -> Self {
        let site = (launch.latitude, launch.longitude, launch.altitude);
        match earth.model {
            EarthModel::Flat => Frame {
                kind: EarthModel::Flat,
                gravity: earth.gravity,
                launch_alt: launch.altitude,
                omega: Vec3::ZERO,
                origin: Vec3::ZERO,
                basis: [Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), Vec3::new(0.0, 0.0, 1.0)],
                site,
            },
            EarthModel::Ecef => Frame {
                kind: EarthModel::Ecef,
                gravity: earth.gravity,
                launch_alt: launch.altitude,
                omega: Vec3::new(0.0, 0.0, earth.omega),
                origin: lla_to_ecef(site.0, site.1, site.2),
                basis: enu_basis(site.0, site.1),
                site,
            },
        }
    }

    pub fn is_flat(&self) -> bool {
        self.kind == EarthModel::Flat
    }

    /// Earth rotation vector in the frame.
    pub fn omega(&self) -> Vec3 {
        self.omega
    }

    /// Launch-site position in the frame.
    pub fn origin(&self) -> Vec3 {
        self.origin
    }

    /// Launch-site ENU vector -> integration frame.
    pub fn from_launch_enu(&self, v: Vec3) -> Vec3 {
        self.basis[0] * v.x + self.basis[1] * v.y + self.basis[2] * v.z
    }

    /// Integration-frame vector -> launch-site ENU components.
    pub fn to_launch_enu(&self, v: Vec3) -> Vec3 {
        Vec3::new(v.dot(self.basis[0]), v.dot(self.basis[1]), v.dot(self.basis[2]))
    }

    /// Position in the frame -> launch-site ENU coordinates (tangent plane at the launch site).
    pub fn position_to_launch_enu(&self, p: Vec3) -> Vec3 {
        self.to_launch_enu(p - self.origin)
    }

    /// Height above the launch altitude and local axes at position `p`.
    pub fn local(&self, p: Vec3) -> Local {
        match self.kind {
            EarthModel::Flat => Local { height: p.z, east: self.basis[0], north: self.basis[1], up: self.basis[2] },
            EarthModel::Ecef => {
                let (lat, lon, alt) = ecef_to_lla(p);
                let [east, north, up] = enu_basis(lat, lon);
                Local { height: alt - self.launch_alt, east, north, up }
            }
        }
    }

    /// Geodetic (lat deg, lon deg, altitude MSL m) of position `p`.
    pub fn lla(&self, p: Vec3) -> (f64, f64, f64) {
        match self.kind {
            EarthModel::Flat => crate::geo::enu_to_lla(self.site, p),
            EarthModel::Ecef => ecef_to_lla(p),
        }
    }

    /// Gravitational acceleration at `p` (without centrifugal term).
    pub fn gravity(&self, p: Vec3, loc: &Local, earth: &Earth) -> Vec3 {
        match self.kind {
            EarthModel::Flat => earth.flat_gravity(self.launch_alt, loc.height),
            EarthModel::Ecef => match self.gravity {
                GravityModel::Constant => self.gravitation_from_effective(p, loc.up * -G0),
                GravityModel::InverseSquare => {
                    // Same magnitude law as the flat frame, directed along the geodetic down.
                    let r = R_EARTH / (R_EARTH + self.launch_alt + loc.height);
                    self.gravitation_from_effective(p, loc.up * (-G0 * r * r))
                }
                GravityModel::J2 => j2_gravity(p),
            },
        }
    }

    /// Convert an effective (apparent) gravity vector, which already contains the centrifugal
    /// acceleration, into pure gravitation: g_grav = g_eff + Omega x (Omega x p). The equations
    /// of motion in the rotating frame add -Omega x (Omega x p) again (`apparent_accel`), so the
    /// net acceleration equals g_eff exactly, as in the flat frame (no double counting).
    fn gravitation_from_effective(&self, p: Vec3, g_eff: Vec3) -> Vec3 {
        g_eff + self.omega.cross(self.omega.cross(p))
    }

    /// Coriolis + centrifugal acceleration of a body at `p` moving with velocity `v` relative to
    /// the frame: -2 Omega x v - Omega x (Omega x p). Zero for the flat frame.
    pub fn apparent_accel(&self, p: Vec3, v: Vec3) -> Vec3 {
        if self.omega == Vec3::ZERO {
            return Vec3::ZERO;
        }
        -(self.omega.cross(v) * 2.0) - self.omega.cross(self.omega.cross(p))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn launch() -> Launch {
        Launch { latitude: 34.7, longitude: 139.4, altitude: 20.0, rail_length: 5.0, elevation_deg: 85.0, azimuth_deg: 270.0, roll_deg: 0.0 }
    }

    #[test]
    fn ecef_frame_geometry() {
        let earth = Earth { model: EarthModel::Ecef, ..Earth::default() };
        let f = Frame::new(&earth, &launch());
        let l0 = f.local(f.origin());
        assert!(l0.height.abs() < 1e-4);
        // 1 km above the site is 1 km of height; launch ENU round trip
        let p = f.origin() + f.from_launch_enu(Vec3::new(0.0, 0.0, 1000.0));
        assert!((f.local(p).height - 1000.0).abs() < 1e-3);
        let enu = f.position_to_launch_enu(p);
        assert!(enu.x.abs() < 1e-6 && enu.y.abs() < 1e-6 && (enu.z - 1000.0).abs() < 1e-6);
        // gravitation + centrifugal = effective gravity along the geodetic down, magnitude g0 (R/(R+h))^2
        let g = f.gravity(f.origin(), &l0, &earth);
        let r = 6_371_000.0 / (6_371_000.0 + 20.0);
        let net = g + f.apparent_accel(f.origin(), Vec3::ZERO);
        assert!((net.norm() - G0 * r * r).abs() < 1e-9);
        assert!((net.dot(l0.up) + net.norm()).abs() < 1e-9);
        // centrifugal acceleration at 34.7 deg: Omega^2 R cos(lat) ~ 0.0277
        let a = f.apparent_accel(f.origin(), Vec3::ZERO);
        assert!((a.norm() - 0.0277).abs() < 0.002, "{}", a.norm());
        // Coriolis on a rising body (up) deflects it west
        let c = f.apparent_accel(Vec3::ZERO, f.from_launch_enu(Vec3::new(0.0, 0.0, 100.0)));
        assert!(f.to_launch_enu(c).x < 0.0);
    }

    /// Net acceleration at rest (gravitation + centrifugal) must equal the effective gravity,
    /// purely vertical: no horizontal residual that would make a vertical flight drift north.
    #[test]
    fn ecef_net_gravity_has_no_horizontal_residual() {
        let j2_earth = Earth { model: EarthModel::Ecef, gravity: GravityModel::J2, ..Earth::default() };
        for gravity in [GravityModel::Constant, GravityModel::InverseSquare] {
            let earth = Earth { model: EarthModel::Ecef, gravity, ..Earth::default() };
            let f = Frame::new(&earth, &launch());
            let fj = Frame::new(&j2_earth, &launch());
            for h in [0.0, 2_000.0, 20_000.0, 80_000.0] {
                let p = f.origin() + f.from_launch_enu(Vec3::new(0.0, 0.0, h));
                let loc = f.local(p);
                let net = f.gravity(p, &loc, &earth) + f.apparent_accel(p, Vec3::ZERO);
                let net_enu = f.to_launch_enu(net);
                assert!(net_enu.y.abs() < 2e-4 && net_enu.x.abs() < 2e-4, "{gravity:?} h={h} net {net_enu:?}");
                // J2 (already consistent) has the same small horizontal residual scale
                let locj = fj.local(p);
                let netj = fj.to_launch_enu(fj.gravity(p, &locj, &j2_earth) + fj.apparent_accel(p, Vec3::ZERO));
                assert!(netj.y.abs() < 0.05, "j2 north {}", netj.y);
                assert!((net_enu.y).abs() <= netj.y.abs() + 5e-4);
            }
        }
    }
}
