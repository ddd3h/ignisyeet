//! Watertight structured panel mesh of a body of revolution with trapezoidal biconvex fins.
//!
//! Layout
//! * Body: quads on (axial station, circumferential node) with the nose tip collapsed to a point
//!   (triangles). Behind the base a solid cylindrical fairing closed by a hemisphere stands in for
//!   the separated base region (see `PanelOptions::tail_radii`): a bare open base with a wake
//!   tube does not carry the slender-body normal force of the body in a Dirichlet formulation.
//! * Fins: for every fin the circumferential node set contains the two root edges. Inside the root
//!   chord the lens-shaped footprint of the fin is cut out of the body: the two root nodes of a
//!   ring sit at the half-thickness `h(x)` of the biconvex section and merge into one node at the
//!   leading and trailing edge, so the fin sides attach to the body without gaps.
//! * Wakes: from the fin trailing edges straight aft along +x, with growing panel lengths. They
//!   carry doublet only.
//!
//! Geometry conventions: x is measured aft from the nose tip; fin `k` sits at azimuth
//! `phi_k = 2 pi k / N + pi / N` with radial direction `(0, cos phi, sin phi)` and
//! tangential ("upper") direction `(0, -sin phi, cos phi)`.

use crate::{FinSection, PanelOptions};
use anyhow::{bail, Result};
use geom::{Geometry, Vec3};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::fmt::Write as _;
use std::io::Write as _;

/// Which kind of surface a panel belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Body,
    Fin,
    /// Solid fairing behind the base that stands in for the separated wake region; it closes
    /// the body and carries no friction, supersonic load or force.
    Tail,
}

/// A planar (triangular or quadrilateral) panel with its local coordinate system.
#[derive(Debug, Clone)]
pub struct Panel {
    /// Vertex ids into [`Mesh::verts`] (unused for wake panels).
    pub ids: [usize; 4],
    /// Number of corners (3 or 4).
    pub n_v: usize,
    pub corners: [Vec3; 4],
    pub centroid: Vec3,
    /// Unit normal, outward (towards the fluid). For wake panels: from the lower to the upper side.
    pub normal: Vec3,
    pub area: f64,
    /// Local in-plane axes: `e1`, `e2 = normal x e1`.
    pub e1: Vec3,
    pub e2: Vec3,
    /// Largest corner-to-corner distance.
    pub diameter: f64,
    pub part: Part,
    /// Running length from the leading point (nose tip / fin leading edge) for friction [m].
    pub run: f64,
}

impl Panel {
    /// Builds a panel from its corners (counter-clockwise about the intended normal).
    pub fn new(corners: [Vec3; 4], n_v: usize, part: Part, run: f64) -> Panel {
        let mut p = Panel {
            ids: [usize::MAX; 4],
            n_v,
            corners,
            centroid: Vec3::ZERO,
            normal: Vec3::ZERO,
            area: 0.0,
            e1: Vec3::ZERO,
            e2: Vec3::ZERO,
            diameter: 0.0,
            part,
            run,
        };
        p.update_geometry();
        p
    }

    /// Recomputes centroid, normal, area, local axes and size from the corners.
    pub fn update_geometry(&mut self) {
        let n = self.n_v;
        if n == 3 {
            self.corners[3] = self.corners[2];
        }
        let c = &self.corners;
        let cross = if n == 3 { (c[1] - c[0]).cross(c[2] - c[0]) } else { (c[2] - c[0]).cross(c[3] - c[1]) };
        self.area = 0.5 * cross.norm();
        self.normal = cross.normalized();
        let mut s = Vec3::ZERO;
        for v in &c[..n] {
            s += *v;
        }
        self.centroid = s / n as f64;
        let t = c[1] - c[0];
        self.e1 = (t - self.normal * t.dot(self.normal)).normalized();
        self.e2 = self.normal.cross(self.e1);
        let mut d: f64 = 0.0;
        for i in 0..n {
            for j in i + 1..n {
                d = d.max((c[i] - c[j]).norm());
            }
        }
        self.diameter = d;
    }

