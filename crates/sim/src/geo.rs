//! WGS84 conversions between geodetic, ECEF and local ENU coordinates.

use geom::Vec3;

const A: f64 = 6_378_137.0;
const F: f64 = 1.0 / 298.257_223_563;

fn e2() -> f64 {
    F * (2.0 - F)
}

/// Geodetic (deg, deg, m) to ECEF [m].
pub fn lla_to_ecef(lat: f64, lon: f64, alt: f64) -> Vec3 {
    let (sl, cl) = lat.to_radians().sin_cos();
    let (so, co) = lon.to_radians().sin_cos();
    let n = A / (1.0 - e2() * sl * sl).sqrt();
    Vec3::new((n + alt) * cl * co, (n + alt) * cl * so, (n * (1.0 - e2()) + alt) * sl)
}

/// ECEF to geodetic (deg, deg, m) using Bowring's method.
pub fn ecef_to_lla(p: Vec3) -> (f64, f64, f64) {
    let b = A * (1.0 - F);
    let ep2 = (A * A - b * b) / (b * b);
    let lon = p.y.atan2(p.x);
    let r = p.x.hypot(p.y);
    let th = (p.z * A).atan2(r * b);
    let lat = (p.z + ep2 * b * th.sin().powi(3)).atan2(r - e2() * A * th.cos().powi(3));
    let n = A / (1.0 - e2() * lat.sin().powi(2)).sqrt();
    let alt = r / lat.cos() - n;
    (lat.to_degrees(), lon.to_degrees(), alt)
}

/// East, north and up unit vectors (in ECEF) at a geodetic latitude/longitude [deg].
pub fn enu_basis(lat: f64, lon: f64) -> [Vec3; 3] {
    let (sl, cl) = lat.to_radians().sin_cos();
    let (so, co) = lon.to_radians().sin_cos();
    [Vec3::new(-so, co, 0.0), Vec3::new(-sl * co, -sl * so, cl), Vec3::new(cl * co, cl * so, sl)]
}

/// Local ENU offset from a geodetic origin to geodetic coordinates.
pub fn enu_to_lla(origin: (f64, f64, f64), enu: Vec3) -> (f64, f64, f64) {
    let (lat, lon, alt) = origin;
    let [e, n, u] = enu_basis(lat, lon);
    ecef_to_lla(lla_to_ecef(lat, lon, alt) + e * enu.x + n * enu.y + u * enu.z)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_offsets() {
        let (lat, lon, alt) = ecef_to_lla(lla_to_ecef(34.7, 139.4, 120.0));
        assert!((lat - 34.7).abs() < 1e-9 && (lon - 139.4).abs() < 1e-9 && (alt - 120.0).abs() < 1e-4);
        // 1 km north is about 0.009 deg of latitude.
        let (lat2, lon2, _) = enu_to_lla((34.7, 139.4, 0.0), Vec3::new(0.0, 1000.0, 0.0));
        assert!((lat2 - 34.7 - 0.009_013).abs() < 2e-5 && (lon2 - 139.4).abs() < 1e-9);
    }
}
