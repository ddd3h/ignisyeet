# SPDX-License-Identifier: GPL-3.0-or-later
# Copyright (C) 2025-2026 西濱大将 (NISHIHAMA Daisuke)
"""Write the two drag-coefficient curves that RocketPy is fed with (doc/data/xcheck_tools/drag_*.csv).

Usage: python make_drag_inputs.py <work dir>
  drag_ign.csv  IgnisYeet: aero_drag.csv of the case "nowind" (cd_off, cd_on at alpha = 0, extra_cd = 0)
  drag_or.csv   OpenRocket: total drag coefficient at alpha = 0 (fins "rounded"); OpenRocket has no
                separate power-on curve, so cd_off = cd_on
"""

import sys
from pathlib import Path

import pandas as pd

sys.path.insert(0, str(Path(__file__).resolve().parent))
import common  # noqa: E402

work = Path(sys.argv[1])
common.DATA.mkdir(parents=True, exist_ok=True)
a = pd.read_csv(work / "ign" / "nowind" / "aero_drag.csv")
a[["mach", "cd_off", "cd_on"]].round(6).to_csv(common.DATA / "drag_ign.csv", index=False)
o = pd.read_csv(work / "or" / "or_static_rounded.csv")
o0 = pd.DataFrame({"mach": [0.0], "cd_off": [o.cd_total.iloc[0]], "cd_on": [o.cd_total.iloc[0]]})
o1 = pd.DataFrame({"mach": o.mach, "cd_off": o.cd_total, "cd_on": o.cd_total})
pd.concat([o0, o1]).round(6).to_csv(common.DATA / "drag_or.csv", index=False)
print("wrote", common.DATA / "drag_ign.csv", common.DATA / "drag_or.csv")