    fn scaled_lateral(&self, beta: f64) -> Panel {
        let mut p = self.clone();
        for c in &mut p.corners {
            c.y *= beta;
            c.z *= beta;
        }
        p.update_geometry();
        p
    }
}

/// A wake column: a strip of wake panels whose doublet strength follows the Kutta condition.
#[derive(Debug, Clone)]
pub struct WakeColumn {
    /// Surface panel on the upper side (fin) or the edge panel (base ring).
    pub upper: usize,
    /// Surface panel on the lower side; `None` for the base ring (interior potential is zero).
    pub lower: Option<usize>,
    /// Range of this column's panels in [`Mesh::wake_panels`].
    pub panels: std::ops::Range<usize>,
}

/// Edge-adjacent surface panel used for surface gradients.
#[derive(Debug, Clone, Copy)]
pub struct Neighbor {
    pub panel: usize,
    /// Vertex ids of the shared edge.
    pub edge: [usize; 2],
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub verts: Vec<Vec3>,
    pub panels: Vec<Panel>,
    pub wake_panels: Vec<Panel>,
    pub wake_cols: Vec<WakeColumn>,
    pub neighbors: Vec<Vec<Neighbor>>,
    /// Vertex-id pairs of edges shared by two panels that must not be differenced (Kutta lines).
    kutta_edges: Vec<[usize; 2]>,
}

fn edge_key(a: usize, b: usize) -> (usize, usize) {
    (a.min(b), a.max(b))
}

impl Mesh {
    /// Mesh with the lateral coordinates (y, z) scaled by `beta` (Goethert transformation).
    pub fn scaled_lateral(&self, beta: f64) -> Mesh {
        let mut m = self.clone();
        for v in &mut m.verts {
            v.y *= beta;
            v.z *= beta;
        }
        m.panels = self.panels.iter().map(|p| p.scaled_lateral(beta)).collect();
        m.wake_panels = self.wake_panels.iter().map(|p| p.scaled_lateral(beta)).collect();
        m
    }

    /// Number of edges used by exactly one panel and by more than two panels.
    pub fn edge_census(&self) -> (Vec<[usize; 2]>, usize) {
        let mut cnt: HashMap<(usize, usize), usize> = HashMap::new();
        for p in &self.panels {
            let v = &p.ids[..p.n_v];
            for k in 0..p.n_v {
                *cnt.entry(edge_key(v[k], v[(k + 1) % p.n_v])).or_default() += 1;
            }
        }
        let open = cnt.iter().filter(|(_, &c)| c == 1).map(|(&(a, b), _)| [a, b]).collect();
        let over = cnt.values().filter(|&&c| c > 2).count();
        (open, over)
    }

    /// Copy rotated by `angle` [rad] about the x axis (used to align fins with the flow in tests).
    pub fn rotated_about_x(&self, angle: f64) -> Mesh {
        let (s, c) = angle.sin_cos();
        let rot = |v: Vec3| Vec3::new(v.x, c * v.y - s * v.z, s * v.y + c * v.z);
        let mut m = self.clone();
        for v in &mut m.verts {
            *v = rot(*v);
        }
        for p in m.panels.iter_mut().chain(m.wake_panels.iter_mut()) {
            for k in &mut p.corners {
                *k = rot(*k);
            }
            p.update_geometry();
        }
        m
    }

