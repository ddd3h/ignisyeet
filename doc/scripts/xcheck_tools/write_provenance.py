# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Write doc/data/xcheck_tools/provenance.json: versions, the jar's SHA256 and the run date.

Usage: OPENROCKET_JAR=... python write_provenance.py <ignisyeet binary>
"""

import datetime
import hashlib
import json
import os
import platform
import subprocess
import sys
from importlib import metadata
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

JAR_URL = "https://github.com/openrocket/openrocket/releases/download/release-24.12/OpenRocket-24.12.jar"


def run(*cmd):
    r = subprocess.run(cmd, capture_output=True, text=True, cwd=common.ROOT)
    return (r.stdout + r.stderr).strip()


jar = Path(os.environ["OPENROCKET_JAR"])
java = run("java", "-version").splitlines()[0]
info = {
    "run_date": datetime.date.today().isoformat(),
    "ignisyeet": {"version": run(sys.argv[1], "--version").splitlines()[0], "repository_commit_at_run": run("git", "rev-parse", "--short", "HEAD")},
    "openrocket": {
        "version": "24.12",
        "jar": jar.name,
        "jar_url": JAR_URL,
        "jar_sha256": hashlib.sha256(jar.read_bytes()).hexdigest(),
        "java": java,
        "access": "JPype1 " + metadata.version("JPype1") + " against info.openrocket.core.* (the Python package orhelper is not used: it targets the pre-2022 package net.sf.openrocket)",
        "simulation": "Extended Barrowman aerodynamics, 6-DOF RK4, time step 0.01 s (0.002 s checked), Flat Earth, ISA atmosphere at the launch altitude",
    },
    "rocketpy": {"version": metadata.version("rocketpy"), "numpy": metadata.version("numpy"), "scipy": metadata.version("scipy"), "pandas": metadata.version("pandas"),
                 "simulation": "6-DOF, LSODA, rtol = atol = 1e-8"},
    "python": platform.python_version(),
    "platform": platform.platform(),
    "inputs": ["examples/sample.toml", "examples/sample_rocket.stl", "examples/sample_motor.eng"],
}
common.DATA.mkdir(parents=True, exist_ok=True)
(common.DATA / "provenance.json").write_text(json.dumps(info, indent=2, ensure_ascii=False) + "\n")
print(json.dumps(info, indent=2, ensure_ascii=False))
