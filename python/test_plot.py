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


if __name__ == "__main__":
    unittest.main()