    /// Writes surface and wake panels as a VTK legacy polydata file (debugging aid).
    pub fn write_vtk(&self, path: &std::path::Path) -> Result<()> {
        let all: Vec<&Panel> = self.panels.iter().chain(self.wake_panels.iter()).collect();
        let mut s = String::from("# vtk DataFile Version 3.0\npanel mesh\nASCII\nDATASET POLYDATA\n");
        let npts: usize = all.iter().map(|p| p.n_v).sum();
        writeln!(s, "POINTS {npts} double")?;
        for p in &all {
            for c in &p.corners[..p.n_v] {
                writeln!(s, "{} {} {}", c.x, c.y, c.z)?;
            }
        }
        writeln!(s, "POLYGONS {} {}", all.len(), npts + all.len())?;
        let mut k = 0;
        for p in &all {
            write!(s, "{}", p.n_v)?;
            for _ in 0..p.n_v {
                write!(s, " {k}")?;
                k += 1;
            }
            writeln!(s)?;
        }
        writeln!(s, "CELL_DATA {}\nSCALARS kind int 1\nLOOKUP_TABLE default", all.len())?;
        for (i, _) in all.iter().enumerate() {
            writeln!(s, "{}", if i < self.panels.len() { 0 } else { 1 })?;
        }
        std::fs::File::create(path)?.write_all(s.as_bytes())?;
        Ok(())
    }

    /// Mesh for a rocket described by `geom`, with options from `opt`.
    pub fn from_geometry(geom: &Geometry, opt: &PanelOptions) -> Result<Mesh> {
        if opt.body_axial < 6 || opt.body_circ < 8 {
            bail!("panel mesh too coarse (body_axial >= 6, body_circ >= 8 required)");
        }
        let l = geom.length;
        let mut pts = vec![(0.0, 0.0)];
        for p in &geom.profile {
            if p.x > 0.0 && p.x < l && p.x > pts.last().unwrap().0 {
                pts.push((p.x, p.r));
            }
        }
        pts.push((l, geom.base_radius));
        let radius = |x: f64| interp(&pts, x);

        let fin = match &geom.fins {
            Some(f) if f.count > 0 && f.span > 0.0 && f.root_chord > 0.0 && f.thickness > 0.0 => {
                if opt.fin_section != FinSection::Biconvex {
                    bail!("unsupported fin section");
                }
                let mut x_le = f.x_le_root.max(0.0);
                let mut x_te = f.x_le_root + f.root_chord;
                if x_te >= l * 0.995 {
                    x_te = l;
                }
                if x_le >= x_te - 1e-6 {
                    bail!("fin root lies outside the body");
                }
                x_le = x_le.min(x_te - 1e-6);
                Some(FinSpec { set: f.clone(), x_le, x_te })
            }
            _ => None,
        };
        let (stations, range) = stations(l, fin.as_ref().map(|f| (f.x_le, f.x_te)), opt.body_axial, opt.fin_chord);
        let radii: Vec<f64> = stations.iter().map(|&x| radius(x)).collect();
        let spec = fin.map(|f| (f, range.unwrap(), opt.fin_span.max(2)));
        build(&stations, &radii, opt.body_circ, spec, wake_len(opt.wake_length, l), opt.tail_radii)
    }

    /// Body of revolution from explicit stations and radii (no fins, no wake). A base that does
    /// not close to a point gets the tail fairing of `tail_radii` base radii.
    pub fn body_of_revolution(stations: &[f64], radii: &[f64], n_circ: usize, tail_radii: f64) -> Result<Mesh> {
        build(stations, radii, n_circ, None, None, tail_radii)
    }
}

fn wake_len(factor: f64, l: f64) -> Option<f64> {
    (factor > 0.0).then_some(factor * l)
}

fn interp(pts: &[(f64, f64)], x: f64) -> f64 {
    if x <= pts[0].0 {
        return pts[0].1;
    }
    let k = pts.partition_point(|p| p.0 <= x);
    if k >= pts.len() {
        return pts[pts.len() - 1].1;
    }
    let (a, b) = (pts[k - 1], pts[k]);
    a.1 + (b.1 - a.1) * (x - a.0) / (b.0 - a.0)
}

/// Cosine-spaced stations x0..x1 with `n` panels.
pub fn cosine_stations(x0: f64, x1: f64, n: usize) -> Vec<f64> {
    (0..=n).map(|i| x0 + (x1 - x0) * 0.5 * (1.0 - (PI * i as f64 / n as f64).cos())).collect()
}

