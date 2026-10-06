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


if __name__ == "__main__":
    unittest.main()
