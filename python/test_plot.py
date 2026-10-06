# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Tests of plot.py: uv run --project python python -m unittest discover -s python"""

import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np
import pandas as pd

HERE = Path(__file__).parent
sys.path.insert(0, str(HERE))
import plot  # noqa: E402

GEN_COLUMNS = "mach,alpha_deg,beta_deg,cx_on,cx_off,cy,cz,cl,cm,cn,cl_delta,clp,sq0,sq1,sq2,sr0,sr1,sr2"


def write_general_table(out, diameter=0.1):
    """A general (alpha, beta) table with CD0 = 0.4 (off) / 0.3 (on), CN_alpha = 3 /rad and Xcp = 0.7 m."""
    rows = []
    for mach in (0.3, 0.8):
        for beta in (0.0, 10.0):
            for alpha in (-2.0, 0.0, 2.0, 10.0):
                a = np.radians(alpha)
                cz = -3.0 * a
                cm = cz * 0.7 / diameter
                rows.append([mach, alpha, beta, -0.3, -0.4, 0.0, cz, 0.0, cm, 3.0 * a, 0, 0, 0, 0, 0, 0, 0, 0])
    pd.DataFrame(rows, columns=GEN_COLUMNS.split(",")).to_csv(out / "aero_table.csv", index=False)
    (out / "aero_table.json").write_text(json.dumps({"ref_diameter": diameter}))