/// Axial stations; with a fin root range `[x_le, x_te]` the range gets `n_chord` panels and the
/// stations at `x_le` / `x_te` are members of the set. Returns the stations and the ring indices
/// of the fin root leading and trailing edge.
fn stations(l: f64, fin: Option<(f64, f64)>, n: usize, n_chord: usize) -> (Vec<f64>, Option<(usize, usize)>) {
    let Some((x_le, x_te)) = fin else { return (cosine_stations(0.0, l, n), None) };
    let n_fin = n_chord.clamp(2, n - 4);
    let rest = n - n_fin;
    let (l_pre, l_post) = (x_le, l - x_te);
    let (n_pre, n_post) = if l_post <= 1e-9 * l {
        (rest, 0)
    } else if l_pre <= 1e-9 * l {
        (0, rest)
    } else {
        let a = ((rest as f64 * l_pre / (l_pre + l_post)).round() as usize).clamp(2, rest - 2);
        (a, rest - a)
    };
    let mut s = Vec::new();
    if n_pre > 0 {
        s.extend(cosine_stations(0.0, x_le, n_pre));
        s.pop();
    }
    let i_le = s.len();
    s.extend(cosine_stations(x_le, x_te, n_fin));
    let i_te = s.len() - 1;
    if n_post > 0 {
        s.pop();
        s.extend(cosine_stations(x_te, l, n_post));
    }
    (s, Some((i_le, i_te)))
}

struct FinSpec {
    set: geom::FinSet,
    x_le: f64,
    x_te: f64,
}

struct Builder {
    verts: Vec<Vec3>,
    panels: Vec<Panel>,
    wake_panels: Vec<Panel>,
    wake_cols: Vec<WakeColumn>,
    kutta: Vec<[usize; 2]>,
}

impl Builder {
    fn vertex(&mut self, p: Vec3) -> usize {
        self.verts.push(p);
        self.verts.len() - 1
    }

    /// Adds a panel with vertex ids in any consistent order; orients it so that its normal
    /// agrees with `hint`. Collapsed corners turn quads into triangles. Returns the index.
    fn add(&mut self, ids: [usize; 4], hint: Vec3, part: Part, run: f64) -> Option<usize> {
        let mut v: Vec<usize> = Vec::with_capacity(4);
        for id in ids {
            if v.last() != Some(&id) {
                v.push(id);
            }
        }
        while v.len() > 1 && v[0] == v[v.len() - 1] {
            v.pop();
        }
        if v.len() < 3 {
            return None;
        }
        let mk = |v: &[usize]| {
            let mut c = [Vec3::ZERO; 4];
            for (k, &i) in v.iter().enumerate() {
                c[k] = self.verts[i];
            }
            Panel::new(c, v.len(), part, run)
        };
        let mut p = mk(&v);
        if p.area < 1e-14 {
            return None;
        }
        if p.normal.dot(hint) < 0.0 {
            v.reverse();
            p = mk(&v);
        }
        for (k, &i) in v.iter().enumerate() {
            p.ids[k] = i;
        }
        self.panels.push(p);
        Some(self.panels.len() - 1)
    }

