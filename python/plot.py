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
"""

import argparse
import json
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


def seq_colors(n):
    """n evenly spaced steps of the sequential ramp, light to dark."""
    if n == 1:
        return [SEQ[len(SEQ) // 2]]
    return [SEQ[round(i * (len(SEQ) - 1) / (n - 1))] for i in range(n)]


def save(fig, out, name):
    path = out / "plots" / name
    path.parent.mkdir(parents=True, exist_ok=True)
    fig.savefig(path, dpi=150, bbox_inches="tight")
    plt.close(fig)
    print(f"wrote {path}")


def plot_geometry(out):
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


def plot_aero(out):
    d = pd.read_csv(out / "aero_drag.csv")
    t = pd.read_csv(out / "aero_table.csv")
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


def _tri(ax, x, y, c, norm, cmap, max_dy):
    """Smooth-shaded triangulated Cp map; triangles spanning gaps (fin footprints, seams) are masked."""
    import matplotlib.tri as mtri

    tri = mtri.Triangulation(x, y)
    yy = np.asarray(y)[tri.triangles]
    tri.set_mask(np.ptp(yy, axis=1) > max_dy)
    return ax.tripcolor(tri, np.asarray(c), cmap=cmap, norm=norm, shading="gouraud")


def plot_panel(out):
    """Surface Cp of the panel solution: unrolled body map and one fin (both sides)."""
    from matplotlib.colors import TwoSlopeNorm

    d = pd.read_csv(out / "panel_cp.csv")
    lim = max(float(np.percentile(np.abs(d.cp), 99)), 0.1)
    norm = TwoSlopeNorm(vmin=-lim, vcenter=0.0, vmax=lim)
    cmap = "RdBu_r"  # diverging, centred at Cp = 0: blue suction, red compression
    body, fin = d[d.part == "body"], d[d.part == "fin"]
    fig = plt.figure(figsize=(11, 6.8), layout="constrained")
    gs = fig.add_gridspec(2, 2, height_ratios=[1, 1.15])

    ax = fig.add_subplot(gs[0, :])
    th = np.degrees(np.arctan2(body.z, body.y))
    sc = _tri(ax, body.x.values, th.values, body.cp.values, norm, cmap, 25.0)
    ax.set_xlabel("x aft of nose tip [m]")
    ax.set_ylabel("θ [deg]  (y: 0°, z: 90°)")
    ax.set_yticks([-180, -90, 0, 90, 180])
    ax.set_title("Body panels, unrolled")
    ax.grid(False)

    # One fin: the one nearest the 45 deg azimuth, split by side of the fin plane.
    phi = np.arctan2(fin.z, fin.y)
    k = fin[np.abs(np.angle(np.exp(1j * (phi - np.radians(45))))) < np.radians(20)]
    ph = np.radians(45)
    side = k.ny * (-np.sin(ph)) + k.nz * np.cos(ph)
    rho = np.hypot(k.y, k.z)
    for col, (sel, name) in enumerate(((side > 0, "Fin, +side"), (side <= 0, "Fin, −side"))):
        a = fig.add_subplot(gs[1, col])
        _tri(a, k.x[sel].values, rho[sel].values, k.cp[sel].values, norm, cmap, 1e9)
        a.set_xlabel("x aft of nose tip [m]")
        a.set_ylabel("radial distance [m]")
        a.set_title(name)
        a.set_aspect("equal", adjustable="datalim")
        a.grid(False)
    cb = fig.colorbar(sc, ax=fig.axes, shrink=0.85, pad=0.02)
    cb.set_label("Cp")
    save(fig, out, "panel.png")


def plot_trajectory(out):
    tr = pd.read_csv(out / "trajectory.csv")
    summ = json.loads((out / "summary.json").read_text()) if (out / "summary.json").exists() else {}
    fig = plt.figure(figsize=(7, 6))
    ax = fig.add_subplot(projection="3d")
    for phase, c in (("rail", SERIES[2]), ("free", SERIES[0]), ("parachute", SERIES[1])):
        s = tr[tr.phase == phase]
        if len(s):
            ax.plot(s.east, s.north, s.up, color=c, label=phase)
    ax.scatter([0], [0], [0], color=INK, s=30, label="launch site")
    ax.set_xlabel("East [m]")
    ax.set_ylabel("North [m]")
    ax.set_zlabel("Up [m]")
    ax.set_title(f"Trajectory ({summ.get('descent', '')})")
    ax.legend(loc="upper left")
    save(fig, out, "trajectory_3d.png")

    # Rigid-body panels cover powered flight and coast; the parachute descent only adds a flat tail.
    body = tr[tr.phase != "parachute"]
    free = body[body.phase == "free"]
    asc = body[(body.vel_u >= 0) & (body.airspeed > 10)]
    panels = [
        ("up", "Altitude above launch site [m]", tr),
        ("airspeed", "Airspeed [m/s]", body),
        ("mach", "Mach", body),
        ("alpha_deg", "Angle of attack [deg]", free),
        ("stability_cal", "Static margin [cal] (ascent)", asc),
        ("dyn_pressure", "Dynamic pressure [Pa]", body),
        ("thrust", "Thrust [N]", body[body.t <= body.t[body.thrust > 0].max() * 1.2] if (body.thrust > 0).any() else body),
        ("mass", "Mass [kg]", body),
        ("pitch_deg", "Pitch [deg]", free),
    ]
    fig, axs = plt.subplots(3, 3, figsize=(12, 9), sharex=False)
    for ax, (col, title, data) in zip(axs.flat, panels):
        ax.plot(data.t, data[col], color=SERIES[0])
        ax.set_title(title)
        ax.set_xlabel("t [s]")
    if summ:
        a = axs[0, 0]
        a.plot([summ["apogee_time"]], [summ["apogee"]], "o", color=SERIES[0], markersize=8, markerfacecolor=SURFACE, markeredgewidth=2)
        a.annotate(f"apogee {summ['apogee']:.0f} m", (summ["apogee_time"], summ["apogee"]), xytext=(6, -12), textcoords="offset points", color=INK2)
    fig.tight_layout()
    save(fig, out, "trajectory.png")


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
        s = d[(d.descent == mode) & (d.status == "ok")]
        st = stats[mode]
        ax.scatter(s.landing_east, s.landing_north, s=6, color=SERIES[0], alpha=0.35, linewidths=0, label=f"landing points (n = {st['n']})")
        for k, ls in ((1, "-"), (3, "--")):
            e = st[f"ellipse_{k}sigma"]
            x, y = ellipse_xy(st["mean_east"], st["mean_north"], e["semi_major"], e["semi_minor"], e["major_axis_bearing_deg"])
            ax.plot(x, y, color=SERIES[1], linestyle=ls, linewidth=1.5, label=f"{k}σ ellipse")
        ax.plot([st["mean_east"]], [st["mean_north"]], marker="+", color=SERIES[1], markersize=11, markeredgewidth=2, linestyle="none", label="mean")
        ax.plot([0], [0], marker="*", color=INK, markersize=12, linestyle="none", label="launch site")
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlabel("East [m]")
        ax.set_ylabel("North [m]")
        ax.set_title(f"Monte Carlo landing dispersion — {mode} ({st['failed']} failed)")
        ax.legend(loc="best")
    fig.tight_layout()
    save(fig, out, "dispersion_mc.png")


def plot_dispersion(out):
    if (out / "dispersion_mc.csv").exists() and (out / "dispersion_summary.json").exists():
        plot_dispersion_mc(out)
    if (out / "dispersion.csv").exists():
        plot_dispersion_grid(out)


def plot_dispersion_grid(out):
    d = pd.read_csv(out / "dispersion.csv")
    modes = [m for m in ("ballistic", "parachute") if (d.descent == m).any()]
    fig, axs = plt.subplots(1, len(modes), figsize=(7.2 * len(modes), 6), squeeze=False)
    speeds = sorted(d.wind_speed.unique())
    colors = dict(zip(speeds, seq_colors(len(speeds))))
    for ax, mode in zip(axs[0], modes):
        s = d[d.descent == mode]
        for v in speeds:
            r = s[s.wind_speed == v].sort_values("wind_direction_deg")
            e = np.append(r.landing_east.values, r.landing_east.values[:1])
            n = np.append(r.landing_north.values, r.landing_north.values[:1])
            ax.plot(e, n, "-o", color=colors[v], markersize=4, linewidth=1.5, label=f"{v:g} m/s")
        ax.plot([0], [0], marker="*", color=INK, markersize=12, linestyle="none", label="launch site")
        ax.set_aspect("equal", adjustable="datalim")
        ax.set_xlabel("East [m]")
        ax.set_ylabel("North [m]")
        far = s.landing_distance.max()
        ax.set_title(f"Landing dispersion — {mode} (max {far:.0f} m)")
        ax.legend(title="wind speed", loc="upper left", bbox_to_anchor=(1.01, 1.0))
    fig.tight_layout()
    save(fig, out, "dispersion.png")


def plot_aero_compare(out, other):
    """Overlay CNα, Xcp and CD0 (power off) vs Mach of two aerodynamic tables."""
    fig, axs = plt.subplots(1, 3, figsize=(13, 3.8))
    for path, color, ls in ((out, SERIES[0], "-"), (other, SERIES[1], "--")):
        t = pd.read_csv(path / "aero_table.csv")
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


PLOTS = {"geometry": plot_geometry, "aero": plot_aero, "panel": plot_panel, "trajectory": plot_trajectory, "dispersion": plot_dispersion}
REQUIRES = {"geometry": "geometry.json", "aero": "aero_table.csv", "panel": "panel_cp.csv", "trajectory": "trajectory.csv", "dispersion": ("dispersion.csv", "dispersion_mc.csv")}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("what", choices=["all", *PLOTS])
    ap.add_argument("out_dir", type=Path)
    ap.add_argument("--compare", type=Path, metavar="DIR", help="second output directory; overlays its aero_table.csv in aero_compare.png")
    a = ap.parse_args()
    names = list(PLOTS) if a.what == "all" else [a.what]
    for name in names:
        req = REQUIRES[name] if isinstance(REQUIRES[name], tuple) else (REQUIRES[name],)
        if any((a.out_dir / r).exists() for r in req):
            PLOTS[name](a.out_dir)
            if name == "aero" and a.compare is not None:
                if not (a.compare / "aero_table.csv").exists():
                    raise SystemExit(f"{a.compare / 'aero_table.csv'} not found")
                plot_aero_compare(a.out_dir, a.compare)
        elif a.what != "all":
            raise SystemExit(f"none of {', '.join(req)} found in {a.out_dir}; run the matching ignisyeet command first")
        else:
            print(f"skip {name}: {' / '.join(req)} not found")


if __name__ == "__main__":
    main()