class GeneralTable(unittest.TestCase):
    def test_totals_come_from_cx_cz_and_cm(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            write_general_table(out)
            t = plot.axisymmetric_view(pd.read_csv(out / "aero_table.csv"))
            d = plot.general_totals(t, out)
            self.assertEqual(list(d.mach), [0.3, 0.8])
            np.testing.assert_allclose(d.cd_off, 0.4)
            np.testing.assert_allclose(d.cd_on, 0.3)
            np.testing.assert_allclose(d.cna, 3.0)
            np.testing.assert_allclose(d.xcp, 0.7)

    def test_aero_plot_without_drag_file_handles_a_general_table(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            write_general_table(out)
            plot.plot_aero(out)
            self.assertTrue((out / "plots" / "aero.png").stat().st_size > 0)


class FailedRun(unittest.TestCase):
    def run_plot(self, out):
        return subprocess.run([sys.executable, str(HERE / "plot.py"), "trajectory", str(out)], capture_output=True, text=True)

    def test_failure_marker_is_reported_instead_of_plotting(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "summary.json").write_text(json.dumps({"failed": True, "error": "state diverged"}))
            self.assertEqual(plot.last_sim_failure(out), "state diverged")
            r = self.run_plot(out)
            self.assertNotEqual(r.returncode, 0)
            self.assertIn("the last sim run failed: state diverged; not plotting the old trajectory", r.stderr)

    def test_failed_run_removes_the_old_trajectory_plots(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "plots").mkdir()
            for name in ("trajectory.png", "trajectory_3d.png", "aero.png"):
                (out / "plots" / name).write_bytes(b"old")
            (out / "summary.json").write_text(json.dumps({"failed": True, "error": "state diverged"}))
            r = self.run_plot(out)
            self.assertNotEqual(r.returncode, 0)
            self.assertFalse((out / "plots" / "trajectory.png").exists())
            self.assertFalse((out / "plots" / "trajectory_3d.png").exists())
            self.assertTrue((out / "plots" / "aero.png").exists(), "plots of other kinds are kept")
            self.assertIn("removed", r.stdout)

    def test_a_normal_summary_is_not_a_failure(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertIsNone(plot.last_sim_failure(out))
            (out / "summary.json").write_text(json.dumps({"landed": True}))
            self.assertIsNone(plot.last_sim_failure(out))


def synthetic_panel_csv(out, fin_azimuths_deg, r_body=0.05, span=0.05):
    """Body panels on a cylinder plus fin panels (both sides) at the given azimuths [deg]."""
    rows = []
    for x in np.linspace(0.1, 1.0, 12):
        for th in np.radians(np.arange(-175, 180, 10)):
            rows.append(["body", x, r_body * np.cos(th), r_body * np.sin(th), np.cos(th), np.sin(th), -0.1 * np.cos(th)])
    for az in fin_azimuths_deg:
        a = np.radians(az)
        for x in np.linspace(0.8, 1.0, 5):
            for rr in np.linspace(r_body, r_body + span, 5):
                for sgn in (1, -1):
                    off = sgn * 0.002
                    y = rr * np.cos(a) - off * np.sin(a)
                    z = rr * np.sin(a) + off * np.cos(a)
                    rows.append(["fin", x, y, z, -sgn * np.sin(a), sgn * np.cos(a), 0.3 * sgn])
    pd.DataFrame(rows, columns=["part", "x", "y", "z", "ny", "nz", "cp"]).to_csv(out / "panel_cp.csv", index=False)


class PanelPlot(unittest.TestCase):
    def draw(self, azimuths):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            synthetic_panel_csv(out, azimuths)
            d = pd.read_csv(out / "panel_cp.csv")
            phi0 = plot.fin_azimuth(d[d.part == "fin"])
            plot.plot_panel(out)
            self.assertTrue((out / "plots" / "panel.png").stat().st_size > 0)
            return phi0

    def test_finless_rocket_draws_the_body_only(self):
        self.assertIsNone(self.draw([]))

    def test_eight_fins_at_22_5_and_67_5_degrees(self):
        az = [22.5 + 45 * i for i in range(8)]
        phi0 = self.draw(az)
        deg = np.degrees(phi0)
        self.assertLess(min(abs((deg - a + 180) % 360 - 180) for a in az), 1.0)

    def test_four_fins_at_45_degrees(self):
        self.assertAlmostEqual(np.degrees(self.draw([45, 135, -135, -45])) % 90, 45.0, places=3)


class FinSideQuads(unittest.TestCase):
    def grid(self):
        """Corners of a swept 4 x 3 panel grid on a uniform (affine) lattice and the centroids in file order."""
        i, j = np.meshgrid(np.arange(5), np.arange(4), indexing="ij")
        v = np.stack([1.3 + 0.02 * i + 0.01 * j, 0.05 + 0.03 * j], axis=-1)
        c = 0.25 * (v[:-1, :-1] + v[1:, :-1] + v[:-1, 1:] + v[1:, 1:])  # chord-major, root to tip
        return v, c.reshape(-1, 2)

    def test_uniform_grid_recovers_the_panel_corners(self):
        v, c = self.grid()
        q = plot.fin_side_quads(c[:, 0], c[:, 1])
        self.assertEqual(q.shape, (12, 4, 2))
        np.testing.assert_allclose(q[0], [v[0, 0], v[1, 0], v[1, 1], v[0, 1]], atol=1e-12)
        np.testing.assert_allclose(q[-1], [v[3, 2], v[4, 2], v[4, 3], v[3, 3]], atol=1e-12)
        np.testing.assert_allclose(q.mean(axis=1), c, atol=1e-12)

    def test_unordered_centroids_are_not_a_grid(self):
        _, c = self.grid()
        c = c[np.random.default_rng(1).permutation(len(c))]
        self.assertIsNone(plot.fin_side_quads(c[:, 0], c[:, 1]))


class Trajectory3d(unittest.TestCase):
    def test_equal_axes_and_no_legend_entry_for_a_short_rail(self):
        t = np.linspace(0, 100, 201)
        tr = pd.DataFrame({"t": t, "east": -5 * t, "north": -60 * t, "up": 4000 * np.sin(np.pi * t / 200)})
        tr["phase"] = np.where(t < 0.6, "rail", np.where(t < 50, "free", "parachute"))
        tr.loc[tr.phase == "rail", ["east", "north", "up"]] = [[0, 0, 0], [0.5, 0.5, 5.0]]
        fig = plot.plt.figure()
        ax = fig.add_subplot(projection="3d")
        plot.draw_path_3d(ax, tr)
        spans = [np.ptp(lim) for lim in (ax.get_xlim(), ax.get_ylim(), ax.get_zlim())]
        np.testing.assert_allclose(spans, spans[0])
        self.assertGreaterEqual(spans[0], 6000.0)  # north runs 0.5 .. -6000 m
        self.assertEqual(ax.get_zlim()[0], 0.0)
        self.assertEqual([h.get_label() for h in ax.get_legend_handles_labels()[0]], ["free", "parachute", "launch site"])
        plot.plt.close(fig)


class TrajectoryAltitude(unittest.TestCase):
    def frame(self):
        t = np.linspace(0, 10, 11)
        up = np.array([0, 10, 30, 60, 100, 120, 110, 80, 40, 10, -3.0])
        # ECEF: geodetic altitude differs from the tangent-plane coordinate by the downrange curvature drop
        alt = 20.0 + up + np.linspace(0, 31, 11)
        return pd.DataFrame({"t": t, "up": up, "alt": alt, "vel_u": np.gradient(up, t)})

    def test_height_is_alt_minus_launch_altitude(self):
        tr = self.frame()
        h = plot.altitude_above_launch(tr)
        self.assertEqual(h.iloc[0], 0.0)
        np.testing.assert_allclose(h, tr.alt - 20.0)
        self.assertGreater(abs(h.iloc[-1] - tr.up.iloc[-1]), 30.0)

    def test_vertical_velocity_follows_altitude(self):
        tr = self.frame()
        np.testing.assert_allclose(plot.vertical_velocity(tr), np.gradient(tr.alt, tr.t))
        tr.loc[3, "t"] = tr.t[2]  # time not strictly increasing: fall back to vel_u
        np.testing.assert_allclose(plot.vertical_velocity(tr), tr.vel_u)


class FlatEarthAltitude(unittest.TestCase):
    def frame(self, flat):
        t = np.linspace(0, 10, 11)
        drop = np.linspace(0, 16, 11)  # curvature drop over the downrange distance
        h = np.array([0, 10, 30, 60, 100, 120, 110, 80, 40, 10, 0.0])
        if flat:
            # Flat Earth: up is the integrated height (0 at touchdown); alt is converted from the tangent plane.
            up, alt = h, 20.0 + h + drop
        else:
            # ECEF: alt is the true height; the tangent-plane up falls below it.
            up, alt = h - drop, 20.0 + h
        return pd.DataFrame({"t": t, "up": up, "alt": alt, "vel_u": np.gradient(up, t)}), h

    def test_flat_earth_uses_up_and_vel_u(self):
        tr, h = self.frame(True)
        height, vv = plot.height_and_vertical_velocity(tr, {"earth_model": "flat"})
        np.testing.assert_allclose(height, h)
        np.testing.assert_allclose(vv, tr.vel_u)

    def test_ecef_uses_alt_minus_launch_altitude(self):
        tr, h = self.frame(False)
        height, vv = plot.height_and_vertical_velocity(tr, {"earth_model": "ecef"})
        np.testing.assert_allclose(height, h)
        np.testing.assert_allclose(vv, np.gradient(tr.alt, tr.t))

    def test_model_detected_from_the_touchdown_without_earth_model(self):
        self.assertTrue(plot.flat_earth(self.frame(True)[0], {"landed": True}))
        self.assertFalse(plot.flat_earth(self.frame(False)[0], {"landed": True}))
        # Nothing to tell from: the default earth.model (flat).
        self.assertTrue(plot.flat_earth(self.frame(False)[0], {"landed": False}))


class SkippedPlots(unittest.TestCase):
    def test_skipped_plot_removes_its_old_png_and_keeps_the_others(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "plots").mkdir()
            for name in ("dispersion.png", "dispersion_mc.png", "geometry.png"):
                (out / "plots" / name).write_bytes(b"old")
            # geometry.json without profile.csv: the geometry plot is skipped.
            (out / "geometry.json").write_text("{}")
            r = subprocess.run([sys.executable, str(HERE / "plot.py"), "all", str(out)], capture_output=True, text=True)
            self.assertNotEqual(r.returncode, 0)  # nothing plotted at all
            for name in ("dispersion.png", "dispersion_mc.png", "geometry.png"):
                self.assertFalse((out / "plots" / name).exists(), name)
            self.assertIn("plot skipped", r.stdout)

    def test_grid_only_removes_the_old_monte_carlo_png(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            (out / "plots").mkdir()
            (out / "plots" / "dispersion_mc.png").write_bytes(b"old")
            rows = [["ballistic", v, d, 100.0 * v * np.sin(np.radians(d)), 100.0 * v * np.cos(np.radians(d)), 100.0 * v, True] for v in (1.0, 2.0) for d in (0, 90, 180, 270)]
            pd.DataFrame(rows, columns=["descent", "wind_speed", "wind_direction_deg", "landing_east", "landing_north", "landing_distance", "landed"]).to_csv(out / "dispersion.csv", index=False)
            plot.plot_dispersion(out)
            self.assertTrue((out / "plots" / "dispersion.png").stat().st_size > 3)
            self.assertFalse((out / "plots" / "dispersion_mc.png").exists())


class ProfileWind(unittest.TestCase):
    def write(self, out, name, obj):
        (out / name).write_text(json.dumps(obj))

    def test_dispersion_only_output_uses_wind_model(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.assertFalse(plot.profile_wind(out))
            self.write(out, "dispersion_meta.json", {"wind_model": "profile", "wind_scale": 1.0})
            self.assertTrue(plot.profile_wind(out))
            self.write(out, "dispersion_meta.json", {"wind_model": "power"})
            # the study that wrote dispersion.csv decides, not a summary.json of another run
            self.write(out, "summary.json", {"wind_model": "profile", "wind_scale": 1.0})
            self.assertFalse(plot.profile_wind(out))

    def test_monte_carlo_summary_and_older_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, "dispersion_summary.json", {"wind_model": "profile"})
            self.assertTrue(plot.profile_wind(out))
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, "dispersion_meta.json", {"cases": 1})
            self.write(out, "summary.json", {"wind_scale": 2.0})
            self.assertTrue(plot.profile_wind(out))

    def test_grid_legend_title(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            rows = []
            for v in (5.0, 10.0):
                for d in (0.0, 90.0, 180.0, 270.0):
                    rows.append({"descent": "parachute", "wind_speed": v, "wind_direction_deg": d, "landing_east": v * np.sin(np.radians(d)), "landing_north": v * np.cos(np.radians(d)), "landing_distance": v, "apogee": 100.0, "landed": True})
            pd.DataFrame(rows).to_csv(out / "dispersion.csv", index=False)
            self.write(out, "dispersion_meta.json", {"wind_model": "profile", "wind_scale": 1.0})
            titles, labels = [], []
            orig = plot.plt.Axes.legend

            def spy(ax, *a, **k):
                titles.append(k.get("title"))
                labels.append(list(a[1]) if len(a) > 1 else None)
                return orig(ax, *a, **k)

            plot.plt.Axes.legend = spy
            try:
                plot.plot_dispersion_grid(out)
            finally:
                plot.plt.Axes.legend = orig
            self.assertEqual(titles, ["profile mean speed, highest first"])
            self.assertEqual(labels, [["10 m/s", "5 m/s", "launch site"]])


class StaleOutputs(unittest.TestCase):
    def write(self, out, name, obj):
        (out / name).write_text(obj if isinstance(obj, str) else json.dumps(obj))

    def test_matching_hashes_are_silent(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, "summary.json", {"config_hash": "a"})
            self.write(out, "dispersion_meta.json", {"flight_config_hash": "a"})
            self.write(out, "dispersion_summary.json", {"flight_config_hash": "a"})
            self.assertEqual(plot.stale_warnings(out), [])

    def test_sidecars_are_compared_without_summary(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, "dispersion_meta.json", {"flight_config_hash": "new"})
            self.write(out, "dispersion_summary.json", {"flight_config_hash": "old"})
            w = plot.stale_warnings(out)
            self.assertEqual(len(w), 1)
            self.assertIn("dispersion_meta.json", w[0])
            self.assertIn("dispersion_summary.json", w[0])

    def test_summary_mismatch_names_files(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, "summary.json", {"config_hash": "a"})
            self.write(out, "dispersion_meta.json", {"flight_config_hash": "b"})
            w = plot.stale_warnings(out)
            self.assertEqual(len(w), 1)
            self.assertIn("summary.json", w[0])

    def test_truncated_json_warns_instead_of_failing(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.write(out, "summary.json", '{"config_hash": "a"')
            self.write(out, "dispersion_meta.json", {"flight_config_hash": "a"})
            w = plot.stale_warnings(out)
            self.assertEqual(len(w), 1)
            self.assertIn("summary.json", w[0])


def capture_figures(fn, *args):
    """Runs a plot function and returns {file name: figure} of the figures it saves (not written)."""
    figs = {}
    orig = plot.save

    def keep(fig, out, name):
        figs[name] = fig

    plot.save = keep
    try:
        fn(*args)
    finally:
        plot.save = orig
    return figs


def banner(fig):
    return "\n".join(t.get_text() for t in fig.texts)


def labels(fig):
    return [lbl for ax in fig.axes for lbl in ax.get_legend_handles_labels()[1]]


class Usability(unittest.TestCase):
    CLEAN = {"stable": True, "unstable": {"ballistic": 0}, "stability_unknown_runs": {"ballistic": 0}, "broadside_runs": {"ballistic": 0}, "aero_table_range": {"angles_in_range": True}}

    def test_clean_dispersion_has_no_banner(self):
        self.assertEqual(plot.dispersion_usability({**self.CLEAN, "failed": 0}), (None, []))
        self.assertEqual(plot.dispersion_usability({}), (None, []))

    def test_unusable_dispersion(self):
        j = {
            **self.CLEAN,
            "stable": False,
            "static_margin_min_cal": -0.31,
            "unstable": {"ballistic": 3, "parachute": 0},
            "unstable_min_static_margin_cal": {"ballistic": -0.52, "parachute": 1.2},
            "stability_unknown_runs": {"ballistic": 1, "parachute": 0},
            "aero_table_range": {"angles_in_range": False, "angle_beyond": 12, "max_abs_alpha_deg": 97.0},
            "failed": 2,
            "cases": 40,
        }
        head, lines = plot.dispersion_usability(j)
        self.assertEqual(head, "RESULTS NOT USABLE")
        self.assertEqual(
            lines,
            [
                "nominal flight statically UNSTABLE (min static margin -0.31 cal)",
                "3 ballistic runs statically UNSTABLE (min static margin down to -0.52 cal)",
                "1 ballistic run with unknown static stability",
                "aero table angle range exceeded (12 lookups, max |alpha| 97 deg)",
                "2 of 40 cases failed or did not land",
            ],
        )
        head, lines = plot.dispersion_usability({**self.CLEAN, "stable": None, "stability_unknown": "the yaw normal-force slope is zero at Mach 0.30"})
        self.assertEqual((head, lines), ("RESULTS NOT USABLE", ["nominal static stability unknown: the yaw normal-force slope is zero at Mach 0.30"]))

    def test_incomplete_and_broadside(self):
        mc = {**self.CLEAN, "samples": 100, "descents": [{"descent": "ballistic", "failed": 4}, {"descent": "parachute", "failed": 0}]}
        self.assertEqual(plot.dispersion_usability(mc), ("INCOMPLETE", ["4 of 100 ballistic samples failed or did not land"]))
        self.assertEqual(
            plot.dispersion_usability({**self.CLEAN, "broadside_runs": {"ballistic": 2}}),
            ("CAUTION", ["2 ballistic runs fell broadside (large-angle aerodynamics, not validated)"]),
        )

    def test_trajectory_usability(self):
        ok = {"landed": True, "stable": True, "fell_broadside": False, "aero_table_range": {"angles_in_range": True}}
        self.assertEqual(plot.trajectory_usability(ok), (None, []))
        self.assertEqual(plot.trajectory_usability({}), (None, []))
        bad = {**ok, "stable": False, "static_margin_min_cal": -0.2, "static_margin_min_t": 1.5, "static_margin_min_mach": 0.4}
        self.assertEqual(plot.trajectory_usability(bad), ("RESULTS NOT USABLE", ["statically UNSTABLE (min static margin -0.20 cal at t = 1.50 s, Mach 0.40)"]))
        unk = {**ok, "stable": None, "stability_unknown": "no margin"}
        self.assertEqual(plot.trajectory_usability(unk), ("RESULTS NOT USABLE", ["static stability could not be evaluated (no margin)"]))
        ang = {**ok, "aero_table_range": {"angles_in_range": False, "angle_beyond": 5, "max_abs_alpha_deg": 120.0}}
        self.assertEqual(plot.trajectory_usability(ang)[0], "RESULTS NOT USABLE")
        self.assertEqual(plot.trajectory_usability({**ok, "landed": False, "max_time": 300})[0], "INCOMPLETE")
        self.assertEqual(
            plot.trajectory_usability({**ok, "fell_broadside": True, "broadside_time": 42.0}),
            ("CAUTION", ["the ballistic descent fell broadside for 42 s (large-angle aerodynamics, not validated)"]),
        )

    def mc_csv(self, out):
        rows = []
        for i, (stable, broad) in enumerate([("true", 0.0), ("true", 0.0), ("false", 0.0), ("", 0.0), ("true", 30.0), ("true", 0.0)]):
            rows.append({"sample": i, "descent": "ballistic", "status": "ok", "landing_east": 10.0 * i, "landing_north": 5.0 * i, "stable": stable, "broadside_time": broad})
        rows.append({"sample": 6, "descent": "ballistic", "status": "failed", "landing_east": None, "landing_north": None, "stable": "", "broadside_time": None})
        pd.DataFrame(rows).to_csv(out / "dispersion_mc.csv", index=False)

    def test_monte_carlo_plot_marks_samples_and_shows_the_banner(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.mc_csv(out)
            summ = {
                **self.CLEAN,
                "samples": 7,
                "unstable": {"ballistic": 1},
                "unstable_min_static_margin_cal": {"ballistic": -0.4},
                "stability_unknown_runs": {"ballistic": 1},
                "broadside_runs": {"ballistic": 1},
                "descents": [{"descent": "ballistic", "n": 6, "failed": 1, "mean_east": 25.0, "mean_north": 12.5}],
            }
            (out / "dispersion_summary.json").write_text(json.dumps(summ))
            fig = capture_figures(plot.plot_dispersion_mc, out)["dispersion_mc.png"]
            text = banner(fig)
            self.assertIn("RESULTS NOT USABLE", text)
            self.assertIn("1 ballistic run statically UNSTABLE", text)
            self.assertIn("1 of 7 ballistic samples failed or did not land", text)
            lbl = labels(fig)
            self.assertIn("statically unstable (1)", lbl)
            self.assertIn("stability unknown (1)", lbl)
            self.assertIn("fell broadside (1)", lbl)

    def test_grid_plot_marks_cases_and_shows_the_banner(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            rows = []
            for v in (5.0, 10.0):
                for d in (0.0, 90.0, 180.0, 270.0):
                    st = "false" if (v, d) == (10.0, 90.0) else "true"
                    rows.append({"descent": "ballistic", "wind_speed": v, "wind_direction_deg": d, "landing_east": v * np.sin(np.radians(d)), "landing_north": v * np.cos(np.radians(d)), "landing_distance": v, "landed": True, "stable": st, "broadside_time": 0.0})
            pd.DataFrame(rows).to_csv(out / "dispersion.csv", index=False)
            meta = {**self.CLEAN, "cases": 8, "failed": 0, "unstable": {"ballistic": 1}, "unstable_min_static_margin_cal": {"ballistic": -0.1}}
            (out / "dispersion_meta.json").write_text(json.dumps(meta))
            fig = capture_figures(plot.plot_dispersion_grid, out)["dispersion.png"]
            self.assertIn("RESULTS NOT USABLE: 1 ballistic run statically UNSTABLE", banner(fig).replace("\n", " "))
            self.assertIn("statically unstable (1)", labels(fig))
            # A clean study draws no banner and no extra legend entries.
            pd.DataFrame([{**r, "stable": "true"} for r in rows]).to_csv(out / "dispersion.csv", index=False)
            (out / "dispersion_meta.json").write_text(json.dumps({**self.CLEAN, "cases": 8, "failed": 0}))
            fig = capture_figures(plot.plot_dispersion_grid, out)["dispersion.png"]
            self.assertEqual(banner(fig), "")
            self.assertFalse(any("unstable" in x for x in labels(fig)))

    def test_monte_carlo_plot_shows_the_p999(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            self.mc_csv(out)
            st = {"descent": "ballistic", "n": 6, "failed": 1, "mean_east": 25.0, "mean_north": 12.5, "max_landing_distance": 60.0,
                  "landing_distance_p50": 20.0, "landing_distance_p90": 40.0, "landing_distance_p99": 50.0, "landing_distance_p999": 55.0}
            (out / "dispersion_summary.json").write_text(json.dumps({**self.CLEAN, "samples": 7, "descents": [st]}))
            fig = capture_figures(plot.plot_dispersion_mc, out)["dispersion_mc.png"]
            text = "\n".join(t.get_text() for ax in fig.axes for t in ax.texts)
            self.assertIn("P50 / P90 / P99 / P99.9: 20 / 40 / 50 / 55 m", text)

    def test_grid_outline_breaks_at_broadside_cases(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            rows = []
            for d in (0.0, 90.0, 180.0, 270.0):
                far = d == 90.0
                e, n = (3000.0, 0.0) if far else (100.0 * np.sin(np.radians(d)), 100.0 * np.cos(np.radians(d)))
                rows.append({"descent": "ballistic", "wind_speed": 4.0, "wind_direction_deg": d, "landing_east": e, "landing_north": n, "landing_distance": np.hypot(e, n), "landed": True, "stable": "true", "broadside_time": 140.0 if far else 0.0})
            df = pd.DataFrame(rows)
            e, n = plot.grid_polyline(df)
            self.assertTrue(np.isnan(e[1]) and np.isnan(n[1]))
            self.assertEqual(int(np.isnan(e).sum()), 1)
            self.assertEqual((e[0], e[-1]), (e[0], e[0]))  # closed outline
            df.to_csv(out / "dispersion.csv", index=False)
            (out / "dispersion_meta.json").write_text(json.dumps({**self.CLEAN, "cases": 4, "failed": 0}))
            fig = capture_figures(plot.plot_dispersion_grid, out)["dispersion.png"]
            ax = fig.axes[0]
            # no line segment reaches the broadside landing; its marker is still drawn
            for line in ax.get_lines():
                if line.get_linestyle() == "-":
                    x = np.asarray(line.get_xdata(), dtype=float)
                    self.assertFalse(np.any(x == 3000.0))
            self.assertTrue(any(np.any(np.asarray(l.get_xdata(), dtype=float) == 3000.0) for l in ax.get_lines() if l.get_marker() == "o"))
            self.assertIn("fell broadside (1)", labels(fig))

    def test_stability_panel_is_the_cp_cg_separation_off_the_rail(self):
        src = (HERE / "plot.py").read_text()
        self.assertIn("CP–CG at flown α [cal]", src)
        self.assertNotIn('"Static margin [cal] (ascent)"', src)

    def test_trajectory_plots_show_the_banner(self):
        with tempfile.TemporaryDirectory() as tmp:
            out = Path(tmp)
            t = np.linspace(0.0, 10.0, 21)
            up = 50.0 * t - 5.0 * t * t
            zero = 0.0 * t
            cols = {
                "t": t,
                "phase": ["rail"] + ["free"] * 20,
                "east": zero,
                "north": zero,
                "up": up,
                "alt": up,
                "vel_u": 50.0 - 10.0 * t,
                "airspeed": 20.0 + zero,
                "mach": 0.1 + zero,
                "alpha_deg": zero,
                "stability_cal": 1.0 + zero,
                "dyn_pressure": zero,
                "thrust": zero,
                "mass": 1.0 + zero,
                "pitch_deg": zero,
            }
            pd.DataFrame(cols).to_csv(out / "trajectory.csv", index=False)
            (out / "summary.json").write_text(json.dumps({"landed": True, "stable": False, "static_margin_min_cal": -0.5, "earth_model": "flat", "apogee": 125.0, "apogee_time": 5.0}))
            figs = capture_figures(plot.plot_trajectory, out)
            for name in ("trajectory.png", "trajectory_3d.png"):
                self.assertIn("RESULTS NOT USABLE: statically UNSTABLE (min static margin -0.50 cal)", banner(figs[name]).replace("\n", " "), name)


if __name__ == "__main__":
    unittest.main()