    /// Wake strip behind the edge `(p0, p1)`, with growing panel lengths totalling `len`, the
    /// first one `first` long; `(upper, lower)` are the panels the Kutta condition refers to.
    fn wake_strip(&mut self, (p0, p1): (Vec3, Vec3), hint: Vec3, (len, first): (f64, f64), (upper, lower): (usize, Option<usize>)) {
        let nw = 4;
        // Growth ratio so that first * (1 + r + ... + r^(nw-1)) = len.
        let (mut lo, mut hi) = (1.0f64, 50.0f64);
        for _ in 0..80 {
            let r = 0.5 * (lo + hi);
            let s: f64 = (0..nw).map(|k| r.powi(k)).sum::<f64>() * first;
            if s < len {
                lo = r;
            } else {
                hi = r;
            }
        }
        let ratio = 0.5 * (lo + hi);
        let start = self.wake_panels.len();
        let mut d0 = 0.0;
        let mut dl = first;
        for _ in 0..nw {
            let d1 = d0 + dl;
            let ex = Vec3::new(1.0, 0.0, 0.0);
            let mut c = [p0 + ex * d0, p0 + ex * d1, p1 + ex * d1, p1 + ex * d0];
            let mut p = Panel::new(c, 4, Part::Body, 0.0);
            if p.normal.dot(hint) < 0.0 {
                c.swap(1, 3);
                p = Panel::new(c, 4, Part::Body, 0.0);
            }
            self.wake_panels.push(p);
            d0 = d1;
            dl *= ratio;
        }
        self.wake_cols.push(WakeColumn { upper, lower, panels: start..self.wake_panels.len() });
    }
}

