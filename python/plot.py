# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Plot IgnisYeet outputs.

Usage:
    uv run --project python python/plot.py all <out_dir>
    uv run --project python python/plot.py {geometry,aero,panel,trajectory,dispersion} <out_dir>
    uv run --project python python/plot.py aero <out_dir> --compare <other_out_dir>

Figures are written to <out_dir>/plots/*.png. The dispersion plot draws dispersion.csv (wind grid,
dispersion.png) and/or dispersion_mc.csv + dispersion_summary.json (Monte Carlo, dispersion_mc.png).
The panel subcommand draws the surface pressure coefficient from panel_cp.csv (method = "panel";
alpha = 4 deg at the lowest subsonic Mach number) as panel.png; `all` includes it when the file exists.
With the panel method aero_drag.csv has no breakdown columns and the aero plot shows the total CD0.
With --compare, aero_compare.png overlays CNα, Xcp and CD0 of both aerodynamic tables
(labels are the directory names).

The trajectory and dispersion figures carry a banner when the run's JSON reports a problem: RESULTS NOT
USABLE (statically unstable or unknown static stability, aero-table angle range exceeded), INCOMPLETE
(runs that failed or did not land) or CAUTION (ballistic runs that fell broadside). Landing points of
unstable runs are marked with a red x, of unknown stability with a triangle and of broadside falls with
a diamond (`stable` and `broadside_time` columns of the dispersion CSV files).
"""

import argparse
import json
import sys
from pathlib import Path

import matplotlib

matplotlib.use("Agg")
import matplotlib.pyplot as plt  # noqa: E402
import numpy as np  # noqa: E402
import pandas as pd  # noqa: E402

# Validated categorical order (light surface) and a single-hue sequential ramp.
SERIES = ["#2a78d6", "#eb6834", "#1baf7a", "#eda100", "#e87ba4", "#008300", "#4a3aa7", "#e34948"]
SEQ = ["#86b6ef", "#6da7ec", "#5598e7", "#3987e5", "#2a78d6", "#256abf", "#1c5cab", "#184f95", "#104281", "#0d366b"]
SURFACE = "#fcfcfb"
INK = "#0b0b0b"
INK2 = "#52514e"
GRID = "#e4e3df"

plt.rcParams.update({
    "figure.facecolor": SURFACE,
    "axes.facecolor": SURFACE,
    "savefig.facecolor": SURFACE,
    "axes.edgecolor": GRID,
    "axes.labelcolor": INK2,
    "axes.titlecolor": INK,
    "axes.titlesize": 11,
    "axes.titleweight": "bold",
    "axes.titlelocation": "left",
    "axes.labelsize": 9,
    "axes.grid": True,
    "grid.color": GRID,
    "grid.linewidth": 0.6,
    "axes.spines.top": False,
    "axes.spines.right": False,
    "xtick.color": INK2,
    "ytick.color": INK2,
    "xtick.labelsize": 8,
    "ytick.labelsize": 8,
    "legend.frameon": False,
    "legend.fontsize": 8,
    "lines.linewidth": 2.0,
    "font.size": 9,
})


def speed_colors(n):
    """n distinguishable colours for wind-speed curves: viridis sampled evenly (the light yellow end is left out)."""
    cmap = plt.get_cmap("viridis")
    if n == 1:
        return [cmap(0.35)]
    return [cmap(0.9 * i / (n - 1)) for i in range(n)]


def seq_colors(n):
    """n evenly spaced steps of the sequential ramp, light to dark."""
    if n == 1:
        return [SEQ[len(SEQ) // 2]]
    return [SEQ[round(i * (len(SEQ) - 1) / (n - 1))] for i in range(n)]


SAVED = 0  # figures written so far: a plot function that draws nothing leaves it unchanged


def save(fig, out, name):
    global SAVED
    SAVED += 1
    path = out / "plots" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"wrote {path}")


def remove_old_plots(out, names):
    """Removes plots/<name> of an earlier run for a plot that is skipped now, so that no stale figure stays."""
    for name in names:
        f = out / "plots" / name
        if f.exists():
            f.unlink()
            print(f"removed {f} (plot skipped; the file was from an earlier run)")


def plot_geometry(out):
    if not (out / "profile.csv").exists():
        print("skip geometry: profile.csv not found (run `ignisyeet geom` again)")
        remove_old_plots(out, PNGS["geometry"])
        return
    g = json.loads((out / "geometry.json").read_text())
    p = pd.read_csv(out / "profile.csv")
    fig, ax = plt.subplots(figsize=(11, 3.2))
    x = np.concatenate([[0.0], p.x, [g["length"]]])
    r = np.concatenate([[0.0], p.r, [g["base_radius"]]])
    ax.fill_between(x, -r, r, color=SERIES[0], alpha=0.18, linewidth=0)
    ax.plot(x, r, color=SERIES[0], label="body (extracted)")
    ax.plot(x, -r, color=SERIES[0])
    fin = p[p.fin_span > 0]
    if len(fin):
        ax.plot(fin.x, fin.r + fin.fin_span, ".", color=SERIES[1], markersize=3, label="fin tip (slices)")
    f = g.get("fins")
    if f:
        rb = f["body_radius"]
        xs = [f["x_le_root"], f["x_le_root"] + f["sweep"], f["x_le_root"] + f["sweep"] + f["tip_chord"], f["x_le_root"] + f["root_chord"]]
        ys = [rb, rb + f["span"], rb + f["span"], rb]
        ax.plot(xs, ys, color=SERIES[1], label=f"fin model ({f['count']} fins)")
        ax.plot(xs, [-y for y in ys], color=SERIES[1])
    drag = out / "aero_drag.csv"
    if drag.exists():
        d = pd.read_csv(drag)
        xcp = float(np.interp(0.3, d.mach, d.xcp))
        ax.plot([xcp], [0], "o", color=INK, markersize=8, markerfacecolor=SURFACE, markeredgewidth=2)
        ax.annotate(f"CP (M0.3) {xcp:.3f} m", (xcp, 0), xytext=(0, 10), textcoords="offset points", ha="center", color=INK2)
    ax.set_aspect("equal")
    ax.set_xlabel("x aft of nose tip [m]")
    ax.set_ylabel("r [m]")
    ax.set_title("Extracted geometry")
    ax.legend(loc="upper left")
    save(fig, out, "geometry.png")


def axisymmetric_view(t):
    """Rows of the beta = 0, alpha >= 0 plane of a general table with CN = -CZ (axisymmetric tables are returned unchanged)."""
    if "beta_deg" not in t.columns:
        return t
    return t[(t.beta_deg == 0) & (t.alpha_deg >= 0)].assign(cn=lambda r: -r.cz)


def general_totals(t, out):
    """Per-Mach cd_off, cd_on, cna and xcp of a general table (beta = 0 view with cz, cm, cx_on, cx_off).

    CD0 is -CX at alpha = 0; CNα and Xcp come from the smallest positive angle of attack
    (CNα = |CZ| / alpha, Xcp = CM * D / CZ with D the reference diameter of the sidecar), as in `ignisyeet aero`.
    """
    z = t[t.alpha_deg == 0].sort_values("mach").set_index("mach")
    d = pd.DataFrame({"mach": z.index, "cd_off": -z.cx_off.to_numpy(), "cd_on": -z.cx_on.to_numpy()}).set_index("mach")
    pos = t[t.alpha_deg > 0]
    if len(pos):
        a = pos.alpha_deg.min()
        r = pos[pos.alpha_deg == a].sort_values("mach").set_index("mach")
        diameter = json.loads((out / "aero_table.json").read_text()).get("ref_diameter") if (out / "aero_table.json").exists() else None
        d["cna"] = r.cz.abs() / np.radians(a)
        d["xcp"] = (r.cm * diameter / r.cz) if diameter else np.nan
    else:
        d["cna"] = np.nan
        d["xcp"] = np.nan
    return d.reset_index()


def plot_aero(out):
    t = axisymmetric_view(pd.read_csv(out / "aero_table.csv"))
    if (out / "aero_drag.csv").exists():
        d = pd.read_csv(out / "aero_drag.csv")
    elif "cx_off" in t.columns:
        # General table (sideslip, STL or external): coefficients are CX/CY/CZ; there is no drag breakdown file.
        d = general_totals(t, out)
    else:
        # CFD tables have no per-Mach drag file (no component breakdown): the alpha = 0 rows give the totals.
        d = t[t.alpha_deg == 0].sort_values("mach").rename(columns={"ca_off": "cd_off", "ca_on": "cd_on"})
    fig, axs = plt.subplots(2, 2, figsize=(11, 7.5))

    ax = axs[0, 0]
    keys = ["friction", "nose", "base", "fins", "boattail", "extra"]
    if all(k in d.columns for k in keys):
        parts = [("friction", "skin friction"), ("nose", "nose pressure"), ("base", "base"), ("fins", "fin LE/TE"), ("boattail", "boattail"), ("extra", "extra")]
        parts = [(k, lbl) for k, lbl in parts if d[k].abs().max() > 0]
        ax.stackplot(d.mach, *[d[k] for k, _ in parts], labels=[lbl for _, lbl in parts], colors=SERIES[: len(parts)], alpha=0.85, edgecolor=SURFACE, linewidth=1)
        ax.plot(d.mach, d.cd_on, color=INK, linewidth=1.2, linestyle="--", label="total, power on")
        ax.set_title("Zero-lift drag breakdown (power off)")
        ax.legend(loc="upper right", ncol=2)
    else:
        # Panel method: the axial force is integrated over the surface, so only the total exists.
        ax.plot(d.mach, d.cd_off, color=SERIES[0], label="power off")
        ax.plot(d.mach, d.cd_on, color=SERIES[1], linestyle="--", label="power on")
        ax.set_title("Zero-lift drag (total)")
        ax.legend(loc="upper right")
    ax.set_xlabel("Mach")
    ax.set_ylabel("CD0")

    ax = axs[0, 1]
    ax.plot(d.mach, d.cna, color=SERIES[0])
    ax.set_title("Normal-force slope CNα")
    ax.set_xlabel("Mach")
    ax.set_ylabel("CNα [1/rad]")

    ax = axs[1, 0]
    ax.plot(d.mach, d.xcp, color=SERIES[0])
    ax.set_title("Centre of pressure (α → 0)")
    ax.set_xlabel("Mach")
    ax.set_ylabel("Xcp aft of nose [m]")

    ax = axs[1, 1]
    machs = np.sort(t.mach.unique())
    picks = [m for m in (0.3, 0.8, 1.2, 2.0, 3.0) if machs.min() <= m <= machs.max()]
    for m, c in zip(picks, seq_colors(len(picks))):
        near = machs[np.argmin(np.abs(machs - m))]
        s = t[t.mach == near]
        ax.plot(s.alpha_deg, s.cn, color=c, label=f"M {near:.2f}")
    ax.set_title("Normal force vs angle of attack")
    ax.set_xlabel("α [deg]")
    ax.set_ylabel("CN")
    ax.legend(loc="upper left")
    fig.tight_layout()
    save(fig, out, "aero.png")


def _tri(ax, x, y, c, norm, cmap, max_dy, y_scale=1.0, shading="gouraud"):
    """Smooth-shaded triangulated Cp map; triangles spanning gaps (fin footprints, seams) are masked.

    The Delaunay triangulation is built on (x, y_scale * y) so that the panels are close to square in
    the triangulation (for the unrolled body: y in degrees, y_scale = body radius in m per degree);
    otherwise long slivers along the narrow panel columns smear values along the body.
    """
    import matplotlib.tri as mtri

    x, y = np.asarray(x), np.asarray(y)
    tri = mtri.Triangulation(x, y, mtri.Triangulation(x, y * y_scale).triangles)
    yy = np.asarray(y)[tri.triangles]
    tri.set_mask(np.ptp(yy, axis=1) > max_dy)
    return ax.tripcolor(tri, np.asarray(c), cmap=cmap, norm=norm, shading=shading)


def fin_azimuth(fin):
    """Azimuth [rad, atan2(z, y)] of the centre plane of the fin that reaches farthest from the axis, or None without fin panels."""
    if len(fin) < 3:
        return None
    phi = np.arctan2(fin.z.to_numpy(), fin.y.to_numpy())
    rho = np.hypot(fin.y.to_numpy(), fin.z.to_numpy())
    tip = phi[np.argmax(rho)]
    # the fin is thin: the circular mean of its points near that azimuth lies in its centre plane
    near = np.abs(np.angle(np.exp(1j * (phi - tip)))) < np.radians(10)
    return float(np.angle(np.exp(1j * phi[near]).mean()))


def fin_side_quads(x, rho):
    """Panel outlines (n, 4, 2) in (x, rho) of one fin side, rebuilt from its panel centroids; None if not a structured grid.

    panel_cp.csv lists the side panels of a fin chord station by chord station, root to tip within a
    station, so the centroids form an n_chord x n_span grid in file order. The corners are the means
    of the four surrounding centroids, with one row of centroids extrapolated linearly beyond every
    edge of the grid (root, tip, leading and trailing edge).
    """
    x, rho = np.asarray(x, float), np.asarray(rho, float)
    drops = np.flatnonzero(np.diff(rho) < 0)
    if len(x) < 4 or len(drops) == 0:
        return None
    ns = int(drops[0]) + 1
    if ns < 2 or len(x) % ns:
        return None
    nc = len(x) // ns
    g = np.stack([x, rho], axis=-1).reshape(nc, ns, 2)
    if nc < 2 or np.any(np.diff(g[:, :, 1], axis=1) <= 0) or np.any(np.diff(g[:, :, 0], axis=0) <= 0):
        return None
    p = np.empty((nc + 2, ns + 2, 2))
    p[1:-1, 1:-1] = g
    p[0, 1:-1], p[-1, 1:-1] = 2 * g[0] - g[1], 2 * g[-1] - g[-2]
    p[:, 0], p[:, -1] = 2 * p[:, 1] - p[:, 2], 2 * p[:, -2] - p[:, -3]
    v = 0.25 * (p[:-1, :-1] + p[1:, :-1] + p[:-1, 1:] + p[1:, 1:])
    return np.stack([v[:-1, :-1], v[1:, :-1], v[1:, 1:], v[:-1, 1:]], axis=2).reshape(-1, 4, 2)


def plot_panel(out):
    """Surface Cp of the panel solution: unrolled body map and one fin (both sides)."""
    from matplotlib.collections import PolyCollection
    from matplotlib.colors import TwoSlopeNorm

    d = pd.read_csv(out / "panel_cp.csv")
    lim = max(float(np.percentile(np.abs(d.cp), 99)), 0.1)
    norm = TwoSlopeNorm(vmin=-lim, vcenter=0.0, vmax=lim)
    cmap = "RdBu_r"  # diverging, centred at Cp = 0: blue suction, red compression
    body, fin = d[d.part == "body"], d[d.part == "fin"]
    phi0 = fin_azimuth(fin)
    fig = plt.figure(figsize=(11, 6.8 if phi0 is not None else 3.6), layout="constrained")
    gs = fig.add_gridspec(2 if phi0 is not None else 1, 2, height_ratios=[1, 1.15] if phi0 is not None else [1])

    ax = fig.add_subplot(gs[0, :])
    th = np.degrees(np.arctan2(body.z, body.y))
    r_body = float(np.median(np.hypot(body.y, body.z)))
    sc = _tri(ax, body.x.values, th.values, body.cp.values, norm, cmap, 25.0, y_scale=np.radians(1.0) * r_body)
    ax.set_xlabel("x aft of nose tip [m]")
    ax.set_ylabel("θ [deg]  (y: 0°, z: 90°)")
    ax.set_yticks([-180, -90, 0, 90, 180])
    ax.set_title("Body panels, unrolled")
    ax.grid(False)

    if phi0 is None:
        print("panel: no fin panels in panel_cp.csv; drawing the body only")
    else:
        # One fin (the one whose centre plane is at phi0), split by side of the fin plane.
        phi = np.arctan2(fin.z, fin.y)
        k = fin[np.abs(np.angle(np.exp(1j * (phi - phi0)))) < np.radians(12)]
        side = k.ny * (-np.sin(phi0)) + k.nz * np.cos(phi0)
        rho = np.hypot(k.y, k.z)
        # |side| > 0.5 keeps the two side faces; the tip-cap panels (radial normal, side ~ 0) belong to neither.
        for col, (sel, name) in enumerate(((side > 0.5, "Fin, +side"), (side < -0.5, "Fin, −side"))):
            a = fig.add_subplot(gs[1, col])
            x, r, c = k.x[sel].values, rho[sel].values, k.cp[sel].values
            quads = fin_side_quads(x, r)
            if quads is not None:
                # one flat-coloured quad per panel, no interpolation between panels
                a.add_collection(PolyCollection(quads, array=c, cmap=cmap, norm=norm, edgecolors="face", linewidths=0.2))
                a.autoscale_view()
            elif sel.sum() >= 3:
                try:
                    _tri(a, x, r, c, norm, cmap, 1e9, shading="flat")
                except (ValueError, RuntimeError):
                    pass  # degenerate (collinear) point set: leave the panel empty
            a.set_xlabel("x aft of nose tip [m]")
            a.set_ylabel("radial distance [m]")
            a.set_title(name)
            a.set_aspect("equal", adjustable="datalim")
            a.grid(False)
    cb = fig.colorbar(sc, ax=fig.axes, shrink=0.85, pad=0.02, extend="both")
    cb.set_label("Cp")
    save(fig, out, "panel.png")


def draw_path_3d(ax, tr):
    """Flight path by phase on a 3-D axis with equal scaling on all three axes.

    The axes span a cube of the largest extent of the path, so a steep climb looks steep. A phase whose
    run is shorter than 1 % of that extent (a 5 m launch rail) is drawn but left out of the legend.
    """
    pts = tr[["east", "north", "up"]].to_numpy(float)
    pts = pts[np.isfinite(pts).all(axis=1)]
    lo, hi = (pts.min(axis=0), pts.max(axis=0)) if len(pts) else (np.zeros(3), np.ones(3))
    half = 0.5 * max(float((hi - lo).max()), 1.0)
    for phase, c in (("rail", SERIES[2]), ("free", SERIES[0]), ("parachute", SERIES[1])):
        s = tr[tr.phase == phase]
        if len(s):
            ext = float(np.ptp(s[["east", "north", "up"]].to_numpy(float), axis=0).max())
            ax.plot(s.east, s.north, s.up, color=c, label=phase if ext > 0.02 * half else None)
    ax.scatter([0], [0], [0], color=INK, s=30, label="launch site")
    mid = 0.5 * (lo + hi)
    ax.set_xlim(mid[0] - half, mid[0] + half)
    ax.set_ylim(mid[1] - half, mid[1] + half)
    z0 = max(mid[2] - half, min(lo[2], 0.0))  # ground level at the bottom when the cube leaves room for it
    ax.set_zlim(z0, z0 + 2 * half)
    ax.set_box_aspect((1, 1, 1))


def plot_trajectory(out):
    tr = pd.read_csv(out / "trajectory.csv")
    summ = json.loads((out / "summary.json").read_text()) if (out / "summary.json").exists() else {}
    fig = plt.figure(figsize=(7, 6))
    ax = fig.add_subplot(projection="3d")
    draw_path_3d(ax, tr)
    ax.set_xlabel("East [m]")
    ax.set_ylabel("North [m]")
    ax.set_zlabel("Up [m]")
    not_landed = summ.get("landed") is False
    ax.set_title(f"Trajectory ({summ.get('descent', '')})" + ("  -  did not land" if not_landed else ""))
    ax.legend(loc="upper left")
    usability = trajectory_usability(summ)
    draw_usability(fig, *usability)
    save(fig, out, "trajectory_3d.png")

    # Rigid-body panels cover powered flight and coast; the parachute descent only adds a flat tail.
    # Altitude above the launch site as summary.json reports the apogee: the integrated height `up` on a flat
    # Earth, the geodetic alt minus the launch altitude on an ECEF Earth (see height_and_vertical_velocity).
    height, vert_vel = height_and_vertical_velocity(tr, summ)
    tr = tr.assign(height=height, vert_vel=vert_vel)
    body = tr[tr.phase != "parachute"]
    free = body[body.phase == "free"]
    # Ascent off the rail: on the rail the angle of attack is the wind's, not a flown one.
    asc = body[(body.vert_vel >= 0) & (body.airspeed > 10) & (body.phase != "rail")]
    panels = [
        ("height", "Altitude above launch site [m]", tr),
        ("airspeed", "Airspeed [m/s]", body),
        ("mach", "Mach", body),
        ("alpha_deg", "Angle of attack [deg]", free),
        ("stability_cal", "CP–CG separation at the flown α [cal] (ascent)", asc),
        ("dyn_pressure", "Dynamic pressure [Pa]", body),
        ("thrust", "Thrust [N]", body[body.t <= body.t[body.thrust > 0].max() * 1.2] if (body.thrust > 0).any() else body),
        ("mass", "Mass [kg]", body),
        ("pitch_deg", "Pitch [deg]", free),
    ]
    # Sideslip and roll rate exist in trajectories of this version (absent in older files).
    if "beta_deg" in tr.columns:
        panels.append(("beta_deg", "Sideslip β [deg]", free))
    if "roll_rate" in tr.columns:
        panels.append(("roll_rate", "Roll rate [rad/s]", free))
    nrows = -(-len(panels) // 3)
    fig, axs = plt.subplots(nrows, 3, figsize=(12, 3 * nrows), sharex=False, squeeze=False)
    for ax in axs.flat[len(panels):]:
        ax.set_visible(False)
    for ax, (col, title, data) in zip(axs.flat, panels):
        ax.plot(data.t, data[col], color=SERIES[0])
        ax.set_title(title)
        ax.set_xlabel("t [s]")
    a = axs[0, 0]
    if summ.get("apogee") is not None and summ.get("apogee_time") is not None:
        a.plot([summ["apogee_time"]], [summ["apogee"]], "o", color=SERIES[0], markersize=8, markerfacecolor=SURFACE, markeredgewidth=2)
        a.annotate(f"apogee {summ['apogee']:.0f} m", (summ["apogee_time"], summ["apogee"]), xytext=(6, -12), textcoords="offset points", color=INK2)
    if _finite(summ.get("main_deploy_time")) and _finite(summ.get("main_deploy_altitude")):
        # dual deploy: the main canopy opens on the way down (square marker, like the apogee circle)
        a.plot([summ["main_deploy_time"]], [summ["main_deploy_altitude"]], "s", color=SERIES[1], markersize=8, markerfacecolor=SURFACE, markeredgewidth=2)
        a.annotate(f"main {summ['main_deploy_altitude']:.0f} m", (summ["main_deploy_time"], summ["main_deploy_altitude"]), xytext=(6, 8), textcoords="offset points", color=INK2)
    if not_landed:
        a.text(0.97, 0.06, f"did not land within {summ.get('max_time', tr.t.max()):g} s", transform=a.transAxes, ha="right", color=SERIES[7])
    fig.tight_layout()
    draw_usability(fig, *usability)
    save(fig, out, "trajectory.png")


def flat_earth(tr, summ):
    """True for a flat-Earth run: summary.json's `earth_model`, or, for a summary without it, the touchdown row.

    A flat-Earth touchdown row has up = 0 exactly while its alt (converted from the tangent-plane position)
    carries the curvature drop; an ECEF touchdown has alt at the launch altitude and a negative tangent-plane
    up. When neither tells (no landing, short range), the default earth.model = "flat" is assumed.
    """
    model = summ.get("earth_model")
    if model in ("flat", "ecef"):
        return model == "flat"
    if summ.get("landed") is True and len(tr) > 1:
        up_end, h_end = tr.up.iloc[-1], tr.alt.iloc[-1] - tr.alt.iloc[0]
        if up_end == 0.0 and abs(h_end) > 1e-3:
            return True
        if abs(h_end) <= 1e-3 and up_end < -1e-3:
            return False
    return True


def height_and_vertical_velocity(tr, summ):
    """Height above the launch site [m] and its rate [m/s], as summary.json measures the apogee.

    Flat Earth: the integrated height `up` and `vel_u` (the exported alt is converted from the tangent-plane
    position and adds about distance^2 / (2 R), 16 m at 14 km). ECEF: geodetic alt minus the launch altitude
    and its rate (the tangent-plane `up` falls below the true height with distance).
    """
    if flat_earth(tr, summ):
        return tr.up, tr.vel_u
    return altitude_above_launch(tr), vertical_velocity(tr)


def altitude_above_launch(tr):
    """Geodetic altitude minus the launch altitude (the first row's alt), as summary.json reports apogee."""
    return tr.alt - tr.alt.iloc[0]


def vertical_velocity(tr):
    """Local vertical velocity [m/s]: d(alt)/dt (the tangent-plane vel_u when time does not increase strictly)."""
    t, alt = tr.t.to_numpy(), tr.alt.to_numpy()
    if len(t) > 1 and np.all(np.diff(t) > 0):
        return pd.Series(np.gradient(alt, t), index=tr.index)
    return tr.vel_u


def _finite(v):
    return isinstance(v, (int, float)) and not isinstance(v, bool) and np.isfinite(v)


NOT_USABLE = "RESULTS NOT USABLE"
INCOMPLETE = "INCOMPLETE"
CAUTION = "CAUTION"
BROADSIDE_LIMIT = 10.0  # s; a ballistic run that flew longer broadside counts as fallen broadside (as the CLI)


def _runs(n, what):
    """"n what" with the plural s for n != 1."""
    return f"{n} {what}" + ("" if n == 1 else "s")


def _angle_range_line(j):
    r = j.get("aero_table_range") or {}
    if r.get("angles_in_range") is not False:
        return None
    n, a = r.get("angle_beyond"), r.get("max_abs_alpha_deg")
    detail = ", ".join(([f"{n} lookups"] if isinstance(n, int) else []) + ([f"max |alpha| {a:.0f} deg"] if _finite(a) else []))
    return "aero table angle range exceeded" + (f" ({detail})" if detail else "")


def _headline(bad, incomplete, caution):
    """(headline, lines): the worst level that has a line, and every line, worst first."""
    lines = bad + incomplete + caution
    head = NOT_USABLE if bad else INCOMPLETE if incomplete else CAUTION if caution else None
    return head, lines


def dispersion_usability(j):
    """Headline and reasons from dispersion_meta.json (wind grid) or dispersion_summary.json (Monte Carlo).

    RESULTS NOT USABLE: the nominal flight or runs are statically unstable or of unknown static stability, or the
    aero table's angle range was exceeded (the CLI exits with status 3); INCOMPLETE: runs failed or did not land;
    CAUTION: ballistic runs fell broadside. (None, []) when there is nothing to report or the file predates these keys.
    """
    bad, incomplete, caution = [], [], []
    if j.get("stable") is False:
        m = j.get("static_margin_min_cal")
        bad.append("nominal flight statically UNSTABLE" + (f" (min static margin {m:.2f} cal)" if _finite(m) else ""))
    elif "stable" in j and j["stable"] is None and j.get("stability_unknown"):
        bad.append(f"nominal static stability unknown: {j['stability_unknown']}")
    worst = j.get("unstable_min_static_margin_cal") or {}
    for mode, n in (j.get("unstable") or {}).items():
        if n:
            w = worst.get(mode)
            bad.append(f"{_runs(n, mode + ' run')} statically UNSTABLE" + (f" (min static margin down to {w:.2f} cal)" if _finite(w) else ""))
    for mode, n in (j.get("stability_unknown_runs") or {}).items():
        if n:
            bad.append(f"{_runs(n, mode + ' run')} with unknown static stability")
    angle = _angle_range_line(j)
    if angle:
        bad.append(angle)
    if isinstance(j.get("failed"), int) and j["failed"] > 0:
        incomplete.append(f"{j['failed']} of {j.get('cases', '?')} cases failed or did not land")
    for d in j.get("descents") or []:
        if d.get("failed"):
            incomplete.append(f"{d['failed']} of {j.get('samples', '?')} {d.get('descent', '')} samples failed or did not land")
    for mode, n in (j.get("broadside_runs") or {}).items():
        if n:
            caution.append(f"{_runs(n, mode + ' run')} fell broadside (large-angle aerodynamics, not validated)")
    return _headline(bad, incomplete, caution)


def trajectory_usability(s):
    """Headline and reasons from summary.json, as dispersion_usability: unstable or unknown static stability and an
    exceeded aero-table angle range are not usable, a flight that did not land is incomplete, a broadside ballistic
    descent a caution."""
    bad, incomplete, caution = [], [], []
    if s.get("stable") is False:
        m, t, mach = s.get("static_margin_min_cal"), s.get("static_margin_min_t"), s.get("static_margin_min_mach")
        at = f" at t = {t:.2f} s, Mach {mach:.2f}" if _finite(t) and _finite(mach) else ""
        bad.append("statically UNSTABLE" + (f" (min static margin {m:.2f} cal{at})" if _finite(m) else ""))
    elif "stable" in s and s["stable"] is None:
        bad.append(f"static stability could not be evaluated ({s.get('stability_unknown') or 'no margin evaluated'})")
    angle = _angle_range_line(s)
    if angle:
        bad.append(angle)
    if s.get("landed") is False:
        mt = s.get("max_time")
        incomplete.append("the flight did not land" + (f" within {mt:g} s" if _finite(mt) else ""))
    if s.get("fell_broadside") is True:
        bt = s.get("broadside_time")
        caution.append("the ballistic descent fell broadside" + (f" for {bt:.0f} s" if _finite(bt) else "") + " (large-angle aerodynamics, not validated)")
    return _headline(bad, incomplete, caution)


def draw_usability(fig, head, lines):
    """Prominent banner above the figure (nothing when head is None): red for RESULTS NOT USABLE, amber otherwise."""
    if head is None:
        return
    color = SERIES[7] if head == NOT_USABLE else "#b06d00"
    text = f"{head}: " + "\n".join(lines)
    fig.text(0.5, 1.0, text, ha="center", va="bottom", fontsize=11, fontweight="bold", color=color, transform=fig.transFigure,
             bbox={"boxstyle": "round,pad=0.5", "facecolor": SURFACE, "edgecolor": color, "linewidth": 2})


def run_flags(d):
    """Masks of the landed runs to mark: statically unstable, stability unknown (`stable` column) and fallen broadside
    (`broadside_time`, ballistic). All False for files without the columns."""
    none = pd.Series(False, index=d.index)
    if "stable" in d.columns:
        st = d["stable"].astype(str).str.strip().str.lower()
        unstable, unknown = st == "false", ~st.isin(["true", "false"])
    else:
        unstable, unknown = none, none
    broadside = none
    if "broadside_time" in d.columns:
        broadside = (d.descent == "ballistic") & (pd.to_numeric(d["broadside_time"], errors="coerce") > BROADSIDE_LIMIT)
    return unstable, unknown, broadside


def mark_runs(ax, s):
    """Overlays markers on the landing points of `s` that are statically unstable (red x), of unknown static stability
    (amber triangle) or fell broadside (hollow diamond)."""
    unstable, unknown, broadside = run_flags(s)
    for mask, kw, label in (
        (unstable, {"marker": "x", "color": SERIES[7], "s": 46, "linewidths": 2.0}, "statically unstable"),
        (unknown, {"marker": "^", "facecolors": "none", "edgecolors": "#b06d00", "s": 46, "linewidths": 1.6}, "stability unknown"),
        (broadside, {"marker": "D", "facecolors": "none", "edgecolors": SERIES[6], "s": 40, "linewidths": 1.4}, "fell broadside"),
    ):
        if mask.any():
            r = s[mask]
            ax.scatter(r.landing_east, r.landing_north, zorder=5, label=f"{label} ({int(mask.sum())})", **kw)


def read_json(f):
    """The JSON object in file f, or {} when it is missing or unreadable."""
    try:
        j = json.loads(f.read_text())
    except (OSError, ValueError):
        return {}
    return j if isinstance(j, dict) else {}


def ellipse_xy(mean_e, mean_n, semi_major, semi_minor, bearing_deg, n=200):
    """Closed ellipse in (east, north); the major axis points along a bearing clockwise from north."""
    th = np.radians(bearing_deg)
    u = np.array([np.sin(th), np.cos(th)])  # major axis (east, north)
    v = np.array([np.cos(th), -np.sin(th)])  # minor axis
    t = np.linspace(0.0, 2.0 * np.pi, n)
    pts = np.outer(np.cos(t) * semi_major, u) + np.outer(np.sin(t) * semi_minor, v)
    return mean_e + pts[:, 0], mean_n + pts[:, 1]


def plot_dispersion_mc(out):
    d = pd.read_csv(out / "dispersion_mc.csv")
    summ = json.loads((out / "dispersion_summary.json").read_text())
    stats = {s["descent"]: s for s in summ["descents"]}
    modes = [m for m in ("ballistic", "parachute") if m in stats]
    fig, axs = plt.subplots(1, len(modes), figsize=(6.4 * len(modes), 5.6), squeeze=False)
    for ax, mode in zip(axs[0], modes):
        s = d[(d.descent == mode) & (d.status == "ok")].dropna(subset=["landing_east", "landing_north"])
        st = stats[mode]
        ax.scatter(s.landing_east, s.landing_north, s=6, color=SERIES[0], alpha=0.35, linewidths=0, label=f"landing points (n = {st.get('n', len(s))})")
        mark_runs(ax, s)
        if st.get("mean_east") is not None and st.get("mean_north") is not None:
            for k, ls in ((1, "-"), (3, "--")):
                # With fewer than 2 samples the spread is undefined and the ellipses are null.
                e = st.get(f"ellipse_{k}sigma") or {}
                vals = [e.get(f) for f in ("semi_major", "semi_minor", "major_axis_bearing_deg")]
                if not all(_finite(v) for v in vals):
                    continue
                x, y = ellipse_xy(st["mean_east"], st["mean_north"], *vals)
                ax.plot(x, y, color=SERIES[1], linestyle=ls, linewidth=1.5, label=f"{k}σ ellipse")
            ax.plot([st["mean_east"]], [st["mean_north"]], marker="+", color=SERIES[1], markersize=11, markeredgewidth=2, linestyle="none", label="mean")
        ax.plot([0], [0], marker="*", color=INK, markersize=12, linestyle="none", label="launch site")
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlabel("East [m]")
        ax.set_ylabel("North [m]")
        failed = st.get("failed", 0)
        ax.set_title(f"Monte Carlo landing dispersion — {mode}" + (f" ({failed} failed or not landed)" if failed else ""))
        ax.legend(loc="best")
        lines = []
        if not st.get("n"):
            lines.append("no sample landed")
        far = st.get("max_landing_distance")
        if far is not None:
            ge = "≥ " if st.get("max_landing_distance_is_lower_bound") else ""
            lines.append(f"max distance {ge}{far:.0f} m")
        pct = [st.get(f"landing_distance_p{q}") for q in (50, 90, 99)]
        if all(v is not None for v in pct):
            p999 = st.get("landing_distance_p999")
            if p999 is not None:
                lines.append("P50 / P90 / P99 / P99.9: " + " / ".join(f"{v:.0f}" for v in pct + [p999]) + " m")
            else:
                lines.append("P50 / P90 / P99: " + " / ".join(f"{v:.0f}" for v in pct) + " m")
        if st.get("complete") is False:
            lines.append("statistics incomplete")
        if lines:
            ax.text(0.02, 0.02, "\n".join(lines), transform=ax.transAxes, fontsize=8, color=INK2, va="bottom")
    fig.tight_layout()
    draw_usability(fig, *dispersion_usability(summ))
    save(fig, out, "dispersion_mc.png")


def plot_dispersion(out):
    if (out / "dispersion_mc.csv").exists() and (out / "dispersion_summary.json").exists():
        plot_dispersion_mc(out)
    else:
        if (out / "dispersion_mc.csv").exists():
            print("skip dispersion_mc: dispersion_summary.json not found (run `ignisyeet dispersion` again)")
        remove_old_plots(out, ["dispersion_mc.png"])
    if (out / "dispersion.csv").exists():
        plot_dispersion_grid(out)
    else:
        remove_old_plots(out, ["dispersion.png"])


def profile_wind(out):
    """True when the outputs come from a wind.model = "profile" run.

    dispersion_meta.json, dispersion_summary.json and summary.json carry `wind_model`; the first file
    that has it decides. A file without it counts as a profile run when it carries `wind_scale`.
    """
    for name in ("dispersion_meta.json", "dispersion_summary.json", "summary.json"):
        f = out / name
        if f.exists():
            try:
                j = json.loads(f.read_text())
            except (OSError, ValueError):
                continue
            if not isinstance(j, dict):
                continue
            if "wind_model" in j:
                return j["wind_model"] == "profile"
            if "wind_scale" in j:
                return True
    return False


def grid_polyline(r):
    """East and north of the closed outline of one wind speed's landing points `r` (sorted by direction), with a
    break (NaN) at every case that fell broadside: such a landing lies far off the outline of its neighbours,
    and a line through it would cut across the figure. Broadside cases keep their markers."""
    _, _, broadside = run_flags(r)
    e = r.landing_east.where(~broadside).to_numpy(dtype=float)
    n = r.landing_north.where(~broadside).to_numpy(dtype=float)
    return np.append(e, e[:1]), np.append(n, n[:1])


def plot_dispersion_grid(out):
    d = pd.read_csv(out / "dispersion.csv")
    modes = [m for m in ("ballistic", "parachute") if (d.descent == m).any()]
    n_failed = {}
    if "landed" in d.columns:
        ok = d.landed.astype(str).str.lower().isin(["true", "1"]) & d.landing_east.notna()
        n_failed = d[~ok].descent.value_counts().to_dict()
        d = d[ok]
    fig, axs = plt.subplots(1, len(modes), figsize=(7.2 * len(modes), 6), squeeze=False)
    speeds = sorted(d.wind_speed.unique())
    colors = dict(zip(speeds, speed_colors(len(speeds))))
    for ax, mode in zip(axs[0], modes):
        s = d[d.descent == mode]
        for v in speeds:
            r = s[s.wind_speed == v].sort_values("wind_direction_deg")
            if r.empty:
                continue
            e, n = grid_polyline(r)
            ax.plot(e, n, "-o", color=colors[v], markersize=4, linewidth=1.5, label=f"{v:g} m/s")
            # the broadside cases the outline skips keep their speed's marker
            b = run_flags(r)[2]
            if b.any():
                ax.plot(r.landing_east[b], r.landing_north[b], "o", color=colors[v], markersize=4)
        ax.plot([0], [0], marker="*", color=INK, markersize=12, linestyle="none", label="launch site")
        n_speed = len(ax.get_legend_handles_labels()[1]) - 1  # speed curves drawn before the launch site
        mark_runs(ax, s)
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlabel("East [m]")
        ax.set_ylabel("North [m]")
        failed = n_failed.get(mode, 0)
        if s.empty:
            ax.set_title(f"Landing dispersion — {mode}: no case landed ({failed} failed)")
        else:
            far = s.landing_distance.max()
            ax.set_title(f"Landing dispersion — {mode} (max {'≥ ' if failed else ''}{far:.0f} m" + (f", {failed} did not land)" if failed else ")"))
        # Highest wind speed first. That curve is not necessarily the outermost one (a ballistic
        # landing can lie farthest out at an intermediate speed), so the legend orders by speed.
        handles, labels = ax.get_legend_handles_labels()
        # Speeds were drawn in ascending order; the launch site and the run markers follow them.
        order = list(range(n_speed))[::-1] + list(range(n_speed, len(labels)))
        title = ("profile mean speed" if profile_wind(out) else "wind speed") + ", highest first"
        ax.legend([handles[i] for i in order], [labels[i] for i in order], title=title, loc="upper left", bbox_to_anchor=(1.01, 1.0))
    fig.tight_layout()
    draw_usability(fig, *dispersion_usability(read_json(out / "dispersion_meta.json")))
    save(fig, out, "dispersion.png")


def plot_aero_compare(out, other):
    """Overlay CNα, Xcp and CD0 (power off) vs Mach of two aerodynamic tables."""
    fig, axs = plt.subplots(1, 3, figsize=(13, 3.8))
    for path, color, ls in ((out, SERIES[0], "-"), (other, SERIES[1], "--")):
        t = pd.read_csv(path / "aero_table.csv")
        if "beta_deg" in t.columns:
            raise SystemExit(f"{path / 'aero_table.csv'} is a general (alpha, beta) table; --compare needs axisymmetric tables")
        t = t[t.alpha_deg == 0].sort_values("mach")
        r = path.resolve()
        label = f"{r.parent.name}/{r.name}" if out.resolve().name == other.resolve().name else r.name
        axs[0].plot(t.mach, t.cna, color=color, linestyle=ls, label=label)
        axs[1].plot(t.mach, t.xcp, color=color, linestyle=ls, label=label)
        axs[2].plot(t.mach, t.ca_off, color=color, linestyle=ls, label=label)
    for ax, (title, ylabel) in zip(axs, (("Normal-force slope CNα", "CNα [1/rad]"), ("Centre of pressure (α → 0)", "Xcp aft of nose [m]"), ("Zero-lift drag (power off)", "CD0"))):
        ax.set_title(title)
        ax.set_xlabel("Mach")
        ax.set_ylabel(ylabel)
    axs[0].legend(loc="best")
    fig.tight_layout()
    save(fig, out, "aero_compare.png")


def last_sim_failure(out):
    """The error message when summary.json is the failure marker `{"failed": true, "error": ...}` of a failed sim run, else None."""
    f = out / "summary.json"
    if not f.exists():
        return None
    try:
        j = json.loads(f.read_text())
    except ValueError:
        return None
    return str(j.get("error", "unknown error")) if j.get("failed") is True else None


def stale_warnings(out):
    """Warnings (strings) for outputs that belong to different flight configurations.

    summary.json carries the flight configuration hash as ``config_hash``, dispersion_meta.json and
    dispersion_summary.json as ``flight_config_hash``. Every file that exists is compared with the others;
    a file that cannot be parsed or has no hash is reported.
    """
    hashes, msgs = {}, []
    for name, key in (("summary.json", "config_hash"), ("dispersion_meta.json", "flight_config_hash"), ("dispersion_summary.json", "flight_config_hash")):
        f = out / name
        if not f.exists():
            continue
        try:
            j = json.loads(f.read_text())
        except (OSError, ValueError) as e:
            msgs.append(f"warning: {f} cannot be read ({e}); rerun the command that writes it")
            continue
        h = j.get(key) if isinstance(j, dict) else None
        if h is None:
            if name != "summary.json":
                msgs.append(f"warning: {f} has no {key}; rerun `ignisyeet dispersion`")
        else:
            hashes[name] = h
    if len(set(hashes.values())) > 1:
        names = ", ".join(str(out / n) for n in hashes)
        msgs.append(f"warning: {names} were written for different flight configurations ({', '.join(f'{n}: {h}' for n, h in hashes.items())}); rerun `ignisyeet sim` and `ignisyeet dispersion` before reading the plots together")
    return msgs


def warn_stale(out):
    for m in stale_warnings(out):
        print(m, file=sys.stderr)


PLOTS = {"geometry": plot_geometry, "aero": plot_aero, "panel": plot_panel, "trajectory": plot_trajectory, "dispersion": plot_dispersion}
# Files each plot writes; a skipped plot removes them (an old figure must not pass for the current run).
PNGS = {"geometry": ["geometry.png"], "aero": ["aero.png"], "panel": ["panel.png"], "trajectory": ["trajectory_3d.png", "trajectory.png"], "dispersion": ["dispersion.png", "dispersion_mc.png"]}
REQUIRES = {"geometry": "geometry.json", "aero": "aero_table.csv", "panel": "panel_cp.csv", "trajectory": "trajectory.csv", "dispersion": ("dispersion.csv", "dispersion_mc.csv")}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("what", choices=["all", *PLOTS])
    ap.add_argument("out_dir", type=Path)
    ap.add_argument("--compare", type=Path, metavar="DIR", help="second output directory; overlays its aero_table.csv in aero_compare.png")
    a = ap.parse_args()
    if not a.out_dir.is_dir():
        sys.exit(f"error: output directory {a.out_dir} not found; run the ignisyeet commands first")
    failed = last_sim_failure(a.out_dir)
    if failed is not None and a.what == "trajectory":
        remove_old_plots(a.out_dir, PNGS["trajectory"])
        sys.exit(f"error: the last sim run failed: {failed}; not plotting the old trajectory")
    if failed is None:
        warn_stale(a.out_dir)
    names = list(PLOTS) if a.what == "all" else [a.what]
    plotted = 0
    for name in names:
        req = REQUIRES[name] if isinstance(REQUIRES[name], tuple) else (REQUIRES[name],)
        if name == "trajectory" and failed is not None:
            print(f"skip trajectory: the last sim run failed: {failed}; not plotting the old trajectory")
            remove_old_plots(a.out_dir, PNGS[name])
        elif any((a.out_dir / r).exists() for r in req):
            before = SAVED
            PLOTS[name](a.out_dir)
            if SAVED > before:
                plotted += 1
            else:
                remove_old_plots(a.out_dir, PNGS[name])
                if a.what != "all":
                    raise SystemExit(f"error: nothing to plot for {name} in {a.out_dir}")
            if name == "aero" and a.compare is not None:
                if not (a.compare / "aero_table.csv").exists():
                    raise SystemExit(f"{a.compare / 'aero_table.csv'} not found")
                plot_aero_compare(a.out_dir, a.compare)
        elif a.what != "all":
            remove_old_plots(a.out_dir, PNGS[name])
            raise SystemExit(f"none of {', '.join(req)} found in {a.out_dir}; run the matching ignisyeet command first")
        else:
            print(f"skip {name}: {' / '.join(req)} not found")
            remove_old_plots(a.out_dir, PNGS[name])
    if plotted == 0:
        sys.exit(f"error: nothing to plot in {a.out_dir}")


if __name__ == "__main__":
    main()