fn build(stations: &[f64], radii: &[f64], n_circ: usize, fin: Option<(FinSpec, (usize, usize), usize)>, wake: Option<f64>, tail_radii: f64) -> Result<Mesh> {
    let na_real = stations.len() - 1;
    let l = stations[na_real];
    // Solid tail fairing: a cylinder of `tail_radii` base radii closed by a hemisphere.
    let mut stations = stations.to_vec();
    let mut radii = radii.to_vec();
    let r_b = radii[na_real];
    if r_b > 1e-9 && tail_radii > 0.0 {
        let tl = tail_radii * r_b;
        for x in cosine_stations(l, l + tl, 8).into_iter().skip(1) {
            stations.push(x);
            radii.push(r_b);
        }
        for i in 1..=8 {
            let t = 0.5 * PI * i as f64 / 8.0;
            stations.push(l + tl + r_b * t.sin());
            radii.push(r_b * t.cos());
        }
    }
    let (stations, radii) = (&stations[..], &radii[..]);
    let na = stations.len() - 1;
    let mut b = Builder { verts: vec![], panels: vec![], wake_panels: vec![], wake_cols: vec![], kutta: vec![] };

    // --- circumferential node layout -------------------------------------------------------
    let nf = fin.as_ref().map_or(0, |f| f.0.set.count);
    let (c_nodes, per_sector) = if nf == 0 { (n_circ, 0) } else { (nf * (n_circ / nf).max(3), (n_circ / nf).max(3)) };
    let phi = |k: usize| 2.0 * PI * k as f64 / nf as f64 + PI / nf as f64;
    // Half-thickness of the fin at ring i and the maximal half-angle of the removed strip.
    let mut half_t = vec![0.0; na + 1];
    let mut delta_max = 0.0;
    if let Some((f, (i_le, i_te), _)) = &fin {
        for i in *i_le..=*i_te {
            let xi = (stations[i] - f.x_le) / (f.x_te - f.x_le);
            half_t[i] = 0.5 * f.set.thickness * 4.0 * xi * (1.0 - xi);
        }
        half_t[*i_le] = 0.0;
        half_t[*i_te] = 0.0;
        let r_min = radii[*i_le..=*i_te].iter().cloned().fold(f64::INFINITY, f64::min).max(1e-9);
        let dm = (0.5 * f.set.thickness / r_min).min(0.5).asin();
        // Keep the strip well inside its sector.
        delta_max = dm.min(0.25 * 2.0 * PI / nf as f64);
    }
    // Base angle of each circumferential node (strip nodes at +-delta_max).
    let mut theta = vec![0.0; c_nodes];
    if nf == 0 {
        for (c, t) in theta.iter_mut().enumerate() {
            *t = 2.0 * PI * c as f64 / c_nodes as f64;
        }
    } else {
        let m = per_sector - 1; // regular panels per sector
        for k in 0..nf {
            let base = k * per_sector;
            theta[base] = phi(k) - delta_max;
            let t0 = phi(k) + delta_max;
            let t1 = phi(k) + 2.0 * PI / nf as f64 - delta_max;
            for q in 0..m {
                theta[base + 1 + q] = t0 + (t1 - t0) * q as f64 / m as f64;
            }
        }
    }

    // --- body vertices -----------------------------------------------------------------------
    let strip_col = |k: usize| k * per_sector;
    let mut bv = vec![vec![0usize; c_nodes]; na + 1];
    for i in 0..=na {
        let x = stations[i];
        let r = radii[i];
        if r <= 1e-12 {
            let id = b.vertex(Vec3::new(x, 0.0, 0.0));
            bv[i].fill(id);
            continue;
        }
        for c in 0..c_nodes {
            let mut t = theta[c];
            if let Some((_, (i_le, i_te), _)) = &fin {
                if i >= *i_le && i <= *i_te {
                    let dh = (half_t[i] / r).min(0.5).asin();
                    for k in 0..nf {
                        if c == strip_col(k) {
                            t = phi(k) - dh;
                        } else if c == strip_col(k) + 1 {
                            t = phi(k) + dh;
                        }
                    }
                }
            }
            bv[i][c] = b.vertex(Vec3::new(x, r * t.cos(), r * t.sin()));
        }
        if let Some((_, (i_le, i_te), _)) = &fin {
            if i >= *i_le && i <= *i_te && half_t[i] <= 0.0 {
                for k in 0..nf {
                    bv[i][strip_col(k) + 1] = bv[i][strip_col(k)];
                }
            }
        }
    }

    // --- body panels -------------------------------------------------------------------------
    let mut s_arc = vec![0.0; na + 1];
    for i in 0..na {
        s_arc[i + 1] = s_arc[i] + (stations[i + 1] - stations[i]).hypot(radii[i + 1] - radii[i]);
    }
    let mut body_idx = vec![vec![None; c_nodes]; na];
    for i in 0..na {
        for c in 0..c_nodes {
            if let Some((_, (i_le, i_te), _)) = &fin {
                if i >= *i_le && i < *i_te && (0..nf).any(|k| c == strip_col(k)) {
                    continue; // removed: covered by the fin
                }
            }
            let c1 = (c + 1) % c_nodes;
            let ids = [bv[i][c], bv[i + 1][c], bv[i + 1][c1], bv[i][c1]];
            let mid = (b.verts[ids[0]] + b.verts[ids[1]] + b.verts[ids[2]] + b.verts[ids[3]]) / 4.0;
            let hint = Vec3::new(0.0, mid.y, mid.z);
            let hint = if hint.norm() < 1e-12 { Vec3::new(-1.0, 0.0, 0.0) } else { hint };
            let part = if i >= na_real { Part::Tail } else { Part::Body };
            body_idx[i][c] = b.add(ids, hint, part, 0.5 * (s_arc[i] + s_arc[i + 1]));
        }
    }

    // --- fins --------------------------------------------------------------------------------
    if let Some((f, (i_le, i_te), ns)) = &fin {
        let (i_le, i_te, ns) = (*i_le, *i_te, *ns);
        let set = &f.set;
        let chord_root = f.x_te - f.x_le;
        let r_tip = set.body_radius + set.span;
        let s_j: Vec<f64> = (0..=ns).map(|j| (0.5 * PI * j as f64 / ns as f64).sin()).collect();
        let nc = i_te - i_le;
        for k in 0..nf {
            let ph = phi(k);
            let (er, et) = (Vec3::new(0.0, ph.cos(), ph.sin()), Vec3::new(0.0, -ph.sin(), ph.cos()));
            let mut fu = vec![vec![0usize; ns + 1]; nc + 1];
            let mut fl = fu.clone();
            for i in 0..=nc {
                let ring = i_le + i;
                let xi = (stations[ring] - f.x_le) / chord_root;
                let h = half_t[ring];
                let rho0 = {
                    let p = b.verts[bv[ring][strip_col(k)]];
                    p.y * er.y + p.z * er.z
                };
                fu[i][0] = bv[ring][strip_col(k) + 1];
                fl[i][0] = bv[ring][strip_col(k)];
                for (j, &s) in s_j.iter().enumerate().skip(1) {
                    let chord = chord_root + (set.tip_chord - set.root_chord) * s;
                    let x = f.x_le + set.sweep * s + xi * chord;
                    let rho = rho0 + s * (r_tip - rho0);
                    let up = b.vertex(Vec3::new(x, 0.0, 0.0) + er * rho + et * h);
                    fu[i][j] = up;
                    fl[i][j] = if h > 0.0 { b.vertex(Vec3::new(x, 0.0, 0.0) + er * rho - et * h) } else { up };
                }
            }
            let mut te_up = vec![None; ns];
            let mut te_lo = vec![None; ns];
            for i in 0..nc {
                let xi_m = (stations[i_le + i] + stations[i_le + i + 1]) * 0.5 - f.x_le;
                for j in 0..ns {
                    let s_m = 0.5 * (s_j[j] + s_j[j + 1]);
                    let chord = chord_root + (set.tip_chord - set.root_chord) * s_m;
                    let run = (xi_m / chord_root * chord).max(1e-6);
                    let u = b.add([fu[i][j], fu[i + 1][j], fu[i + 1][j + 1], fu[i][j + 1]], et, Part::Fin, run);
                    let lo = b.add([fl[i][j], fl[i + 1][j], fl[i + 1][j + 1], fl[i][j + 1]], -et, Part::Fin, run);
                    if i == nc - 1 {
                        te_up[j] = u;
                        te_lo[j] = lo;
                    }
                }
                let run = (xi_m / chord_root * set.tip_chord).max(1e-6);
                b.add([fu[i][ns], fu[i + 1][ns], fl[i + 1][ns], fl[i][ns]], er, Part::Fin, run);
            }
            // Kutta lines and wake strips along the trailing edge.
            let first = 0.25 * l;
            let len = wake.unwrap_or(0.0).max(2.0 * first);
            for j in 0..ns {
                let (Some(u), Some(lo)) = (te_up[j], te_lo[j]) else { continue };
                b.kutta.push([fu[nc][j], fu[nc][j + 1]]);
                if wake.is_some() {
                    b.wake_strip((b.verts[fu[nc][j]], b.verts[fu[nc][j + 1]]), et, (len, first), (u, Some(lo)));
                }
            }
        }
    }

    // --- adjacency ---------------------------------------------------------------------------
    let mut edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
    for (i, p) in b.panels.iter().enumerate() {
        for k in 0..p.n_v {
            edges.entry(edge_key(p.ids[k], p.ids[(k + 1) % p.n_v])).or_default().push(i);
        }
    }
    let kutta: std::collections::HashSet<(usize, usize)> = b.kutta.iter().map(|e| edge_key(e[0], e[1])).collect();
    let mut neighbors = vec![Vec::new(); b.panels.len()];
    for (&(a, c), ps) in &edges {
        if ps.len() == 2 && !kutta.contains(&(a, c)) {
            neighbors[ps[0]].push(Neighbor { panel: ps[1], edge: [a, c] });
            neighbors[ps[1]].push(Neighbor { panel: ps[0], edge: [a, c] });
        }
    }
    for n in &mut neighbors {
        n.sort_by_key(|x| x.panel);
    }
    Ok(Mesh { verts: b.verts, panels: b.panels, wake_panels: b.wake_panels, wake_cols: b.wake_cols, neighbors, kutta_edges: b.kutta })
}

impl Mesh {
    /// Vertex-id pairs of the Kutta (fin trailing-edge) lines.
    pub fn kutta_edges(&self) -> &[[usize; 2]] {
        &self.kutta_edges
    }
}
